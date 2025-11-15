use crate::plugin_config_repository::plugin::Plugin;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use tracing::info;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DefaultPluginConfig {
    pub plugins: BTreeMap<String, Plugin>,
}

// TODO Change to hashmap again?
impl DefaultPluginConfig {
    pub fn new(plugins: BTreeMap<String, Plugin>) -> DefaultPluginConfig {
        DefaultPluginConfig { plugins }
    }
}

impl Default for DefaultPluginConfig {
    fn default() -> Self {
        DefaultPluginConfig::new(BTreeMap::new())
    }
}

#[cfg_attr(test, mockall::automock)]
impl PluginConfig for DefaultPluginConfig {
    fn get_plugin_by_asset_id(&self, asset_id: &str) -> Option<Plugin> {
        self.plugins
            .iter()
            .find(|(_, p)| p.asset_id == asset_id)
            .map(|(_, p)| p.clone())
    }

    fn get_plugin_by_name(&self, name: &str) -> Option<Plugin> {
        self.plugins.get(name).cloned()
    }

    fn remove_plugins(&self, plugins: HashSet<String>) -> DefaultPluginConfig {
        let mut _plugins = self.plugins.clone();
        for plugin_key in plugins {
            _plugins.remove(&plugin_key);
            info!("Removed plugin: {}", plugin_key);
        }

        DefaultPluginConfig::new(_plugins)
    }

    fn add_plugins(&self, plugins: &BTreeMap<String, Plugin>) -> DefaultPluginConfig {
        let mut _plugins = self.plugins.clone();
        for (key, plugin) in plugins {
            _plugins.insert(key.clone(), plugin.clone());
            info!("Added/Updated plugin: {}", key);
        }

        DefaultPluginConfig::new(_plugins)
    }

    fn get_plugins(&self, only_plugin_config: bool) -> BTreeMap<String, Plugin> {
        if only_plugin_config {
            self.plugins
                .iter()
                .filter(|(_, p)| p.plugin_cfg_path.is_some())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        } else {
            self.plugins.clone()
        }
    }
}

pub trait PluginConfig {
    fn get_plugin_by_asset_id(&self, asset_id: &str) -> Option<Plugin>;
    fn get_plugin_by_name(&self, name: &str) -> Option<Plugin>;
    fn remove_plugins(&self, plugins: HashSet<String>) -> DefaultPluginConfig;
    fn add_plugins(&self, plugins: &BTreeMap<String, Plugin>) -> DefaultPluginConfig;
    fn get_plugins(&self, only_plugin_config: bool) -> BTreeMap<String, Plugin>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_plugin_map() -> BTreeMap<String, Plugin> {
        BTreeMap::from([
            ("plugin_1".to_string(), Plugin::create_mock_plugin_1()),
            ("plugin_2".to_string(), Plugin::create_mock_plugin_2()),
        ])
    }

    fn setup_test_plugin_config() -> DefaultPluginConfig {
        let plugins: BTreeMap<String, Plugin> = setup_test_plugin_map();
        DefaultPluginConfig::new(plugins)
    }

    fn setup_test_plugin_config_non_existent_config() -> DefaultPluginConfig {
        DefaultPluginConfig::default()
    }

    // get_plugins

    #[test]
    fn test_get_plugins_should_return_empty_plugins() {
        let plugin_config = setup_test_plugin_config_non_existent_config();
        assert!(plugin_config.plugins.is_empty());
    }

    #[test]
    fn test_get_plugins_should_return_non_empty_plugins() {
        let plugin_config = setup_test_plugin_config();
        assert!(!plugin_config.plugins.is_empty());
    }

    #[test]
    fn test_get_plugins_should_return_correct_plugins_from_plugin_config_file() {
        let plugin_config = setup_test_plugin_config();
        let expected = BTreeMap::from([
            ("plugin_1".to_string(), Plugin::create_mock_plugin_1()),
            ("plugin_2".to_string(), Plugin::create_mock_plugin_2()),
        ]);

        let result = plugin_config.plugins;
        assert_eq!(result.len(), expected.len());
        assert_eq!(result, expected);
    }

    // add_plugins

