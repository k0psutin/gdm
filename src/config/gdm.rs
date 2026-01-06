use crate::config::{AppConfig, DefaultAppConfig};
use crate::models::{Plugin, PluginSource};
use crate::services::{DefaultFileService, FileService};

use anyhow::{Context, Result};
use serde_derive::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DefaultGdmConfigMetadata {
    pub plugins: BTreeMap<String, Plugin>,
}

impl DefaultGdmConfigMetadata {
    pub fn new(plugins: BTreeMap<String, Plugin>) -> DefaultGdmConfigMetadata {
        DefaultGdmConfigMetadata { plugins }
    }
}

impl Default for DefaultGdmConfigMetadata {
    fn default() -> Self {
        DefaultGdmConfigMetadata::new(BTreeMap::new())
    }
}

#[cfg_attr(test, mockall::automock)]
impl GdmConfigMetadata for DefaultGdmConfigMetadata {
    fn get_plugin_by_asset_id(&self, asset_id: &str) -> Option<Plugin> {
        self.plugins
            .iter()
            .find(|(_, p)| {
                if let Some(PluginSource::AssetLibrary { asset_id: id }) = &p.source {
                    id == asset_id
                } else {
                    false
                }
            })
            .map(|(_, p)| p.clone())
    }

    fn get_plugin_by_name(&self, name: &str) -> Option<Plugin> {
        self.plugins.get(name).cloned()
    }

    fn remove_plugins(&self, plugins: HashSet<String>) -> DefaultGdmConfigMetadata {
        let mut _plugins = self.plugins.clone();
        for plugin_key in plugins {
            _plugins.remove(&plugin_key);
            info!("Removed plugin: {}", plugin_key);
        }

        DefaultGdmConfigMetadata::new(_plugins)
    }

