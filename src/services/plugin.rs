use crate::api::{AssetListResponse, AssetResponse, AssetStoreAPI, DefaultAssetStoreAPI};
use crate::config::{
    AppConfig, DefaultAppConfig, DefaultGdmConfig, DefaultGodotConfig, GdmConfig, GodotConfig,
};
use crate::installers::{AssetLibraryInstaller, GitInstaller, PluginInstaller};
use crate::models::{Plugin, PluginSource};
use crate::services::{
    DefaultExtractService, DefaultFileService, DefaultGitService, FileService, PluginParser,
    StagingService,
};
use crate::ui::{Operation, OperationManager};
use crate::utils::Utils;

use anyhow::{Context, Result, bail};
use futures::future::try_join_all;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tracing::info;

pub struct DefaultPluginService {
    pub godot_config: Box<dyn GodotConfig>,
    pub gdm_config: Box<dyn GdmConfig>,

    pub app_config: DefaultAppConfig,
    pub file_service: Arc<dyn FileService + Send + Sync>,
    pub asset_store_api: Arc<dyn AssetStoreAPI + Send + Sync>,
    #[allow(dead_code)]
    pub parser: PluginParser,

    installers: Vec<Box<dyn PluginInstaller>>,
}

impl Default for DefaultPluginService {
    fn default() -> Self {
        let asset_store_api = Arc::new(DefaultAssetStoreAPI::default());
        let extract_service = Arc::new(DefaultExtractService::default());
        let git_service = Arc::new(DefaultGitService::default());
        let file_service = Arc::new(DefaultFileService);
        let parser = Arc::new(PluginParser::new(file_service.clone()));

        // Create app config for staging service
        let app_config = DefaultAppConfig::default();

        // Create staging service for unified workflow
        let staging_service = Arc::new(StagingService::new(
            file_service.clone(),
            parser.clone(),
            &app_config,
        ));

        // Create installers WITH staging support
        let asset_installer = AssetLibraryInstaller::with_staging(
            asset_store_api.clone(),
            extract_service,
            staging_service.clone(),
        );

        let git_installer =
            GitInstaller::with_staging(git_service, parser.clone(), staging_service);

        Self {
            godot_config: Box::new(DefaultGodotConfig::default()),
            gdm_config: Box::new(DefaultGdmConfig::default()),
            app_config,
            parser: parser.as_ref().clone(),
            file_service,
            asset_store_api,
            installers: vec![Box::new(asset_installer), Box::new(git_installer)],
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
        asset_store_api: Arc<dyn AssetStoreAPI + Send + Sync>,
        installers: Vec<Box<dyn PluginInstaller>>,
    ) -> Self {
        Self {
            godot_config,
            gdm_config,
            app_config,
            parser: PluginParser::new(file_service.clone()),
            file_service,
            asset_store_api,
            installers,
        }
    }
}

impl PluginService for DefaultPluginService {
    async fn process_install(&self, plugins: &[Plugin]) -> Result<BTreeMap<String, Plugin>> {
        let mut final_results = BTreeMap::new();
        let mut installer_futures = Vec::new();

        let total_plugins_count = plugins.len();

        let operation_manager = Arc::new(OperationManager::new(Operation::Install)?);

        for (index, plugin) in plugins.iter().enumerate() {
            if let Some(source) = &plugin.source {
                let installer = self.installers.iter().find(|inst| inst.can_handle(source));
                if let Some(installer) = installer {
                    installer_futures.push(installer.install(
                        vec![plugin.clone()],
                        operation_manager.clone(),
                        index,
                        total_plugins_count,
                    ));
                }
            }
        }

        let results_list = try_join_all(installer_futures).await?;

        operation_manager.finish();

        for installed_map in results_list {
            final_results.extend(installed_map);
        }

        self.finish_plugins_operation(&final_results)?;

        Ok(final_results)
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
        info!("Finished processing {} plugins successfully", plugins.len());
        Ok(())
    }

    /// Helper to find metadata for a plugin before adding it (Asset Lib only)
    async fn find_asset_metadata(
        &self,
        name: &str,
        asset_id: &str,
        version: &str,
    ) -> Result<AssetResponse> {
        let godot_version = self.godot_config.get_godot_version_from_project()?;

        if !version.is_empty() && !asset_id.is_empty() {
            return self
                .asset_store_api
                .get_asset_by_id_and_version(asset_id, version)
                .await
                .with_context(|| {
                    format!(
                        "Failed to find plugin with asset ID '{}' and version '{}'",
                        asset_id, version
                    )
                });
        }

        if !name.is_empty() && !version.is_empty() {
            return self
                .asset_store_api
                .find_asset_by_asset_name_and_version_and_godot_version(
                    name,
                    version,
                    &godot_version,
                )
                .await
                .with_context(|| {
                    format!(
                        "Failed to find plugin with name '{}' and version '{}'",
                        name, version
                    )
                });
        }

        if !name.is_empty() || !asset_id.is_empty() {
            return self
                .asset_store_api
                .find_asset_by_id_or_name_and_version(asset_id, name, &godot_version)
                .await
                .with_context(|| {
                    let error_base = "No asset found with";
                    if !name.is_empty() {
                        format!("{} name '{}'", error_base, name)
                    } else {
                        format!("{} asset ID '{}'", error_base, asset_id)
                    }
                });
        }

        bail!("No name or asset ID provided")
    }

