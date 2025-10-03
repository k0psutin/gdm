use std::fs;
use serde::{Serialize, Deserialize};
use serde_json;
use crate::settings::Settings;

#[derive(Serialize, Deserialize, Debug)]
pub struct PluginConfig {
    pub plugins: std::collections::HashMap<String, Plugin>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Plugin {
    pub asset_id: String,
    pub title: String,
    pub version: String,
}

pub fn read_config() -> anyhow::Result<PluginConfig> {
    let settings = Settings::get_settings().unwrap();
    if fs::exists(settings.config_file_name).is_err() {
        return Err(anyhow::anyhow!("Configuration file not found"));
    }
    let file = fs::File::open(settings.config_file_name)?;
    let config: PluginConfig = serde_json::from_reader(file)?;

    if config.plugins.is_empty() {
        return Err(anyhow::anyhow!("No plugins found in configuration"));
    }

    Ok(config)
}

pub fn write_config(plugin_config: &PluginConfig)  {
     let settings = Settings::get_settings().unwrap();  
    let file = fs::File::create(settings.config_file_name);

    if file.is_err() {
        eprintln!("Failed to create configuration file");
        return;
    }

    let result = serde_json::to_writer(file.unwrap(), plugin_config);

    if result.is_err() {
        eprintln!("Failed to write configuration to file");
    }
}