    fn add_plugins(&self, plugins: &BTreeMap<String, Plugin>) -> DefaultGdmConfigMetadata {
        let mut _plugins = self.plugins.clone();
        for (key, plugin) in plugins {
            _plugins.insert(key.clone(), plugin.clone());
            info!("Added/Updated plugin: {}", key);
        }

        DefaultGdmConfigMetadata::new(_plugins)
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

pub trait GdmConfigMetadata {
    fn get_plugin_by_asset_id(&self, asset_id: &str) -> Option<Plugin>;
    fn get_plugin_by_name(&self, name: &str) -> Option<Plugin>;
    fn remove_plugins(&self, plugins: HashSet<String>) -> DefaultGdmConfigMetadata;
    fn add_plugins(&self, plugins: &BTreeMap<String, Plugin>) -> DefaultGdmConfigMetadata;
    fn get_plugins(&self, only_plugin_config: bool) -> BTreeMap<String, Plugin>;
}

pub struct DefaultGdmConfig {
    pub app_config: DefaultAppConfig,
    pub file_service: Arc<dyn FileService + Send + Sync + 'static>,
}

impl Default for DefaultGdmConfig {
    fn default() -> Self {
        DefaultGdmConfig {
            file_service: Arc::new(DefaultFileService),
            app_config: DefaultAppConfig::default(),
        }
    }
}

impl DefaultGdmConfig {
    #[allow(unused)]
    pub fn new(
        app_config: DefaultAppConfig,
        file_service: Arc<dyn FileService + Send + Sync + 'static>,
    ) -> Self {
        DefaultGdmConfig {
            app_config,
            file_service,
        }
    }
}

#[cfg_attr(test, mockall::automock)]
impl GdmConfig for DefaultGdmConfig {
    fn add_plugins(&self, plugins: &BTreeMap<String, Plugin>) -> Result<DefaultGdmConfigMetadata> {
        debug!("Adding plugins: {:?}", plugins.keys());
        let plugin_config = self.load()?;
        let updated_plugin_config = plugin_config.add_plugins(plugins);
        self.save(&updated_plugin_config)?;
        info!("Added plugins {:?}", updated_plugin_config.plugins.keys());
        Ok(updated_plugin_config)
    }

    fn remove_plugins(&self, plugin_keys: HashSet<String>) -> Result<DefaultGdmConfigMetadata> {
        debug!("Removing plugins: {:?}", plugin_keys);
        let plugin_config = self.load()?;
        let updated_plugin_config = plugin_config.remove_plugins(plugin_keys);
        self.save(&updated_plugin_config)?;
        info!("Removed plugins {:?}", updated_plugin_config.plugins.keys());
        Ok(updated_plugin_config)
    }

    fn get_plugin_by_name(&self, name: &str) -> Option<(String, Plugin)> {
        let plugin_config = self.load().ok()?;
        let plugin: Option<Plugin> = plugin_config.get_plugin_by_name(name);
        if let Some(p) = plugin {
            return Some((name.to_string(), p));
        }
        None
    }

    fn get_plugin_by_asset_id(&self, asset_id: &str) -> Result<Option<Plugin>> {
        let plugin_config = self.load()?;
        Ok(plugin_config.get_plugin_by_asset_id(asset_id))
    }

    /// Returns a sorted list of plugins in a tuple of (key, Plugin)
    ///
    /// The list is sorted by the plugin key in ascending order
    fn get_plugins(&self) -> Result<BTreeMap<String, Plugin>> {
        let plugin_config = self.load()?;
        Ok(plugin_config.plugins.clone())
    }

    fn has_installed_plugins(&self) -> Result<bool> {
        let plugins = self.get_plugins()?;

        Ok(!plugins.is_empty())
    }

    fn load(&self) -> Result<DefaultGdmConfigMetadata> {
        let config_file_path = self.app_config.get_config_file_path();

        if !self.file_service.file_exists(config_file_path)? {
            return Ok(DefaultGdmConfigMetadata::default());
        }
        let content = self.file_service.read_file_cached(config_file_path)?;
        let config: DefaultGdmConfigMetadata =
            serde_json::from_str(&content).with_context(|| {
                format!(
                    "Failed to parse plugin config file: {}",
                    config_file_path.display()
                )
            })?;
        Ok(config)
    }

    fn save(&self, config: &DefaultGdmConfigMetadata) -> Result<String> {
        let config_file_path = self.app_config.get_config_file_path();

        let content = serde_json::to_string_pretty(config).with_context(|| {
            format!(
                "Failed to serialize configuration to JSON: {}",
                config_file_path.display()
            )
        })?;

        self.file_service.write_file(config_file_path, &content)?;
        info!(
            "Saved plugin config with plugins: {:?}",
            config.plugins.keys()
        );
        Ok(content)
    }
}

pub trait GdmConfig {
    fn add_plugins(&self, plugins: &BTreeMap<String, Plugin>) -> Result<DefaultGdmConfigMetadata>;
    fn get_plugin_by_asset_id(&self, asset_id: &str) -> Result<Option<Plugin>>;
    fn get_plugin_by_name(&self, name: &str) -> Option<(String, Plugin)>;
    fn get_plugins(&self) -> Result<BTreeMap<String, Plugin>>;
    fn has_installed_plugins(&self) -> Result<bool>;
    fn load(&self) -> Result<DefaultGdmConfigMetadata>;
    fn remove_plugins(&self, plugin_keys: HashSet<String>) -> Result<DefaultGdmConfigMetadata>;
    fn save(&self, config: &DefaultGdmConfigMetadata) -> Result<String>;
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use mockall::predicate::*;
    use serde_json::json;

    use crate::config::DefaultAppConfig;
    use crate::models::Plugin;
    use crate::services::{DefaultFileService, MockDefaultFileService};

    fn setup_test_plugin_map() -> BTreeMap<String, Plugin> {
        BTreeMap::from([
            ("plugin_1".to_string(), Plugin::create_mock_plugin_1()),
            ("plugin_2".to_string(), Plugin::create_mock_plugin_2()),
        ])
    }

    fn setup_test_plugin_config() -> DefaultGdmConfigMetadata {
        let plugins: BTreeMap<String, Plugin> = setup_test_plugin_map();
        DefaultGdmConfigMetadata::new(plugins)
    }

    fn setup_test_plugin_config_non_existent_config() -> DefaultGdmConfigMetadata {
        DefaultGdmConfigMetadata::default()
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
            Plugin::new_asset_store_plugin(
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
                Plugin::new_asset_store_plugin(
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
                Plugin::new_asset_store_plugin(
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
                Plugin::new_asset_store_plugin(
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
                Plugin::new_asset_store_plugin(
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
                Plugin::new_asset_store_plugin(
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
                Plugin::new_asset_store_plugin(
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
                Plugin::new_asset_store_plugin(
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
                Plugin::new_asset_store_plugin(
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
                Plugin::new_asset_store_plugin(
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
                Plugin::new_asset_store_plugin(
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
            Plugin::new_asset_store_plugin(
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
                Plugin::new_asset_store_plugin(
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
                Plugin::new_asset_store_plugin(
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
                Plugin::new_asset_store_plugin(
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
        assert_eq!(
            plugin.source,
            Some(PluginSource::AssetLibrary {
                asset_id: "54321".to_string()
            })
        );
    }

    // remove_installed_plugin

    #[test]
    fn test_should_remove_plugins() {
        let plugin_config = setup_test_plugin_config();
        let plugins_to_remove: HashSet<String> = vec!["plugin_1".to_string()].into_iter().collect();
        let updated_plugin_config = plugin_config.remove_plugins(plugins_to_remove);
        let expected = BTreeMap::from([(
            "plugin_2".to_string(),
            Plugin::new_asset_store_plugin(
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
        let expected_plugin = Plugin::new_asset_store_plugin(
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

    // load

    #[test]
    fn test_load_non_existent_file_should_return_default_config() {
        let plugin_config_repository = DefaultGdmConfig::new(
            DefaultAppConfig::new(
                None,
                Some(String::from("tests/mocks/non_existent_file.json")),
                None,
                None,
                None,
            ),
            Arc::new(DefaultFileService),
        );
        let result = plugin_config_repository.load();
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.plugins.len(), 0);
    }

    #[test]
    fn test_load_should_return_correct_plugin_config() {
        let plugin_config_repository = DefaultGdmConfig::new(
            DefaultAppConfig::new(
                None,
                Some(String::from("tests/mocks/gdm.json")),
                None,
                None,
                None,
            ),
            Arc::new(DefaultFileService),
        );
        let result = plugin_config_repository.load();
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.plugins.len(), 2);

        let mock_plugins = BTreeMap::from([
            ("plugin_1".to_string(), Plugin::create_mock_plugin_1()),
            ("plugin_2".to_string(), Plugin::create_mock_plugin_2()),
        ]);

        let expected_plugin_config = DefaultGdmConfigMetadata::new(mock_plugins);
        assert_eq!(config, expected_plugin_config);
    }

    #[test]
    fn test_get_plugins_should_return_correct_plugins() {
        let app_config = DefaultAppConfig::new(
            None,
            Some(String::from("tests/mocks/gdm.json")),
            None,
            None,
            None,
        );
        let plugin_config_repository =
            DefaultGdmConfig::new(app_config, Arc::new(DefaultFileService));
        let plugins = plugin_config_repository.get_plugins();
        assert!(plugins.is_ok());
        let plugins = plugins.unwrap();
        assert_eq!(plugins.len(), 2);

        let expected_plugins = BTreeMap::from([
            ("plugin_1".to_string(), Plugin::create_mock_plugin_1()),
            ("plugin_2".to_string(), Plugin::create_mock_plugin_2()),
        ]);
        assert_eq!(plugins, expected_plugins);
    }

    // add_plugins

    fn setup_mock_plugin_config_repository_for_add_and_remove_plugins() -> DefaultGdmConfig {
        const TEST_FILE_PATH_STR: &str = "tests/mocks/gdm.json";
        let test_file_path = Path::new(TEST_FILE_PATH_STR);

        let mut mock_file_service = MockDefaultFileService::new();
        mock_file_service
            .expect_read_file_cached()
            .with(eq(test_file_path))
            .returning(|path| Ok(std::fs::read_to_string(path).unwrap()));
        mock_file_service
            .expect_file_exists()
            .with(eq(test_file_path))
            .returning(|_| Ok(true));
        mock_file_service
            .expect_write_file()
            .returning(|_, _| Ok(()));

        let app_config = DefaultAppConfig::new(
            None,
            Some(String::from(TEST_FILE_PATH_STR)),
            None,
            None,
            None,
        );
        let file_service = Arc::new(mock_file_service);
        DefaultGdmConfig::new(app_config, file_service)
    }

    #[test]
    fn test_add_plugins_should_add_plugins_to_config() {
        let plugin_config_repository =
            setup_mock_plugin_config_repository_for_add_and_remove_plugins();

        let new_plugins =
            BTreeMap::from([("plugin_3".to_string(), Plugin::create_mock_plugin_3())]);
        let result = plugin_config_repository.add_plugins(&new_plugins);
        assert!(result.is_ok());
        let updated_config = result.unwrap();

        let expected_plugins = BTreeMap::from([
            ("plugin_1".to_string(), Plugin::create_mock_plugin_1()),
            ("plugin_2".to_string(), Plugin::create_mock_plugin_2()),
            ("plugin_3".to_string(), Plugin::create_mock_plugin_3()),
        ]);

        assert_eq!(updated_config.plugins, expected_plugins);
    }

    // get_plugin_by_name

    #[test]
    fn test_get_plugin_by_name_should_return_plugin_if_exists() {
        let app_config = DefaultAppConfig::new(
            None,
            Some(String::from("tests/mocks/gdm.json")),
            None,
            None,
            None,
        );
        let plugin_config_repository =
            DefaultGdmConfig::new(app_config, Arc::new(DefaultFileService));
        let plugin_key = "plugin_1";
        let key = plugin_config_repository.get_plugin_by_name(plugin_key);
        assert_eq!(
            key,
            Some((plugin_key.to_string(), Plugin::create_mock_plugin_1()))
        );
    }

    #[test]
    fn test_get_plugin_by_name_should_return_none_if_plugin_does_not_exist() {
        let app_config = DefaultAppConfig::new(
            None,
            Some(String::from("tests/mocks/gdm.json")),
            None,
            None,
            None,
        );
        let plugin_config_repository =
            DefaultGdmConfig::new(app_config, Arc::new(DefaultFileService));
        let plugin_key = "nonexistent_plugin";
        let key = plugin_config_repository.get_plugin_by_name(plugin_key);
        assert_eq!(key, None);
    }

    #[test]
    fn test_remove_plugins_should_remove_specified_plugins() {
        let plugin_config_repository =
            setup_mock_plugin_config_repository_for_add_and_remove_plugins();

        let plugins_to_remove = HashSet::from(["plugin_1".to_string()]);

        let result = plugin_config_repository.remove_plugins(plugins_to_remove);
        assert!(result.is_ok());
        let updated_config = result.unwrap();

        let expected_plugins = BTreeMap::from([(
            "plugin_2".to_string(),
            Plugin::new_asset_store_plugin(
                "12345".to_string(),
                Some("addons/super_plugin/plugin.cfg".into()),
                "Super Plugin".to_string(),
                "2.1.3".to_string(),
                "MIT".to_string(),
                vec![],
            ),
        )]);

        assert_eq!(updated_config.plugins, expected_plugins);
    }

    #[test]
    fn test_remove_plugins_should_not_remove_anything_if_keys_do_not_exist() {
        let plugin_config_repository =
            setup_mock_plugin_config_repository_for_add_and_remove_plugins();

        let plugins_to_remove = HashSet::from(["nonexistent_plugin".to_string()]);

        let result = plugin_config_repository.remove_plugins(plugins_to_remove);
        assert!(result.is_ok());
        let updated_config = result.unwrap();

        let expected_plugins = BTreeMap::from([
            ("plugin_1".to_string(), Plugin::create_mock_plugin_1()),
            ("plugin_2".to_string(), Plugin::create_mock_plugin_2()),
        ]);

        assert_eq!(updated_config.plugins, expected_plugins);
    }

    #[test]
    fn test_remove_plugins_should_return_default_config_if_all_removed() {
        let plugin_config_repository =
            setup_mock_plugin_config_repository_for_add_and_remove_plugins();

        let plugins_to_remove = HashSet::from(["plugin_1".to_string(), "plugin_2".to_string()]);

        let result = plugin_config_repository.remove_plugins(plugins_to_remove);
        assert!(result.is_ok());
        let updated_config = result.unwrap();

        let expected_plugins = BTreeMap::new();

        assert_eq!(updated_config.plugins, expected_plugins);
    }

    // save

    #[test]
    fn test_save_should_return_correct_string_content_on_empty_config() {
        let plugin_config_repository =
            setup_mock_plugin_config_repository_for_add_and_remove_plugins();

        let plugin_config = DefaultGdmConfigMetadata::new(BTreeMap::new());

        let result = plugin_config_repository.save(&plugin_config);
        assert!(result.is_ok());

        assert_eq!(result.unwrap(), String::from("{\n  \"plugins\": {}\n}"));
    }

    #[test]
    fn test_save_should_return_correct_json_with_plugins() {
        let plugin_config_repository =
            setup_mock_plugin_config_repository_for_add_and_remove_plugins();

        let plugins = BTreeMap::from([
            ("plugin_1".to_string(), Plugin::create_mock_plugin_1()),
            ("plugin_2".to_string(), Plugin::create_mock_plugin_2()),
        ]);

        let plugin_config = DefaultGdmConfigMetadata::new(plugins);

        let result = plugin_config_repository.save(&plugin_config);
        assert!(result.is_ok());

        let expected = json!({
            "plugins": {
                "plugin_1": {
                    "source": {
                      "asset_id": "54321"
                    },
                    "title": "Awesome Plugin",
                    "version": "1.0.0",
                    "license": "MIT",
                    "plugin_cfg_path": "addons/awesome_plugin/plugin.cfg",
                    "sub_assets": []
                },
                "plugin_2": {
                    "source": {
                        "asset_id": "12345"
                    },
                    "title": "Super Plugin",
                    "version": "2.1.3",
                    "license": "MIT",
                    "plugin_cfg_path": "addons/super_plugin/plugin.cfg",
                    "sub_assets": []
                }
            }
        });

        let saved = result.unwrap();
        let saved_json: serde_json::Value = serde_json::from_str(&saved).unwrap();
        assert_eq!(saved_json, expected);
    }

    #[test]
    fn test_save_should_return_correct_json_with_plugins_with_sub_assets() {
        let plugin_config_repository =
            setup_mock_plugin_config_repository_for_add_and_remove_plugins();

        let plugins = BTreeMap::from([("plugin_1".to_string(), Plugin::create_mock_plugin_3())]);

        let plugin_config = DefaultGdmConfigMetadata::new(plugins);

        let result = plugin_config_repository.save(&plugin_config);
        assert!(result.is_ok());

        let expected = json!({
            "plugins": {
                "plugin_1": {
                    "source": {
                        "asset_id": "345678",
                    },
                    "title": "Some Library",
                    "version": "3.3.3",
                    "sub_assets": [
                        "sub_asset1",
                        "sub_asset2"
                    ],
                    "license": "MIT",
                }
            }
        });
        let saved = result.unwrap();
        let saved_json: serde_json::Value = serde_json::from_str(&saved).unwrap();
        assert_eq!(saved_json, expected);
    }

    // has_installed_plugins

    #[test]
    fn test_has_installed_plugins_should_return_true_if_plugins_exist() {
        let app_config = DefaultAppConfig::new(
            None,
            Some(String::from("tests/mocks/gdm.json")),
            None,
            None,
            None,
        );
        let plugin_config_repository =
            DefaultGdmConfig::new(app_config, Arc::new(DefaultFileService));
        let result = plugin_config_repository.has_installed_plugins();
        assert!(result.is_ok());
        let has_plugins = result.unwrap();
        assert!(has_plugins);
    }

    #[test]
    fn test_has_installed_plugins_should_return_false_if_plugins_do_not_exist() {
        let app_config = DefaultAppConfig::new(
            None,
            Some(String::from("tests/mocks/gdm_non_existent.json")),
            None,
            None,
            None,
        );
        let plugin_config_repository =
            DefaultGdmConfig::new(app_config, Arc::new(DefaultFileService));
        let result = plugin_config_repository.has_installed_plugins();
        assert!(result.is_ok());
        let has_plugins = result.unwrap();
        assert!(!has_plugins);
    }
}
