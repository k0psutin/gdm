use crate::api::AssetStoreAPI;
use crate::api::asset_response::AssetResponse;
use crate::app_config::AppConfig;
use crate::godot_config::GodotConfig;
use crate::plugin_config::PluginConfig;
use crate::utils::Utils;

use anyhow::{Context, Result, anyhow};
use futures::future::try_join_all;
use indicatif::{ProgressBar, ProgressStyle};
use std::cmp::min;
use std::collections;
use tokio::fs;
use tokio::io;
use url::Url;

pub struct PluginService {
    app_config: &'static AppConfig,
    godot_config: GodotConfig,
    asset_store_api: AssetStoreAPI,
    plugin_config: &'static PluginConfig,
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

    /// For testing purposes
    fn default(
        godot_config: GodotConfig,
        asset_store_api: AssetStoreAPI,
        plugin_config: &'static PluginConfig,
        app_config: &'static AppConfig,
    ) -> Self {
        Self {
            godot_config,
            asset_store_api,
            plugin_config,
            app_config,
        }
    }

    pub async fn install_plugins(&self) -> Result<()> {
        let plugins = self.plugin_config.get_plugins();

        let mut handles = Vec::new();

        for plugin in plugins.values().clone() {
            let asset_id = plugin.get_asset_id();
            let task = self.install_plugin(Some(&asset_id), None);
            handles.push(task);
        }

        try_join_all(handles).await?;

        println!("");
        println!("done.");

        Ok(())
    }

    pub async fn install_plugin(
        &self,
        asset_id: Option<&String>,
        name: Option<&String>,
    ) -> Result<(String, AssetResponse)> {
        let asset = self.find_plugin_by_id_or_name(asset_id, name).await?;

        let downloaded_path = self.download_plugin(&asset).await?;
        let plugin_root_folder = self.extract_plugin(&asset, &downloaded_path).await?;

        Ok((plugin_root_folder, asset))
    }

    pub async fn add_plugin_by_id_or_name(
        &self,
        asset_id: Option<&String>,
        name: Option<&String>,
    ) -> Result<()> {
        let (plugin_root_folder, asset) = self.install_plugin(asset_id, name).await?;
        self.add_plugin_to_config(&asset, &plugin_root_folder)?;
        Ok(())
    }

    fn add_plugin_to_config(&self, asset: &AssetResponse, plugin_root_folder: &str) -> Result<()> {
        let plugin_config = PluginConfig::new();

        let plugin = crate::plugin_config::Plugin::new(
            asset.get_asset_id().to_string(),
            asset.get_title().to_string(),
            asset.get_version().to_string(),
        );

        let new_plugins = collections::HashMap::from([(plugin_root_folder.to_string(), plugin)]);

        plugin_config.add_plugins(new_plugins)?;

        // let plugin_already_installed = self.godot_config.is_plugin_installed(plugin_root_folder);

        // if plugin_already_installed {
        //     println!("Plugin already installed. Trying to update configuration.");
        // }

        let plugins_to_add = vec![plugin_root_folder.to_string()];
        self.godot_config
            .add_installed_plugin(plugins_to_add)
            .with_context(|| {
                format!(
                    "Failed to add plugin {} to Godot project configuration",
                    plugin_root_folder
                )
            })?;

        println!("Plugin added to Godot project configuration.");
        Ok(())
    }

    async fn find_plugin_by_id_or_name(
        &self,
        asset_id: Option<&String>,
        name: Option<&String>,
    ) -> Result<AssetResponse> {
        if asset_id.is_some() {
            let id = asset_id.unwrap();
            let asset = self.asset_store_api.fetch_asset_by_id(id.as_str()).await?;
            return Ok(asset);
        } else if name.is_some() {
            let name = name.unwrap();
            let params =
                collections::HashMap::from([("filter", name.as_str()), ("godot_version", "4.5")]);
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
            //         if plugin.version == asset.get_version() {
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
            //             asset.get_version()
            //         );
            //     }
            //     None => {}
            // }
            return Ok(asset);
        } else {
            return Err(anyhow!("No name or asset ID provided"));
        }
    }

    async fn download_plugin(&self, asset: &AssetResponse) -> Result<String> {
        let pb_message =
            ProgressBar::no_length().with_style(ProgressStyle::with_template("{msg}").unwrap());
        pb_message.set_message(format!("Downloading plugin: {}", asset.get_title(),));

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

        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .unwrap()
        .progress_chars("#>-"));

        while let Some(chunk) = res.chunk().await? {
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            pb.set_position(new);
            io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
        }

        pb.finish_and_clear();
        pb_message.finish_and_clear();

