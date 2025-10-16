use crate::api::asset_list_response::AssetListResponse;
use crate::api::asset_response::AssetResponse;
use crate::api::{AssetStoreAPI, AssetStoreAPIImpl, DownloadedPlugin};
use crate::app_config::{AppConfig, AppConfigImpl};
use crate::extract_service::{ExtractService, ExtractServiceImpl};
use crate::godot_config_repository::GodotConfigRepository;
use crate::plugin_config_repository::PluginConfigRepository;
use crate::plugin_config_repository::plugin::Plugin;
use crate::utils::Utils;

use anyhow::{Context, Result, anyhow};
use futures::future::try_join_all;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::fs;
use tokio::task::JoinSet;
use tracing::{debug, error};

#[derive(Default)]
pub struct PluginService {
    godot_config_repository: GodotConfigRepository,
    asset_store_api: AssetStoreAPI,
    plugin_config_repository: PluginConfigRepository,
    app_config: AppConfig,
    extract_service: ExtractService,
}

impl PluginService {
    fn new(
        godot_config_repository: Option<GodotConfigRepository>,
        asset_store_api: Option<AssetStoreAPI>,
        plugin_config_repository: Option<PluginConfigRepository>,
        app_config: Option<AppConfig>,
        extract_service: Option<ExtractService>,
    ) -> Self {
        Self {
            godot_config_repository: godot_config_repository.unwrap_or_default(),
            asset_store_api: asset_store_api.unwrap_or_default(),
            plugin_config_repository: plugin_config_repository.unwrap_or_default(),
            app_config: app_config.unwrap_or_default(),
            extract_service: extract_service.unwrap_or_default(),
        }
    }
}

impl PluginServiceImpl for PluginService {
    fn get_plugin_config_repository(&self) -> &PluginConfigRepository {
        &self.plugin_config_repository
    }

    fn get_asset_store_api(&self) -> &AssetStoreAPI {
        &self.asset_store_api
    }

    fn get_godot_config_repository(&self) -> &GodotConfigRepository {
        &self.godot_config_repository
    }

    fn get_app_config(&self) -> &AppConfig {
        &self.app_config
    }

    fn get_extract_service(&self) -> &ExtractService {
        &self.extract_service
    }
}

pub trait PluginServiceImpl {
    fn get_app_config(&self) -> &AppConfig;
    fn get_extract_service(&self) -> &ExtractService;
    fn get_godot_config_repository(&self) -> &GodotConfigRepository;
    fn get_asset_store_api(&self) -> &AssetStoreAPI;
    fn get_plugin_config_repository(&self) -> &PluginConfigRepository;

