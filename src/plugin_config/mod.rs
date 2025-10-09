use crate::app_config::AppConfig;
use anyhow::Context;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections;
use std::fs;
use std::sync::OnceLock;

#[derive(Serialize, Deserialize, Debug)]
pub struct PluginConfig {
    plugins: collections::HashMap<String, Plugin>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Plugin {
    asset_id: String,
    title: String,
    version: String,
}

impl Plugin {
    pub fn new(asset_id: String, title: String, version: String) -> Plugin {
        Plugin {
            asset_id,
            title,
            version,
        }
    }

    pub fn get_asset_id(&self) -> &String {
        &self.asset_id
    }

    pub fn get_title(&self) -> &String {
        &self.title
    }

    pub fn get_version(&self) -> &String {
        &self.version
    }
}

impl PluginConfig {
    pub fn new<'a>() -> &'a PluginConfig {
        static INSTANCE: OnceLock<PluginConfig> = OnceLock::new();
        INSTANCE.get_or_init(|| Self::init(AppConfig::new().get_config_file_name()))
    }

    pub fn copy(plugins: collections::HashMap<String, Plugin>) -> PluginConfig {
        PluginConfig { plugins }
    }

    pub fn get_plugins(&self) -> &collections::HashMap<String, Plugin> {
        &self.plugins
    }

    pub fn get_plugin_by_asset_id(&self, asset_id: String) -> Option<&Plugin> {
        let result = self.plugins.values().find(|p| p.asset_id == asset_id);
        match result {
            Some(plugin) => Some(plugin),
            None => None,
        }
    }

    pub fn get_plugin_by_name(&self, name: &str) -> Option<String> {
        let plugin_name = self.plugins.get_key_value(name);
        match plugin_name {
            Some((key, _)) => Some(key.to_string()),
            None => None,
        }
    }

    // TODO create a method that checks if a plugin is already installed by asset id
    // e.g. addons/<plugin> exists and is listed in gdm.json
    pub fn check_if_plugin_already_installed_by_asset_id(&self, asset_id: &str) -> Option<&Plugin> {
        let plugin = self.get_plugin_by_asset_id(asset_id.to_string());
        match plugin {
            Some(p) => {
                println!(
                    "Plugin with asset ID {} is already installed: {} (version {})",
                    asset_id, p.title, p.version
                );
                Some(p)
            }
            None => None,
        }
    }

    pub fn default(config_file_name: &str) -> PluginConfig {
        Self::init(config_file_name)
    }

    fn init(config_file_name: &str) -> PluginConfig {
        let config_file = fs::File::open(config_file_name);

        match config_file {
            Ok(file) => {
                let config: PluginConfig = serde_json::from_reader(file).unwrap();

                if config.plugins.is_empty() {
                    return PluginConfig {
                        plugins: collections::HashMap::new(),
                    };
                }

                config
            }
            Err(_) => {
                return PluginConfig {
                    plugins: collections::HashMap::new(),
                };
            }
        }
    }

    pub fn add_plugins(&self, new_plugins: collections::HashMap<String, Plugin>) -> Result<()> {
        let new_plugin_config = self.update_plugins(new_plugins);
        self.write_config(&new_plugin_config)?;
        Ok(())
    }

    pub fn remove_installed_plugin(&self, plugin_key: &str) -> Result<()> {
        self.remove_plugins(vec![plugin_key.to_string()])?;
        Ok(())
    }

    fn remove_plugins(&self, plugins_to_remove: Vec<String>) -> Result<()> {
        let mut plugins_copy = self.plugins.clone();

        for plugin_key in plugins_to_remove {
            plugins_copy.remove(&plugin_key);
        }

        let new_plugin_config = PluginConfig::copy(plugins_copy);
        self.write_config(&new_plugin_config)?;
        Ok(())
    }

    fn update_plugins(&self, new_plugins: collections::HashMap<String, Plugin>) -> PluginConfig {
        let mut plugins_copy = self.plugins.clone();

        for (key, plugin) in new_plugins {
            plugins_copy.insert(key, plugin);
        }

        PluginConfig::copy(plugins_copy)
    }

    fn write_config(&self, plugin_config: &PluginConfig) -> Result<()> {
        let config_file_name = AppConfig::new().get_config_file_name();
        let file = fs::File::create(config_file_name).with_context(|| {
            format!(
                "Failed to create or open the configuration file: {}",
                config_file_name
            )
        })?;

        serde_json::to_writer_pretty(file, plugin_config).with_context(|| {
            format!(
                "Failed to write configuration to file: {}",
                config_file_name
            )
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_return_non_empty_plugins_from_plugin_config_file() {
        let plugin_config = PluginConfig::default("test/mocks/gdm.json");
        assert!(!plugin_config.plugins.is_empty());
    }

    #[test]
    fn test_should_return_correct_plugins_keys_from_plugin_config_file() {
        let plugin_config = PluginConfig::default("test/mocks/gdm.json");
        assert_eq!(plugin_config.plugins.len(), 2);
        assert!(plugin_config.plugins.contains_key("plugin_1"));
        assert!(plugin_config.plugins.contains_key("plugin_2"));
    }

    #[test]
    fn test_should_add_new_plugins() {
        let plugin_config = PluginConfig::default("test/mocks/gdm.json");
        let mut new_plugins = collections::HashMap::new();
        new_plugins.insert(
            "plugin_3".to_string(),
            Plugin {
                asset_id: "67890".to_string(),
                title: "New Plugin".to_string(),
                version: "1.0.0".to_string(),
            },
        );
        let updated_plugin_config = plugin_config.update_plugins(new_plugins);
        assert!(updated_plugin_config.plugins.contains_key("plugin_1"));
        assert!(updated_plugin_config.plugins.contains_key("plugin_2"));
        assert!(updated_plugin_config.plugins.contains_key("plugin_3"));
    }

    #[test]
    fn test_should_return_correct_plugins_from_plugin_config_file() {
        let plugin_config = PluginConfig::default("test/mocks/gdm.json");
        let plugin_1 = plugin_config.plugins.get("plugin_1").unwrap();
        let plugin_2 = plugin_config.plugins.get("plugin_2").unwrap();
        let expected_plugin_1 = Plugin {
            asset_id: "54321".to_string(),
            title: "Awesome Plugin".to_string(),
            version: "1.0.0".to_string(),
        };
        let expected_plugin_2 = Plugin {
            asset_id: "12345".to_string(),
            title: "Super Plugin".to_string(),
            version: "2.1.3".to_string(),
        };
        assert_eq!(plugin_1.asset_id, expected_plugin_1.asset_id);
        assert_eq!(plugin_1.title, expected_plugin_1.title);
        assert_eq!(plugin_1.version, expected_plugin_1.version);
        assert_eq!(plugin_2.title, expected_plugin_2.title);
        assert_eq!(plugin_2.version, expected_plugin_2.version);
        assert_eq!(plugin_2.asset_id, expected_plugin_2.asset_id);
    }
}
