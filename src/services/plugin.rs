use crate::config::{
    AppConfig, DefaultAppConfig, DefaultGdmConfig, DefaultGodotConfig, GdmConfig, GodotConfig,
};
use crate::models::{Asset, Plugin, PluginSource};
use crate::services::{
    AssetStoreService, DefaultAssetStoreService, DefaultFileService, DefaultInstallService,
    FileService, InstallService,
};
use crate::ui::{Operation, OperationManager};
use crate::utils::Utils;

use anyhow::{Context, Result, bail};
use futures::future::try_join_all;
use std::collections::{BTreeMap, HashSet};
use std::path::{Component, Path};
use std::sync::Arc;
use tracing::info;

pub struct DefaultPluginService {
    pub godot_config: Box<dyn GodotConfig>,
    pub gdm_config: Box<dyn GdmConfig>,
    pub app_config: DefaultAppConfig,
    pub file_service: Arc<dyn FileService + Send + Sync>,
    pub asset_store_service: Arc<dyn AssetStoreService + Send + Sync>,
    pub install_service: Arc<dyn InstallService + Send + Sync>,
}

impl Default for DefaultPluginService {
    fn default() -> Self {
        let asset_store_service = Arc::new(DefaultAssetStoreService::default());
        let file_service = Arc::new(DefaultFileService);
        let install_service = Arc::new(DefaultInstallService::default());

        // Create app config for staging service
        let app_config = DefaultAppConfig::default();

        Self {
            godot_config: Box::new(DefaultGodotConfig::default()),
            gdm_config: Box::new(DefaultGdmConfig::default()),
            app_config,
            file_service,
            asset_store_service,
            install_service,
        }
    }
}

impl DefaultPluginService {
    #[allow(unused)]
    pub fn new(
        godot_config: Box<dyn GodotConfig>,
        gdm_config: Box<dyn GdmConfig>,
        app_config: DefaultAppConfig,
        file_service: Arc<dyn FileService + Send + Sync>,
        asset_store_service: Arc<dyn AssetStoreService + Send + Sync>,
        install_service: Arc<dyn InstallService + Send + Sync>,
    ) -> Self {
        Self {
            godot_config,
            gdm_config,
            app_config,
            file_service,
            asset_store_service,
            install_service,
        }
    }

    fn resolve_godot_version(&self, version_opt: Option<&str>) -> Result<String> {
        match version_opt {
            Some(v) => Ok(v.to_string()),
            None => self.godot_config.get_godot_version_from_project(),
        }
    }

    async fn install_asset_plugin(&self, asset: Asset) -> Result<()> {
        let plugin = Plugin::from(asset);
        let installed = self.process_install(&[plugin]).await?;
        self.add_plugins(&installed)?;
        info!(
            "Dependencies installed successfully: {:?}",
            installed.keys().collect::<Vec<_>>()
        );
        Ok(())
    }
}

fn parse_git_url_slugs(git_url: &str) -> Result<(String, String)> {
    // Normalize: strip .git suffix
    let url = git_url.trim().trim_end_matches(".git");

    let path = if let Ok(parsed_url) = url::Url::parse(url) {
        parsed_url.path().to_string()
    } else if let Some((_, path)) = url.split_once(':') {
        path.to_string()
    } else {
        url.to_string()
    };

    // Extract path segments
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if segments.len() >= 2 {
        let developer = segments[segments.len() - 2].to_lowercase();
        let repository = segments[segments.len() - 1]
            .trim_end_matches(".git")
            .to_lowercase();
        Ok((developer, repository))
    } else {
        bail!("Git URL must include both a publisher and repository name")
    }
}

fn validate_plugin_folder_name(folder_name: &str) -> Result<()> {
    let path = Path::new(folder_name);
    let valid = !folder_name.is_empty()
        && !folder_name.contains(['/', '\\'])
        && !path.is_absolute()
        && path.components().count() == 1
        && matches!(path.components().next(), Some(Component::Normal(_)));

    if !valid {
        bail!("Invalid dependency folder name: {folder_name}");
    }

    Ok(())
}

impl PluginService for DefaultPluginService {
    async fn process_install(&self, plugins: &[Plugin]) -> Result<BTreeMap<String, Plugin>> {
        let operation_manager = Arc::new(OperationManager::new(Operation::Install)?);

        let results = self
            .install_service
            .install(plugins, operation_manager.clone())
            .await?;

        operation_manager.finish();

        self.finish_plugins_operation(&results)?;

        Ok(results)
    }

    fn finish_plugins_operation(&self, plugins: &BTreeMap<String, Plugin>) -> Result<()> {
        if plugins.is_empty() {
            return Ok(());
        }

        let operation_manager = OperationManager::new(Operation::Finished)?;
        for (index, plugin) in plugins.values().enumerate() {
            let finished_bar = operation_manager.add_progress_bar(
                index,
                plugins.len(),
                &plugin.title,
                &plugin.get_version(),
            )?;
            finished_bar.finish();
        }
        operation_manager.finish();
        info!(
            "Finished processing {} dependencies successfully",
            plugins.len()
        );
        Ok(())
    }

    async fn install_all_plugins(&self) -> Result<BTreeMap<String, Plugin>> {
        if !self.gdm_config.has_dependencies()? {
            bail!("No dependencies installed.");
        }

        let all_plugins_map = self.gdm_config.get_dependencies()?;
        let all_plugins: Vec<Plugin> = all_plugins_map.values().cloned().collect();

        let installed_plugins = self.process_install(&all_plugins).await?;

        self.add_plugins(&installed_plugins)?;
        info!("All dependencies installed successfully");
        Ok(installed_plugins)
    }

    async fn add_plugin_by_name_and_version_and_godot_version(
        &self,
        name: &str,
        version: Option<&str>,
        godot_version: Option<&str>,
    ) -> Result<()> {
        let resolved_godot_version = self.resolve_godot_version(godot_version)?;

        let assets = self
            .asset_store_service
            .search_assets_by_name(name, version, Some(&resolved_godot_version))
            .await?;

        if assets.is_empty() {
            bail!("No dependencies found with name: {}", name)
        }

        if assets.len() > 1 {
            bail!("Found too many dependencies with name: {}", name)
        }

        let asset = assets
            .into_iter()
            .next()
            .context("Exact dependency lookup returned no dependency")?;
        self.install_asset_plugin(asset).await
    }

