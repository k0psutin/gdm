pub mod plugin;
pub mod plugin_config;

use crate::app_config::{AppConfig, DefaultAppConfig};
use crate::file_service::{DefaultFileService, FileService};
use crate::plugin_config_repository::plugin::Plugin;
use crate::plugin_config_repository::plugin_config::{DefaultPluginConfig, PluginConfig};

use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info};

pub struct DefaultPluginConfigRepository {
    pub app_config: DefaultAppConfig,
    pub file_service: Arc<dyn FileService + Send + Sync + 'static>,
}

impl Default for DefaultPluginConfigRepository {
    fn default() -> Self {
        DefaultPluginConfigRepository {
            file_service: Arc::new(DefaultFileService),
            app_config: DefaultAppConfig::default(),
        }
    }
}

impl DefaultPluginConfigRepository {
    #[allow(unused)]
    pub fn new(
        app_config: DefaultAppConfig,
        file_service: Arc<dyn FileService + Send + Sync + 'static>,
    ) -> Self {
        DefaultPluginConfigRepository {
            app_config,
            file_service,
        }
    }
}

#[cfg_attr(test, mockall::automock)]
impl PluginConfigRepository for DefaultPluginConfigRepository {
    fn add_plugins(&self, plugins: &BTreeMap<String, Plugin>) -> Result<DefaultPluginConfig> {
        debug!("Adding plugins: {:?}", plugins.keys());
        let plugin_config = self.load()?;
        let updated_plugin_config = plugin_config.add_plugins(plugins);
        self.save(&updated_plugin_config)?;
        info!("Added plugins {:?}", updated_plugin_config.plugins.keys());
        Ok(updated_plugin_config)
    }

    fn remove_plugins(&self, plugin_keys: HashSet<String>) -> Result<DefaultPluginConfig> {
        debug!("Removing plugins: {:?}", plugin_keys);
        let plugin_config = self.load()?;
        let updated_plugin_config = plugin_config.remove_plugins(plugin_keys);
        self.save(&updated_plugin_config)?;
        info!("Removed plugins {:?}", updated_plugin_config.plugins.keys());
        Ok(updated_plugin_config)
    }

    fn get_plugin_key_by_name(&self, name: &str) -> Option<String> {
        let plugin_config = self.load().ok()?;
        let plugin = plugin_config.get_plugin_by_name(name);
        match plugin {
            Some(_) => Some(name.to_string()),
            _ => None,
        }
    }

    fn get_plugin_by_asset_id(&self, asset_id: &str) -> Option<Plugin> {
        let plugin_config = self.load().ok()?;
        plugin_config.get_plugin_by_asset_id(asset_id)
    }

    /// Returns a sorted list of plugins in a tuple of (key, Plugin)
    ///
    /// The list is sorted by the plugin key in ascending order
    fn get_plugins(&self) -> Result<BTreeMap<String, Plugin>> {
        let plugin_config = self.load()?;
        Ok(plugin_config.plugins.clone())
    }

    fn load(&self) -> Result<DefaultPluginConfig> {
        let config_file_path = self.app_config.get_config_file_path();

        if !self.file_service.file_exists(config_file_path)? {
            return Ok(DefaultPluginConfig::default());
        }
        let content = self.file_service.read_file_cached(config_file_path)?;
        let config: DefaultPluginConfig = serde_json::from_str(&content).with_context(|| {
            format!(
                "Failed to parse plugin config file: {}",
                config_file_path.display()
            )
        })?;
        Ok(config)
    }

