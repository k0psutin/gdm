pub mod plugin;
pub mod plugin_config;

use crate::app_config::{AppConfig, AppConfigImpl};
use crate::file_service::{FileService, FileServiceInternal};
use crate::plugin_config_repository::plugin::Plugin;
use crate::plugin_config_repository::plugin_config::{PluginConfig, PluginConfigImpl};

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct PluginConfigRepository {
    app_config: AppConfig,
    file_service: FileService,
}

impl PluginConfigRepository {
    pub fn new(app_config: AppConfig, file_service: FileService) -> Self {
        PluginConfigRepository {
            app_config,
            file_service,
        }
    }

    pub fn add_plugins(&self, plugins: HashMap<String, Plugin>) -> Result<PluginConfig> {
        let plugin_config = self.load()?;
        let updated_plugin_config = plugin_config.add_plugins(plugins);
        self.save(&updated_plugin_config)?;
        Ok(updated_plugin_config)
    }

    pub fn remove_plugins(&self, plugin_keys: HashSet<String>) -> Result<PluginConfig> {
        let plugin_config = self.load()?;
        let updated_plugin_config = plugin_config.remove_plugins(plugin_keys);
        self.save(&updated_plugin_config)?;
        Ok(updated_plugin_config)
    }

    pub fn get_plugin_key_by_name(&self, name: &str) -> Option<String> {
        let plugin_config = self.load().ok()?;
        plugin_config.get_plugin_key_by_name(name)
    }

    pub fn check_if_plugin_already_installed_by_asset_id(&self, asset_id: &str) -> Option<Plugin> {
        let plugin_config = self.load().ok()?;
        plugin_config
            .check_if_plugin_already_installed_by_asset_id(asset_id)
            .cloned()
    }

    /// Returns a sorted list of plugins in a tuple of (key, Plugin)
    ///
    /// The list is sorted by the plugin key in ascending order
    pub fn get_plugins(&self) -> Result<Vec<(String, Plugin)>> {
        let plugin_config = self.load()?;
        let mut plugins = plugin_config
            .get_plugins()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();
        plugins.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(plugins)
    }
}

#[cfg_attr(test, mockall::automock)]
impl PluginConfigRepositoryImpl for PluginConfigRepository {
    fn get_file_service(&self) -> &FileService {
        &self.file_service
    }

    fn get_app_config(&self) -> &AppConfig {
        &self.app_config
    }
}

pub trait PluginConfigRepositoryImpl {
    fn get_file_service(&self) -> &FileService;
    fn get_app_config(&self) -> &AppConfig;

    fn load(&self) -> Result<PluginConfig> {
        let config_file_path = self.get_app_config().get_config_file_path();
        self.load_plugin_config(&config_file_path)
    }

    fn save(&self, config: &PluginConfig) -> Result<()> {
        self.save_plugin_config(config)
    }

    fn load_plugin_config(&self, path: &str) -> Result<PluginConfig> {
        let file_service = self.get_file_service();

        if !file_service.file_exists(PathBuf::from(path)) {
            return Ok(PluginConfig::default());
        }
        let content = file_service.read_file_cached(PathBuf::from(path))?;
        let config: PluginConfig = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse plugin config file: {}", path))?;
        Ok(config)
    }

    fn save_plugin_config(&self, config: &PluginConfig) -> Result<()> {
        let config_file_path = self.get_app_config().get_config_file_path();
        let file = self
            .get_file_service()
            .create_file(PathBuf::from(config_file_path.clone()))?;

        serde_json::to_writer_pretty(file, config).with_context(|| {
            format!(
                "Failed to write configuration to file: {}",
                config_file_path
            )
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_non_existent_file_should_return_default_config() {
        let plugin_config_repository = PluginConfigRepository::default();
        let result = plugin_config_repository.load_plugin_config("non_existent_file.json");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.get_plugins().len(), 0);
    }

    #[test]
    fn test_load_should_return_correct_plugin_config() {
        let plugin_config_repository = PluginConfigRepository::default();
        let result = plugin_config_repository.load_plugin_config("test/mocks/gdm.json");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.get_plugins().len(), 2);

        let expected_plugin_config = PluginConfig::new(HashMap::from([
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
        ]));
        assert_eq!(config, expected_plugin_config);
    }

    #[test]
    fn test_get_plugins_should_return_correct_plugins() {
        let app_config = AppConfig::new(
            None,
            Some(String::from("test/mocks/gdm.json")),
            None,
            None,
            None,
        );
        let plugin_config_repository = PluginConfigRepository::new(app_config, FileService);
        let plugins = plugin_config_repository.get_plugins().unwrap();
        assert_eq!(plugins.len(), 2);

        let expected_plugin_config = Vec::from([
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
}