    async fn add_plugin_by_publisher_and_asset_slug_and_version_and_godot_version(
        &self,
        publisher_slug: Option<&str>,
        asset_slug: Option<&str>,
        version: Option<&str>,
        godot_version: Option<&str>,
    ) -> Result<()> {
        let resolved_godot_version = self.resolve_godot_version(godot_version)?;
        let asset_slug = asset_slug.unwrap_or_default();
        let publisher_slug = publisher_slug.unwrap_or_default();

        if asset_slug.is_empty() || publisher_slug.is_empty() {
            bail!(
                "Need to specify publisher_slug/asset_slug or --publisher_slug <slug> and --asset_slug <slug>"
            )
        }

        let asset_response = self
            .asset_store_service
            .get_asset(
                publisher_slug,
                asset_slug,
                version,
                Some(&resolved_godot_version),
            )
            .await?;

        let plugin = self
            .gdm_config
            .get_dependency_by_asset_slug(&asset_response.asset_slug)?;

        if let Some(existing) = plugin {
            let new_plugin = Plugin::from(asset_response.clone());
            if new_plugin != existing {
                println!(
                    "Updating dependency '{}' from {} to {}",
                    existing.title,
                    existing.get_version(),
                    new_plugin.get_version()
                );
            } else {
                println!(
                    "Dependency '{}' is already in dependencies.",
                    existing.title
                );
            }
        }

        self.install_asset_plugin(asset_response).await
    }

    async fn add_plugin_by_git_url_and_reference(
        &self,
        git_url: Option<&str>,
        git_reference: Option<&str>,
    ) -> Result<()> {
        let git_url = git_url.ok_or_else(|| anyhow::anyhow!("Git URL must be provided."))?;
        let reference = git_reference.unwrap_or("main");

        if git_url.is_empty() {
            bail!("Git URL must be provided.")
        }

        let (publisher_slug, asset_slug) = parse_git_url_slugs(git_url)?;

        let plugin = Plugin {
            source: PluginSource::Git {
                url: git_url.to_string(),
                reference: reference.to_string(),
                publisher_slug: publisher_slug.clone(),
                asset_slug: asset_slug.clone(),
            },
            plugin_cfg_path: None,
            title: String::new(),
            version: String::new(),
            sub_assets: Vec::new(),
            license: None,
        };

        let installed = self.process_install(&[plugin]).await?;

        self.add_plugins(&installed)?;

        info!(
            "Dependencies installed successfully: {:?}",
            installed.keys().collect::<Vec<_>>()
        );
        Ok(())
    }

    fn add_plugins(&self, plugins: &BTreeMap<String, Plugin>) -> Result<()> {
        let plugin_config = self.gdm_config.add_dependencies(plugins)?;
        info!("Adding dependencies {:?}", plugin_config);
        self.godot_config.save(plugin_config)?;
        info!(
            "Added {} dependencies to configuration successfully",
            plugins.len()
        );
        Ok(())
    }

    /// Removes a plugin by its configuration key (folder name) from the configuration and deletes its folder.
    async fn remove_plugin_by_config_key(&self, config_key: &str) -> Result<()> {
        if !self.gdm_config.has_dependencies()? {
            bail!("No dependencies installed.");
        }

        let plugin = self.gdm_config.get_dependency_by_config_key(config_key)?;
        let addon_folder = self.app_config.get_addon_folder_path();

        match plugin {
            Some(plugin) => {
                validate_plugin_folder_name(config_key)?;
                for sub_asset in &plugin.sub_assets {
                    validate_plugin_folder_name(sub_asset)?;
                }

                let installed_plugins = self.gdm_config.get_dependencies()?;
                let referenced_folders: std::collections::HashSet<&str> = installed_plugins
                    .iter()
                    .filter(|(key, _)| key.as_str() != config_key)
                    .flat_map(|(key, plugin)| {
                        std::iter::once(key.as_str())
                            .chain(plugin.sub_assets.iter().map(String::as_str))
                    })
                    .collect();

                let folders_to_remove = std::iter::once(config_key)
                    .chain(plugin.sub_assets.iter().map(String::as_str))
                    .filter(|folder| !referenced_folders.contains(folder))
                    .collect::<std::collections::HashSet<_>>();
                let main_folder_path =
                    Utils::plugin_name_to_addon_folder_path(&addon_folder, Path::new(config_key));
                let main_folder_exists = self.file_service.directory_exists(&main_folder_path);

                for folder in folders_to_remove {
                    if folder == config_key && !main_folder_exists {
                        continue;
                    }
                    let folder_path =
                        Utils::plugin_name_to_addon_folder_path(&addon_folder, Path::new(folder));
                    if self.file_service.directory_exists(&folder_path) {
                        if folder == config_key {
                            println!("Removing dependency folder: {}", folder_path.display());
                        } else {
                            println!("Removing sub-dependency folder: {}", folder_path.display());
                        }
                        self.file_service.remove_dir_all(&folder_path)?
                    }
                }

                if !main_folder_exists {
                    println!("Dependency folder does not exist, removing from config only.");
                }

                let plugin_config = self
                    .gdm_config
                    .remove_dependencies(HashSet::from([config_key.to_string()]))
                    .context(format!(
                        "Failed to remove dependency {} from configuration",
                        config_key
                    ))?;

                self.godot_config.save(plugin_config)?;
                println!("Dependency {} removed successfully.", config_key);
                Ok(())
            }
            None => {
                println!("Dependency {} is not installed.", config_key);
                Ok(())
            }
        }
    }

