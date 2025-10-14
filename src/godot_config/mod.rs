mod parser;

use anyhow::{Result, anyhow};

use crate::app_config::AppConfig;
use crate::utils::Utils;
use crate::plugin_config::Plugin;
use std::collections::HashMap;
use parser::Parser;


#[faux::create]
pub struct GodotConfig {
    config_version: i32,
    plugins: Vec<String>,
    godot_version: String,
    parser: Parser,
}

#[faux::methods]
impl GodotConfig {
    pub fn new(godot_project_file_path: &str) -> GodotConfig {
        let parser = Parser::new(godot_project_file_path);
        let config_map = parser.get_parsed_project();
        let config_version = config_map
            .get("config_version")
            .and_then(|v| v.get(0))
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(5); // Default to version 5 if not found or invalid 
        let plugins = config_map.get("enabled").cloned().unwrap_or_default();
        let godot_version = config_map
            .get("config/features")
            .and_then(|v| v.get(0))
            .cloned()
            .unwrap_or_default();
        GodotConfig {
            config_version,
            plugins,
            godot_version,
            parser: parser,
        }
    }

    pub fn default() -> GodotConfig {
        let godot_project_file_path = AppConfig::default().get_godot_project_file_path();
        GodotConfig::new(&godot_project_file_path)
    }

    pub fn get_godot_version(&self) -> Result<String> {
        if !self.godot_version.is_empty() {
            return Ok(self.godot_version.clone());
        }
        self.get_default_godot_version()
    }

    pub fn get_default_godot_version(&self) -> Result<String> {
        match self.config_version {
            5 => Ok("4.5".to_string()),
            4 => Ok("3.6".to_string()),
            _ => Err(anyhow!(
                "Unsupported config_version: {}",
                self.config_version
            )),
        }
    }

    pub fn plugin_path_to_resource_path(&self, plugin_path: String) -> String {
        let addon_folder_path = AppConfig::default().get_addon_folder_path();
        Utils::plugin_folder_to_resource_path(format!("{}/{}", addon_folder_path, plugin_path))
    }

    pub fn get_installed_plugins(&self) -> Vec<String> {
        self.plugins.clone()
    }

    pub fn is_plugin_installed(&self, plugin_root_folder: String) -> bool {
        let plugin_resource_path = self.plugin_path_to_resource_path(plugin_root_folder);
        self.plugins.contains(&plugin_resource_path)
    }

    fn add_plugins(&self, plugins_to_add: Vec<String>) -> Vec<String> {
        let mut plugins = self.get_installed_plugins();
        for plugin in plugins_to_add {
            let resource_path = self.plugin_path_to_resource_path(plugin.clone());

            if !self.is_plugin_installed(plugin) {
                plugins.push(resource_path);
            }
        }

        return plugins;
    }

    pub fn add_installed_plugins(&self, plugins: HashMap<String, Plugin>) -> Result<()>{
        let plugin_keys = plugins.keys().cloned().collect::<Vec<String>>();
        let updated_plugins = self.add_plugins(plugin_keys);
        self.update_godot_project_file(updated_plugins)?;
        Ok(())
    }

    fn remove_plugins(&self, plugins_to_remove: Vec<String>) -> Vec<String> {
        let mut plugins = self.get_installed_plugins();
        for plugin in plugins_to_remove {
            let resource_path = self.plugin_path_to_resource_path(plugin.clone());

            if self.is_plugin_installed(plugin) {
                plugins.retain(|p| p != &resource_path);
            }
        }

        return plugins;
    }

    pub fn remove_installed_plugin(&self, plugins_to_remove: Vec<String>) -> Result<()> {
        let plugins = self.remove_plugins(plugins_to_remove);
        self.update_godot_project_file(plugins)?;
        Ok(())
    }

    pub fn update_godot_project_file(&self, plugins: Vec<String>) -> Result<()> {
        let godot_project_file_path = AppConfig::default().get_godot_project_file_path();
        self.parser.update_plugins(&godot_project_file_path, plugins)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_path_to_resource_path_should_return_correct_path() {
        let godot_config = GodotConfig::new("test/mocks/project_with_plugins_and_version.godot");
        let resource_path = godot_config.plugin_path_to_resource_path(String::from("gut"));
        assert_eq!(resource_path, "res://addons/gut/plugin.cfg");
    }

    // get_godot_version

    #[test]
    fn test_get_godot_version_should_return_4_5_if_it_exists_with_config_version_5() {
        let godot_config = GodotConfig::new("test/mocks/project_with_plugins_and_version.godot");
        let version = godot_config.get_godot_version();
        assert!(version.is_ok());
        let version = version.unwrap();
        assert_eq!(version, "4.5");
    }

    #[test]
    fn test_get_godot_version_should_return_3_6_when_config_version_is_4() {
        let godot_config = GodotConfig::new("test/mocks/project_with_old_config.godot");
        let version = godot_config.get_godot_version();
        assert!(version.is_ok());
        let version = version.unwrap();
        assert_eq!(version, "3.6");
    }

    #[test]
    fn test_get_godot_version_should_return_error_if_config_version_is_unsupported() {
        let godot_config = GodotConfig::new("test/mocks/project_with_unsupported_config.godot");
        let version = godot_config.get_godot_version();
        assert!(version.is_err());
    }

    // get_installed_plugins

    #[test]
    fn test_get_godot_version_should_return_correct_plugins() {
        let godot_config = GodotConfig::new("test/mocks/project_with_plugins_and_version.godot");
        let plugins = godot_config.get_installed_plugins();
        assert_eq!(
            plugins,
            vec![
                "res://addons/test_plugin/plugin.cfg",
            ]
        );
        drop(godot_config);
    }

    // is_plugin_installed

    #[test]
    fn test_is_plugin_installed_should_return_false_if_plugin_does_not_exist() {
        let godot_config = GodotConfig::new("test/mocks/project_with_plugins_and_version.godot");
        let is_installed = godot_config.is_plugin_installed(String::from("non_existent_plugin"));
        assert!(!is_installed);
    }

    #[test]
    fn test_is_plugin_installed_should_return_true_if_plugin_exists() {
        let godot_config = GodotConfig::new("test/mocks/project_with_plugins_and_version.godot");
        let is_installed = godot_config.is_plugin_installed(String::from("test_plugin"));
        assert!(is_installed);
    }
}