    async fn install_all_plugins(&self) -> Result<()> {
        let plugins = self.get_plugin_config_repository().get_plugins()?;
        self.install_plugins(plugins).await?;

        println!();
        println!("done.");

        Ok(())
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
    async fn install_plugin(&self, name: String, asset_id: String, version: String) -> Result<()> {
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
        let cache_folder = self.get_app_config().get_cache_folder_path();

        let pb_download = self.create_download_progress_bar(
            &m,
            1,
            1,
            String::from("Downloading"),
            title,
            version,
        )?;
        let downloaded_plugin = self
            .get_asset_store_api()
            .download_plugin(&asset.clone(), pb_download, cache_folder.to_string())
            .await?;
        let plugin = downloaded_plugin.get_plugin();
        let pb_extract_service = self.create_extract_service_progress_bar(
            &m,
            1,
            1,
            String::from("ExtractServiceing"),
            plugin.get_title().to_string(),
            plugin.get_version_string().to_string(),
        )?;

        let addon_folder_path = self.get_app_config().get_addon_folder_path().to_string();
        let plugin_root_folder = self
            .get_extract_service()
            .extract_plugin(
                downloaded_plugin.get_file_path().clone(),
                addon_folder_path,
                pb_extract_service,
            )
            .await?;

        let plugins = HashMap::from([(plugin_root_folder.clone(), asset.to_plugin())]);
        self.add_plugins(plugins)?;
        Ok(())
    }

    /// Downloads and extract_services all plugins under /addons folder
    async fn download_and_extract_service_plugins(
        &self,
        main_start_message: String,
        plugins: Vec<AssetResponse>,
    ) -> Result<HashMap<String, Plugin>> {
        let cache_folder = self.get_app_config().get_cache_folder_path();
        let addon_folder = self.get_app_config().get_addon_folder_path();

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
            let cache_folder_clone = cache_folder.clone();
            download_tasks.spawn(async move {
                asset_store_api
                    .download_plugin(&plugin, pb_task, cache_folder_clone)
                    .await
            });
        }

        let result = download_tasks.join_all().await;
        let download_tasks = result
            .into_iter()
            .collect::<Result<Vec<DownloadedPlugin>>>()?;

        let mut extract_service_tasks = JoinSet::new();

        pb_main.set_message("ExtractServiceing plugins");

        for (index, downloaded_plugin) in download_tasks.clone().into_iter().enumerate() {
            let plugin = downloaded_plugin.get_plugin().clone();
            let file_path = downloaded_plugin.get_file_path().clone();
            let pb_task = self.create_extract_service_progress_bar(
                &pb_multi,
                index + 1,
                download_tasks.len(),
                String::from("ExtractServiceing"),
                plugin.get_title().to_string(),
                plugin.get_version_string().to_string(),
            )?;
            let extract_service = self.get_extract_service().clone();
            let addon_folder = addon_folder.clone();
            extract_service_tasks.spawn(async move {
                extract_service
                    .extract_plugin(file_path, addon_folder, pb_task)
                    .await
            });
        }

        while let Some(res) = extract_service_tasks.join_next().await {
            let _ = res??;
        }

        let installed_plugins = download_tasks
            .into_iter()
            .map(|p| (p.get_root_folder().clone(), p.get_plugin().to_plugin()))
            .collect::<HashMap<String, Plugin>>();

        pb_main.finish_and_clear();

        for (index, plugin) in installed_plugins.values().enumerate() {
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
        plugins: Vec<(String, Plugin)>,
    ) -> Result<HashMap<String, Plugin>> {
        let mut asset = Vec::new();
        for (_, plugin) in plugins {
            let asset_id = plugin.get_asset_id().clone();
            let version = plugin.get_version().clone();
            let asset_request = self.find_plugin_by_asset_id_and_version(asset_id, version);
            asset.push(asset_request);
        }

        let assets = try_join_all(asset).await?;

        let installed_plugins = self
            .download_and_extract_service_plugins(String::from("Downloading plugins"), assets)
            .await?;

        self.add_plugins(installed_plugins.clone())?;

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

    fn add_plugins(&self, plugins: HashMap<String, Plugin>) -> Result<()> {
        let godot_config_repository = self.get_godot_config_repository();
        let plugin_config_repository = self.get_plugin_config_repository();
        let plugin_config = plugin_config_repository.add_plugins(plugins)?;
        godot_config_repository.update_plugins(plugin_config)?;
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
            let id = asset_id;
            let ver = version;
            let asset = self
                .get_asset_store_api()
                .get_asset_by_id_and_version(id.as_str(), ver.as_str())
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
            let params = HashMap::from([("filter", name.as_str()), ("godot_version", "4.5")]);
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
                let plugin_folder_path =
                    Utils::plugin_name_to_addon_folder_path(plugin_name.clone(), addon_folder);

                // Remove plugin directory if it exists
                // TODO use fileservice
                if fs::try_exists(&plugin_folder_path).await? {
                    println!("Removing plugin folder: {}", plugin_folder_path);
                    fs::remove_dir_all(&plugin_folder_path).await?;
                } else {
                    println!(
                        "Plugin folder does not exist, trying to remove from {}",
                        self.get_app_config().get_config_file_path()
                    );
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
                    .update_plugins(plugin_config)
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
    async fn update_plugins(&self) -> Result<()> {
        // TODO Not working
        let plugin_config_repository = self.get_plugin_config_repository();
        let plugins = plugin_config_repository.get_plugins()?;

        debug!("Checking for plugin updates for {:?} plugins", plugins);

        let mut plugins_to_update = Vec::new();

        for (_, plugin) in plugins {
            let asset = self
                .get_asset_store_api()
                .get_asset_by_id(plugin.get_asset_id().clone())
                .await?;
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
            let updated_plugins = self
                .download_and_extract_service_plugins(
                    String::from("Updating plugins"),
                    plugins_to_update.clone(),
                )
                .await?;
            plugin_config_repository.add_plugins(updated_plugins.clone())?;
            println!("Plugins updated successfully.");
        }
        Ok(())
    }

    async fn get_asset_list_response_by_name_or_version(
        &self,
        name: &str,
        version: &str,
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
            parsed_version.as_str()
        } else {
            version
        };

        let params = HashMap::from([("filter", name), ("godot_version", version)]);
        let asset_results = self.get_asset_store_api().get_assets(params).await?;
        Ok(asset_results)
    }

    async fn search_assets_by_name_or_version(&self, name: &str, version: &str) -> Result<()> {
        let asset_list_response = self
            .get_asset_list_response_by_name_or_version(name, version)
            .await?;

        match asset_list_response.get_result_len() {
            0 => println!("No assets found matching \"{}\"", name),
            1 => println!("Found 1 asset matching \"{}\":", name),
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
    use super::*;

    use crate::{
        api::MockAssetStoreAPI,
        app_config::{AppConfig, MockAppConfig},
        extract_service::MockExtractService,
        godot_config_repository::MockGodotConfigRepository,
        plugin_service::PluginServiceImpl,
    };

    pub struct MockPluginService {
        godot_config_repository: MockGodotConfigRepository,
        asset_store_api: MockAssetStoreAPI,
        plugin_config_repository: MockGodotConfigRepository,
        app_config: MockAppConfig,
        extract_service: MockExtractService,
    }

    impl PluginServiceImpl for MockPluginService {
        fn get_plugin_config_repository(&self) -> &MockGodotConfigRepository {
            &self.plugin_config_repository
        }

        fn get_asset_store_api(&self) -> &MockAssetStoreAPI {
            &self.asset_store_api
        }

        fn get_godot_config_repository(&self) -> &GodotConfigRepository {
            &self.godot_config_repository
        }

        fn get_app_config(&self) -> &MockAppConfig {
            &self.app_config
        }

        fn get_extract_service(&self) -> &MockExtractService {
            &self.extract_service
        }
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
        let name = "some_non_existent_plugin_name";
        let version = "4.5";
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
        let name = "Godot Unit Testing";
        let version = "4.5";
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
        let name = "";
        let version = "4.5";
        let result = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_without_version_should_return_response()
     {
        let plugin_service = setup_plugin_service();
        let name = "Godot Unit Testing";
        let version = "";
        let result = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_without_name_or_version_should_return_err()
     {
        let plugin_service = setup_plugin_service();
        let name = "";
        let version = "";
        let result = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result.is_err());
    }

    // install_all_plugins

    #[tokio::test]
    async fn test_install_plugins_should_install_all_plugins_in_config() {
        let plugin_service = setup_plugin_service();
        let result = plugin_service.install_all_plugins().await;
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    // install_plugins

    #[tokio::test]
    async fn test_install_plugin_with_asset_id_and_no_version_should_return_err() {
        let plugin_service = setup_plugin_service();
        let name = String::default();
        let asset_id = String::from("1709");
        let version = String::default();
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_install_plugin_with_only_version_should_return_err() {
        let plugin_service = setup_plugin_service();
        let name = String::default();
        let asset_id = String::default();
        let version = String::from("9.1.0");
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_err());
    }
    #[tokio::test]
    async fn test_install_plugin_with_asset_id_and_version_should_install_plugin() {
        let plugin_service = setup_plugin_service();
        let name = String::new();
        let asset_id = String::from("1709");
        let version = String::from("9.1.0");
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_install_plugin_with_asset_id_should_install_plugin() {
        let plugin_service = setup_plugin_service();
        let name = String::new();
        let asset_id = String::from("1709");
        let version = String::new();
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_install_plugin_with_name_should_install_plugin() {
        let plugin_service = setup_plugin_service();
        let name = String::from("Godot Unit Testing");
        let asset_id = String::new();
        let version = String::new();
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_ok());
    }

    // add_plugin_by_id_or_name_and_version
    // add_plugins_to_godot_project
    // remove_plugin_by_name
    // check_outdated_plugins
    // update_plugins
    // search_assets_by_name_or_version
}