    /// Fetches plugins listed in the dependency file without version pinning (for update checking)
    async fn fetch_latest_assets(&self) -> Result<Vec<Asset>> {
        let plugins = self.gdm_config.get_dependencies()?;
        let godot_version = self.godot_config.get_godot_version_from_project()?;

        let mut assets_futures = Vec::new();

        for plugin in plugins.values() {
            if let PluginSource::AssetStore {
                asset_slug,
                publisher_slug,
            } = &plugin.source
            {
                let godot_version_clone = godot_version.clone();
                assets_futures.push(async move {
                    self.asset_store_service
                        .get_asset(
                            &publisher_slug.clone(),
                            &asset_slug.clone(),
                            None,
                            Some(&godot_version_clone),
                        )
                        .await
                });
            }
        }

        let fetched_assets: Vec<Asset> = try_join_all(assets_futures)
            .await
            .context("Failed to fetch latest dependencies from Asset Store API")?;

        Ok(fetched_assets)
    }

    async fn check_outdated_plugins(&self) -> Result<()> {
        if !self.gdm_config.has_dependencies()? {
            bail!("No dependencies installed.");
        }

        let installed_latest = self.fetch_latest_assets().await?;
        let mut plugins_to_update = Vec::new();

        println!(
            "{0: <40} {1: <20} {2: <20}",
            "Dependency", "Current", "Latest"
        );

        for asset in installed_latest {
            let plugin = self
                .gdm_config
                .get_dependency_by_asset_slug(&asset.asset_slug)?;

            if let Some(curr) = plugin {
                let latest_plugin = Plugin::from(asset);
                let has_update = latest_plugin > curr;

                if has_update {
                    plugins_to_update.push(latest_plugin.clone());
                }

                println!(
                    "{0: <40} {1: <20} {2: <20} {3}",
                    curr.title,
                    curr.get_version(),
                    latest_plugin.get_version(),
                    if has_update { "(update available)" } else { "" }
                );
            }
        }
        println!();

        if plugins_to_update.is_empty() {
            println!("All dependencies are up to date.");
        } else {
            println!("To update dependencies, use: gdm update");
        }
        Ok(())
    }

    async fn update_plugins(&self) -> Result<BTreeMap<String, Plugin>> {
        let plugins_map = self.gdm_config.get_dependencies()?;

        if plugins_map.is_empty() {
            bail!("No dependencies installed.");
        }

        let installed_latest = self.fetch_latest_assets().await?;
        let mut plugins_to_install = Vec::new();

        for asset in installed_latest {
            let plugin = self
                .gdm_config
                .get_dependency_by_asset_slug(&asset.asset_slug)?;

            if let Some(curr) = plugin {
                let latest_plugin = Plugin::from(asset);
                if latest_plugin > curr {
                    plugins_to_install.push(latest_plugin);
                }
            }
        }

        if plugins_to_install.is_empty() {
            println!("All dependencies are up to date.");
            return Ok(BTreeMap::new());
        }

        let updated_plugins = self.process_install(&plugins_to_install).await?;

        self.add_plugins(&updated_plugins)?;
        println!("Dependencies updated successfully.");
        Ok(updated_plugins)
    }

    async fn search_assets_by_name_and_version_and_godot_version(
        &self,
        name: &str,
        version: Option<&str>,
        godot_version: Option<&str>,
    ) -> Result<()> {
        let resolved_godot_version = self.resolve_godot_version(godot_version)?;

        let assets = self
            .asset_store_service
            .search_assets(name, version, Some(&resolved_godot_version))
            .await?;
        match assets.len() {
            0 => println!("No dependencies found matching \"{}\"", name),
            1 => println!("Found 1 dependency matching \"{}\":", name),
            n => println!("Found {} dependencies matching \"{}\":", n, name),
        }

        println!();

        // Calculate max widths (column 0: name, 1: version, 2: license, 3: votes)
        let max_name = assets
            .iter()
            .map(|a| format!("{}/{}", a.publisher_slug, a.asset_slug).len())
            .max()
            .unwrap_or(0);
        let max_ver = assets.iter().map(|a| a.version.len()).max().unwrap_or(0);
        let max_lic = assets.iter().map(|a| a.license.len()).max().unwrap_or(0);
        // Votes column: length of "👍 +NNN" or "👎 -NNN" – we'll build strings first
        let vote_strs: Vec<String> = assets
            .iter()
            .map(|a| {
                let (icon, vote) = if a.score >= 0 {
                    ("👍", a.score)
                } else {
                    ("👎", -a.score)
                };
                format!("{} {:+}", icon, vote) // + sign for explicit sign
            })
            .collect();
        let max_vote = vote_strs.iter().map(|v| v.len()).max().unwrap_or(0);

        for (i, asset) in assets.iter().enumerate() {
            let plugin = self
                .gdm_config
                .get_dependency_by_asset_slug(&asset.asset_slug)?;
            let installed_marker = if plugin.is_some() { " [installed]" } else { "" };
            let icon = if asset.score >= 0 { "👍" } else { "👎" };
            let vote_display = format!("{} {:+}", icon, asset.score);
            let asset_name = format!("{}/{}", asset.publisher_slug, asset.asset_slug);

            println!(
                "{:>2}{}. {:<name$}  {:<ver$}  {:<lic$}  {:<vote$}{}",
                " ",
                i + 1,
                asset_name,
                asset.version,
                asset.license,
                vote_display,
                installed_marker,
                name = max_name,
                ver = max_ver,
                lic = max_lic,
                vote = max_vote,
            );
            let description_truncated: String = asset.description.chars().take(120).collect();
            let description = if description_truncated.len() == 120 {
                description_truncated
                    .chars()
                    .take(117)
                    .chain(['.', '.', '.'])
                    .collect()
            } else {
                description_truncated
            };
            println!("{:>5}{}", " ", description);
            println!();
        }

        if assets.len() == 1 {
            let asset = assets.first().unwrap();
            println!(
                "To install the dependency, use: gdm add {}/{} or gdm add --asset-slug {} --publisher-slug {}",
                asset.publisher_slug, asset.asset_slug, asset.asset_slug, asset.publisher_slug
            );
        } else {
            println!("To install a dependency, use: gdm add <publisher_slug>/<asset_slug>");
        }
        Ok(())
    }
}

pub trait PluginService {
    async fn install_all_plugins(&self) -> Result<BTreeMap<String, Plugin>>;

    async fn add_plugin_by_git_url_and_reference(
        &self,
        git_url: Option<&str>,
        git_reference: Option<&str>,
    ) -> Result<()>;

