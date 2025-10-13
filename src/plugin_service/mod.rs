use crate::api::AssetStoreAPI;
use crate::api::asset_list_response::AssetListResponse;
use crate::api::asset_response::AssetResponse;
use crate::app_config::AppConfig;
use crate::extract;
use crate::file_service::FileService;
use crate::godot_config::GodotConfig;
use crate::plugin_config::{Plugin, PluginConfig};
use crate::utils::Utils;

use anyhow::{Context, Result, anyhow};
use futures::future::try_join_all;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::header::CONTENT_LENGTH;
use std::cmp::min;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tokio::io;
use tokio::task::JoinSet;
use url::Url;

pub struct PluginService {
    app_config: AppConfig,
    godot_config: GodotConfig,
    asset_store_api: AssetStoreAPI,
    plugin_config: PluginConfig,
}

#[derive(Debug, Clone)]
pub struct DownloadedPlugin {
    root_folder: String,
    file_path: String,
    plugin: AssetResponse,
}

async fn extract_plugin(file_path: String, pb_task: ProgressBar) -> Result<String> {
    let app_config = AppConfig::new();
    let addon_folder_path = app_config.get_addon_folder_path().to_string();

    let extracted_folder =
        crate::extract::extract_zip_file(file_path.clone(), addon_folder_path, pb_task)?;
    FileService::remove_file(&PathBuf::from(file_path))?;
    Ok(extracted_folder)
}

async fn download_plugin(
    asset: AssetResponse,
    pb_task: ProgressBar,
) -> Result<DownloadedPlugin> {
    let download_url = asset.get_download_url();

    let url = Url::parse(&download_url)?;

    let cache_folder = AppConfig::new().get_cache_folder_path();

    let filename = url
        .path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or("temp_file.zip");
    let filepath = format!("{}/{}", cache_folder, filename);

    if !fs::try_exists(cache_folder).await? {
        fs::create_dir(cache_folder).await?;
    }

    if fs::try_exists(&filepath).await? {
        fs::remove_file(&filepath).await?;
    }

    let api = AssetStoreAPI::new();
    let mut res = api.download_asset(download_url).await?;
  
    let mut downloaded = 0;
    let total_size = res.headers().get(CONTENT_LENGTH).and_then(|value| value.to_str().ok()?.parse().ok()).unwrap_or(0);
    pb_task.set_length(total_size);

    let mut file = fs::File::create(&filepath).await?;

    while let Some(chunk) = res.chunk().await? {

        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb_task.set_position(new);
        let result = io::AsyncWriteExt::write_all(&mut file, &chunk).await;
        result?;
    }

    // TODO: Verify download integrity if possible
    // asset.get_download_commit();

    pb_task.finish_and_clear();

    match res.error_for_status() {
        Ok(_) => {
            let root_folder = extract::get_root_dir_from_archive(&filepath)?;
            Ok(DownloadedPlugin {
                root_folder,
                file_path: filepath,
                plugin: asset.clone(),
            })
        }
        Err(e) => Err(anyhow::anyhow!("Failed to fetch file: {}", e)),
    }
}

impl PluginService {
    pub fn new() -> Self {
        Self {
            godot_config: GodotConfig::new(),
            asset_store_api: AssetStoreAPI::new(),
            plugin_config: PluginConfig::new(),
            app_config: AppConfig::new(),
        }
    }

    #[cfg(not(tarpaulin_include))]
    fn default(
        godot_config: Option<GodotConfig>,
        asset_store_api: Option<AssetStoreAPI>,
        plugin_config: Option<PluginConfig>,
        app_config: Option<AppConfig>,
    ) -> Self {
        Self {
            godot_config: godot_config.unwrap_or(GodotConfig::new()),
            asset_store_api: asset_store_api.unwrap_or(AssetStoreAPI::new()),
            plugin_config: plugin_config.unwrap_or(PluginConfig::new()),
            app_config: app_config.unwrap_or(AppConfig::new()),
        }
    }