    fn save(&self, config: &DefaultPluginConfig) -> Result<String> {
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

pub trait PluginConfigRepository {
    fn add_plugins(&self, plugins: &BTreeMap<String, Plugin>) -> Result<DefaultPluginConfig>;
    fn get_plugin_by_asset_id(&self, asset_id: &str) -> Option<Plugin>;
    fn get_plugin_key_by_name(&self, name: &str) -> Option<String>;
    fn get_plugins(&self) -> Result<BTreeMap<String, Plugin>>;
    fn load(&self) -> Result<DefaultPluginConfig>;
    fn remove_plugins(&self, plugin_keys: HashSet<String>) -> Result<DefaultPluginConfig>;
    fn save(&self, config: &DefaultPluginConfig) -> Result<String>;
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use mockall::predicate::*;

    use crate::app_config::DefaultAppConfig;
    use crate::file_service::{DefaultFileService, MockDefaultFileService};

    // load

    #[test]
    fn test_load_non_existent_file_should_return_default_config() {
        let plugin_config_repository = DefaultPluginConfigRepository::new(
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
        let plugin_config_repository = DefaultPluginConfigRepository::new(
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
            (
                "plugin_1".to_string(),
                Plugin::new(
                    "54321".to_string(),
                    "Awesome Plugin".to_string(),
                    "1.0.0".to_string(),
                    "MIT".to_string(),
                ),
            ),
            (
                "plugin_2".to_string(),
                Plugin::new(
                    "12345".to_string(),
                    "Super Plugin".to_string(),
                    "2.1.3".to_string(),
                    "MIT".to_string(),
                ),
            ),
        ]);

        let expected_plugin_config = DefaultPluginConfig::new(mock_plugins);
        assert_eq!(config, expected_plugin_config);
    }

    // get_plugins

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
            DefaultPluginConfigRepository::new(app_config, Arc::new(DefaultFileService));
        let plugins = plugin_config_repository.get_plugins();
        println!("{:?}", plugins);
        assert!(plugins.is_ok());
        let plugins = plugins.unwrap();
        assert_eq!(plugins.len(), 2);

        let expected_plugin_config = BTreeMap::from([
            (
                "plugin_1".to_string(),
                Plugin::new(
                    "54321".to_string(),
                    "Awesome Plugin".to_string(),
                    "1.0.0".to_string(),
                    "MIT".to_string(),
                ),
            ),
            (
                "plugin_2".to_string(),
                Plugin::new(
                    "12345".to_string(),
                    "Super Plugin".to_string(),
                    "2.1.3".to_string(),
                    "MIT".to_string(),
                ),
            ),
        ]);
        assert_eq!(plugins, expected_plugin_config);
    }

    // add_plugins

    fn setup_mock_plugin_config_repository_for_add_and_remove_plugins()
    -> DefaultPluginConfigRepository {
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
        DefaultPluginConfigRepository::new(app_config, file_service)
    }

    #[test]
    fn test_add_plugins_should_add_plugins_to_config() {
        let plugin_config_repository =
            setup_mock_plugin_config_repository_for_add_and_remove_plugins();

        let new_plugins = BTreeMap::from([(
            "plugin_3".to_string(),
            Plugin::new(
                "67890".to_string(),
                "Mega Plugin".to_string(),
                "0.9.0".to_string(),
                "MIT".to_string(),
            ),
        )]);
        let result = plugin_config_repository.add_plugins(&new_plugins);
        assert!(result.is_ok());
        let updated_config = result.unwrap();

        let expected_plugins = BTreeMap::from([
            (
                "plugin_1".to_string(),
                Plugin::new(
                    "54321".to_string(),
                    "Awesome Plugin".to_string(),
                    "1.0.0".to_string(),
                    "MIT".to_string(),
                ),
            ),
            (
                "plugin_2".to_string(),
                Plugin::new(
                    "12345".to_string(),
                    "Super Plugin".to_string(),
                    "2.1.3".to_string(),
                    "MIT".to_string(),
                ),
            ),
            (
                "plugin_3".to_string(),
                Plugin::new(
                    "67890".to_string(),
                    "Mega Plugin".to_string(),
                    "0.9.0".to_string(),
                    "MIT".to_string(),
                ),
            ),
        ]);

        assert_eq!(updated_config.plugins, expected_plugins);
    }

    // get_plugin_key_by_name

    #[test]
    fn test_get_plugin_key_by_name_should_return_key_if_plugin_exists() {
        let app_config = DefaultAppConfig::new(
            None,
            Some(String::from("tests/mocks/gdm.json")),
            None,
            None,
            None,
        );
        let plugin_config_repository =
            DefaultPluginConfigRepository::new(app_config, Arc::new(DefaultFileService));
        let plugin_key = "plugin_1";
        let key = plugin_config_repository.get_plugin_key_by_name(plugin_key);
        assert_eq!(key, Some(plugin_key.to_string()));
    }

    #[test]
    fn test_get_plugin_key_by_name_should_return_none_if_plugin_does_not_exist() {
        let app_config = DefaultAppConfig::new(
            None,
            Some(String::from("tests/mocks/gdm.json")),
            None,
            None,
            None,
        );
        let plugin_config_repository =
            DefaultPluginConfigRepository::new(app_config, Arc::new(DefaultFileService));
        let plugin_key = "nonexistent_plugin";
        let key = plugin_config_repository.get_plugin_key_by_name(plugin_key);
        assert_eq!(key, None);
    }

    // remove_plugins
    // TODO Mock file service to avoid actual file writes
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
            Plugin::new(
                "12345".to_string(),
                "Super Plugin".to_string(),
                "2.1.3".to_string(),
                "MIT".to_string(),
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
            (
                "plugin_1".to_string(),
                Plugin::new(
                    "54321".to_string(),
                    "Awesome Plugin".to_string(),
                    "1.0.0".to_string(),
                    "MIT".to_string(),
                ),
            ),
            (
                "plugin_2".to_string(),
                Plugin::new(
                    "12345".to_string(),
                    "Super Plugin".to_string(),
                    "2.1.3".to_string(),
                    "MIT".to_string(),
                ),
            ),
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

        let plugin_config = DefaultPluginConfig::new(BTreeMap::new());

        let result = plugin_config_repository.save(&plugin_config);
        assert!(result.is_ok());

        assert_eq!(result.unwrap(), String::from("{\n  \"plugins\": {}\n}"));
    }

    #[test]
    fn test_save_should_return_correct_string_content_with_plugins() {
        let plugin_config_repository =
            setup_mock_plugin_config_repository_for_add_and_remove_plugins();

        let plugins = BTreeMap::from([
            (
                "plugin_1".to_string(),
                Plugin::new(
                    "54321".to_string(),
                    "Awesome Plugin".to_string(),
                    "1.0.0".to_string(),
                    "MIT".to_string(),
                ),
            ),
            (
                "plugin_2".to_string(),
                Plugin::new(
                    "12345".to_string(),
                    "Super Plugin".to_string(),
                    "2.1.3".to_string(),
                    "MIT".to_string(),
                ),
            ),
        ]);

        let plugin_config = DefaultPluginConfig::new(plugins);

        let result = plugin_config_repository.save(&plugin_config);
        assert!(result.is_ok());

        assert_eq!(
            result.unwrap(),
            String::from(
                "{\n  \"plugins\": {\n    \"plugin_1\": {\n      \"asset_id\": \"54321\",\n      \"title\": \"Awesome Plugin\",\n      \"version\": \"1.0.0\",\n      \"license\": \"MIT\"\n    },\n    \"plugin_2\": {\n      \"asset_id\": \"12345\",\n      \"title\": \"Super Plugin\",\n      \"version\": \"2.1.3\",\n      \"license\": \"MIT\"\n    }\n  }\n}"
            )
        );
    }
}
