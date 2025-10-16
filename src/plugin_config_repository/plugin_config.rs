use crate::plugin_config_repository::plugin::Plugin;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PluginConfig {
    plugins: HashMap<String, Plugin>,
}

impl PluginConfig {
    pub fn new(plugins: HashMap<String, Plugin>) -> PluginConfig {
        PluginConfig { plugins }
    }  
}

impl Default for PluginConfig {
    fn default() -> Self {
        PluginConfig::new(HashMap::new())
    }
}

#[cfg_attr(test, mockall::automock)]
impl PluginConfigImpl for PluginConfig {
    fn get_plugins(&self) -> &HashMap<String, Plugin> {
        &self.plugins
    }
}

pub trait PluginConfigImpl {
    fn get_plugins(&self) -> &HashMap<String, Plugin>;

    fn copy(&self, plugins: Option<HashMap<String, Plugin>>, _godot_version: Option<String>) -> PluginConfig {
        let plugins = match plugins {
            Some(p) => p,
            None => self.get_plugins().clone(),
        };

        PluginConfig { plugins }
    }

    fn get_plugin_by_asset_id(&self, asset_id: String) -> Option<&Plugin> {
        let result = self.get_plugins().values().find(|p| p.get_asset_id() == asset_id);
        match result {
            Some(plugin) => Some(plugin),
            None => None,
        }
    }

    fn get_plugin_key_by_name(&self, name: &str) -> Option<String> {
        let plugin_name = self.get_plugins().get_key_value(name);
        Some(plugin_name)?.map(|(key, _)| key.clone())
    }

    // TODO create a method that checks if a plugin is already installed by asset id
    // e.g. addons/<plugin> exists and is listed in gdm.json
    fn check_if_plugin_already_installed_by_asset_id(&self, asset_id: &str) -> Option<&Plugin> {
        let plugin = self.get_plugin_by_asset_id(asset_id.to_string());
        match plugin {
            Some(p) => {
                println!(
                    "Plugin with asset ID {} is already installed: {} (version {})",
                    asset_id, p.get_title(), p.get_version()
                );
                Some(p)
            }
            None => None,
        }
    }

    fn remove_plugins(&self, plugins: HashSet<String>) -> PluginConfig {
        let mut _plugins = self.get_plugins().clone();

        for plugin_key in plugins {
            _plugins.remove(&plugin_key);
        }

        self.copy(Some(_plugins), None)
    }

    fn add_plugins(&self, new_plugins: HashMap<String, Plugin>) -> PluginConfig {
        let mut plugins_copy = self.get_plugins().clone();

        for (key, plugin) in new_plugins {
            plugins_copy.insert(key, plugin);
        }

        let mut plugins = plugins_copy.into_iter().collect::<Vec<_>>();

        plugins.sort_by(|a, b| a.0.cmp(&b.0));

        self.copy(Some(HashMap::from_iter(plugins)), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_plugin_config() -> PluginConfig {
        PluginConfig::default()
    }

    fn setup_test_plugin_config_non_existent_config() -> PluginConfig {
        PluginConfig::default()
    }

    #[test]
    fn test_should_return_non_empty_plugins_from_plugin_config_file() {
        let plugin_config = setup_test_plugin_config();
        assert!(!plugin_config.get_plugins().is_empty());
    }

    #[test]
    fn test_should_return_empty_plugins_from_non_existent_plugin_config_file() {
        let plugin_config = setup_test_plugin_config_non_existent_config();
        assert!(plugin_config.get_plugins().is_empty());
    }


    #[test]
    fn test_should_return_correct_plugins_from_plugin_config_file() {
        let plugin_config = setup_test_plugin_config();
        let expected = vec![Plugin::new(
            "54321".to_string(),
            "Awesome Plugin".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
        ), Plugin::new(
            "12345".to_string(),
            "Super Plugin".to_string(),
            "2.1.3".to_string(),
            "Apache-2.0".to_string(),
        )];

        let result = plugin_config.get_plugins().values().cloned().collect::<Vec<Plugin>>();
        assert_eq!(result.len(), expected.len());
        assert_eq!(result, expected);
    }

    // update_plugins

    #[test]
    fn test_should_add_new_plugins() {
        let plugin_config = setup_test_plugin_config();
        let mut new_plugins = HashMap::new();
        new_plugins.insert(
            "plugin_3".to_string(),
            Plugin::new(
                "67890".to_string(),
                "New Plugin".to_string(),
                "1.0.0".to_string(),
                "GPL-3.0".to_string(),
            ),
        );
        let updated_plugin_config = plugin_config.add_plugins(new_plugins);
        let expected = serde_json::json!({
            "plugins": {
                "plugin_1": {
                    "asset_id": "54321",
                    "title": "Awesome Plugin",
                    "version": "1.0.0",
                    "license": "MIT"
                },
                "plugin_2": {
                    "asset_id": "12345",
                    "title": "Super Plugin",
                    "version": "2.1.3",
                    "license": "Apache-2.0"
                },
                "plugin_3": {
                    "asset_id": "67890",
                    "title": "New Plugin",
                    "version": "1.0.0",
                    "license": "GPL-3.0"
                }
            }
        });
        let actual = serde_json::to_value(&updated_plugin_config).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_should_add_new_plugins_in_correct_order() {
        let plugin_config = setup_test_plugin_config();
        let mut new_plugins = HashMap::new();
        new_plugins.insert(
            "a_plugin".to_string(),
            Plugin::new(
                "67890".to_string(),
                "New Plugin".to_string(),
                "1.0.0".to_string(),
                "GPL-3.0".to_string(),
            ),
        );
        let updated_plugin_config = plugin_config.add_plugins(new_plugins);
        let expected = serde_json::json!({
            "plugins": {
                "a_plugin": {
                    "asset_id": "67890",
                    "title": "New Plugin",
                    "version": "1.0.0",
                    "license": "GPL-3.0"
                },
                "plugin_1": {
                    "asset_id": "54321",
                    "title": "Awesome Plugin",
                    "version": "1.0.0",
                    "license": "MIT"
                },
                "plugin_2": {
                    "asset_id": "12345",
                    "title": "Super Plugin",
                    "version": "2.1.3",
                    "license": "Apache-2.0"
                },
            }
        });
        let actual = serde_json::to_value(&updated_plugin_config).unwrap();
        assert_eq!(actual, expected);
    }

    // get_plugins

    #[test]
    fn test_get_plugins_should_return_empty_vec_if_no_plugins_installed() {
        let plugin_config = setup_test_plugin_config();
        let plugins = plugin_config.get_plugins();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_get_plugins_should_return_correct_plugins() {
       let plugin_config = setup_test_plugin_config();
        let plugins = plugin_config.get_plugins();
        assert!(!plugins.is_empty());
         assert_eq!(plugins.len(), 2);
        let plugin_1 = plugins.get("plugin_1").unwrap();
        assert_eq!(plugin_1.get_asset_id(), "54321");
        assert_eq!(plugin_1.get_title(), "Awesome Plugin");
        assert_eq!(plugin_1.get_version(), "1.0.0");
        let plugin_2 = plugins.get("plugin_2").unwrap();
        assert_eq!(plugin_2.get_asset_id(), "12345");
        assert_eq!(plugin_2.get_title(), "Super Plugin");
        assert_eq!(plugin_2.get_version(), "2.1.3");
    }

    // get_plugin_key_by_name
    // add_plugins
    // remove_installed_plugin
    // remove_plugins
    // write_config
}
