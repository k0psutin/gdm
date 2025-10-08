use crate::api::AssetStoreAPI;
use crate::api::asset_response::AssetResponse;
use crate::app_config::AppConfig;
use crate::godot_config;
use crate::godot_config::GodotConfig;
use crate::plugin_config::PluginConfig;
use crate::utils::Utils;
use anyhow::{Result, anyhow};
use indicatif::{ProgressBar, ProgressStyle};
use std::cmp::min;
use std::collections;
use tokio::fs;
use tokio::io;
use url::Url;

pub struct PluginService {
    godot_config: GodotConfig,
    asset_store_api: AssetStoreAPI,
}

impl PluginService {
    pub fn new() -> Self {
        Self {
            godot_config: GodotConfig::new(),
            asset_store_api: AssetStoreAPI::new(),
        }
    }

    pub async fn add_plugin_by_id_or_name(
        &self,
        asset_id: Option<&String>,
        name: Option<&String>,
    ) -> Result<()> {
        let asset = self.find_plugin_by_id_or_name(asset_id, name).await?;
        println!(
            "Downloading plugin: {} (ID: {})",
            asset.get_title(),
            asset.get_asset_id()
        );
        let downloaded_path = self.download_plugin(&asset).await?;

        let plugin_root_folder = self.extract_plugin(&downloaded_path).await?;
        self.add_plugin_to_config(&asset, &plugin_root_folder)?;
        Ok(())
    }

    fn add_plugin_to_config(&self, asset: &AssetResponse, plugin_root_folder: &str) -> Result<()> {
        let plugin_config = PluginConfig::new();

        let plugin = crate::plugin_config::Plugin {
            asset_id: asset.get_asset_id().to_string(),
            title: asset.get_title().to_string(),
            version: asset.get_version().to_string(),
        };

        let new_plugins = collections::HashMap::from([(plugin_root_folder.to_string(), plugin)]);

        plugin_config.add_plugins(new_plugins);

        // let plugin_already_installed = self.godot_config.is_plugin_installed(plugin_root_folder);

        // if plugin_already_installed {
        //     println!("Plugin already installed. Trying to update configuration.");
        // }
        
        let plugins_to_add = vec![plugin_root_folder.to_string()];
        let result = self.godot_config.add_installed_plugin(plugins_to_add);
        match result {
            Ok(_) => {
                println!("Plugin added to Godot project configuration.");
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to update Godot project configuration: {}", e);
                Err(e)
            }
        }
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

            let plugin_config = PluginConfig::new();
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

        pb.finish();

        // TODO: Verify download integrity if possible
        // asset.get_download_hash();

        match res.error_for_status() {
            Ok(_) => Ok(filepath),
            Err(e) => Err(anyhow::anyhow!("Failed to fetch file: {}", e)),
        }
    }

    async fn extract_plugin(&self, file_path: &str) -> Result<String> {
        let addon_folder_path = AppConfig::new().get_addon_folder_path();
        println!("Extracting plugin to: {}", addon_folder_path);
        let extracted_folder = crate::extract::extract_zip_file(file_path, addon_folder_path);
        fs::remove_file(file_path).await?;
        println!("Plugin extracted!");
        extracted_folder
    }

    pub async fn remove_plugin_by_name(&self, name: &str) -> Result<()> {
        let plugin_config = PluginConfig::new();
        let plugin_option = plugin_config.get_plugin_by_name(name);

        println!("Removing plugin: {}", name);

        match plugin_option {
            Some(plugin) => {
                // Remove plugin directory

                let plugin_folder_path = Utils::plugin_name_to_addon_folder_path(&plugin.title);
                fs::remove_dir_all(plugin_folder_path).await?;
                
                // Remove plugin from plugin config
                plugin_config.remove_installed_plugin(&plugin.title);

                // Remove plugin from godot project config
                self.godot_config.remove_installed_plugin(vec![plugin.title.to_string()])?;
                println!("Plugin {} removed successfully.", name);
                Ok(())
            }
            None => Err(anyhow!("Plugin {} is not installed.", name)),
        }
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