    pub async fn install_all_plugins(&self) -> Result<()> {
        let plugins = self
            .plugin_config
            .get_plugins()
            .values()
            .into_iter()
            .map(|p| p.clone())
            .collect::<Vec<Plugin>>();

        self.install_plugins(plugins).await?;

        println!("");
        println!("done.");

        Ok(())
    }

    fn create_finished_install_bar(&self, m: &MultiProgress, index: usize, total: usize, action: String, title: String, version: String) -> ProgressBar {
        let pb_task = m.add(ProgressBar::new(1));
         pb_task.set_style(ProgressStyle::with_template("{prefix} {msg}")
        .unwrap()
        .progress_chars("#>-"));
        pb_task.set_prefix(format!("[{}/{}]", index, total));
        pb_task.set_message(format!("{}: {} ({})", action, title, version));
        pb_task
    }

    fn create_download_progress_bar_task(&self, m: &MultiProgress, index: usize, total: usize, action: String, title: String, version: String) -> ProgressBar {
        let pb_task = m.add(ProgressBar::new(5000000));
         pb_task.set_style(ProgressStyle::with_template("{spinner:.green} {prefix} {msg} [{elapsed_precise}] [{bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .unwrap()
        .progress_chars("#>-"));
        pb_task.set_prefix(format!("[{}/{}]", index, total));
        pb_task.set_message(format!("{}: {} ({})", action, title, version));
        pb_task
    }

    fn create_extract_progress_bar_task(&self, m: &MultiProgress, index: usize, total: usize, action: String, title: String, version: String) -> ProgressBar {
        let pb_task = m.add(ProgressBar::new(5000000));
         pb_task.set_style(ProgressStyle::with_template("{spinner:.green} {prefix} {msg} [{elapsed_precise}] [{bar:.cyan/blue}] {pos:>7}/{len:7} ({eta})")
        .unwrap()
        .progress_chars("#>-"));
        pb_task.set_prefix(format!("[{}/{}]", index, total));
        pb_task.set_message(format!("{}: {} ({})", action, title, version));
        pb_task
    }

    pub async fn install_plugin(
        &self,
        name: Option<&String>,
        asset_id: Option<&String>,
        version: Option<&String>,
    ) -> Result<HashMap<String, Plugin>> {
        let _name = name.unwrap_or(&String::from("")).to_string();
        let _asset_id = asset_id.unwrap_or(&String::from("")).to_string();
        let _version = version.unwrap_or(&String::from("")).to_string();
        let asset: AssetResponse;
        if !_version.is_empty() && !_asset_id.is_empty() {
            asset = self
                .find_plugin_by_asset_id_and_version(_asset_id, _version)
                .await?;
        } else if !_name.is_empty() || !_asset_id.is_empty() {
            asset = self.find_plugin_by_id_or_name(_asset_id, _name).await?;
        } else {
            return Err(anyhow!("No name or asset ID provided"));
        }

        let m = MultiProgress::new();
        let plugin = self.download_plugin(&asset).await?;
        let pb_task = self.create_extract_progress_bar_task(&m, 1, 1, String::from("Extracting"), plugin.plugin.get_title().to_string(), plugin.plugin.get_version_string().to_string());
        let plugin_root_folder = self.extract_plugin(plugin.file_path.clone(), pb_task)?;

        Ok(HashMap::from([(plugin_root_folder, asset.to_plugin())]))
    }

    async fn install_plugins(
        &self,
        plugins: Vec<Plugin>,
    ) -> Result<HashMap<String, Plugin>> {
        let mut asset = Vec::new();
        for plugin in plugins {
            let asset_id = plugin.get_asset_id().clone();
            let version = plugin.get_version().clone();
            let asset_request = self.find_plugin_by_asset_id_and_version(asset_id, version);
            asset.push(asset_request);
        }

        let assets = try_join_all(asset).await?;
       
        let mut download_tasks = JoinSet::new();
        let pb_multi = MultiProgress::new();
        let pb_main = pb_multi.add(ProgressBar::new(0));
        pb_main.set_style(ProgressStyle::with_template("{spinner:.green} {msg}")?);
        pb_main.set_message("Downloading plugins");
        pb_main.enable_steady_tick(Duration::from_millis(100));

        for (index, asset) in assets.iter().enumerate() {
            let plugin = asset.clone();
            let pb_task = self.create_download_progress_bar_task(&pb_multi, index + 1, assets.len(), String::from("Downloading"), plugin.get_title().to_string(), plugin.get_version_string().to_string());
            download_tasks.spawn(download_plugin(plugin, pb_task));
        }

        let result = download_tasks.join_all().await;
        let download_tasks = result.into_iter().collect::<Result<Vec<DownloadedPlugin>>>()?;

        let mut extract_tasks = JoinSet::new();

        pb_main.set_message("Extracting plugins");

        for (index, downloaded_plugin) in download_tasks.clone().into_iter().enumerate() {
            let plugin = downloaded_plugin.plugin.clone();
            let file_path = downloaded_plugin.file_path.clone();
            let pb_task = self.create_extract_progress_bar_task(&pb_multi, index + 1, download_tasks.len(), String::from("Extracting"), plugin.get_title().to_string(), plugin.get_version_string().to_string());
            extract_tasks.spawn(extract_plugin(file_path, pb_task));
        }

        while let Some(res) = extract_tasks.join_next().await {
            let _ = res??;
        }

        let installed_plugins = download_tasks
            .into_iter()
            .map(|p| (p.root_folder, p.plugin.to_plugin()))
            .collect::<HashMap<String, Plugin>>();

        pb_main.finish_and_clear();

        for (index, plugin) in installed_plugins.values().enumerate() {
            let finished_bar = self.create_finished_install_bar(&pb_multi, index + 1, installed_plugins.len(), String::from("Installed"), plugin.get_title().to_string(), plugin.get_version().to_string());
            finished_bar.finish();
        }

        Ok(installed_plugins)
    }

    pub async fn add_plugin_by_id_or_name_and_version(
        &self,
        asset_id: Option<&String>,
        name: Option<&String>,
        version: Option<&String>,
    ) -> Result<()> {
        let installed_plugins = self.install_plugin(asset_id, name, version).await?;
        self.add_plugins_to_config(installed_plugins.clone())?;
        self.add_plugins_to_godot_project(installed_plugins)?;
        Ok(())
    }

    fn add_plugins_to_godot_project(&self, plugins: HashMap<String, Plugin>) -> Result<()> {
        self.godot_config
            .add_installed_plugins(plugins)
            .with_context(|| format!("Failed to add plugins to Godot project configuration",))?;

        println!("Plugins added to Godot project configuration.");
        Ok(())
    }

    fn add_plugins_to_config(&self, plugins: HashMap<String, Plugin>) -> Result<()> {
        self.plugin_config.add_plugins(plugins)?;
        Ok(())
    }

    async fn find_plugin_by_asset_id_and_version(
        &self,
        asset_id: String,
        version: String,
    ) -> Result<AssetResponse> {
        if !asset_id.is_empty() && !version.is_empty() {
            let id = asset_id;
            let ver = version;
            let asset = self
                .asset_store_api
                .search_asset_by_id_and_version(id.as_str(), ver.as_str())
                .await?;
            return Ok(asset);
        } else {
            return Err(anyhow!(
                "Both asset ID and version must be provided to search by version."
            ));
        }
    }

    async fn find_plugin_by_id_or_name(
        &self,
        asset_id: String,
        name: String,
    ) -> Result<AssetResponse> {
        if !asset_id.is_empty() {
            let asset = self
                .asset_store_api
                .fetch_asset_by_id(asset_id.as_str())
                .await?;
            return Ok(asset);
        } else if !name.is_empty() {
            let params = HashMap::from([("filter", name.as_str()), ("godot_version", "4.5")]);
            let asset_results = self.asset_store_api.search_assets(params).await?;

            if asset_results.get_result_len() != 1 {
                return Err(anyhow!(
                    "Expected to find exactly one asset matching \"{}\", but found {}. Please refine your search or use --asset-id.",
                    name,
                    asset_results.get_result_len()
                ));
            }
            let asset = asset_results.get_asset_list_item_by_index(0).unwrap();
            let id = asset.get_asset_id().to_owned();
            let asset = self.asset_store_api.fetch_asset_by_id(id.as_str()).await?;

            self.plugin_config
                .check_if_plugin_already_installed_by_asset_id(asset.get_asset_id());
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
            return Ok(asset);
        } else {
            return Err(anyhow!("No name or asset ID provided"));
        }
    }

    async fn download_plugin(
        &self,
        asset: &AssetResponse,
    ) -> Result<DownloadedPlugin> {
        println!(
            "Downloading plugin: {} ({})",
            asset.get_title(),
            asset.get_version_string()
        );

        let download_url = asset.get_download_url();

        let url = Url::parse(&download_url)?;

        let cache_folder = AppConfig::new().get_cache_folder_path();

        let filename = url
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("temp_file.zip");
        let filepath = format!("{}/{}", cache_folder, filename);

        if !fs::try_exists(cache_folder).await? {
            fs::create_dir(cache_folder).await?;
        }

        if fs::try_exists(&filepath).await? {
            fs::remove_file(&filepath).await?;
        }

        let mut res = self.asset_store_api.download_asset(download_url).await?;
        let mut downloaded = 0;
        let total_size = res.content_length().unwrap_or(0);

        let mut file = fs::File::create(&filepath).await?;
        let pb_task = ProgressBar::new(total_size);
        pb_task.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .unwrap()
        .progress_chars("#>-"));

        while let Some(chunk) = res.chunk().await.unwrap() {
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            pb_task.set_position(new);
            let result = io::AsyncWriteExt::write_all(&mut file, &chunk).await;
            result.unwrap();
        }

        pb_task.finish();

        // TODO: Verify download integrity if possible
        // asset.get_download_commit();

        match res.error_for_status() {
            Ok(_) => {
                let root_folder = extract::get_root_dir_from_archive(&filepath)?;
                Ok(DownloadedPlugin {
                    root_folder,
                    file_path: filepath,
                    plugin: asset.clone(),
                })
            }
            Err(e) => Err(anyhow::anyhow!("Failed to fetch file: {}", e)),
        }
    }