    async fn add_plugin_by_publisher_and_asset_slug_and_version_and_godot_version(
        &self,
        publisher_slug: Option<&str>,
        asset_slug: Option<&str>,
        version: Option<&str>,
        godot_version: Option<&str>,
    ) -> Result<()>;

    async fn add_plugin_by_name_and_version_and_godot_version(
        &self,
        name: &str,
        version: Option<&str>,
        godot_version: Option<&str>,
    ) -> Result<()>;

    fn add_plugins(&self, plugins: &BTreeMap<String, Plugin>) -> Result<()>;

    async fn remove_plugin_by_config_key(&self, config_key: &str) -> Result<()>;

    async fn fetch_latest_assets(&self) -> Result<Vec<Asset>>;

    async fn check_outdated_plugins(&self) -> Result<()>;
    async fn update_plugins(&self) -> Result<BTreeMap<String, Plugin>>;

    async fn search_assets_by_name_and_version_and_godot_version(
        &self,
        name: &str,
        version: Option<&str>,
        godot_version: Option<&str>,
    ) -> Result<()>;

    fn finish_plugins_operation(&self, plugins: &BTreeMap<String, Plugin>) -> Result<()>;

    async fn process_install(&self, plugins: &[Plugin]) -> Result<BTreeMap<String, Plugin>>;
}

#[cfg(test)]
mod tests {
    use anyhow::Ok;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    use mockall::predicate::*;

    use crate::config::{
        DefaultAppConfig, GdmManifest, MockDefaultGdmConfig, MockDefaultGodotConfig,
    };
    use crate::models::{Asset, Plugin, PluginSource};
    use crate::services::asset_store::MockDefaultAssetStoreService;
    use crate::services::{
        DefaultPluginService, MockDefaultFileService, MockDefaultInstallService, PluginService,
    };

    // Helper to setup standard mocks
    fn setup_plugin_service_mocks() -> DefaultPluginService {
        let mut godot_config_repository = MockDefaultGodotConfig::default();
        let mut install_service = MockDefaultInstallService::default();

        // Setup install service to return installed plugins
        install_service.expect_install().returning(|plugins, _| {
            let mut result = BTreeMap::new();
            for plugin in plugins {
                // Extract folder name from plugin_cfg_path (e.g., "addons/test_plugin/plugin.cfg" -> "test_plugin")
                let folder_name = if let Some(ref path_str) = plugin.plugin_cfg_path {
                    let path = std::path::Path::new(path_str.as_str());
                    path.parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or(&plugin.title)
                        .to_string()
                } else {
                    plugin.title.clone()
                };
                result.insert(folder_name, plugin.clone());
            }
            Ok(result)
        });

        godot_config_repository
            .expect_save()
            .returning(|_path| Ok(()));

        godot_config_repository
            .expect_validate_project_file()
            .returning(|| Ok(()));

        godot_config_repository
            .expect_get_godot_version_from_project()
            .returning(|| Ok("4.5".to_string()));

        let mut asset_store_service = MockDefaultAssetStoreService::default();

        let mut plugin_config_repository = MockDefaultGdmConfig::default();
        plugin_config_repository
            .expect_add_dependencies()
            .returning(|_plugins| Ok(GdmManifest::new(_plugins.clone())));

        plugin_config_repository
            .expect_remove_dependencies()
            .returning(|_plugin_names| Ok(GdmManifest::default()));

        plugin_config_repository
            .expect_get_dependency_by_asset_slug()
            .returning(|_asset_slug| Ok(None));

        plugin_config_repository
            .expect_has_dependencies()
            .returning(|| Ok(true));

        let app_config = DefaultAppConfig::default();

        let file_service = Arc::new(MockDefaultFileService::default());

        plugin_config_repository
            .expect_get_dependencies()
            .returning(|| {
                Ok(BTreeMap::from([(
                    String::from("test_plugin"),
                    Plugin::create_mock_plugin_1(),
                )]))
            });

        plugin_config_repository
            .expect_get_dependency_by_asset_slug()
            .returning(|_asset_slug| Ok(Some(Plugin::create_mock_plugin_1())));

        asset_store_service
            .expect_search_assets()
            .returning(|_, _, _| {
                Ok(vec![Asset {
                    file_path: PathBuf::new(),
                    publisher_slug: "1234_publisher".to_string(),
                    asset_slug: "1234".to_string(),
                    license: "MIT".to_string(),
                    score: 5,
                    version: "1.1.1".to_string(),
                    title: "Test Plugin".to_string(),
                    description: "Some description".to_string(),
                    size: 100.0,
                    download_url: "download_url".to_string(),
                }])
            });

        asset_store_service
            .expect_get_asset()
            .withf(|publisher_slug, asset_slug, version, opt2| {
                asset_slug == "1234"
                    && publisher_slug == "1234_publisher"
                    && version == &Some("1.0.0")
                    && opt2.is_none()
            })
            .returning(|asset_slug, version, _, _| {
                Err(anyhow::anyhow!(
                    "Asset with slug {} and version {} not found",
                    asset_slug,
                    version
                ))
            });
        asset_store_service
            .expect_get_asset()
            .withf(|publisher_slug, asset_slug, version, opt2| {
                asset_slug == "1234"
                    && publisher_slug == "1234_publisher"
                    && version == &Some("1.1.1")
                    && opt2.is_none()
            })
            .returning(|_, _, _, _| {
                Ok(Asset {
                    file_path: PathBuf::new(),
                    publisher_slug: "1234_publisher".to_string(),
                    asset_slug: "1234".to_string(),
                    license: "MIT".to_string(),
                    score: 5,
                    version: "1.1.1".to_string(),
                    title: "Test Plugin".to_string(),
                    description: "Some description".to_string(),
                    size: 100.0,
                    download_url: "download_url".to_string(),
                })
            });

        asset_store_service
            .expect_download_asset()
            .returning(|_, _pb| {
                Ok(Asset {
                    file_path: PathBuf::new(),
                    publisher_slug: "1234_publisher".to_string(),
                    asset_slug: "1234".to_string(),
                    license: "MIT".to_string(),
                    score: 5,
                    version: "1.1.1".to_string(),
                    title: "Test Plugin".to_string(),
                    description: "Some description".to_string(),
                    size: 100.0,
                    download_url: "http://example.com/test_plugin.zip".to_string(),
                })
            });
        asset_store_service
            .expect_search_assets()
            .returning(|_, _, _| {
                Ok(vec![Asset {
                    file_path: PathBuf::new(),
                    publisher_slug: "1234_publisher".to_string(),
                    asset_slug: "1234".to_string(),
                    license: "MIT".to_string(),
                    score: 5,
                    version: "1.1.1".to_string(),
                    title: "Test Plugin".to_string(),
                    description: "Some description".to_string(),
                    size: 100.0,
                    download_url: "download_url".to_string(),
                }])
            });

        let asset_store_service_arc = Arc::new(asset_store_service);
        let install_service_arc = Arc::new(install_service);

        DefaultPluginService::new(
            Box::new(godot_config_repository),
            Box::new(plugin_config_repository),
            app_config,
            file_service,
            asset_store_service_arc,
            install_service_arc,
        )
    }

