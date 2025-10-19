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
use crate::utils::Utils;

use anyhow::{Context, Result, anyhow};
use futures::future::try_join_all;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tracing::{debug, error};

#[cfg(not(tarpaulin_include))]
pub struct DefaultPluginService {
    godot_config_repository: Box<dyn GodotConfigRepository>,
    asset_store_api: Arc<dyn AssetStoreAPI + Send + Sync>,
    plugin_config_repository: Box<dyn PluginConfigRepository>,
    app_config: DefaultAppConfig,
    extract_service: Arc<dyn ExtractService + Send + Sync>,
    file_service: Arc<dyn FileService + Send + Sync>,
}

impl From<AssetResponse> for Plugin {
    fn from(asset_response: AssetResponse) -> Self {
        Plugin::new(
            asset_response.get_asset_id(),
            asset_response.get_title(),
            asset_response.get_version_string(),
            asset_response.get_license(),
        )
    }
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
    #[allow(dead_code)]
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
    fn get_plugin_config_repository(&self) -> &dyn PluginConfigRepository {
        &*self.plugin_config_repository
    }

    fn get_asset_store_api(&self) -> Arc<dyn AssetStoreAPI + Send + Sync> {
        Arc::clone(&self.asset_store_api)
    }

    fn get_godot_config_repository(&self) -> &dyn GodotConfigRepository {
        &*self.godot_config_repository
    }

    fn get_app_config(&self) -> &DefaultAppConfig {
        &self.app_config
    }

    fn get_extract_service(&self) -> Arc<dyn ExtractService + Send + Sync> {
        Arc::clone(&self.extract_service)
    }

    fn get_file_service(&self) -> Arc<dyn FileService + Send + Sync> {
        Arc::clone(&self.file_service)
    }
}

pub trait PluginService {
    fn get_app_config(&self) -> &DefaultAppConfig;
    fn get_godot_config_repository(&self) -> &dyn GodotConfigRepository;
    fn get_plugin_config_repository(&self) -> &dyn PluginConfigRepository;

    fn get_file_service(&self) -> Arc<dyn FileService + Send + Sync>;
    fn get_asset_store_api(&self) -> Arc<dyn AssetStoreAPI + Send + Sync>;
    fn get_extract_service(&self) -> Arc<dyn ExtractService + Send + Sync>;

    async fn install_all_plugins(&self) -> Result<BTreeMap<String, Plugin>> {
        let plugins = self.get_plugin_config_repository().get_plugins()?;
        self.install_plugins(&plugins).await?;

        println!();
        println!("done.");

        Ok(plugins)
    }

    fn create_finished_install_bar(
        &self,
        m: &MultiProgress,
        index: usize,
        total: usize,
        action: String,
        title: String,
        version: String,
    ) -> Result<ProgressBar> {
        let pb_task = m.add(ProgressBar::new(1));
        pb_task.set_style(
            ProgressStyle::with_template("{prefix} {msg}")
                .with_context(|| "Failed to create progress bar")?
                .progress_chars("#>-"),
        );
        pb_task.set_prefix(format!("[{}/{}]", index, total));
        pb_task.set_message(format!("{}: {} ({})", action, title, version));
        Ok(pb_task)
    }

