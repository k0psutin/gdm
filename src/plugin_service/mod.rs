pub mod asset_library_installer;
pub mod git_installer;
pub mod plugin_installer;

#[cfg(test)]
mod mod_tests;

use crate::api::asset_list_response::AssetListResponse;
use crate::api::asset_response::AssetResponse;
use crate::api::{AssetStoreAPI, DefaultAssetStoreAPI};
use crate::app_config::{AppConfig, DefaultAppConfig};
use crate::extract_service::DefaultExtractService;
use crate::file_service::{DefaultFileService, FileService};
use crate::git_service::DefaultGitService;
use crate::godot_config_repository::{DefaultGodotConfigRepository, GodotConfigRepository};
use crate::operation_manager::OperationManager;
use crate::operation_manager::operation::Operation;
use crate::plugin_config_repository::plugin::{Plugin, PluginSource};
use crate::plugin_config_repository::{DefaultPluginConfigRepository, PluginConfigRepository};
use crate::plugin_service::asset_library_installer::AssetLibraryInstaller;
use crate::plugin_service::git_installer::GitInstaller;
use crate::plugin_service::plugin_installer::PluginInstaller;
use crate::utils::Utils;

use anyhow::{Context, Result, bail};
use futures::future::try_join_all;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tracing::info;

pub struct DefaultPluginService {
    pub godot_config_repository: Box<dyn GodotConfigRepository>,
    pub plugin_config_repository: Box<dyn PluginConfigRepository>,

    pub app_config: DefaultAppConfig,
    pub file_service: Arc<dyn FileService + Send + Sync>,
    pub asset_store_api: Arc<dyn AssetStoreAPI + Send + Sync>,

    installers: Vec<Box<dyn PluginInstaller>>,
}

impl Default for DefaultPluginService {
    fn default() -> Self {
        let asset_store_api = Arc::new(DefaultAssetStoreAPI::default());
        let extract_service = Arc::new(DefaultExtractService::default());
        let git_service = Arc::new(DefaultGitService::default());

        let asset_installer = AssetLibraryInstaller::new(asset_store_api.clone(), extract_service);
        let git_installer = GitInstaller::new(git_service);

        Self {
            godot_config_repository: Box::new(DefaultGodotConfigRepository::default()),
            plugin_config_repository: Box::new(DefaultPluginConfigRepository::default()),
            app_config: DefaultAppConfig::default(),
            file_service: Arc::new(DefaultFileService),
            asset_store_api,
            installers: vec![Box::new(asset_installer), Box::new(git_installer)],
        }
    }
}

impl DefaultPluginService {
    #[allow(unused)]
    pub fn new(
        godot_config_repository: Box<dyn GodotConfigRepository>,
        plugin_config_repository: Box<dyn PluginConfigRepository>,
        app_config: DefaultAppConfig,
        file_service: Arc<dyn FileService + Send + Sync>,
        asset_store_api: Arc<dyn AssetStoreAPI + Send + Sync>,
        installers: Vec<Box<dyn PluginInstaller>>,
    ) -> Self {
        Self {
            godot_config_repository,
            plugin_config_repository,
            app_config,
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
        let godot_version = self
            .godot_config_repository
            .get_godot_version_from_project()?;

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
        if !self.plugin_config_repository.has_installed_plugins()? {
            bail!("No plugins installed.");
        }

        let all_plugins_map = self.plugin_config_repository.get_plugins()?;
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
                .plugin_config_repository
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
        let plugin_config = self.plugin_config_repository.add_plugins(plugins)?;
        self.godot_config_repository.save(plugin_config)?;
        info!(
            "Added {} plugins to configuration successfully",
            plugins.len()
        );
        Ok(())
    }

    async fn remove_plugin_by_name(&self, name: &str) -> Result<()> {
        if !self.plugin_config_repository.has_installed_plugins()? {
            bail!("No plugins installed.");
        }

        let installed_plugin = self.plugin_config_repository.get_plugin_by_name(name);
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
                    .plugin_config_repository
                    .remove_plugins(HashSet::from([plugin_name.clone()]))
                    .context(format!(
                        "Failed to remove plugin {} from configuration",
                        plugin_name
                    ))?;

                self.godot_config_repository.save(plugin_config)?;
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
        let plugins = self.plugin_config_repository.get_plugins()?;
        let godot_version = self
            .godot_config_repository
            .get_godot_version_from_project()?;

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
        if !self.plugin_config_repository.has_installed_plugins()? {
            bail!("No plugins installed.");
        }

        let installed_latest = self.fetch_latest_assets().await?;
        let mut plugins_to_update = Vec::new();

        println!("{0: <40} {1: <20} {2: <20}", "Plugin", "Current", "Latest");

        for asset in installed_latest {
            let current_plugin_opt = self
                .plugin_config_repository
                .get_plugin_by_asset_id(&asset.asset_id)?;

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
        let plugins_map = self.plugin_config_repository.get_plugins()?;

        if plugins_map.is_empty() {
            bail!("No plugins installed.");
        }

        let installed_latest = self.fetch_latest_assets().await?;
        let mut plugins_to_install = Vec::new();

        for asset in installed_latest {
            if let Some(curr) = self
                .plugin_config_repository
                .get_plugin_by_asset_id(&asset.asset_id)?
            {
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
        let parsed_version = self
            .godot_config_repository
            .get_godot_version_from_project()?;

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