    fn setup_name_add_plugin_service(assets: Vec<Asset>) -> DefaultPluginService {
        let mut asset_store_service = MockDefaultAssetStoreService::default();
        asset_store_service
            .expect_search_assets_by_name()
            .times(1)
            .returning(move |_, _, _| Ok(assets.clone()));

        DefaultPluginService::new(
            Box::new(MockDefaultGodotConfig::default()),
            Box::new(MockDefaultGdmConfig::default()),
            DefaultAppConfig::default(),
            Arc::new(MockDefaultFileService::default()),
            Arc::new(asset_store_service),
            Arc::new(MockDefaultInstallService::default()),
        )
    }

    fn name_search_asset(asset_slug: &str) -> Asset {
        Asset {
            file_path: PathBuf::new(),
            publisher_slug: "publisher".to_string(),
            asset_slug: asset_slug.to_string(),
            license: "MIT".to_string(),
            score: 5,
            version: "1.2.3".to_string(),
            title: "Test Plugin".to_string(),
            description: "Description".to_string(),
            size: 100.0,
            download_url: "https://example.com/test-plugin.zip".to_string(),
        }
    }

    #[tokio::test]
    async fn test_add_plugin_by_name_uses_name_search() {
        let mut godot_config_repository = MockDefaultGodotConfig::default();
        godot_config_repository
            .expect_save()
            .times(1)
            .returning(|_| Ok(()));

        let mut install_service = MockDefaultInstallService::default();
        install_service
            .expect_install()
            .times(1)
            .returning(|plugins, _| {
                assert_eq!(plugins.len(), 1);
                Ok(BTreeMap::from([(
                    String::from("test_plugin"),
                    plugins[0].clone(),
                )]))
            });

        let mut plugin_config_repository = MockDefaultGdmConfig::default();
        plugin_config_repository
            .expect_add_dependencies()
            .times(1)
            .withf(|plugins| plugins.len() == 1 && plugins.contains_key("test_plugin"))
            .returning(|plugins| Ok(GdmManifest::new(plugins.clone())));

        let mut asset_store_service = MockDefaultAssetStoreService::default();
        let asset = Asset {
            file_path: PathBuf::new(),
            publisher_slug: "publisher".to_string(),
            asset_slug: "test-plugin".to_string(),
            license: "MIT".to_string(),
            score: 5,
            version: "1.2.3".to_string(),
            title: "Test Plugin".to_string(),
            description: "Description".to_string(),
            size: 100.0,
            download_url: "https://example.com/test-plugin.zip".to_string(),
        };
        asset_store_service
            .expect_search_assets_by_name()
            .times(1)
            .withf(|name, version, godot_version| {
                name == "  Test Plugin  "
                    && version == &Some("1.2.3")
                    && godot_version == &Some("4.6")
            })
            .returning(move |_, _, _| Ok(vec![asset.clone()]));

        let service = DefaultPluginService::new(
            Box::new(godot_config_repository),
            Box::new(plugin_config_repository),
            DefaultAppConfig::default(),
            Arc::new(MockDefaultFileService::default()),
            Arc::new(asset_store_service),
            Arc::new(install_service),
        );

        let result = service
            .add_plugin_by_name_and_version_and_godot_version(
                "  Test Plugin  ",
                Some("1.2.3"),
                Some("4.6"),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_plugin_by_name_returns_error_for_zero_exact_matches() {
        let service = setup_name_add_plugin_service(vec![]);

        let error = service
            .add_plugin_by_name_and_version_and_godot_version("Test Plugin", None, Some("4.6"))
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "No dependencies found with name: Test Plugin"
        );
    }

    #[tokio::test]
    async fn test_add_plugin_by_name_returns_error_for_multiple_exact_matches() {
        let service = setup_name_add_plugin_service(vec![
            name_search_asset("test-plugin-1"),
            name_search_asset("test-plugin-2"),
        ]);

        let error = service
            .add_plugin_by_name_and_version_and_godot_version("Test Plugin", None, Some("4.6"))
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Found too many dependencies with name: Test Plugin"
        );
    }

    #[tokio::test]
    async fn test_add_plugin_by_slug_forwards_publisher_before_asset() {
        let mut asset_store_service = MockDefaultAssetStoreService::default();
        asset_store_service
            .expect_get_asset()
            .times(1)
            .withf(|publisher_slug, asset_slug, version, godot_version| {
                publisher_slug == "publisher"
                    && asset_slug == "asset"
                    && version == &Some("1.2.3")
                    && godot_version == &Some("4.6")
            })
            .returning(|_, _, _, _| Err(anyhow::anyhow!("forwarding verified")));

        let service = DefaultPluginService::new(
            Box::new(MockDefaultGodotConfig::default()),
            Box::new(MockDefaultGdmConfig::default()),
            DefaultAppConfig::default(),
            Arc::new(MockDefaultFileService::default()),
            Arc::new(asset_store_service),
            Arc::new(MockDefaultInstallService::default()),
        );

        let error = service
            .add_plugin_by_publisher_and_asset_slug_and_version_and_godot_version(
                Some("publisher"),
                Some("asset"),
                Some("1.2.3"),
                Some("4.6"),
            )
            .await
            .unwrap_err();

        assert_eq!(error.to_string(), "forwarding verified");
    }

