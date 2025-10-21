#[cfg(test)]
mod mod_tests;
mod operation;
mod operation_manager;

use crate::api::asset::Asset;
use crate::api::asset_list_response::AssetListResponse;
use crate::api::asset_response::AssetResponse;
use crate::api::{AssetStoreAPI, DefaultAssetStoreAPI};
use crate::app_config::{AppConfig, DefaultAppConfig};
use crate::extract_service::{DefaultExtractService, ExtractService};
use crate::file_service::{DefaultFileService, FileService};
use crate::godot_config_repository::{DefaultGodotConfigRepository, GodotConfigRepository};
use crate::plugin_config_repository::plugin::Plugin;
use crate::plugin_config_repository::{DefaultPluginConfigRepository, PluginConfigRepository};
use crate::plugin_service::operation::Operation;
use crate::plugin_service::operation_manager::OperationManager;
use crate::utils::Utils;

use anyhow::{Context, Result, anyhow};
use futures::future::try_join_all;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, error, warn};

#[cfg(not(tarpaulin_include))]
pub struct DefaultPluginService {
    pub godot_config_repository: Box<dyn GodotConfigRepository>,
    pub asset_store_api: Arc<dyn AssetStoreAPI + Send + Sync>,
    pub plugin_config_repository: Box<dyn PluginConfigRepository>,
    pub app_config: DefaultAppConfig,
    pub extract_service: Arc<dyn ExtractService + Send + Sync>,
    pub file_service: Arc<dyn FileService + Send + Sync>,
}