    #[test]
    fn test_should_add_new_plugins() {
        let plugin_config = setup_test_plugin_config();
        let new_plugins = BTreeMap::from([(
            "plugin_3".to_string(),
            Plugin::new(
                "67890".to_string(),
                Some("addons/super_plugin/plugin.cfg".into()),
                "New Plugin".to_string(),
                "1.0.0".to_string(),
                "GPL-3.0".to_string(),
                vec![],
            ),
        )]);

        let updated_plugin_config = plugin_config.add_plugins(&new_plugins);
        let actual = updated_plugin_config.plugins.clone();

        let expected = BTreeMap::from([
            ("plugin_1".to_string(), Plugin::create_mock_plugin_1()),
            ("plugin_2".to_string(), Plugin::create_mock_plugin_2()),
            (
                "plugin_3".to_string(),
                Plugin::new(
                    "67890".to_string(),
                    Some("addons/super_plugin/plugin.cfg".into()),
                    "New Plugin".to_string(),
                    "1.0.0".to_string(),
                    "GPL-3.0".to_string(),
                    vec![],
                ),
            ),
        ]);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_should_replace_old_plugins() {
        let plugin_config = setup_test_plugin_config();
        let new_plugins = BTreeMap::from([
            (
                "plugin_1".to_string(),
                Plugin::new(
                    "54321".to_string(),
                    Some("addons/awesome_plugin/plugin.cfg".into()),
                    "Awesome Plugin".to_string(),
                    "1.8.0".to_string(),
                    "MIT".to_string(),
                    vec![],
                ),
            ),
            (
                "plugin_2".to_string(),
                Plugin::new(
                    "12345".to_string(),
                    Some("addons/super_plugin/plugin.cfg".into()),
                    "Super Plugin".to_string(),
                    "2.1.3".to_string(),
                    "MIT".to_string(),
                    vec![],
                ),
            ),
        ]);

        let updated_plugin_config = plugin_config.add_plugins(&new_plugins);
        let actual = updated_plugin_config.plugins.clone();

        let expected = BTreeMap::from([
            (
                "plugin_1".to_string(),
                Plugin::new(
                    "54321".to_string(),
                    Some("addons/awesome_plugin/plugin.cfg".into()),
                    "Awesome Plugin".to_string(),
                    "1.8.0".to_string(),
                    "MIT".to_string(),
                    vec![],
                ),
            ),
            (
                "plugin_2".to_string(),
                Plugin::new(
                    "12345".to_string(),
                    Some("addons/super_plugin/plugin.cfg".into()),
                    "Super Plugin".to_string(),
                    "2.1.3".to_string(),
                    "MIT".to_string(),
                    vec![],
                ),
            ),
        ]);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_should_not_add_duplicate_plugins() {
        let plugin_config = setup_test_plugin_config();
        let new_plugins = BTreeMap::from([
            (
                "plugin_1".to_string(),
                Plugin::new(
                    "54321".to_string(),
                    Some("addons/awesome_plugin/plugin.cfg".into()),
                    "Awesome Plugin".to_string(),
                    "1.8.0".to_string(),
                    "MIT".to_string(),
                    vec![],
                ),
            ),
            (
                "plugin_1".to_string(),
                Plugin::new(
                    "54321".to_string(),
                    Some("addons/awesome_plugin/plugin.cfg".into()),
                    "Awesome Plugin".to_string(),
                    "1.8.0".to_string(),
                    "MIT".to_string(),
                    vec![],
                ),
            ),
            (
                "plugin_2".to_string(),
                Plugin::new(
                    "12345".to_string(),
                    Some("addons/super_plugin/plugin.cfg".into()),
                    "Super Plugin".to_string(),
                    "2.1.3".to_string(),
                    "Apache-2.0".to_string(),
                    vec![],
                ),
            ),
        ]);

        let updated_plugin_config = plugin_config.add_plugins(&new_plugins);
        let actual = updated_plugin_config.plugins.clone();

        let expected = BTreeMap::from([
            (
                "plugin_1".to_string(),
                Plugin::new(
                    "54321".to_string(),
                    Some("addons/awesome_plugin/plugin.cfg".into()),
                    "Awesome Plugin".to_string(),
                    "1.8.0".to_string(),
                    "MIT".to_string(),
                    vec![],
                ),
            ),
            (
                "plugin_2".to_string(),
                Plugin::new(
                    "12345".to_string(),
                    Some("addons/super_plugin/plugin.cfg".into()),
                    "Super Plugin".to_string(),
                    "2.1.3".to_string(),
                    "Apache-2.0".to_string(),
                    vec![],
                ),
            ),
        ]);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_should_add_new_plugins_in_correct_order() {
        let plugin_config = setup_test_plugin_config();
        let new_plugins = BTreeMap::from([(
            "a_plugin".to_string(),
            Plugin::new(
                "67890".to_string(),
                Some("addons/super_plugin/plugin.cfg".into()),
                "New Plugin".to_string(),
                "1.0.0".to_string(),
                "GPL-3.0".to_string(),
                vec![],
            ),
        )]);
        let updated_plugin_config = plugin_config.add_plugins(&new_plugins);
        let actual = updated_plugin_config.plugins.clone();

        let expected = BTreeMap::from([
            (
                "a_plugin".to_string(),
                Plugin::new(
                    "67890".to_string(),
                    Some("addons/super_plugin/plugin.cfg".into()),
                    "New Plugin".to_string(),
                    "1.0.0".to_string(),
                    "GPL-3.0".to_string(),
                    vec![],
                ),
            ),
            (
                "plugin_1".to_string(),
                Plugin::new(
                    "54321".to_string(),
                    Some("addons/awesome_plugin/plugin.cfg".into()),
                    "Awesome Plugin".to_string(),
                    "1.0.0".to_string(),
                    "MIT".to_string(),
                    vec![],
                ),
            ),
            (
                "plugin_2".to_string(),
                Plugin::new(
                    "12345".to_string(),
                    Some("addons/super_plugin/plugin.cfg".into()),
                    "Super Plugin".to_string(),
                    "2.1.3".to_string(),
                    "Apache-2.0".to_string(),
                    vec![],
                ),
            ),
        ]);
        assert_eq!(actual, expected);
    }

    // get_plugin_by_name

    #[test]
    fn test_get_plugin_key_by_name_should_return_correct_key() {
        let plugin_config = setup_test_plugin_config();
        let plugin_opt = plugin_config.get_plugin_by_name("plugin_1");
        assert!(plugin_opt.is_some());
        let plugin = plugin_opt.unwrap();
        assert_eq!(plugin.asset_id, "54321".to_string());
    }

    // remove_installed_plugin

    #[test]
    fn test_should_remove_plugins() {
        let plugin_config = setup_test_plugin_config();
        let plugins_to_remove: HashSet<String> = vec!["plugin_1".to_string()].into_iter().collect();
        let updated_plugin_config = plugin_config.remove_plugins(plugins_to_remove);
        let expected = BTreeMap::from([(
            "plugin_2".to_string(),
            Plugin::new(
                "12345".to_string(),
                Some("addons/super_plugin/plugin.cfg".into()),
                "Super Plugin".to_string(),
                "2.1.3".to_string(),
                "Apache-2.0".to_string(),
                vec![],
            ),
        )]);
        let actual = updated_plugin_config.plugins.clone();
        assert_eq!(actual, expected);
    }

    // remove_plugins

    #[test]
    fn test_should_remove_multiple_plugins() {
        let plugin_config = setup_test_plugin_config();
        let plugins_to_remove: HashSet<String> =
            HashSet::from(["plugin_1".to_string(), "plugin_2".to_string()]);
        let updated_plugin_config = plugin_config.remove_plugins(plugins_to_remove);

        let expected = BTreeMap::new();
        let actual = updated_plugin_config.plugins.clone();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_should_not_panic_on_removing_non_existent_plugins() {
        let plugin_config = setup_test_plugin_config();
        let plugins_to_remove: HashSet<String> = HashSet::from(["non_existent_plugin".to_string()]);
        let updated_plugin_config = plugin_config.remove_plugins(plugins_to_remove);

        assert_eq!(
            updated_plugin_config.plugins.clone(),
            plugin_config.plugins.clone()
        );
    }

    // get_plugin_by_asset_id

    #[test]
    fn test_get_plugin_by_asset_id_should_return_correct_plugin() {
        let plugin_config = setup_test_plugin_config();
        let plugin = plugin_config.get_plugin_by_asset_id("54321");
        let expected_plugin = Plugin::new(
            "54321".to_string(),
            Some("addons/awesome_plugin/plugin.cfg".into()),
            "Awesome Plugin".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
            vec![],
        );
        assert_eq!(plugin, Some(expected_plugin));
    }

    #[test]
    fn test_get_plugin_by_asset_id_should_return_none() {
        let plugin_config = setup_test_plugin_config();
        let plugin = plugin_config.get_plugin_by_asset_id("4321");
        assert_eq!(plugin, None);
    }

    // get_plugins
    #[test]
    fn test_get_plugins_should_return_all_plugins() {
        let mut plugin_config = setup_test_plugin_config();
        plugin_config = plugin_config.add_plugins(&BTreeMap::from([(
            "plugin_3".to_string(),
            Plugin::create_mock_plugin_3(),
        )]));
        let plugins = plugin_config.get_plugins(false);
        let expected_plugins = BTreeMap::from([
            ("plugin_1".to_string(), Plugin::create_mock_plugin_1()),
            ("plugin_2".to_string(), Plugin::create_mock_plugin_2()),
            ("plugin_3".to_string(), Plugin::create_mock_plugin_3()),
        ]);
        assert_eq!(plugins, expected_plugins);
    }

    #[test]
    fn test_get_plugins_should_return_plugins_with_plugin_config() {
        let mut plugin_config = setup_test_plugin_config();
        plugin_config = plugin_config.add_plugins(&BTreeMap::from([(
            "plugin_3".to_string(),
            Plugin::create_mock_plugin_3(),
        )]));
        let plugins = plugin_config.get_plugins(true);
        let expected_plugins = BTreeMap::from([
            ("plugin_1".to_string(), Plugin::create_mock_plugin_1()),
            ("plugin_2".to_string(), Plugin::create_mock_plugin_2()),
        ]);
        assert_eq!(plugins, expected_plugins);
    }
}