    #[test]
    fn test_parse_git_url_slugs_normalizes_to_lowercase() {
        assert_eq!(
            super::parse_git_url_slugs("https://github.com/SomeUser/MyPlugin.git").unwrap(),
            ("someuser".to_string(), "myplugin".to_string())
        );
    }

    #[test]
    fn test_parse_git_url_slugs_rejects_url_without_repository_name() {
        assert!(super::parse_git_url_slugs("https://github.com/SomeUser").is_err());
    }

    #[test]
    fn test_validate_plugin_folder_name_rejects_non_single_components() {
        for invalid_name in [
            "",
            ".",
            "..",
            "nested/plugin",
            "/absolute",
            "nested\\plugin",
        ] {
            assert!(
                super::validate_plugin_folder_name(invalid_name).is_err(),
                "{invalid_name:?} should be rejected"
            );
        }

        assert!(super::validate_plugin_folder_name("valid_plugin").is_ok());
    }

    // install_all_plugins

    #[tokio::test]
    async fn test_install_plugins_should_install_all_plugins_in_config() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service.install_all_plugins().await;
        assert!(result.is_ok());
        let installed_plugins = result.unwrap();

        let expected_plugins =
            BTreeMap::from([(String::from("asset_54321"), Plugin::create_mock_plugin_1())]);