#[cfg(not(tarpaulin_include))]
impl Default for DefaultPluginService {
    fn default() -> Self {
        Self {
            godot_config_repository: Box::new(DefaultGodotConfigRepository::default()),
            asset_store_api: Arc::new(DefaultAssetStoreAPI::default()),
            plugin_config_repository: Box::new(DefaultPluginConfigRepository::default()),
            app_config: DefaultAppConfig::default(),
            extract_service: Arc::new(DefaultExtractService::default()),
            file_service: Arc::new(DefaultFileService),
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl DefaultPluginService {
    #[allow(unused)]
    fn new(
        godot_config_repository: Box<dyn GodotConfigRepository>,
        asset_store_api: Arc<dyn AssetStoreAPI + Send + Sync>,
        plugin_config_repository: Box<dyn PluginConfigRepository>,
        app_config: DefaultAppConfig,
        extract_service: Arc<dyn ExtractService + Send + Sync>,
        file_service: Arc<dyn FileService + Send + Sync>,
    ) -> Self {
        Self {
            godot_config_repository,
            asset_store_api,
            plugin_config_repository,
            app_config,
            extract_service,
            file_service,
        }
    }
}

impl PluginService for DefaultPluginService {
    async fn install_all_plugins(&self) -> Result<BTreeMap<String, Plugin>> {
        let assets: Vec<AssetResponse> = self.fetch_installed_assets().await?;

        let plugins = self
            .handle_plugin_operations(Operation::Install, &assets)
            .await?;

        self.add_plugins(&plugins)?;
        Ok(plugins)
    }

    /// Installs a single plugin by its name, asset ID, and version
    async fn install_plugin(
        &self,
        name: &str,
        asset_id: &str,
        version: &str,
    ) -> Result<BTreeMap<String, Plugin>> {
        let asset: AssetResponse;
        if !version.is_empty() && !asset_id.is_empty() {
            asset = self
                .find_plugin_by_asset_id_and_version(asset_id.to_string(), version.to_string())
                .await?;
        } else if !name.is_empty() || !asset_id.is_empty() {
            asset = self.find_plugin_by_id_or_name(asset_id, name).await?;
        } else {
            return Err(anyhow!("No name or asset ID provided"));
        }

        let existing_plugin = self
            .plugin_config_repository
            .get_plugin_by_asset_id(&asset.asset_id);

        if let Some(existing_plugin) = existing_plugin {
            let plugin_to_install = Plugin::from(asset.clone());
            if plugin_to_install != existing_plugin {
                let update_action = if plugin_to_install > existing_plugin {
                    "Updating"
                } else {
                    "Downgrading"
                };
                println!(
                    "Plugin '{}' is already installed with version {}. {} to version {}.",
                    existing_plugin.title,
                    existing_plugin.get_version(),
                    update_action,
                    plugin_to_install.get_version()
                );
            } else {
                println!(
                    "Plugin '{}' is already in dependencies.",
                    existing_plugin.title
                );
            }
        }

        let plugins = vec![asset];
        let installed_plugins = self
            .handle_plugin_operations(Operation::Install, &plugins)
            .await?;
        self.add_plugins(&installed_plugins)?;
        Ok(installed_plugins)
    }

    async fn handle_plugin_operations(
        &self,
        operation: Operation,
        plugins: &[AssetResponse],
    ) -> Result<BTreeMap<String, Plugin>> {
        let downloaded_plugins = self.download_plugins_operation(operation, plugins).await?;
        let extracted_plugins = self.extract_plugins_operation(&downloaded_plugins).await?;
        self.finish_plugins_operation(&extracted_plugins)?;
        Ok(extracted_plugins)
    }

    async fn download_plugins_operation(
        &self,
        operation: Operation,
        plugins: &[AssetResponse],
    ) -> Result<Vec<Asset>> {
        let mut download_tasks = JoinSet::new();
        let operation_manager = OperationManager::new(operation)?;

        for (index, _asset) in plugins.iter().enumerate() {
            let asset = _asset.clone();
            let plugin = Plugin::from(asset.clone());
            let pb_task = operation_manager.add_progress_bar(index, plugins.len(), &plugin)?;
            let asset_store_api = self.asset_store_api.clone();
            download_tasks
                .spawn(async move { asset_store_api.download_asset(&asset, pb_task).await });
        }

        let result: Vec<std::result::Result<Asset, anyhow::Error>> =
            download_tasks.join_all().await;

        operation_manager.finish();

        result.into_iter().collect::<Result<Vec<Asset>>>()
    }

    /// Downloads and extract_services all plugins under /addons folder
    async fn extract_plugins_operation(
        &self,
        plugins: &[Asset],
    ) -> Result<BTreeMap<String, Plugin>> {
        let mut extract_service_tasks = JoinSet::new();
        let operation_manager = OperationManager::new(Operation::Extract)?;

        for (index, downloaded_asset) in plugins.iter().enumerate() {
            let asset_response = downloaded_asset.asset_response.clone();
            let file_path = downloaded_asset.file_path.clone();
            let pb_task = operation_manager.add_progress_bar(
                index,
                plugins.len(),
                &Plugin::from(asset_response),
            )?;
            let extract_service = self.extract_service.clone();
            extract_service_tasks
                .spawn(async move { extract_service.extract_plugin(&file_path, pb_task).await });
        }

        while let Some(res) = extract_service_tasks.join_next().await {
            let _ = res??;
        }

        let installed_plugins = plugins
            .iter()
            .map(|downloaded_asset| {
                let asset_response = downloaded_asset.asset_response.clone();
                let plugin: Plugin = Plugin::from(asset_response);
                let plugin_name = downloaded_asset.root_folder.clone();
                (plugin_name, plugin)
            })
            .collect::<BTreeMap<String, Plugin>>();

        Ok(installed_plugins)
    }

    fn finish_plugins_operation(&self, plugins: &BTreeMap<String, Plugin>) -> Result<()> {
        let operation_manager = OperationManager::new(Operation::Finished)?;
        for (index, plugin) in plugins.values().clone().enumerate() {
            let finished_bar = operation_manager.add_progress_bar(index, plugins.len(), plugin)?;
            finished_bar.finish();
        }
        operation_manager.finish();
        Ok(())
    }

    async fn add_plugin_by_id_or_name_and_version(
        &self,
        asset_id: Option<String>,
        name: Option<String>,
        version: Option<String>,
    ) -> Result<()> {
        let _name = name.unwrap_or_default();
        let _asset_id = asset_id.unwrap_or_default();
        let _version = version.unwrap_or_default();

        if _name.is_empty() && _asset_id.is_empty() {
            return Err(anyhow!("Either name or asset ID must be provided."));
        }

        self.install_plugin(&_name, &_asset_id, &_version).await?;
        Ok(())
    }

    fn add_plugins(&self, plugins: &BTreeMap<String, Plugin>) -> Result<()> {
        let plugin_config = self.plugin_config_repository.add_plugins(plugins)?;
        self.godot_config_repository.save(plugin_config)?;
        Ok(())
    }

    async fn find_plugin_by_asset_id_and_version(
        &self,
        asset_id: String,
        version: String,
    ) -> Result<AssetResponse> {
        if !asset_id.is_empty() && !version.is_empty() {
            debug!(
                "Finding plugin by asset ID and version: {} {}",
                asset_id, version
            );
            let asset = self
                .asset_store_api
                .get_asset_by_id_and_version(&asset_id, &version)
                .await?;
            Ok(asset)
        } else {
            error!("Asset ID or version is empty");
            Err(anyhow!(
                "Both asset ID and version must be provided to search by version."
            ))
        }
    }

    async fn find_plugin_by_id_or_name(&self, asset_id: &str, name: &str) -> Result<AssetResponse> {
        if !asset_id.is_empty() {
            let asset = self.asset_store_api.get_asset_by_id(asset_id).await?;
            Ok(asset)
        } else if !name.is_empty() {
            let params = HashMap::from([
                ("filter".to_string(), name.to_string()),
                (
                    "godot_version".to_string(),
                    self.godot_config_repository
                        .get_godot_version_from_project()?,
                ),
            ]);
            let asset_results = self.asset_store_api.get_assets(params).await?;

            if asset_results.result.len() != 1 {
                return Err(anyhow!(
                    "Expected to find exactly one asset matching \"{}\", but found {}. Please refine your search or use --asset-id.",
                    name,
                    asset_results.result.len()
                ));
            }
            let asset = asset_results.result.first().unwrap();
            let asset = self
                .asset_store_api
                .get_asset_by_id(&asset.asset_id)
                .await?;

            Ok(asset)
        } else {
            error!("No name or asset ID provided: {}, {}", name, asset_id);
            println!("No name or asset ID provided");
            Err(anyhow!("No name or asset ID provided"))
        }
    }

    async fn remove_plugin_by_name(&self, name: &str) -> Result<()> {
        let installed_plugin = self.plugin_config_repository.get_plugin_key_by_name(name);
        let addon_folder = self.app_config.get_addon_folder_path();

        match installed_plugin {
            Some(plugin_name) => {
                let plugin_folder_path = Utils::plugin_name_to_addon_folder_path(
                    addon_folder,
                    Path::new(plugin_name.as_str()),
                );

                // Remove plugin directory if it exists
                if self.file_service.file_exists(&plugin_folder_path) {
                    println!(
                        "Removing plugin folder: {}",
                        plugin_folder_path.clone().display()
                    );
                    self.file_service.remove_dir_all(&plugin_folder_path)?
                } else {
                    println!("Plugin folder does not exist, trying to remove from gdm config");
                }

                // Remove plugin from plugin config
                let plugin_config = self
                    .plugin_config_repository
                    .remove_plugins(HashSet::from([plugin_name.clone()]))
                    .with_context(|| {
                        format!(
                            "Failed to remove plugin {} from plugin configuration",
                            plugin_name
                        )
                    })?;

                // Remove plugin from godot project config
                self.godot_config_repository
                    .save(plugin_config)
                    .with_context(|| {
                        format!(
                            "Failed to remove plugin {} from Godot project configuration",
                            plugin_name
                        )
                    })?;
                println!("Plugin {} removed successfully.", plugin_name);
                Ok(())
            }
            None => {
                println!("Plugin {} is not installed.", name);
                Ok(())
            }
        }
    }

    /// Fetches plugins listed in the dependency file that are pinned to a version
    async fn fetch_installed_assets(&self) -> Result<Vec<AssetResponse>> {
        let plugins = self.plugin_config_repository.get_plugins()?;

        let mut assets = Vec::new();

        for plugin in plugins.values() {
            let asset_id = plugin.asset_id.to_string();
            let version = plugin.get_version();
            let asset_request = self.find_plugin_by_asset_id_and_version(asset_id, version);
            assets.push(asset_request);
        }

        let fetched_assets: Vec<AssetResponse> = try_join_all(assets)
            .await
            .with_context(|| "Failed to fetch latest plugins from Asset Store API")?;

        Ok(fetched_assets)
    }

    /// Fetches plugins listed in the dependency file without version pinning
    async fn fetch_latest_assets(&self) -> Result<Vec<AssetResponse>> {
        let plugins = self.plugin_config_repository.get_plugins()?;

        let mut assets = Vec::new();

        for plugin in plugins.values() {
            let asset_id = plugin.asset_id.as_str();
            let asset_request = self.find_plugin_by_id_or_name(asset_id, "");
            assets.push(asset_request);
        }

        let fetched_assets: Vec<AssetResponse> = try_join_all(assets)
            .await
            .with_context(|| "Failed to fetch installed plugins from Asset Store API")?;

        Ok(fetched_assets)
    }

    /// Check for outdated plugins by comparing installed versions with the latest versions from the Asset Store
    /// Prints a list of outdated plugins with their current and latest versions, e.g.:
    /// Plugin                  Current    Latest
    /// some_plugin             1.5.0      1.6.0 (update available)
    async fn check_outdated_plugins(&self) -> Result<()> {
        let installed_plugins = self.fetch_latest_assets().await?;
        let plugins = self.plugin_config_repository.get_plugins()?;

        println!("Checking for outdated plugins");
        println!();

        let mut plugins_to_update = Vec::new();

        println!("{0: <40} {1: <20} {2: <20}", "Plugin", "Current", "Latest");
        for asset in installed_plugins {
            let current_plugin = plugins.values().find(|p| p.asset_id == asset.asset_id);
            if current_plugin.is_none() {
                warn!(
                    "Installed plugin with asset ID {} not found in plugin config repository",
                    asset.asset_id
                );
                continue;
            }

            let curr = current_plugin.unwrap();
            let other = Plugin::from(asset);
            let has_an_update = &other > curr;

            if has_an_update {
                plugins_to_update.push(other.clone());
            }

            let version = format!(
                "{} {}",
                other.get_version(),
                if has_an_update {
                    "(update available)"
                } else {
                    ""
                }
            );
            println!(
                "{0: <40} {1: <20} {2: <20}",
                curr.title,
                curr.get_version(),
                version
            );
        }
        println!();

        if plugins_to_update.is_empty() {
            println!("All plugins are up to date.");
        } else {
            println!("To update plugins, use: gdm update");
        }
        Ok(())
    }

    /// Update all installed plugins to their latest versions
    /// Downloads and installs the latest versions of all plugins that have updates available
    /// Updates the plugin configuration file with the new versions
    async fn update_plugins(&self) -> Result<BTreeMap<String, Plugin>> {
        let installed_plugins = self.fetch_latest_assets().await?;
        let plugins = self.plugin_config_repository.get_plugins()?;

        let mut plugins_to_update = Vec::new();

        for asset in installed_plugins {
            let current_plugin = plugins.values().find(|p| p.asset_id == asset.asset_id);
            if current_plugin.is_none() {
                warn!(
                    "Installed plugin with asset ID {} not found in plugin config repository",
                    asset.asset_id
                );
                continue;
            }
            let curr = current_plugin.unwrap();
            let other = Plugin::from(asset.clone());
            let has_an_update = &other > curr;

            if has_an_update {
                plugins_to_update.push(asset);
            }
        }

        if plugins_to_update.is_empty() {
            println!("All plugins are up to date.");
            return Ok(BTreeMap::new());
        }

        let updated_plugins = self
            .handle_plugin_operations(Operation::Update, &plugins_to_update)
            .await?;

        self.plugin_config_repository
            .add_plugins(&updated_plugins)?;
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
            return Err(anyhow!("No name provided"));
        }

        if version.is_empty() && parsed_version.is_empty() {
            return Err(anyhow!(
                "Couldn't determine Godot version from project.godot. Please provide a version using --godot-version."
            ));
        }

        let version = if version.is_empty() {
            parsed_version
        } else {
            version.to_string()
        };

        let params = HashMap::from([
            ("filter".to_string(), name.to_string()),
            ("godot_version".to_string(), version.to_string()),
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
    async fn install_plugin(
        &self,
        name: &str,
        asset_id: &str,
        version: &str,
    ) -> Result<BTreeMap<String, Plugin>>;

    async fn handle_plugin_operations(
        &self,
        operation: Operation,
        plugins: &[AssetResponse],
    ) -> Result<BTreeMap<String, Plugin>>;
    async fn download_plugins_operation(
        &self,
        operation: Operation,
        plugins: &[AssetResponse],
    ) -> Result<Vec<Asset>>;
    async fn extract_plugins_operation(
        &self,
        plugins: &[Asset],
    ) -> Result<BTreeMap<String, Plugin>>;
    fn finish_plugins_operation(&self, plugins: &BTreeMap<String, Plugin>) -> Result<()>;

    async fn add_plugin_by_id_or_name_and_version(
        &self,
        asset_id: Option<String>,
        name: Option<String>,
        version: Option<String>,
    ) -> Result<()>;
    fn add_plugins(&self, plugins: &BTreeMap<String, Plugin>) -> Result<()>;
    async fn find_plugin_by_asset_id_and_version(
        &self,
        asset_id: String,
        version: String,
    ) -> Result<AssetResponse>;
    async fn find_plugin_by_id_or_name(&self, asset_id: &str, name: &str) -> Result<AssetResponse>;

    async fn remove_plugin_by_name(&self, name: &str) -> Result<()>;

    async fn fetch_installed_assets(&self) -> Result<Vec<AssetResponse>>;
    async fn fetch_latest_assets(&self) -> Result<Vec<AssetResponse>>;

    async fn check_outdated_plugins(&self) -> Result<()>;
    async fn update_plugins(&self) -> Result<BTreeMap<String, Plugin>>;

    async fn get_asset_list_response_by_name_or_version(
        &self,
        name: &str,
        version: &str,
    ) -> Result<AssetListResponse>;
    async fn search_assets_by_name_or_version(&self, name: &str, version: &str) -> Result<()>;
}