    async fn install_all_plugins(&self) -> Result<BTreeMap<String, Plugin>> {
        if !self.gdm_config.has_installed_plugins()? {
            bail!("No plugins installed.");
        }

        let all_plugins_map = self.gdm_config.get_plugins()?;
        let all_plugins: Vec<Plugin> = all_plugins_map.values().cloned().collect();

        let installed_plugins = self.process_install(&all_plugins).await?;

        self.add_plugins(&installed_plugins)?;
        info!("All plugins installed successfully");
        Ok(installed_plugins)
    }

    async fn add_plugin(
        &self,
        asset_id: Option<String>,
        name: Option<String>,
        version: Option<String>,
        git_url: Option<String>,
        git_reference: Option<String>,
    ) -> Result<()> {
        let is_asset_based = asset_id.is_some() || name.is_some() || version.is_some();
        let is_git_based = git_url.is_some() || git_reference.is_some();

        if is_asset_based && is_git_based {
            bail!("Cannot specify name/asset_id/version together with git URL/reference.")
        }

        let plugin_to_install: Plugin;

        if is_asset_based {
            let name = name.unwrap_or_default();
            let asset_id = asset_id.unwrap_or_default();
            let version = version.unwrap_or_default();

            if !name.is_empty() && !asset_id.is_empty() {
                bail!("Cannot specify both name and asset ID.")
            }

            if name.is_empty() && asset_id.is_empty() {
                bail!("Either name or asset ID must be provided.")
            }

            // 1. Verify availability in store and get metadata
            let asset_response = self.find_asset_metadata(&name, &asset_id, &version).await?;

            // 2. Check overlap with existing
            if let Some(existing) = self
                .gdm_config
                .get_plugin_by_asset_id(&asset_response.asset_id)?
            {
                let new_plugin = Plugin::from(asset_response.clone());
                if new_plugin != existing {
                    println!(
                        "Updating plugin '{}' from {} to {}",
                        existing.title,
                        existing.get_version(),
                        new_plugin.get_version()
                    );
                } else {
                    println!("Plugin '{}' is already in dependencies.", existing.title);
                }
            }

            plugin_to_install = Plugin::from(asset_response);
        } else if is_git_based {
            let git_url = git_url.ok_or_else(|| anyhow::anyhow!("Git URL must be provided."))?;
            let reference = git_reference.unwrap_or_else(|| "main".to_string());

            if git_url.is_empty() {
                bail!("Git URL must be provided.")
            }

            plugin_to_install = Plugin {
                source: Some(PluginSource::Git {
                    url: git_url,
                    reference,
                }),
                ..Plugin::default()
            };
        } else {
            bail!("Either name, asset_id, version OR git URL/reference must be provided.")
        }

        let installed = self.process_install(&[plugin_to_install]).await?;

        self.add_plugins(&installed)?;

        info!(
            "Plugins installed successfully: {:?}",
            installed.keys().collect::<Vec<_>>()
        );
        Ok(())
    }

    fn add_plugins(&self, plugins: &BTreeMap<String, Plugin>) -> Result<()> {
        let plugin_config = self.gdm_config.add_plugins(plugins)?;
        self.godot_config.save(plugin_config)?;
        info!(
            "Added {} plugins to configuration successfully",
            plugins.len()
        );
        Ok(())
    }

    async fn remove_plugin_by_name(&self, name: &str) -> Result<()> {
        if !self.gdm_config.has_installed_plugins()? {
            bail!("No plugins installed.");
        }

        let installed_plugin = self.gdm_config.get_plugin_by_name(name);
        let addon_folder = self.app_config.get_addon_folder_path();

        match installed_plugin {
            Some((plugin_name, plugin)) => {
                let plugin_folder_path = Utils::plugin_name_to_addon_folder_path(
                    &addon_folder,
                    Path::new(plugin_name.as_str()),
                );

                if self.file_service.directory_exists(&plugin_folder_path) {
                    println!("Removing plugin folder: {}", plugin_folder_path.display());
                    self.file_service.remove_dir_all(&plugin_folder_path)?
                } else {
                    println!("Plugin folder does not exist, removing from config only.");
                }

                for asset in &plugin.sub_assets {
                    let sub_path = Utils::plugin_name_to_addon_folder_path(
                        &addon_folder,
                        Path::new(asset.as_str()),
                    );
                    if self.file_service.directory_exists(&sub_path) {
                        println!("Removing sub-asset folder: {}", sub_path.display());
                        self.file_service.remove_dir_all(&sub_path)?
                    }
                }

                let plugin_config = self
                    .gdm_config
                    .remove_plugins(HashSet::from([plugin_name.clone()]))
                    .context(format!(
                        "Failed to remove plugin {} from configuration",
                        plugin_name
                    ))?;

                self.godot_config.save(plugin_config)?;
                println!("Plugin {} removed successfully.", plugin_name);
                Ok(())
            }
            None => {
                println!("Plugin {} is not installed.", name);
                Ok(())
            }
        }
    }