        assert_eq!(installed_plugins, expected_plugins);
    }

    // update_plugins

    fn setup_update_plugin_mocks(
        current_plugin_version: &str,
        update_plugin_version: &str,
    ) -> DefaultPluginService {
        let mut godot_config_repository = MockDefaultGodotConfig::default();
        let mut install_service = MockDefaultInstallService::default();

        // Setup install service to return installed plugins with plugin_cfg_path set
        install_service.expect_install().returning(|plugins, _| {
            let mut result = BTreeMap::new();
            for plugin in plugins {
                // For update tests, we need to set the plugin_cfg_path since the real installer would set it
                let mut updated_plugin = plugin.clone();
                if updated_plugin.plugin_cfg_path.is_none() {
                    // Set it to the expected path
                    updated_plugin.plugin_cfg_path = Some("addons/test_plugin/plugin.cfg".into());
                }

                // Extract folder name from plugin_cfg_path
                let folder_name = if let Some(ref path_str) = updated_plugin.plugin_cfg_path {
                    let path = std::path::Path::new(path_str.as_str());
                    path.parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or(&updated_plugin.title)
                        .to_string()
                } else {
                    updated_plugin.title.clone()
                };
                result.insert(folder_name, updated_plugin);
            }
            Ok(result)
        });

        godot_config_repository
            .expect_save()
            .returning(|_path| Ok(()));
        godot_config_repository
            .expect_get_godot_version_from_project()
            .returning(|| Ok("4.5".to_string()));

        let mut asset_store_service = MockDefaultAssetStoreService::default();

        let mut plugin_config_repository = MockDefaultGdmConfig::default();
        plugin_config_repository
            .expect_add_dependencies()
            .returning(|_plugins| Ok(GdmManifest::new(_plugins.clone())));

        plugin_config_repository
            .expect_remove_dependencies()
            .returning(|_plugin_names| Ok(GdmManifest::default()));

        plugin_config_repository
            .expect_has_dependencies()
            .returning(|| Ok(true));

        plugin_config_repository
            .expect_get_dependency_by_asset_slug()
            .returning({
                let version = current_plugin_version.to_string();
                move |_asset_slug| {
                    Ok(Some(Plugin::new(
                        PluginSource::AssetStore {
                            asset_slug: "asset_54321".to_string(),
                            publisher_slug: "publisher_12345".to_string(),
                        },
                        Some("addons/asset_54321/plugin.cfg".into()),
                        "Awesome Plugin".to_string(),
                        version.clone(),
                        Some("MIT".to_string()),
                        vec![],
                    )))
                }
            });

        let app_config = DefaultAppConfig::default();
        let file_service = Arc::new(MockDefaultFileService::default());

        plugin_config_repository
            .expect_get_dependencies()
            .returning({
                let version = current_plugin_version.to_string();
                move || {
                    Ok(BTreeMap::from([(
                        String::from("test_plugin"),
                        Plugin::new(
                            PluginSource::AssetStore {
                                asset_slug: "asset_54321".to_string(),
                                publisher_slug: "publisher_12345".to_string(),
                            },
                            Some("addons/asset_54321/plugin.cfg".into()),
                            "Awesome Plugin".to_string(),
                            version.clone(),
                            Some("MIT".to_string()),
                            vec![],
                        ),
                    )]))
                }
            });

        // Mocks for getting latest assets
        let asset_store_plugin_version = update_plugin_version.to_string();
        asset_store_service.expect_get_asset().returning(
            move |publisher_slug, asset_slug, _version, _godot_version| {
                Ok(Asset {
                    file_path: PathBuf::new(),
                    asset_slug: asset_slug.to_string(),
                    publisher_slug: publisher_slug.to_string(),
                    title: "Test Plugin".to_string(),
                    version: asset_store_plugin_version.clone(),
                    size: 100.0,
                    score: 5,
                    license: "MIT".to_string(),
                    description: "Some description".to_string(),
                    download_url: "https://example.com/test_plugin.zip".to_string(),
                })
            },
        );

        // This mock is crucial for `fetch_latest_assets` inside update_plugins
        asset_store_service
            .expect_search_assets()
            .returning(move |_, version, _godot_version| {
                Ok(vec![Asset {
                    file_path: PathBuf::new(),
                    asset_slug: "1234".to_string(),
                    publisher_slug: "1234_publisher".to_string(),
                    title: "Test Plugin".to_string(),
                    version: version.unwrap_or("1.0.0").to_string(),
                    size: 100.0,
                    score: 5,
                    license: "MIT".to_string(),
                    description: "Some description".to_string(),
                    download_url: "https://example.com/test_plugin.zip".to_string(),
                }])
            });

        asset_store_service
            .expect_download_asset()
            .returning(|asset_response, _pb| {
                Ok(Asset {
                    file_path: PathBuf::new(),
                    asset_slug: asset_response.asset_slug,
                    publisher_slug: asset_response.publisher_slug,
                    title: asset_response.title,
                    version: asset_response.version,
                    size: asset_response.size,
                    score: asset_response.score,
                    license: asset_response.license,
                    description: asset_response.description,
                    download_url: asset_response.download_url,
                })
            });

        let asset_store_service_arc = Arc::new(asset_store_service);
        let install_service_arc = Arc::new(install_service);

        DefaultPluginService::new(
            Box::new(godot_config_repository),
            Box::new(plugin_config_repository),
            app_config,
            file_service,
            asset_store_service_arc,
            install_service_arc,
        )
    }

    #[tokio::test]
    async fn test_update_plugins_should_return_correct_plugins_if_there_is_an_update_1() {
        let plugin_service = setup_update_plugin_mocks("1.1.1", "1.2.0");
        let result = plugin_service.update_plugins().await;
        assert!(result.is_ok());

        let updated_plugins = result.unwrap();
        let expected_updated_plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new(
                PluginSource::AssetStore {
                    publisher_slug: "publisher_12345".to_string(),
                    asset_slug: "asset_54321".to_string(),
                },
                Some("addons/test_plugin/plugin.cfg".into()),
                "Test Plugin".to_string(),
                "1.2.0".to_string(),
                Some("MIT".to_string()),
                vec![],
            ),
        )]);
        assert_eq!(updated_plugins, expected_updated_plugins);
    }

    #[tokio::test]
    async fn test_update_plugins_should_return_correct_plugins_if_there_is_no_update() {
        let plugin_service = setup_update_plugin_mocks("1.1.1", "1.1.1");
        let result = plugin_service.update_plugins().await;
        assert!(result.is_ok());

        let updated_plugins = result.unwrap();
        let expected_updated_plugins = BTreeMap::from([]);
        assert_eq!(updated_plugins, expected_updated_plugins);
    }

    // finish_plugins_operation

    #[test]
    fn test_finish_plugins_operation_should_complete_successfully() {
        // Setup minimal mocks just to satisfy constructor
        let godot_config = MockDefaultGodotConfig::default();
        let plugin_config = MockDefaultGdmConfig::default();
        let app_config = DefaultAppConfig::default();
        let file_service = Arc::new(MockDefaultFileService::default());
        let asset_store = Arc::new(MockDefaultAssetStoreService::default());

        let install_service = MockDefaultInstallService::default();
        let install_service_arc = Arc::new(install_service);

        let plugin_service = DefaultPluginService::new(
            Box::new(godot_config),
            Box::new(plugin_config),
            app_config,
            file_service,
            asset_store,
            install_service_arc,
        );

        // Updated test data: Use Vec instead of BTreeMap
        let plugins =
            BTreeMap::from([(String::from("test_plugin"), Plugin::create_mock_plugin_1())]);

        let result = plugin_service.finish_plugins_operation(&plugins);
        assert!(result.is_ok());
    }

    // check_outdated_plugins tests

    fn setup_check_outdated_mocks(
        installed_plugins: Vec<(&str, &str, &str, &str)>, // (asset_slug, title, version)
        latest_plugins: Vec<(&str, &str, &str, &str)>,    // (asset_slug, title, version)
    ) -> DefaultPluginService {
        let mut godot_config_repository = MockDefaultGodotConfig::default();
        godot_config_repository
            .expect_get_godot_version_from_project()
            .returning(|| Ok("4.5".to_string()));

        let mut asset_store_service = MockDefaultAssetStoreService::default();
        let mut plugin_config_repository = MockDefaultGdmConfig::default();

        plugin_config_repository
            .expect_has_dependencies()
            .returning(|| Ok(true));

        // Setup installed plugins
        let installed_map: BTreeMap<String, Plugin> = installed_plugins
            .iter()
            .map(|(asset_slug, publisher_slug, title, version)| {
                (
                    title.to_lowercase().replace(' ', "_"),
                    Plugin::new_asset_store_plugin(
                        asset_slug.to_string(),
                        publisher_slug.to_string(),
                        Some(
                            format!(
                                "addons/{}/plugin.cfg",
                                title.to_lowercase().replace(' ', "_")
                            )
                            .into(),
                        ),
                        title.to_string(),
                        version.to_string(),
                        "MIT".to_string(),
                        vec![],
                    ),
                )
            })
            .collect();

        let installed_map_clone = installed_map.clone();
        plugin_config_repository
            .expect_get_dependencies()
            .returning(move || Ok(installed_map_clone.clone()));

        // Setup get_dependency_by_asset_slug to return the correct plugin
        let installed_map_for_lookup = installed_map.clone();
        plugin_config_repository
            .expect_get_dependency_by_asset_slug()
            .returning(move |asset_slug| {
                let found = installed_map_for_lookup
                    .values()
                    .find(|p| {
                        if let PluginSource::AssetStore {
                            asset_slug: id,
                            publisher_slug: _,
                        } = &p.source
                        {
                            id == asset_slug
                        } else {
                            false
                        }
                    })
                    .cloned();
                Ok(Some(found.unwrap()))
            });

        let owned_plugins: Vec<(String, String, String, String)> = latest_plugins
            .iter()
            .map(|(a, p, t, v)| (a.to_string(), p.to_string(), t.to_string(), v.to_string()))
            .collect();

        // Setup API to return latest versions
        for (asset_slug, publisher_slug, title, version) in owned_plugins {
            let asset_slug_owned = asset_slug.to_string();
            let publisher_slug_owned = publisher_slug.to_string();
            let title_owned = title.to_string();
            let version_owned = version.to_string();

            asset_store_service
                .expect_get_asset()
                .withf(move |publisher_slug, asset_slug, _, _| {
                    publisher_slug == publisher_slug_owned && asset_slug == asset_slug_owned
                })
                .returning(move |publisher_slug, asset_slug, _, _| {
                    Ok(Asset {
                        file_path: PathBuf::new(),
                        asset_slug: asset_slug.to_string(),
                        publisher_slug: publisher_slug.to_string(),
                        title: title_owned.clone(),
                        version: version_owned.clone(),
                        size: 100.0,
                        score: 5,
                        license: "MIT".to_string(),
                        description: "Description".to_string(),
                        download_url: format!("https://example.com/{}.zip", asset_slug),
                    })
                });
        }

        let app_config = DefaultAppConfig::default();
        let file_service = Arc::new(MockDefaultFileService::default());
        let install_service_arc = Arc::new(MockDefaultInstallService::default());
        let asset_store_service_arc = Arc::new(asset_store_service);

        DefaultPluginService::new(
            Box::new(godot_config_repository),
            Box::new(plugin_config_repository),
            app_config,
            file_service,
            asset_store_service_arc,
            install_service_arc,
        )
    }

    #[tokio::test]
    async fn test_check_outdated_plugins_with_no_updates_available() {
        let installed = vec![
            ("1234", "1234_publisher", "Test Plugin", "1.0.0"),
            ("5678", "5678_publisher", "Another Plugin", "2.5.0"),
        ];
        let latest = vec![
            ("1234", "1234_publisher", "Test Plugin", "1.0.0"),
            ("5678", "5678_publisher", "Another Plugin", "2.5.0"),
        ];

        let plugin_service = setup_check_outdated_mocks(installed, latest);
        let result = plugin_service.check_outdated_plugins().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_outdated_plugins_with_updates_available() {
        let installed = vec![
            ("1234", "1234_publisher", "Test Plugin", "1.0.0"),
            ("5678", "5678_publisher", "Another Plugin", "2.5.0"),
        ];
        let latest = vec![
            ("1234", "1234_publisher", "Test Plugin", "1.2.0"),
            ("5678", "5678_publisher", "Another Plugin", "2.5.0"),
        ];

        let plugin_service = setup_check_outdated_mocks(installed, latest);
        let result = plugin_service.check_outdated_plugins().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_outdated_plugins_with_all_updates_available() {
        let installed = vec![
            ("1234", "1234_publisher", "Test Plugin", "1.0.0"),
            ("5678", "5678_publisher", "Another Plugin", "2.5.0"),
        ];
        let latest = vec![
            ("1234", "1234_publisher", "Test Plugin", "2.0.0"), // Major update
            ("5678", "5678_publisher", "Another Plugin", "3.0.0"), // Major update
        ];

        let plugin_service = setup_check_outdated_mocks(installed, latest);
        let result = plugin_service.check_outdated_plugins().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_outdated_plugins_with_single_plugin() {
        let installed = vec![("1234", "1234_publisher", "Single Plugin", "1.0.0")];
        let latest = vec![("1234", "1234_publisher", "Single Plugin", "1.0.1")]; // Patch update

        let plugin_service = setup_check_outdated_mocks(installed, latest);
        let result = plugin_service.check_outdated_plugins().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_outdated_plugins_with_no_plugins_installed() {
        let godot_config_repository = MockDefaultGodotConfig::default();
        let mut plugin_config_repository = MockDefaultGdmConfig::default();

        plugin_config_repository
            .expect_has_dependencies()
            .returning(|| Ok(false));

        let app_config = DefaultAppConfig::default();
        let file_service = Arc::new(MockDefaultFileService::default());
        let asset_store = Arc::new(MockDefaultAssetStoreService::default());
        let install_service = Arc::new(MockDefaultInstallService::default());

        let plugin_service = DefaultPluginService::new(
            Box::new(godot_config_repository),
            Box::new(plugin_config_repository),
            app_config,
            file_service,
            asset_store,
            install_service,
        );

        let result = plugin_service.check_outdated_plugins().await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "No dependencies installed."
        );
    }

    #[tokio::test]
    async fn test_check_outdated_plugins_with_mixed_updates() {
        let installed = vec![
            ("1111", "1111_publisher", "Up to Date Plugin", "3.0.0"),
            ("2222", "2222_publisher", "Minor Update Plugin", "1.5.0"),
            ("3333", "3333_publisher", "Major Update Plugin", "1.0.0"),
            ("4444", "4444_publisher", "Patch Update Plugin", "2.1.0"),
        ];
        let latest = vec![
            ("1111", "1111_publisher", "Up to Date Plugin", "3.0.0"), // No update
            ("2222", "2222_publisher", "Minor Update Plugin", "1.6.0"), // Minor update
            ("3333", "3333_publisher", "Major Update Plugin", "2.0.0"), // Major update
            ("4444", "4444_publisher", "Patch Update Plugin", "2.1.1"), // Patch update
        ];

        let plugin_service = setup_check_outdated_mocks(installed, latest);
        let result = plugin_service.check_outdated_plugins().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_outdated_plugins_with_semantic_versioning() {
        let installed = vec![
            ("1234", "1234_publisher", "Plugin A", "1.0.0"),
            ("5678", "5678_publisher", "Plugin B", "2.5.10"),
            ("9012", "9012_publisher", "Plugin C", "0.9.0"),
        ];
        let latest = vec![
            ("1234", "1234_publisher", "Plugin A", "1.0.1"), // Patch
            ("5678", "5678_publisher", "Plugin B", "2.6.0"), // Minor
            ("9012", "9012_publisher", "Plugin C", "1.0.0"), // Major (pre-release to stable)
        ];

        let plugin_service = setup_check_outdated_mocks(installed, latest);
        let result = plugin_service.check_outdated_plugins().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_outdated_plugins_preserves_installed_plugin_data() {
        // This test ensures that checking for updates doesn't modify the installed plugins
        let installed = vec![("1234", "1234_publisher", "Test Plugin", "1.0.0")];
        let latest = vec![("1234", "1234_publisher", "Test Plugin", "2.0.0")];

        let plugin_service = setup_check_outdated_mocks(installed, latest);
        let result = plugin_service.check_outdated_plugins().await;

        assert!(result.is_ok());

        // Verify that the installed plugins weren't modified
        let plugins = plugin_service.gdm_config.get_dependencies().unwrap();
        let test_plugin = plugins.values().next().unwrap();
        assert_eq!(test_plugin.get_version(), "1.0.0"); // Should still be old version
    }
}