        // TODO: Verify download integrity if possible
        // asset.get_download_commit();

        match res.error_for_status() {
            Ok(_) => Ok(filepath),
            Err(e) => Err(anyhow::anyhow!("Failed to fetch file: {}", e)),
        }
    }

    async fn extract_plugin(&self, asset: &AssetResponse, file_path: &str) -> Result<String> {
        let pb =
            ProgressBar::no_length().with_style(ProgressStyle::with_template("{msg}").unwrap());
        pb.set_message(format!("Extracting plugin: {}", asset.get_title()));

        let addon_folder_path = self.app_config.get_addon_folder_path();

        let extracted_folder = crate::extract::extract_zip_file(file_path, addon_folder_path)?;
        pb.finish_and_clear();
        fs::remove_file(file_path).await?;
        Ok(extracted_folder)
    }

    pub async fn remove_plugin_by_name(&self, name: &str) -> Result<()> {
        let plugin_config = PluginConfig::new();
        let installed_plugin = plugin_config.get_plugin_by_name(name);

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
    /// Plugin                  Current    Latest (* indicates update available)
    /// some_plugin             1.5.0      1.6.0*
    pub async fn check_outdated_plugins(&self) -> Result<()> {
        println!("Checking for outdated plugins");
        println!();
        
        let mut plugins = self.plugin_config.get_plugins().into_iter().collect::<Vec<_>>();
        plugins.sort_by(|a, b| a.0.cmp(&b.0));

        let mut outdated_plugins = Vec::<String>::new();
        println!("{0: <40} {1: <20} {2: <20}", "Plugin", "Current", "Latest");
        for plugin in plugins {
            let asset = self.asset_store_api.fetch_asset_by_id(&plugin.1.get_asset_id()).await;
            match asset {
                Ok(asset) => {
                    let has_an_update = asset.get_version() != plugin.1.get_version();

                    if has_an_update {
                        outdated_plugins.push(format!("Update {} plugin with: gdm add \"{}\" or gdm add --asset-id {}", plugin.0, plugin.1.get_title(), plugin.1.get_asset_id()));
                    }
                    let version = format!("{} {}", asset.get_version(), if has_an_update { "(update available)" } else { "" });
                    println!("{0: <40} {1: <20} {2: <20}", plugin.0, plugin.1.get_version(), version);
                }
                Err(e) => eprintln!(
                    "Failed to fetch asset info for plugin {}: {}",
                    plugin.1.get_title(), e
                ),
            }
        }
        println!();
        for outdated_plugin in outdated_plugins {
            println!("{}", outdated_plugin);
        }
        Ok(())
    }

    pub async fn search_asset_by_name_or_version(&self, name: &str, version: &str) -> Result<()> {
    let parsed_version = self.godot_config.get_godot_version()?;

    if version.is_empty() && parsed_version.is_empty() {
        println!("Couldn't determine Godot version from project.godot. Please provide a version using --godot-version.");
        return Ok(());
    }

    let version = if version.is_empty() { parsed_version.as_str() } else { version };

    let params = collections::HashMap::from([("filter", name), ("godot_version", version)]);
    let asset_results = AssetStoreAPI::new().search_assets(params).await?;
    match asset_results.get_result_len() {
            0 => println!("No assets found matching \"{}\"", name),
            1 => println!("Found 1 asset matching \"{}\":", name),
            n => println!("Found {} assets matching \"{}\":", n, name),
        }

    asset_results.print_info();

    if asset_results.get_result_len() == 1 {
        let asset = asset_results.get_asset_list_item_by_index(0).unwrap();
            println!("To install the plugin, use: gdm add \"{}\"", asset.get_title());
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

    #[tokio::test]
    async fn test_find_plugin_by_id_or_name_with_id() {
        let plugin_service = PluginService::new();
        let asset_id = Some("1709".to_string());
        let name = None;
        let result = plugin_service
            .find_plugin_by_id_or_name(asset_id.as_ref(), name.as_ref())
            .await;
        assert!(result.is_ok());
        let asset = result.unwrap();
        assert_eq!(asset.get_asset_id(), "1709");
    }

    #[tokio::test]
    async fn test_find_plugin_by_id_or_name_with_name() {
        let plugin_service = PluginService::new();
        let asset_id = None;
        let name = Some("Godot Unit Testing".to_string());
        let result = plugin_service
            .find_plugin_by_id_or_name(asset_id, name.as_ref())
            .await;

        assert!(result.is_ok());
        let asset = result.unwrap();
        assert_eq!(asset.get_title(), "GUT - Godot Unit Testing (Godot 4)");
    }
}