    /// Fetches plugins listed in the dependency file without version pinning (for update checking)
    async fn fetch_latest_assets(&self) -> Result<Vec<AssetResponse>> {
        let plugins = self.gdm_config.get_plugins()?;
        let godot_version = self.godot_config.get_godot_version_from_project()?;

        let mut assets_futures = Vec::new();

        for plugin in plugins.values() {
            if let Some(PluginSource::AssetLibrary { asset_id }) = &plugin.source {
                let id = asset_id.clone();
                let g_ver = godot_version.clone();
                let api = self.asset_store_api.clone();

                assets_futures.push(async move {
                    api.find_asset_by_id_or_name_and_version(&id, "", &g_ver)
                        .await
                });
            }
        }

        let fetched_assets: Vec<AssetResponse> = try_join_all(assets_futures)
            .await
            .context("Failed to fetch latest plugins from Asset Store API")?;

        Ok(fetched_assets)
    }

    async fn check_outdated_plugins(&self) -> Result<()> {
        if !self.gdm_config.has_installed_plugins()? {
            bail!("No plugins installed.");
        }

        let installed_latest = self.fetch_latest_assets().await?;
        let mut plugins_to_update = Vec::new();

        println!("{0: <40} {1: <20} {2: <20}", "Plugin", "Current", "Latest");

        for asset in installed_latest {
            let current_plugin_opt = self.gdm_config.get_plugin_by_asset_id(&asset.asset_id)?;

            if let Some(curr) = current_plugin_opt {
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
            println!("All plugins are up to date.");
        } else {
            println!("To update plugins, use: gdm update");
        }
        Ok(())
    }

    async fn update_plugins(&self) -> Result<BTreeMap<String, Plugin>> {
        let plugins_map = self.gdm_config.get_plugins()?;

        if plugins_map.is_empty() {
            bail!("No plugins installed.");
        }

        let installed_latest = self.fetch_latest_assets().await?;
        let mut plugins_to_install = Vec::new();

        for asset in installed_latest {
            if let Some(curr) = self.gdm_config.get_plugin_by_asset_id(&asset.asset_id)? {
                let latest_plugin = Plugin::from(asset);
                if latest_plugin > curr {
                    plugins_to_install.push(latest_plugin);
                }
            }
        }

        if plugins_to_install.is_empty() {
            println!("All plugins are up to date.");
            return Ok(BTreeMap::new());
        }

        let updated_plugins = self.process_install(&plugins_to_install).await?;

        self.add_plugins(&updated_plugins)?;
        println!("Plugins updated successfully.");
        Ok(updated_plugins)
    }

    async fn get_asset_list_response_by_name_or_version(
        &self,
        name: &str,
        version: &str,
    ) -> Result<AssetListResponse> {
        let parsed_version = self.godot_config.get_godot_version_from_project()?;

        if name.is_empty() {
            bail!("No name provided")
        }

        let effective_version = if version.is_empty() {
            if parsed_version.is_empty() {
                bail!(
                    "Couldn't determine Godot version from project.godot. Please provide a version using --godot-version."
                );
            }
            parsed_version
        } else {
            version.to_string()
        };

        let params = HashMap::from([
            ("filter".to_string(), name.to_string()),
            ("godot_version".to_string(), effective_version),
        ]);

        let asset_results = self.asset_store_api.get_assets(params).await?;
        Ok(asset_results)
    }

    async fn search_assets_by_name_or_version(&self, name: &str, version: &str) -> Result<()> {
        let asset_list_response = self
            .get_asset_list_response_by_name_or_version(name, version)
            .await?;

        match asset_list_response.result.len() {
            0 => println!("No assets found matching \"{}\"", name),
            1 => println!("Found 1 asset matching \"{}\":", name),
            n => println!("Found {} assets matching \"{}\":", n, name),
        }

        asset_list_response.print_info();

        if asset_list_response.result.len() == 1 {
            let asset = asset_list_response.result.first().unwrap();
            println!(
                "To install the plugin, use: gdm add \"{}\" or gdm add --asset-id {}",
                asset.title, asset.asset_id
            );
        } else {
            println!(
                "To install a plugin, use: gdm add --asset-id <asset_id> or narrow down your search"
            );
        }
        Ok(())
    }
}

pub trait PluginService {
    async fn install_all_plugins(&self) -> Result<BTreeMap<String, Plugin>>;

    async fn add_plugin(
        &self,
        asset_id: Option<String>,
        name: Option<String>,
        version: Option<String>,
        git_url: Option<String>,
        git_reference: Option<String>,
    ) -> Result<()>;

    fn add_plugins(&self, plugins: &BTreeMap<String, Plugin>) -> Result<()>;

    async fn remove_plugin_by_name(&self, name: &str) -> Result<()>;

    async fn fetch_latest_assets(&self) -> Result<Vec<AssetResponse>>;

    async fn check_outdated_plugins(&self) -> Result<()>;
    async fn update_plugins(&self) -> Result<BTreeMap<String, Plugin>>;

    async fn get_asset_list_response_by_name_or_version(
        &self,
        name: &str,
        version: &str,
    ) -> Result<AssetListResponse>;
    async fn search_assets_by_name_or_version(&self, name: &str, version: &str) -> Result<()>;

    fn finish_plugins_operation(&self, plugins: &BTreeMap<String, Plugin>) -> Result<()>;

