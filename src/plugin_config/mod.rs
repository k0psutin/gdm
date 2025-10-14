use crate::app_config::AppConfig;
use crate::file_service::FileService;
use anyhow::Context;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PluginConfig {
    plugins: HashMap<String, Plugin>,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
    pub fn new(config_file_name: &str) -> PluginConfig {
       Self::init(config_file_name)
    }

    pub fn copy(plugins: HashMap<String, Plugin>) -> PluginConfig {
        PluginConfig { plugins }
    }

    pub fn get_plugins(&self) -> &HashMap<String, Plugin> {
        &self.plugins
    }

    pub fn get_plugin_by_asset_id(&self, asset_id: String) -> Option<&Plugin> {
        let result = self.plugins.values().find(|p| p.asset_id == asset_id);
        match result {
            Some(plugin) => Some(plugin),
            None => None,
        }
    }

    pub fn get_plugin_key_by_name(&self, name: &str) -> Option<String> {
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

    pub fn default() -> PluginConfig {
        Self::init(&AppConfig::default().get_config_file_name())
    }

    fn init(config_file_name: &str) -> PluginConfig {
        let config_file = FileService::read_file_cached(&PathBuf::from(config_file_name));

        match config_file {
            Ok(file) => {
                let config: PluginConfig = serde_json::from_str(&file).unwrap();

                if config.plugins.is_empty() {
                    return PluginConfig {
                        plugins: HashMap::new(),
                    };
                }

                config
            }
            Err(_) => {
                return PluginConfig {
                    plugins: HashMap::new(),
                };
            }
        }
    }

    pub fn add_plugins(&self, new_plugins: HashMap<String, Plugin>) -> Result<()> {
        let new_plugin_config = self.update_plugins(new_plugins);
        self.write_config(&new_plugin_config)?;
        Ok(())
    }

    pub fn remove_installed_plugin(&self, plugin_key: String) -> Result<()> {
        self.remove_plugins(vec![plugin_key])?;
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

    fn update_plugins(&self, new_plugins: HashMap<String, Plugin>) -> PluginConfig {
        let mut plugins_copy = self.plugins.clone();

        for (key, plugin) in new_plugins {
            plugins_copy.insert(key, plugin);
        }

        let mut plugins = plugins_copy.into_iter().collect::<Vec<_>>();

        plugins.sort_by(|a, b| a.0.cmp(&b.0));

        PluginConfig::copy(HashMap::from_iter(plugins))
    }

    fn write_config(&self, plugin_config: &PluginConfig) -> Result<()> {
        let config_file_name = AppConfig::default().get_config_file_name();
        let file = FileService::create_file(&PathBuf::from(config_file_name))?;

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
        let plugin_config = PluginConfig::new("test/mocks/gdm.json");
        assert!(!plugin_config.plugins.is_empty());
    }

    #[test]
    fn test_should_return_empty_plugins_from_non_existent_plugin_config_file() {
        let plugin_config = PluginConfig::new("test/mocks/non_existent.json");
        assert!(plugin_config.plugins.is_empty());
    }


    #[test]
    fn test_should_return_correct_plugins_from_plugin_config_file() {
        let plugin_config = PluginConfig::new("test/mocks/gdm.json");
        let expected = serde_json::json!({
            "plugins": {
                "plugin_1": {
                    "asset_id": "54321",
                    "title": "Awesome Plugin",
                    "version": "1.0.0"
                },
                "plugin_2": {
                    "asset_id": "12345",
                    "title": "Super Plugin",
                    "version": "2.1.3"
                }
            }
        });
        let actual = serde_json::to_value(&plugin_config).unwrap();
        assert_eq!(actual, expected);
    }

    // update_plugins

    #[test]
    fn test_should_add_new_plugins() {
        let plugin_config = PluginConfig::new("test/mocks/gdm.json");
        let mut new_plugins = HashMap::new();
        new_plugins.insert(
            "plugin_3".to_string(),
            Plugin {
                asset_id: "67890".to_string(),
                title: "New Plugin".to_string(),
                version: "1.0.0".to_string(),
            },
        );
        let updated_plugin_config = plugin_config.update_plugins(new_plugins);
        let expected = serde_json::json!({
            "plugins": {
                "plugin_1": {
                    "asset_id": "54321",
                    "title": "Awesome Plugin",
                    "version": "1.0.0"
                },
                "plugin_2": {
                    "asset_id": "12345",
                    "title": "Super Plugin",
                    "version": "2.1.3"
                },
                "plugin_3": {
                    "asset_id": "67890",
                    "title": "New Plugin",
                    "version": "1.0.0"
                }
            }
        });
        let actual = serde_json::to_value(&updated_plugin_config).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_should_add_new_plugins_in_correct_order() {
        let plugin_config = PluginConfig::new("test/mocks/gdm.json");
        let mut new_plugins = HashMap::new();
        new_plugins.insert(
            "a_plugin".to_string(),
            Plugin {
                asset_id: "67890".to_string(),
                title: "New Plugin".to_string(),
                version: "1.0.0".to_string(),
            },
        );
        let updated_plugin_config = plugin_config.update_plugins(new_plugins);
        let expected = serde_json::json!({
            "plugins": {
                "a_plugin": {
                    "asset_id": "67890",
                    "title": "New Plugin",
                    "version": "1.0.0"
                },
                "plugin_1": {
                    "asset_id": "54321",
                    "title": "Awesome Plugin",
                    "version": "1.0.0"
                },
                "plugin_2": {
                    "asset_id": "12345",
                    "title": "Super Plugin",
                    "version": "2.1.3"
                },
            }
        });
        let actual = serde_json::to_value(&updated_plugin_config).unwrap();
        assert_eq!(actual, expected);
    }

    // get_plugins

    #[test]
    fn test_get_plugins_should_return_empty_vec_if_no_plugins_installed() {
        let plugin_config = PluginConfig::new("test/mocks/empty_gdm.json");
        let plugins = plugin_config.get_plugins();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_get_plugins_should_return_correct_plugins() {
       let plugin_config = PluginConfig::new("test/mocks/gdm.json");
        let plugins = plugin_config.get_plugins();
        assert!(!plugins.is_empty());
         assert_eq!(plugins.len(), 2);
        let plugin_1 = plugins.get("plugin_1").unwrap();
        assert_eq!(plugin_1.asset_id, "54321");
        assert_eq!(plugin_1.title, "Awesome Plugin");
        assert_eq!(plugin_1.version, "1.0.0");
        let plugin_2 = plugins.get("plugin_2").unwrap();
        assert_eq!(plugin_2.asset_id, "12345");
        assert_eq!(plugin_2.title, "Super Plugin");
        assert_eq!(plugin_2.version, "2.1.3");
    }

    // get_plugin_key_by_name
    // add_plugins
    // remove_installed_plugin
    // remove_plugins
    // write_config
}