    fn extract_plugin(&self, file_path: String, pb_task: ProgressBar) -> Result<String> {
        let addon_folder_path = self.app_config.get_addon_folder_path();

        let extracted_folder = crate::extract::extract_zip_file(
            file_path.clone(),
            addon_folder_path.to_string(),
            pb_task
        )?;
        FileService::remove_file(&PathBuf::from(file_path))?;
        Ok(extracted_folder)
    }

    pub async fn remove_plugin_by_name(&self, name: &str) -> Result<()> {
        let plugin_config = PluginConfig::new();
        let installed_plugin = plugin_config.get_plugin_key_by_name(name);

        match installed_plugin {
            Some(plugin_name) => {
                // Remove plugin directory
                let plugin_name = plugin_name;

                let plugin_folder_path = Utils::plugin_name_to_addon_folder_path(&plugin_name);
                println!("Removing plugin folder: {}", plugin_folder_path);

                if fs::try_exists(&plugin_folder_path).await? {
                    fs::remove_dir_all(&plugin_folder_path).await?;
                } else {
                    println!(
                        "Plugin folder does not exist, trying to remove from {}",
                        AppConfig::new().get_config_file_name()
                    );
                }

                // Remove plugin from plugin config
                let plugin_remove_result = plugin_config.remove_installed_plugin(&plugin_name);
                match plugin_remove_result {
                    Ok(_) => {}
                    Err(e) => eprintln!("Failed to remove plugin from plugin configuration: {}", e),
                }

                // Remove plugin from godot project config
                let remove_result = self
                    .godot_config
                    .remove_installed_plugin(vec![plugin_name.clone()]);
                match remove_result {
                    Ok(_) => println!("Plugin {} removed successfully.", plugin_name),
                    Err(e) => eprintln!(
                        "Failed to remove plugin from Godot project configuration: {}",
                        e
                    ),
                }
                Ok(())
            }
            None => Err(anyhow!("Plugin {} is not installed.", name)),
        }
    }