    fn create_download_progress_bar(
        &self,
        m: &MultiProgress,
        index: usize,
        total: usize,
        action: String,
        title: String,
        version: String,
    ) -> Result<ProgressBar> {
        let pb_task = m.add(ProgressBar::new(5000000));
        pb_task.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} {prefix} {msg} [{elapsed_precise}] {bytes} ({bytes_per_sec})",
            )
            .with_context(|| "Failed to create progress bar style")?
            .progress_chars("#>-"),
        );
        pb_task.set_prefix(format!("[{}/{}]", index, total));
        pb_task.set_message(format!("{}: {} ({})", action, title, version));
        Ok(pb_task)
    }

    fn create_extract_service_progress_bar(
        &self,
        m: &MultiProgress,
        index: usize,
        total: usize,
        action: String,
        title: String,
        version: String,
    ) -> Result<ProgressBar> {
        let pb_task = m.add(ProgressBar::new(5000000));
        pb_task.set_style(ProgressStyle::with_template("{spinner:.green} {prefix} {msg} [{elapsed_precise}] [{bar:.cyan/blue}] {pos:>7}/{len:7} ({eta})")
        .with_context(|| "Failed to create progress bar style")?
        .progress_chars("#>-"));
        pb_task.set_prefix(format!("[{}/{}]", index, total));
        pb_task.set_message(format!("{}: {} ({})", action, title, version));
        Ok(pb_task)
    }

    /// Installs a single plugin by its name, asset ID, and version
    async fn install_plugin(
        &self,
        name: String,
        asset_id: String,
        version: String,
    ) -> Result<BTreeMap<String, Plugin>> {
        let asset: AssetResponse;
        if !version.is_empty() && !asset_id.is_empty() {
            let _asset_id = asset_id.clone();
            let _version = version.clone();

            asset = self
                .find_plugin_by_asset_id_and_version(_asset_id, _version)
                .await?;
        } else if !name.is_empty() || !asset_id.is_empty() {
            let _name = name.clone();
            let _asset_id = asset_id.clone();
            asset = self.find_plugin_by_id_or_name(_asset_id, _name).await?;
        } else {
            return Err(anyhow!("No name or asset ID provided"));
        }

        let m = MultiProgress::new();
        let title = asset.get_title().to_string();
        let version = asset.get_version_string().to_string();

        let pb_download = self.create_download_progress_bar(
            &m,
            1,
            1,
            String::from("Downloading"),
            title,
            version,
        )?;
        let downloaded_asset = self
            .get_asset_store_api()
            .download_asset(&asset.clone(), pb_download)
            .await?;
        let asset_response = downloaded_asset.get_asset_response();
        let pb_extract_service = self.create_extract_service_progress_bar(
            &m,
            1,
            1,
            String::from("ExtractServiceing"),
            asset_response.get_title().to_string(),
            asset_response.get_version_string().to_string(),
        )?;

        let plugin_root_folder = self
            .get_extract_service()
            .extract_plugin(downloaded_asset.get_file_path(), pb_extract_service)
            .await?;

        let plugin_name = plugin_root_folder.display().to_string();
        let plugins = BTreeMap::from([(plugin_name, Plugin::from(asset))]);
        self.add_plugins(&plugins)?;
        Ok(plugins)
    }

    /// Downloads and extract_services all plugins under /addons folder
    async fn download_and_extract_service_plugins(
        &self,
        main_start_message: String,
        plugins: Vec<AssetResponse>,
    ) -> Result<BTreeMap<String, Plugin>> {
        let mut download_tasks = JoinSet::new();
        let pb_multi = MultiProgress::new();
        let pb_main = pb_multi.add(ProgressBar::new(0));
        pb_main.set_style(ProgressStyle::with_template("{spinner:.green} {msg}")?);
        pb_main.set_message(main_start_message);
        pb_main.enable_steady_tick(Duration::from_millis(100));

        for (index, asset) in plugins.iter().enumerate() {
            let plugin = asset.clone();
            let pb_task = self.create_download_progress_bar(
                &pb_multi,
                index + 1,
                plugins.len(),
                String::from("Downloading"),
                plugin.get_title().to_string(),
                plugin.get_version_string().to_string(),
            )?;
            let asset_store_api = self.get_asset_store_api().clone();
            download_tasks
                .spawn(async move { asset_store_api.download_asset(&plugin, pb_task).await });
        }

        let result = download_tasks.join_all().await;
        let download_tasks = result.into_iter().collect::<Result<Vec<Asset>>>()?;

        let mut extract_service_tasks = JoinSet::new();

        pb_main.set_message("Extracting plugins");

        for (index, downloaded_asset) in download_tasks.clone().into_iter().enumerate() {
            let asset_response = downloaded_asset.get_asset_response().clone();
            let file_path = downloaded_asset.get_file_path().clone();
            let pb_task = self.create_extract_service_progress_bar(
                &pb_multi,
                index + 1,
                download_tasks.len(),
                String::from("Extracting"),
                asset_response.get_title().to_string(),
                asset_response.get_version_string().to_string(),
            )?;
            let extract_service = self.get_extract_service().clone();
            extract_service_tasks
                .spawn(async move { extract_service.extract_plugin(&file_path, pb_task).await });
        }

        while let Some(res) = extract_service_tasks.join_next().await {
            let _ = res??;
        }

        let installed_plugins = download_tasks
            .into_iter()
            .map(|downloaded_asset| {
                let asset_response = downloaded_asset.get_asset_response().clone();
                let plugin: Plugin = Plugin::from(asset_response.clone());
                let plugin_name = downloaded_asset.get_root_folder().to_string();
                (plugin_name, plugin)
            })
            .collect::<BTreeMap<String, Plugin>>();
        pb_main.finish_and_clear();

        for (index, (_, plugin)) in installed_plugins.iter().enumerate() {
            let finished_bar = self.create_finished_install_bar(
                &pb_multi,
                index + 1,
                installed_plugins.len(),
                String::from("Installed"),
                plugin.get_title().to_string(),
                plugin.get_version().to_string(),
            )?;
            finished_bar.finish();
        }

        Ok(installed_plugins)
    }

    /// Install all plugins from the given list of Plugins
    async fn install_plugins(
        &self,
        plugins: &BTreeMap<String, Plugin>,
    ) -> Result<BTreeMap<String, Plugin>> {
        let mut asset = Vec::new();
        for plugin in plugins.values() {
            let asset_id = plugin.get_asset_id().clone();
            let version = plugin.get_version().clone();
            let asset_request = self.find_plugin_by_asset_id_and_version(asset_id, version);
            asset.push(asset_request);
        }

        let assets: Vec<AssetResponse> = try_join_all(asset).await?;

        let installed_plugins = self
            .download_and_extract_service_plugins(String::from("Downloading plugins"), assets)
            .await?;

        self.add_plugins(&installed_plugins)?;

        Ok(installed_plugins)
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

        self.install_plugin(_name, _asset_id, _version).await?;
        Ok(())
    }

    fn add_plugins(&self, plugins: &BTreeMap<String, Plugin>) -> Result<()> {
        let godot_config_repository = self.get_godot_config_repository();
        let plugin_config_repository = self.get_plugin_config_repository();
        let plugin_config = plugin_config_repository.add_plugins(plugins)?;
        godot_config_repository.save(plugin_config)?;
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
                .get_asset_store_api()
                .get_asset_by_id_and_version(asset_id, version)
                .await?;
            Ok(asset)
        } else {
            error!("Asset ID or version is empty");
            Err(anyhow!(
                "Both asset ID and version must be provided to search by version."
            ))
        }
    }

    async fn find_plugin_by_id_or_name(
        &self,
        asset_id: String,
        name: String,
    ) -> Result<AssetResponse> {
        if !asset_id.is_empty() {
            let asset = self
                .get_asset_store_api()
                .get_asset_by_id(asset_id.clone())
                .await?;
            Ok(asset)
        } else if !name.is_empty() {
            let params = HashMap::from([
                ("filter".to_string(), name.clone()),
                ("godot_version".to_string(), "4.5".to_string()),
            ]);
            let asset_results = self.get_asset_store_api().get_assets(params).await?;

            if asset_results.get_result_len() != 1 {
                return Err(anyhow!(
                    "Expected to find exactly one asset matching \"{}\", but found {}. Please refine your search or use --asset-id.",
                    name,
                    asset_results.get_result_len()
                ));
            }
            let asset = asset_results.get_asset_list_item_by_index(0).unwrap();
            let id = asset.get_asset_id().to_owned();
            let asset = self
                .get_asset_store_api()
                .get_asset_by_id(id.clone())
                .await?;

            // self.get_plugin_config()
            //     .check_if_plugin_already_installed_by_asset_id(asset.get_asset_id());
            // TODO check if plugin is already installed to addons folder
            // let existing_plugin = plugin_config.get_plugin_by_asset_id(asset.get_asset_id().to_string());

            // match existing_plugin {
            //     Some(plugin) => {
            //         if plugin.version == asset.get_version_string() {
            //             return Err(anyhow!(
            //                 "Plugin {} is already installed with the same version {}.",
            //                 plugin.title,
            //                 plugin.version
            //             ));
            //         }
            //         println!(
            //             "Plugin {} is already installed with version {}. Updating to version {}.",
            //             plugin.title,
            //             plugin.version,
            //             asset.get_version_string()
            //         );
            //     }
            //     None => {}
            // }
            Ok(asset)
        } else {
            error!("No name or asset ID provided: {}, {}", name, asset_id);
            println!("No name or asset ID provided");
            Err(anyhow!("No name or asset ID provided"))
        }
    }

    async fn remove_plugin_by_name(&self, name: &str) -> Result<()> {
        let godot_config_repository = self.get_godot_config_repository();
        let plugin_config_repository = self.get_plugin_config_repository();

        let installed_plugin = plugin_config_repository.get_plugin_key_by_name(name);
        let addon_folder = self.get_app_config().get_addon_folder_path();

        match installed_plugin {
            Some(plugin_name) => {
                let plugin_folder_path = Utils::plugin_name_to_addon_folder_path(
                    Path::new(plugin_name.as_str()),
                    addon_folder,
                );

                // Remove plugin directory if it exists
                if self.get_file_service().file_exists(&plugin_folder_path) {
                    println!(
                        "Removing plugin folder: {}",
                        plugin_folder_path.clone().display()
                    );
                    self.get_file_service()
                        .remove_dir_all(&plugin_folder_path)?
                } else {
                    println!("Plugin folder does not exist, trying to remove from gdm config");
                }

                // Remove plugin from plugin config
                let plugin_config = plugin_config_repository
                    .remove_plugins(HashSet::from([plugin_name.clone()]))
                    .with_context(|| {
                        format!(
                            "Failed to remove plugin {} from plugin configuration",
                            plugin_name
                        )
                    })?;

                // Remove plugin from godot project config
                godot_config_repository
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

    /// Check for outdated plugins by comparing installed versions with the latest versions from the Asset Store
    /// Prints a list of outdated plugins with their current and latest versions, e.g.:
    /// Plugin                  Current    Latest
    /// some_plugin             1.5.0      1.6.0 (update available)
    async fn check_outdated_plugins(&self) -> Result<()> {
        let plugin_config_repository = self.get_plugin_config_repository();

        println!("Checking for outdated plugins");
        println!();

        let plugins = plugin_config_repository.get_plugins()?;

        println!("{0: <40} {1: <20} {2: <20}", "Plugin", "Current", "Latest");
        let mut updated_plugins = Vec::new();

        for plugin in plugins {
            let asset = self
                .get_asset_store_api()
                .get_asset_by_id(plugin.1.get_asset_id().clone())
                .await?;

            let has_an_update =
                asset.get_version_string().cmp(&plugin.1.get_version()) == Ordering::Greater;

            if has_an_update {
                updated_plugins.push(asset.clone());
            }

            let version = format!(
                "{} {}",
                asset.get_version_string(),
                if has_an_update {
                    "(update available)"
                } else {
                    ""
                }
            );
            println!(
                "{0: <40} {1: <20} {2: <20}",
                plugin.0,
                plugin.1.get_version(),
                version
            );
        }
        println!();

        if updated_plugins.is_empty() {
            println!("All plugins are up to date.");
        } else {
            println!("To update a plugins, use: gdm update");
        }
        Ok(())
    }

    /// Update all installed plugins to their latest versions
    /// Downloads and installs the latest versions of all plugins that have updates available
    /// Updates the plugin configuration file with the new versions
    async fn update_plugins(&self) -> Result<BTreeMap<String, Plugin>> {
        // TODO Not working
        let plugin_config_repository = self.get_plugin_config_repository();
        let plugins = plugin_config_repository.get_plugins()?;

        debug!("Checking for plugin updates for {:?} plugins", plugins);

        let mut plugins_to_update = Vec::new();
        let mut updated_plugins = BTreeMap::new();

        for (_, plugin) in plugins {
            let asset = self
                .get_asset_store_api()
                .get_asset_by_id(plugin.get_asset_id().clone())
                .await?;
            let asset_plugin = Plugin::from(asset.clone());
            debug!(
                "Comparing plugin {} version {} with latest version {}",
                plugin.get_title(),
                plugin.get_version(),
                asset.get_version_string()
            );
            if asset.get_version_string().cmp(&plugin.get_version()) == Ordering::Greater {
                plugins_to_update.push(asset);
            }
        }

        if plugins_to_update.is_empty() {
            println!("All plugins are up to date.");
        } else {
            updated_plugins = self
                .download_and_extract_service_plugins(
                    String::from("Updating plugins"),
                    plugins_to_update.clone(),
                )
                .await?;
            plugin_config_repository.add_plugins(&updated_plugins)?;
            println!("Plugins updated successfully.");
        }
        Ok(updated_plugins)
    }

    async fn get_asset_list_response_by_name_or_version(
        &self,
        name: String,
        version: String,
    ) -> Result<AssetListResponse> {
        let godot_config_repository = self.get_godot_config_repository();
        let parsed_version = godot_config_repository.get_godot_version_from_project()?;

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
            version
        };

        let params = HashMap::from([
            ("filter".to_string(), name),
            ("godot_version".to_string(), version),
        ]);
        let asset_results = self.get_asset_store_api().get_assets(params).await?;
        Ok(asset_results)
    }

    async fn search_assets_by_name_or_version(&self, name: String, version: String) -> Result<()> {
        let asset_list_response = self
            .get_asset_list_response_by_name_or_version(name.clone(), version.clone())
            .await?;

        match asset_list_response.get_result_len() {
            0 => println!("No assets found matching \"{}\"", name.clone()),
            1 => println!("Found 1 asset matching \"{}\":", name.clone()),
            n => println!("Found {} assets matching \"{}\":", n, name),
        }

        asset_list_response.print_info();

        if asset_list_response.get_result_len() == 1 {
            let asset = asset_list_response.get_asset_list_item_by_index(0).unwrap();
            println!(
                "To install the plugin, use: gdm add \"{}\" or gdm add --asset-id {}",
                asset.get_title(),
                asset.get_asset_id()
            );
        } else {
            println!(
                "To install a plugin, use: gdm add --asset-id <asset_id> or narrow down your search"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::api::MockDefaultAssetStoreAPI;
    use crate::api::asset_list_response::AssetListItem;
    use crate::extract_service::MockDefaultExtractService;
    use crate::file_service::MockDefaultFileService;
    use crate::godot_config_repository::MockDefaultGodotConfigRepository;
    use crate::plugin_config_repository::MockDefaultPluginConfigRepository;
    use crate::plugin_config_repository::plugin_config::DefaultPluginConfig;

    use super::*;
    use mockall::predicate::*;

    fn setup_plugin_service() -> DefaultPluginService {
        DefaultPluginService::default()
    }

    // find_plugin_by_id_or_name

    #[tokio::test]
    async fn test_find_plugin_by_id_or_name_with_id_with_none_parameters_should_return_error() {
        let plugin_service = setup_plugin_service();
        let asset_id = String::from("");
        let name = String::from("");
        let result = plugin_service
            .find_plugin_by_id_or_name(asset_id, name)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_plugin_by_id_or_name_with_id_should_return_asset() {
        let plugin_service = setup_plugin_service();
        let asset_id = String::from("1709");
        let name = String::from("");
        let result = plugin_service
            .find_plugin_by_id_or_name(asset_id, name)
            .await;
        assert!(result.is_ok());
        let asset = result.unwrap();
        assert_eq!(asset.get_asset_id(), "1709");
    }

    #[tokio::test]
    async fn test_find_plugin_by_id_or_name_with_name_should_return_asset() {
        let plugin_service = setup_plugin_service();
        let asset_id = String::from("");
        let name = String::from("Godot Unit Testing");
        let result = plugin_service
            .find_plugin_by_id_or_name(asset_id, name)
            .await;

        assert!(result.is_ok());
        let asset = result.unwrap();
        assert_eq!(asset.get_title(), "GUT - Godot Unit Testing (Godot 4)");
        assert_eq!(asset.get_asset_id(), "1709");
    }

    // find_plugin_by_asset_id_and_version

    #[tokio::test]
    async fn test_find_plugin_by_asset_id_and_version_missing_version_should_return_err() {
        let plugin_service = setup_plugin_service();
        let asset_id = String::from("1709");
        let version = String::from("");
        let result = plugin_service
            .find_plugin_by_asset_id_and_version(asset_id, version)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_plugin_by_asset_id_and_version_missing_asset_id_should_return_err() {
        let plugin_service = setup_plugin_service();
        let asset_id = String::from("");
        let version = String::from("9.1.0");
        let result = plugin_service
            .find_plugin_by_asset_id_and_version(asset_id, version)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_plugin_by_asset_id_and_version_should_return_asset() {
        let plugin_service = setup_plugin_service();
        let asset_id = String::from("1709");
        let version = String::from("9.1.0");
        let result = plugin_service
            .find_plugin_by_asset_id_and_version(asset_id, version)
            .await;

        assert!(result.is_ok());
        let asset = result.unwrap();
        assert_eq!(asset.get_title(), "GUT - Godot Unit Testing (Godot 4)");
        assert_eq!(asset.get_asset_id(), "1709");
        assert_eq!(asset.get_version_string(), "9.1.0");
    }

    // get_asset_list_response_by_name_or_version

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_with_no_results_should_return_ok() {
        let plugin_service = setup_plugin_service();
        let name = "some_non_existent_plugin_name".to_string();
        let version = "4.5".to_string();
        let result_list = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result_list.is_ok());
        let result = result_list.unwrap();
        assert!(result.get_result_len() == 0);
    }

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_with_exact_name_should_return_one_result()
     {
        let plugin_service = setup_plugin_service();
        let name = "Godot Unit Testing".to_string();
        let version = "4.5".to_string();
        let result = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result.is_ok());
        let assets = result.unwrap();
        assert!(assets.get_result_len() == 1);
        let asset = assets.get_asset_list_item_by_index(0).unwrap();
        assert_eq!(asset.get_title(), "GUT - Godot Unit Testing (Godot 4)");
        assert_eq!(asset.get_asset_id(), "1709");
    }

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_without_name_should_return_err() {
        let plugin_service = setup_plugin_service();
        let name = "".to_string();
        let version = "4.5".to_string();
        let result = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_without_version_should_return_response()
     {
        let plugin_service = setup_plugin_service();
        let name = "Godot Unit Testing".to_string();
        let version = "".to_string();
        let result = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_without_name_or_version_should_return_err()
     {
        let plugin_service = setup_plugin_service();
        let name = String::new();
        let version = String::new();
        let result = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result.is_err());
    }

    fn setup_plugin_service_mocks() -> DefaultPluginService {
        let mut godot_config_repository = MockDefaultGodotConfigRepository::default();

        godot_config_repository
            .expect_save()
            .returning(|_path| Ok(()));

        let mut asset_store_api = MockDefaultAssetStoreAPI::default();

        let mut plugin_config_repository = MockDefaultPluginConfigRepository::default();
        plugin_config_repository
            .expect_add_plugins()
            .returning(|_plugins| Ok(DefaultPluginConfig::new(_plugins.clone())));

        plugin_config_repository
            .expect_remove_plugins()
            .returning(|_plugin_names| Ok(DefaultPluginConfig::default()));

        let app_config = DefaultAppConfig::default();
        let mut extract_service = MockDefaultExtractService::default();

        let file_service = Arc::new(MockDefaultFileService::default());

        extract_service
            .expect_extract_plugin()
            .returning(|_file_path, _pb| Ok(PathBuf::from("test_plugin")));
        plugin_config_repository.expect_get_plugins().returning(|| {
            Ok(BTreeMap::from([(
                String::from("test_plugin"),
                Plugin::new(
                    String::from("1234"),
                    String::from("Test Plugin"),
                    String::from("1.1.1"),
                    String::from("MIT"),
                ),
            )]))
        });
        asset_store_api
            .expect_get_asset_by_id_and_version()
            .with(eq("1234".to_string()), eq("1.1.1".to_string()))
            .returning(|asset_id, version| {
                Ok(AssetResponse::new(
                    asset_id,
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    version,
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
                    asset_id,
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
            .expect_download_asset()
            .returning(|asset_response, _pb| {
                Ok(Asset::new(
                    "test_plugin".to_string(),
                    PathBuf::from("test_plugin"),
                    asset_response.clone(),
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
        DefaultPluginService::new(
            Box::new(godot_config_repository),
            Arc::new(asset_store_api),
            Box::new(plugin_config_repository),
            app_config,
            Arc::new(extract_service),
            file_service,
        )
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
            Plugin::new(
                String::from("1234"),
                String::from("Test Plugin"),
                String::from("1.1.1"),
                String::from("MIT"),
            ),
        )]);

        assert_eq!(installed_plugins, expected_plugins);
    }

    // TODO test error case for install_all_plugins

    // install_plugins

    #[tokio::test]
    async fn test_install_plugin_with_asset_id_and_no_version_should_install_asset() {
        let plugin_service = setup_plugin_service_mocks();
        let name = String::default();
        let asset_id = String::from("1234");
        let version = String::default();
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_ok());
        let installed_plugins = result.unwrap();

        let expected_plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new(
                String::from("1234"),
                String::from("Test Plugin"),
                String::from("1.1.1"),
                String::from("MIT"),
            ),
        )]);

        assert_eq!(installed_plugins, expected_plugins);
    }

    #[tokio::test]
    async fn test_install_plugin_with_only_version_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        let name = String::default();
        let asset_id = String::default();
        let version = String::from("1.1.1");
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_install_plugin_with_asset_id_and_version_should_install_plugin() {
        let plugin_service = setup_plugin_service_mocks();
        let name = String::new();
        let asset_id = String::from("1234");
        let version = String::from("1.1.1");
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_ok());

        let installed_plugins = result.unwrap();
        let expected_plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new(
                String::from("1234"),
                String::from("Test Plugin"),
                String::from("1.1.1"),
                String::from("MIT"),
            ),
        )]);
        assert_eq!(installed_plugins, expected_plugins);
    }

    #[tokio::test]
    async fn test_install_plugin_with_name_should_install_plugin() {
        let plugin_service = setup_plugin_service_mocks();
        let name = String::from("Test Plugin");
        let asset_id = String::new();
        let version = String::new();
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_ok());

        let installed_plugins = result.unwrap();
        let expected_plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new(
                String::from("1234"),
                String::from("Test Plugin"),
                String::from("1.1.1"),
                String::from("MIT"),
            ),
        )]);
        assert_eq!(installed_plugins, expected_plugins);
    }

    // TODO test error case for install_plugin

    // download_and_extract_plugins

    #[tokio::test]
    async fn test_download_and_extract_service_plugins_should_return_correct_plugins() {
        let plugin_service = setup_plugin_service_mocks();
        let plugin = vec![AssetResponse::new(
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
        )];
        let result = plugin_service
            .download_and_extract_service_plugins("".to_string(), plugin)
            .await;
        assert!(result.is_ok());

        let extracted_plugins = result.unwrap();
        let expected_plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new(
                String::from("1234"),
                String::from("Test Plugin"),
                String::from("1.1.1"),
                String::from("MIT"),
            ),
        )]);
        assert_eq!(extracted_plugins, expected_plugins);
    }

    // add_plugin_by_id_or_name_and_version
    // add_plugins_to_godot_project
    // remove_plugin_by_name
    // check_outdated_plugins
    // update_plugins

    fn setup_update_plugin_mocks(
        current_plugin_version: String,
        update_plugin_version: String,
    ) -> DefaultPluginService {
        let mut godot_config_repository = MockDefaultGodotConfigRepository::default();

        godot_config_repository
            .expect_save()
            .returning(|_path| Ok(()));

        let mut asset_store_api = MockDefaultAssetStoreAPI::default();

        let mut plugin_config_repository = MockDefaultPluginConfigRepository::default();
        plugin_config_repository
            .expect_add_plugins()
            .returning(|_plugins| Ok(DefaultPluginConfig::new(_plugins.clone())));

        plugin_config_repository
            .expect_remove_plugins()
            .returning(|_plugin_names| Ok(DefaultPluginConfig::default()));

        let app_config = DefaultAppConfig::default();
        let mut extract_service = MockDefaultExtractService::default();

        let file_service = Arc::new(MockDefaultFileService::default());

        extract_service
            .expect_extract_plugin()
            .returning(|_file_path, _pb| Ok(PathBuf::from("test_plugin")));

        let plugin_config_rep_plugin_version = current_plugin_version.clone();
        plugin_config_repository
            .expect_get_plugins()
            .returning(move || {
                Ok(BTreeMap::from([(
                    String::from("test_plugin"),
                    Plugin::new(
                        String::from("1234"),
                        String::from("Test Plugin"),
                        plugin_config_rep_plugin_version.clone(),
                        String::from("MIT"),
                    ),
                )]))
            });
        asset_store_api
            .expect_get_asset_by_id_and_version()
            .with(eq("1234".to_string()), eq(current_plugin_version.clone()))
            .returning(|asset_id, version| {
                Ok(AssetResponse::new(
                    asset_id,
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    version,
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
        let asset_store_plugin_version = update_plugin_version.clone();
        asset_store_api
            .expect_get_asset_by_id()
            .with(eq("1234".to_string()))
            .returning(move |asset_id| {
                Ok(AssetResponse::new(
                    asset_id,
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    asset_store_plugin_version.clone(),
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
                    "test_plugin".to_string(),
                    PathBuf::from("test_plugin"),
                    asset_response.clone(),
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
        DefaultPluginService::new(
            Box::new(godot_config_repository),
            Arc::new(asset_store_api),
            Box::new(plugin_config_repository),
            app_config,
            Arc::new(extract_service),
            file_service,
        )
    }

    #[tokio::test]
    async fn test_update_plugins_should_return_correct_plugins_if_there_is_an_update_1() {
        let plugin_service = setup_update_plugin_mocks("1.1.1".to_string(), "1.2.0".to_string());
        let result = plugin_service.update_plugins().await;
        assert!(result.is_ok());

        let updated_plugins = result.unwrap();
        let expected_updated_plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new(
                String::from("1234"),
                String::from("Test Plugin"),
                String::from("1.2.0"),
                String::from("MIT"),
            ),
        )]);
        assert_eq!(updated_plugins, expected_updated_plugins);
    }

    #[tokio::test]
    async fn test_update_plugins_should_return_correct_plugins_if_there_is_an_update_2() {
        let plugin_service = setup_update_plugin_mocks("1.1.1".to_string(), "1.1.12".to_string());
        let result = plugin_service.update_plugins().await;
        assert!(result.is_ok());

        let updated_plugins = result.unwrap();
        let expected_updated_plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new(
                String::from("1234"),
                String::from("Test Plugin"),
                String::from("1.1.12"),
                String::from("MIT"),
            ),
        )]);
        assert_eq!(updated_plugins, expected_updated_plugins);
    }

    #[tokio::test]
    async fn test_update_plugins_should_return_correct_plugins_if_there_is_no_update() {
        let plugin_service = setup_update_plugin_mocks("1.1.1".to_string(), "1.1.1".to_string());
        let result = plugin_service.update_plugins().await;
        assert!(result.is_ok());

        let updated_plugins = result.unwrap();
        let expected_updated_plugins = BTreeMap::from([]);
        assert_eq!(updated_plugins, expected_updated_plugins);
    }

    // search_assets_by_name_or_version
}