    async fn process_install(&self, plugins: &[Plugin]) -> Result<BTreeMap<String, Plugin>>;

    async fn find_asset_metadata(
        &self,
        name: &str,
        asset_id: &str,
        version: &str,
    ) -> Result<AssetResponse>;
}

#[cfg(test)]
mod tests {
    use anyhow::Ok;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    use mockall::predicate::*;

    use crate::api::{
        Asset, AssetListItem, AssetListResponse, AssetResponse, MockDefaultAssetStoreAPI,
    };
    use crate::config::{
        DefaultAppConfig, DefaultGdmConfigMetadata, MockDefaultGdmConfig, MockDefaultGodotConfig,
    };
    use crate::installers::{AssetLibraryInstaller, GitInstaller};
    use crate::models::Plugin;
    use crate::services::{
        DefaultPluginService, MockDefaultExtractService, MockDefaultFileService,
        MockDefaultGitService, PluginParser, PluginService,
    };

    // Helper to setup the service with specific versioning scenarios
    fn setup_plugin_service_with_versions(
        asset_id: &str,
        plugin_name: &str,
        installed_version: Option<&str>,
        return_version: &str,
        search_name: Option<&str>,
    ) -> DefaultPluginService {
        let mut godot_config_repository = MockDefaultGodotConfig::default();
        let mut asset_store_api = MockDefaultAssetStoreAPI::default();
        let mut plugin_config_repository = MockDefaultGdmConfig::default();
        let mut extract_service = MockDefaultExtractService::default();
        let file_service = Arc::new(MockDefaultFileService::default());
        let git_service_mock = MockDefaultGitService::default();

        // Setup godot config repository
        godot_config_repository.expect_save().returning(|_| Ok(()));

        godot_config_repository
            .expect_get_godot_version_from_project()
            .returning(|| Ok("4.5".to_string()));

        // Setup plugin config repository
        let asset_id_clone = asset_id.to_string();
        let installed_version_clone = installed_version.map(|v| v.to_string());
        let plugin_name_clone = plugin_name.to_string();

        plugin_config_repository
            .expect_get_plugin_by_asset_id()
            .returning(move |_| {
                Ok(installed_version_clone.as_ref().map(|version| {
                    Plugin::new_asset_store_plugin(
                        asset_id_clone.clone(),
                        Some(format!("addons/{}/plugin.cfg", plugin_name_clone).into()),
                        plugin_name_clone.clone(),
                        version.clone(),
                        String::from("MIT"),
                        vec![],
                    )
                }))
            });

        plugin_config_repository
            .expect_add_plugins()
            .returning(|_| Ok(DefaultGdmConfigMetadata::default()));

        // Setup asset store API
        let asset_id_for_api = asset_id.to_string();
        let plugin_name_for_api = plugin_name.to_string();

        // Add get_assets mock if search_name is provided
        if search_name.is_none() {
            asset_store_api
                .expect_get_assets()
                .returning(|_| Ok(AssetListResponse::new(vec![])));
        }

        if let Some(_name) = search_name {
            let asset_id_for_search = asset_id.to_string();
            let plugin_name_for_search = plugin_name.to_string();

            asset_store_api.expect_get_assets().returning(move |_| {
                let asset = AssetListItem::new(
                    asset_id_for_search.clone(),
                    plugin_name_for_search.clone(),
                    "Author".to_string(),
                    "Scripts".to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "official".to_string(),
                    "11".to_string(),
                    "9.1.0".to_string(),
                    "2023-10-01".to_string(),
                );
                Ok(AssetListResponse::new(vec![asset]))
            });

            // Add get_asset_by_id mock for the name search flow
            let asset_id_for_get_by_id = asset_id.to_string();
            let plugin_name_for_get_by_id = plugin_name.to_string();

            asset_store_api
                .expect_get_asset_by_id()
                .returning(move |_| {
                    Ok(AssetResponse::new(
                        asset_id_for_get_by_id.clone(),
                        plugin_name_for_get_by_id.clone(),
                        "11".to_string(),
                        "latest".to_string(),
                        "4.5".to_string(),
                        "5".to_string(),
                        "MIT".to_string(),
                        "Some description".to_string(),
                        "GitHub".to_string(),
                        "commit_hash".to_string(),
                        "2023-10-01".to_string(),
                        format!("https://example.com/{}.zip", asset_id_for_get_by_id),
                    ))
                });
        }

        asset_store_api
            .expect_get_asset_by_id_and_version()
            .returning(move |_, version| {
                Ok(AssetResponse::new(
                    asset_id_for_api.clone(),
                    plugin_name_for_api.clone(),
                    "11".to_string(),
                    version.to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    format!("https://example.com/{}.zip", asset_id_for_api),
                ))
            });

        asset_store_api
            .expect_download_asset()
            .returning(|asset_response, _pb| {
                Ok(Asset::new(
                    PathBuf::from("test_plugin"),
                    asset_response.clone(),
                ))
            });

        let asset_id_owned = asset_id.to_string();
        let plugin_name_owned = plugin_name.to_string();
        let return_version_owned = return_version.to_string();

        asset_store_api
            .expect_find_asset_by_asset_name_and_version_and_godot_version()
            .returning(move |_, _, _| {
                // 2. The closure now owns `asset_id_owned`, which is a String, not a &str
                Ok(AssetResponse::new(
                    asset_id_owned.clone(),
                    plugin_name_owned.clone(),
                    "11".to_string(),
                    return_version_owned.clone(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    format!("https://example.com/{}.zip", asset_id_owned),
                ))
            });

        // Setup extract service
        let extract_asset_version = return_version.to_string();
        let extract_asset_id = asset_id.to_string();
        extract_service
            .expect_extract_asset()
            .returning(move |_file_path, _pb| {
                Ok((
                    String::from("test_plugin"),
                    Plugin::new_asset_store_plugin(
                        extract_asset_id.clone(),
                        Some("addons/test_plugin/plugin.cfg".into()),
                        "Test Plugin".to_string(), // title from AssetResponse in test
                        extract_asset_version.clone(), // version from AssetResponse in test
                        "MIT".to_string(),         // license from AssetResponse in test
                        vec![],
                    ),
                ))
            });

        let app_config = DefaultAppConfig::default();
        let asset_store_api_arc = Arc::new(asset_store_api);
        let extract_service_arc = Arc::new(extract_service);
        let git_service_arc = Arc::new(git_service_mock);
        let parser = Arc::new(PluginParser::new(file_service.clone()));

        // --- NEW: Initialize Installers ---
        let asset_installer =
            AssetLibraryInstaller::new(asset_store_api_arc.clone(), extract_service_arc.clone());
        let git_installer = GitInstaller::new(git_service_arc.clone(), parser);

        DefaultPluginService::new(
            Box::new(godot_config_repository),
            Box::new(plugin_config_repository),
            app_config,
            file_service,
            asset_store_api_arc,
            vec![Box::new(asset_installer), Box::new(git_installer)],
        )
    }

    // Helper to setup standard mocks
    fn setup_plugin_service_mocks() -> DefaultPluginService {
        let mut godot_config_repository = MockDefaultGodotConfig::default();

        godot_config_repository
            .expect_save()
            .returning(|_path| Ok(()));

        godot_config_repository
            .expect_validate_project_file()
            .returning(|| Ok(()));

        godot_config_repository
            .expect_get_godot_version_from_project()
            .returning(|| Ok("4.5".to_string()));

        let mut asset_store_api = MockDefaultAssetStoreAPI::default();

        let mut plugin_config_repository = MockDefaultGdmConfig::default();
        plugin_config_repository
            .expect_add_plugins()
            .returning(|_plugins| Ok(DefaultGdmConfigMetadata::new(_plugins.clone())));

        plugin_config_repository
            .expect_remove_plugins()
            .returning(|_plugin_names| Ok(DefaultGdmConfigMetadata::default()));

        plugin_config_repository
            .expect_get_plugin_by_asset_id()
            .returning(|_asset_id| Ok(None));

        plugin_config_repository
            .expect_has_installed_plugins()
            .returning(|| Ok(true));

        let app_config = DefaultAppConfig::default();
        let mut extract_service = MockDefaultExtractService::default();

        let file_service = Arc::new(MockDefaultFileService::default());

        extract_service
            .expect_extract_asset()
            .returning(|_file_path, _pb| {
                Ok((
                    String::from("test_plugin"),
                    Plugin::new_asset_store_plugin(
                        "1234".to_string(),
                        Some("addons/test_plugin/plugin.cfg".into()),
                        "Test Plugin".to_string(),
                        "1.1.1".to_string(),
                        "MIT".to_string(),
                        vec![],
                    ),
                ))
            });
        plugin_config_repository.expect_get_plugins().returning(|| {
            Ok(BTreeMap::from([(
                String::from("test_plugin"),
                Plugin::new_asset_store_plugin(
                    String::from("1234"),
                    Some("addons/test_plugin/plugin.cfg".into()),
                    String::from("Test Plugin"),
                    String::from("1.1.1"),
                    String::from("MIT"),
                    vec![],
                ),
            )]))
        });

        plugin_config_repository
            .expect_get_plugin_by_asset_id()
            .returning(|_asset_id| {
                Ok(Some(Plugin::new_asset_store_plugin(
                    "1234".to_string(),
                    Some("addons/test_plugin/plugin.cfg".into()),
                    "Test Plugin".to_string(),
                    "1.1.1".to_string(),
                    "MIT".to_string(),
                    vec![],
                )))
            });

        plugin_config_repository
            .expect_get_plugin_by_asset_id()
            .returning(|_asset_id| {
                Ok(Some(Plugin::new_asset_store_plugin(
                    "1234".to_string(),
                    Some("addons/test_plugin/plugin.cfg".into()),
                    "Test Plugin".to_string(),
                    "1.1.1".to_string(),
                    "MIT".to_string(),
                    vec![],
                )))
            });

        asset_store_api
            .expect_find_asset_by_id_or_name_and_version()
            .returning(|_, _, _| {
                Ok(AssetResponse::new(
                    "1234".to_string(),
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    "1.1.1".to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    "https://example.com/test_plugin.zip".to_string(),
                ))
            });

        asset_store_api
            .expect_get_asset_by_id_and_version()
            .with(eq("1234"), eq("1.0.0"))
            .returning(|asset_id, version| {
                Err(anyhow::anyhow!(
                    "Asset with ID {} and version {} not found",
                    asset_id,
                    version
                ))
            });
        asset_store_api
            .expect_get_asset_by_id_and_version()
            .with(eq("1234"), eq("1.1.1"))
            .returning(|asset_id, version| {
                Ok(AssetResponse::new(
                    asset_id.to_string(),
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    version.to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    "https://example.com/test_plugin.zip".to_string(),
                ))
            });
        asset_store_api
            .expect_get_asset_by_id()
            .with(eq("1234".to_string()))
            .returning(|asset_id| {
                Ok(AssetResponse::new(
                    asset_id.to_string(),
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    "1.1.1".to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    "https://example.com/test_plugin.zip".to_string(),
                ))
            });
        asset_store_api.expect_download_asset().returning(|_, _pb| {
            Ok(Asset::new(
                PathBuf::from("test_plugin"),
                AssetResponse::new(
                    "1234".to_string(),
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    "1.1.1".to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    "https://example.com/test_plugin.zip".to_string(),
                ),
            ))
        });
        asset_store_api.expect_get_assets().returning(|_params| {
            Ok(AssetListResponse::new(vec![AssetListItem::new(
                "1234".to_string(),
                "Test Plugin".to_string(),
                "Test Maker".to_string(),
                "Tools".to_string(),
                "4.5".to_string(),
                "5".to_string(),
                "MIT".to_string(),
                "??".to_string(),
                "11".to_string(),
                "1.1.1".to_string(),
                "2023-10-01".to_string(),
            )]))
        });

        let git_service_mock = MockDefaultGitService::default();

        let asset_store_api_arc = Arc::new(asset_store_api);
        let extract_service_arc = Arc::new(extract_service);
        let git_service_arc = Arc::new(git_service_mock);
        let parser = Arc::new(PluginParser::new(file_service.clone()));

        // --- NEW: Initialize Installers ---
        let asset_installer =
            AssetLibraryInstaller::new(asset_store_api_arc.clone(), extract_service_arc.clone());
        let git_installer = GitInstaller::new(git_service_arc.clone(), parser);

        DefaultPluginService::new(
            Box::new(godot_config_repository),
            Box::new(plugin_config_repository),
            app_config,
            file_service,
            asset_store_api_arc,
            vec![Box::new(asset_installer), Box::new(git_installer)],
        )
    }

    // get_asset_list_response_by_name_or_version

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_with_no_results_should_return_ok() {
        let plugin_service = setup_plugin_service_with_versions(
            "1234",
            "some_non_existent_plugin_name",
            Some("1.0.0"),
            "1.0.0",
            None,
        );
        let name = "some_non_existent_plugin_name";
        let version = "4.5";
        let result_list = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result_list.is_ok());
        let result = result_list.unwrap();
        assert!(result.result.is_empty());
    }

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_with_exact_name_should_return_one_result()
     {
        let plugin_service = setup_plugin_service_mocks();
        let name = "Test Plugin";
        let version = "4.5";
        let result = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result.is_ok());
        let assets = result.unwrap();
        assert!(assets.result.len() == 1);
        let asset = assets.result.first().unwrap();
        assert_eq!(asset.title, "Test Plugin");
        assert_eq!(asset.asset_id, "1234");
    }

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_without_name_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        let name = "";
        let version = "4.5";
        let result = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result.is_err());
    }

    // install_all_plugins

    #[tokio::test]
    async fn test_install_plugins_should_install_all_plugins_in_config() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service.install_all_plugins().await;
        assert!(result.is_ok());
        let installed_plugins = result.unwrap();

        let expected_plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new_asset_store_plugin(
                String::from("1234"),
                Some("addons/test_plugin/plugin.cfg".into()),
                String::from("Test Plugin"),
                String::from("1.1.1"),
                String::from("MIT"),
                vec![],
            ),
        )]);

        assert_eq!(installed_plugins, expected_plugins);
    }

    // add_plugin tests (Replaces old install_plugin tests)

    #[tokio::test]
    async fn test_add_plugin_with_asset_id_and_no_version_should_install_asset() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service
            .add_plugin(Some("1234".to_string()), None, None, None, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_plugin_with_only_version_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        // Providing only version
        let result = plugin_service
            .add_plugin(None, None, Some("1.1.1".to_string()), None, None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_plugin_with_asset_id_and_version_should_install_plugin() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service
            .add_plugin(
                Some("1234".to_string()),
                None,
                Some("1.1.1".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_plugin_with_name_should_install_plugin() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service
            .add_plugin(None, Some("Test Plugin".to_string()), None, None, None)
            .await;
        assert!(result.is_ok());
    }

    // Error cases for add_plugin

    #[tokio::test]
    async fn test_add_plugin_with_invalid_asset_id_should_return_err() {
        // We need mocks even for error cases if it reaches the API
        let mut godot_config_repository = MockDefaultGodotConfig::default();
        godot_config_repository
            .expect_get_godot_version_from_project()
            .returning(|| Ok("4.5".to_string()));
        let mut asset_store_api = MockDefaultAssetStoreAPI::default();
        asset_store_api
            .expect_find_asset_by_id_or_name_and_version()
            .returning(|_, _, _| Err(anyhow::anyhow!("Not found")));

        let plugin_config_repository = MockDefaultGdmConfig::default();
        let app_config = DefaultAppConfig::default();
        let file_service = Arc::new(MockDefaultFileService::default());
        let extract_service = Arc::new(MockDefaultExtractService::default());
        let _git_service = Arc::new(MockDefaultGitService::default());

        let asset_store_api_arc = Arc::new(asset_store_api);
        let installer =
            AssetLibraryInstaller::new(asset_store_api_arc.clone(), extract_service.clone());

        let plugin_service = DefaultPluginService::new(
            Box::new(godot_config_repository),
            Box::new(plugin_config_repository),
            app_config,
            file_service,
            asset_store_api_arc,
            vec![Box::new(installer)],
        );

        let result = plugin_service
            .add_plugin(Some("99999".to_string()), None, None, None, None)
            .await;
        assert!(result.is_err());
    }

    // Version comparison tests

    #[tokio::test]
    async fn test_add_plugin_when_newer_version_already_installed_should_downgrade() {
        let plugin_service = setup_plugin_service_with_versions(
            "1234",
            "Test Plugin",
            Some("2.0.0"), // Already installed version (newer)
            "1.5.0",
            None,
        );

        let result = plugin_service
            .add_plugin(
                Some("1234".to_string()),
                None,
                Some("1.5.0".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_plugin_with_name_and_version_should_install_correct_version() {
        let plugin_service = setup_plugin_service_with_versions(
            "1709",
            "GUT - Godot Unit Testing (Godot 4)",
            None, // Not installed
            "9.1.0",
            Some("Godot Unit Testing"), // Enable name search
        );

        let result = plugin_service
            .add_plugin(
                None,
                Some("Godot Unit Testing".to_string()),
                Some("9.1.0".to_string()),
                None,
                None,
            )
            .await;

        assert!(result.is_ok());
    }

    // update_plugins

    fn setup_update_plugin_mocks(
        current_plugin_version: &str,
        update_plugin_version: &str,
    ) -> DefaultPluginService {
        let mut godot_config_repository = MockDefaultGodotConfig::default();

        godot_config_repository
            .expect_save()
            .returning(|_path| Ok(()));
        godot_config_repository
            .expect_get_godot_version_from_project()
            .returning(|| Ok("4.5".to_string()));

        let mut asset_store_api = MockDefaultAssetStoreAPI::default();

        let mut plugin_config_repository = MockDefaultGdmConfig::default();
        plugin_config_repository
            .expect_add_plugins()
            .returning(|_plugins| Ok(DefaultGdmConfigMetadata::new(_plugins.clone())));

        plugin_config_repository
            .expect_remove_plugins()
            .returning(|_plugin_names| Ok(DefaultGdmConfigMetadata::default()));

        plugin_config_repository
            .expect_has_installed_plugins()
            .returning(|| Ok(true));

        let current_plugin_version_owned = current_plugin_version.to_string();

        plugin_config_repository
            .expect_get_plugin_by_asset_id()
            .returning(move |_asset_id| {
                Ok(Some(Plugin::new_asset_store_plugin(
                    "1234".to_string(),
                    Some("addons/test_plugin/plugin.cfg".into()),
                    "Test Plugin".to_string(),
                    current_plugin_version_owned.clone(),
                    "MIT".to_string(),
                    vec![],
                )))
            });

        let app_config = DefaultAppConfig::default();
        let mut extract_service = MockDefaultExtractService::default();
        let file_service = Arc::new(MockDefaultFileService::default());

        let extract_asset_version = update_plugin_version.to_string();
        extract_service
            .expect_extract_asset()
            .returning(move |_file_path, _pb| {
                Ok((
                    String::from("test_plugin"),
                    Plugin::new_asset_store_plugin(
                        "1234".to_string(),
                        Some("addons/test_plugin/plugin.cfg".into()),
                        "test_plugin".to_string(),
                        extract_asset_version.clone(),
                        "MIT".to_string(),
                        vec![],
                    ),
                ))
            });

        plugin_config_repository.expect_get_plugins().returning({
            let current_plugin_version = current_plugin_version.to_string();
            move || {
                Ok(BTreeMap::from([(
                    String::from("test_plugin"),
                    Plugin::new_asset_store_plugin(
                        String::from("1234"),
                        Some("addons/test_plugin/plugin.cfg".into()),
                        String::from("Test Plugin"),
                        current_plugin_version.clone(),
                        String::from("MIT"),
                        vec![],
                    ),
                )]))
            }
        });

        // Mocks for getting latest assets
        let _get_asset_by_id_version = current_plugin_version.to_string();
        asset_store_api
            .expect_get_asset_by_id_and_version()
            .returning(move |asset_id, version| {
                Ok(AssetResponse::new(
                    asset_id.to_string(),
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    version.to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    "https://example.com/test_plugin.zip".to_string(),
                ))
            });

        // This mock is crucial for `fetch_latest_assets` inside update_plugins
        let asset_store_plugin_version = update_plugin_version.to_string();
        asset_store_api
            .expect_find_asset_by_id_or_name_and_version()
            .returning(move |asset_id, _, _| {
                Ok(AssetResponse::new(
                    asset_id.to_string(),
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    asset_store_plugin_version.to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    "https://example.com/test_plugin.zip".to_string(),
                ))
            });

        // Needed for find_asset_metadata if add_plugin is called
        asset_store_api
            .expect_get_asset_by_id()
            .returning(move |asset_id| {
                Ok(AssetResponse::new(
                    asset_id.to_string(),
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    "1.0.0".to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    "https://example.com/test_plugin.zip".to_string(),
                ))
            });

        asset_store_api
            .expect_download_asset()
            .returning(|asset_response, _pb| {
                Ok(Asset::new(
                    PathBuf::from("test_plugin"),
                    asset_response.clone(),
                ))
            });

        let git_service_mock = MockDefaultGitService::default();
        let asset_store_api_arc = Arc::new(asset_store_api);
        let extract_service_arc = Arc::new(extract_service);
        let parser = Arc::new(PluginParser::new(file_service.clone()));

        let asset_installer =
            AssetLibraryInstaller::new(asset_store_api_arc.clone(), extract_service_arc.clone());
        let git_installer = GitInstaller::new(Arc::new(git_service_mock), parser);

        DefaultPluginService::new(
            Box::new(godot_config_repository),
            Box::new(plugin_config_repository),
            app_config,
            file_service,
            asset_store_api_arc,
            vec![Box::new(asset_installer), Box::new(git_installer)],
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
            Plugin::new_asset_store_plugin(
                String::from("1234"),
                Some("addons/test_plugin/plugin.cfg".into()),
                String::from("Test Plugin"),
                String::from("1.2.0"),
                String::from("MIT"),
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

    // remove_plugin_by_name

    #[tokio::test]
    async fn test_remove_plugin_by_name_should_remove_plugin() {
        let mut godot_config_repository = MockDefaultGodotConfig::default();
        godot_config_repository
            .expect_save()
            .returning(|_path| Ok(()));
        godot_config_repository
            .expect_validate_project_file()
            .returning(|| Ok(()));

        let mut plugin_config_repository = MockDefaultGdmConfig::default();
        plugin_config_repository
            .expect_get_plugin_by_name()
            .with(eq("test_plugin"))
            .returning(|_name| Some(("test_plugin".to_string(), Plugin::create_mock_plugin_1())));
        plugin_config_repository
            .expect_remove_plugins()
            .returning(|_names| Ok(DefaultGdmConfigMetadata::default()));
        plugin_config_repository
            .expect_has_installed_plugins()
            .returning(|| Ok(true));

        let mut file_service = MockDefaultFileService::default();
        file_service
            .expect_file_exists()
            .returning(|_path| Ok(true));
        file_service
            .expect_directory_exists()
            .returning(|_path| true);
        file_service
            .expect_remove_dir_all()
            .returning(|_path| Ok(()));

        let git_service_mock = MockDefaultGitService::default();
        let extract_service = Arc::new(MockDefaultExtractService::default());
        let asset_store = Arc::new(MockDefaultAssetStoreAPI::default());
        let file_service_arc = Arc::new(file_service);
        let parser = Arc::new(PluginParser::new(file_service_arc.clone()));

        // Setup Installers even if not used by this method directly,
        // because constructor requires them
        let asset_installer = AssetLibraryInstaller::new(asset_store.clone(), extract_service);
        let git_installer = GitInstaller::new(Arc::new(git_service_mock), parser);

        let plugin_service = DefaultPluginService::new(
            Box::new(godot_config_repository),
            Box::new(plugin_config_repository),
            DefaultAppConfig::default(),
            file_service_arc,
            asset_store,
            vec![Box::new(asset_installer), Box::new(git_installer)],
        );

        let result = plugin_service.remove_plugin_by_name("test_plugin").await;
        assert!(result.is_ok());
    }

    // finish_plugins_operation

    #[test]
    fn test_finish_plugins_operation_should_complete_successfully() {
        // Setup minimal mocks just to satisfy constructor
        let godot_config = MockDefaultGodotConfig::default();
        let plugin_config = MockDefaultGdmConfig::default();
        let app_config = DefaultAppConfig::default();
        let file_service = Arc::new(MockDefaultFileService::default());
        let asset_store = Arc::new(MockDefaultAssetStoreAPI::default());
        let extract = Arc::new(MockDefaultExtractService::default());
        let git_service = MockDefaultGitService::default();
        let parser = Arc::new(PluginParser::new(file_service.clone()));

        let asset_installer = AssetLibraryInstaller::new(asset_store.clone(), extract);
        let git_installer = GitInstaller::new(Arc::new(git_service), parser);

        let plugin_service = DefaultPluginService::new(
            Box::new(godot_config),
            Box::new(plugin_config),
            app_config,
            file_service,
            asset_store,
            vec![Box::new(asset_installer), Box::new(git_installer)],
        );

        // Updated test data: Use Vec instead of BTreeMap
        let plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new_asset_store_plugin(
                String::from("1234"),
                Some("addons/test_plugin/plugin.cfg".into()),
                String::from("Test Plugin"),
                String::from("1.1.1"),
                String::from("MIT"),
                vec![],
            ),
        )]);

        let result = plugin_service.finish_plugins_operation(&plugins);
        assert!(result.is_ok());
    }
}