    /// Check for outdated plugins by comparing installed versions with the latest versions from the Asset Store
    /// Prints a list of outdated plugins with their current and latest versions, e.g.:
    /// Plugin                  Current    Latest
    /// some_plugin             1.5.0      1.6.0 (update available)
    pub async fn check_outdated_plugins(&self) -> Result<()> {
        println!("Checking for outdated plugins");
        println!();

        let mut plugins = self
            .plugin_config
            .get_plugins()
            .into_iter()
            .collect::<Vec<_>>();
        plugins.sort_by(|a, b| a.0.cmp(&b.0));

        println!("{0: <40} {1: <20} {2: <20}", "Plugin", "Current", "Latest");
        let mut has_an_update = false;

        for plugin in plugins {
            let asset = self
                .asset_store_api
                .fetch_asset_by_id(&plugin.1.get_asset_id())
                .await;
            match asset {
                Ok(asset) => {
                    has_an_update = has_an_update || asset.get_version_string() != plugin.1.get_version();

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
                Err(e) => eprintln!(
                    "Failed to fetch asset info for plugin {}: {}",
                    plugin.1.get_title(),
                    e
                ),
            }
        }
        println!();

        if !has_an_update {
            println!("All plugins are up to date.");
        } else {
            println!("To update a plugins, use: gdm update");
        }
        Ok(())
    }

    pub async fn update_plugins(&self) -> Result<()> {
        println!("Updating plugins");
        println!();

        let mut plugins = self
            .plugin_config
            .get_plugins()
            .into_iter()
            .collect::<Vec<_>>();
        plugins.sort_by(|a, b| a.0.cmp(&b.0));

        let mut has_an_update = false;
        let mut updated_plugins = HashMap::new();

        for plugin in plugins {
            let asset = self
                .asset_store_api
                .fetch_asset_by_id(&plugin.1.get_asset_id())
                .await;
            match asset {
                Ok(asset) => {
                    has_an_update = has_an_update || asset.get_version_string() != plugin.1.get_version();
                    if asset.get_version_string() != plugin.1.get_version() {
                        println!(
                            "Updating \"{}\" from version {} to {}",
                            plugin.1.get_title(),
                            plugin.1.get_version(),
                            asset.get_version_string()
                        );
                        let install_result = self
                            .install_plugin(
                                None,
                                Some(&asset.get_asset_id().to_string()),
                                Some(&asset.get_version_string().to_string()),
                            )
                            .await;
                        match install_result {
                            Ok(installed_plugins) => {
                                updated_plugins.extend(installed_plugins);
                            }
                            Err(e) => eprintln!(
                                "Failed to install updated plugin {}: {}",
                                plugin.1.get_title(),
                                e
                            ),
                        }
                        println!();
                    }
                }
                Err(e) => eprintln!(
                    "Failed to fetch asset info for plugin \"{}\": {}",
                    plugin.1.get_title(),
                    e
                ),
            }
        }
        self.plugin_config.add_plugins(updated_plugins.clone())?;
        println!();
        if !has_an_update {
            println!("All plugins are up to date.");
        } else {
            println!("Plugins updated successfully.");
        }
        Ok(())
    }

    async fn get_asset_list_response_by_name_or_version(
        &self,
        name: &str,
        version: &str,
    ) -> Result<AssetListResponse> {
        let parsed_version = self.godot_config.get_godot_version()?;

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
        let asset_results = AssetStoreAPI::new().search_assets(params).await?;
        Ok(asset_results)
    }

    pub async fn search_assets_by_name_or_version(&self, name: &str, version: &str) -> Result<()> {
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

    // Test struct that drops file after tests
    #[derive(Debug)]
    struct TestResource {
        path: String,
    }

    impl Drop for TestResource {
        fn drop(&mut self) {
            std::fs::remove_file(&self.path).unwrap();
        }
    }

    fn setup_plugin_service() -> PluginService {
        let app_config = AppConfig::default(
            None,
            None,
            Some("test/mocks/gdm.json"),
            None,
            Some("test/mocks/project_with_plugins_and_version.godot"),
            Some("test/addons"),
        );
        PluginService::default(
            None,
            None,
            Some(PluginConfig::default("test/mocks/gdm.json")),
            Some(app_config),
        )
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

    // install_plugins

    fn setup_plugin_service_for_plugins(gdm_json_path: &'static str) -> PluginService {
        let app_config = AppConfig::default(
            None,
            None,
            Some(gdm_json_path),
            None,
            Some("test/mocks/project_with_plugins_and_version.godot"),
            Some("test/addons"),
        );
        PluginService::default(
            None,
            None,
            Some(PluginConfig::default(gdm_json_path)),
            Some(app_config),
        )
    }

    #[tokio::test]
    async fn test_install_plugins_should_install_all_plugins_in_config() {
        let plugin_service = setup_plugin_service_for_plugins("test/mocks/gdm_with_plugins.json");
        let result = plugin_service.install_all_plugins().await;
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    // install_plugin

    #[tokio::test]
    async fn test_install_plugin_with_asset_id_and_no_version_should_return_err() {
        let plugin_service = setup_plugin_service_for_plugins("test/mocks/gdm_with_plugins.json");
        let name = None;
        let asset_id = Some("1709".to_string());
        let version = None;
        let result = plugin_service
            .install_plugin(name, asset_id.as_ref(), version)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_install_plugin_with_only_version_should_return_err() {
        let plugin_service = setup_plugin_service_for_plugins("test/mocks/gdm_with_plugins.json");
        let name = None;
        let asset_id = None;
        let version = Some("9.1.0".to_string());
        let result = plugin_service
            .install_plugin(name, asset_id.as_ref(), version.as_ref())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_install_plugin_with_asset_id_and_version_should_install_plugin() {
        let plugin_service = setup_plugin_service_for_plugins("test/mocks/gdm_with_plugins.json");
        let name = None;
        let asset_id = Some("1709".to_string());
        let version = Some("9.1.0".to_string());
        let result = plugin_service
            .install_plugin(name, asset_id.as_ref(), version.as_ref())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_install_plugin_with_asset_id_should_install_plugin() {
        let plugin_service = setup_plugin_service_for_plugins("test/mocks/gdm_with_plugins.json");
        let name = None;
        let asset_id = Some("1709".to_string());
        let version = None;
        let result = plugin_service
            .install_plugin(name, asset_id.as_ref(), version)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_install_plugin_with_name_should_install_plugin() {
        let plugin_service = setup_plugin_service_for_plugins("test/mocks/gdm_with_plugins.json");
        let name = Some(&"Godot Unit Testing".to_string());
        let asset_id = None;
        let version = None;
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_ok());
    }

    // add_plugin_by_id_or_name_and_version
    // add_plugins_to_godot_project
    // download_plugin
    // extract_plugin
    // remove_plugin_by_name
    // check_outdated_plugins
    // update_plugins
    // search_assets_by_name_or_version
}
