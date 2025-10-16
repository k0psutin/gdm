pub mod godot_config;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use anyhow::{Result, anyhow};

use crate::app_config::{AppConfig, AppConfigImpl};
use crate::file_service::{FileService, FileServiceInternal};
use crate::godot_config_repository::godot_config::{GodotConfig, GodotConfigImpl};
use crate::plugin_config_repository::plugin_config::{PluginConfig, PluginConfigImpl};
use crate::utils::Utils;

#[derive(Debug, Clone, Default)]
pub struct GodotConfigRepository {
    file_service: FileService,
    app_config: AppConfig,
}

impl GodotConfigRepository {
    pub fn new(file_service: FileService, app_config: AppConfig) -> GodotConfigRepository {
        GodotConfigRepository {
            file_service,
            app_config,
        }
    }

    pub fn update_plugins(&self, plugin_config: PluginConfig) -> Result<()> {
        self.save(plugin_config)
    }

    pub fn get_godot_version_from_project(&self) -> Result<String> {
        let godot_config = self.load()?;
        godot_config.get_godot_version()
    }
}

#[cfg_attr(test, mockall::automock)]
impl GodotConfigRepositoryImpl for GodotConfigRepository {
    fn get_file_service(&self) -> &FileService {
        &self.file_service
    }

    fn get_app_config(&self) -> &AppConfig {
        &self.app_config
    }
}

pub trait GodotConfigRepositoryImpl {
    fn get_file_service(&self) -> &FileService;
    fn get_app_config(&self) -> &AppConfig;

    fn plugin_root_folder_to_resource_path(&self, plugin_path: String) -> String {
        Utils::plugin_folder_to_resource_path(format!(
            "{}/{}",
            self.get_app_config().get_addon_folder_path(),
            plugin_path
        ))
    }

    fn plugins_to_packed_string_array(&self, plugins: HashSet<String>) -> String {
        let joined = plugins
            .iter()
            .map(|s| {
                format!(
                    "\"{}\"",
                    self.plugin_root_folder_to_resource_path(s.to_string())
                )
            })
            .collect::<Vec<String>>()
            .join(", ");
        format!("PackedStringArray({})", joined)
    }

    fn save(&self, plugin_config: PluginConfig) -> Result<()> {
        let godot_project_file_path = self.get_app_config().get_godot_project_file_path();
        if !self
            .get_file_service()
            .file_exists(PathBuf::from(&godot_project_file_path))
        {
            return Err(anyhow!(
                "Godot project file not found: {}",
                godot_project_file_path
            ));
        }
        let lines = self.update_project_file(&godot_project_file_path, plugin_config)?;
        self.save_project_file(&godot_project_file_path, lines)?;
        Ok(())
    }

    fn load(&self) -> Result<GodotConfig> {
        let godot_project_file_path = self.get_app_config().get_godot_project_file_path();
        if !self
            .get_file_service()
            .file_exists(PathBuf::from(&godot_project_file_path))
        {
            return Err(anyhow!(
                "Godot project file not found: {}",
                godot_project_file_path
            ));
        }
        let config = self.read_godot_project_file(&godot_project_file_path)?;
        Ok(config)
    }

    /// Updates the plugins in the Godot project file and returns the updated lines.
    ///
    /// godot.project plugin format:
    /// ```
    /// [editor_plugins]
    ///
    /// enabled=PackedStringArray("res://addons/gd_flow/plugin.cfg")
    ///
    /// [<next section>]
    /// ```
    fn update_project_file(
        &self,
        godot_project_file_path: &str,
        plugin_config: PluginConfig,
    ) -> Result<Vec<String>> {
        let _plugins = HashSet::from_iter(
            plugin_config
                .get_plugins()
                .iter()
                .map(|p| String::from(p.0))
                .collect::<Vec<String>>(),
        );
        let mut contents = self.load_project_file(godot_project_file_path)?;

        if contents.last().unwrap() != "" {
            contents.push("".to_string());
        }

        let editor_plugins_index = contents
            .iter()
            .position(|line| line.starts_with("[editor_plugins]"));

        if _plugins.is_empty() {
            // If there are no plugins, we need to remove the [editor_plugins] section if it exists.
            if let Some(index) = editor_plugins_index {
                for _ in 0..4 {
                    contents.remove(index);
                }
            }
            return Ok(contents);
        }

        let plugin_index = match editor_plugins_index {
            Some(index) => contents
                .iter()
                .skip(index + 1)
                .position(|line| line.starts_with("enabled="))
                .map(|i| i + index + 1),
            None => None,
        };

        if plugin_index.is_some() {
            contents[plugin_index.unwrap()] =
                format!("enabled={}", self.plugins_to_packed_string_array(_plugins));
            return Ok(contents);
        }

        // If [editor_plugins] section doesn't exists, we need to add it to the project file.
        // I _think_ it should be added alphabetically, but I'm not 100% sure.
        for i in 0..contents.len() {
            let line = &contents[i];
            if line.starts_with("[")
                && line.ends_with("]")
                && line.to_lowercase().cmp(&"[editor_plugins]".to_string())
                    == std::cmp::Ordering::Greater
            {
                let new_lines = vec![
                    "[editor_plugins]".to_string(),
                    "".to_string(),
                    format!("enabled={}", self.plugins_to_packed_string_array(_plugins)),
                    "".to_string(),
                ];
                contents.splice(i..i, new_lines);
                return Ok(contents);
            }
        }

        Err(anyhow!("Failed to update plugins in Godot project file"))
    }

    /// Parses project.godot file and gathers plugins, config_version, and godot_version
    ///
    /// godot.project sections of interest:
    /// ```
    /// config_version=5
    ///
    /// ...
    /// [application]
    ///
    /// ...
    /// config/features=PackedStringArray("4.5", "GL Compatibility")
    /// ...
    ///
    /// [editor_plugins]
    ///
    /// enabled=PackedStringArray("res://addons/gd_flow/plugin.cfg")
    ///
    /// ```
    ///
    fn read_godot_project_file(&self, godot_project_file_path: &str) -> Result<GodotConfig> {
        let contents = self.load_project_file(godot_project_file_path)?;
        let mut output: HashMap<String, Vec<String>> = HashMap::new();
        output.insert("config/plugins".to_string(), vec![]);
        output.insert("config_version".to_string(), vec![]);

        for line in contents {
            if line.starts_with("config/features=")
                || line.starts_with("enabled=")
                || line.starts_with("config_version")
            {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim().to_string();
                    let mut value = parts[1].trim().to_string();
                    if value.starts_with("PackedStringArray") {
                        value = value.replace("PackedStringArray(", "").replace(")", "");
                        let parts: Vec<String> = value
                            .split(',')
                            .map(|s| s.replace('"', "").trim().to_string())
                            .collect();
                        output.insert(key, parts);
                    } else {
                        output.insert(key, vec![value]);
                    }
                }
            }
        }

        let config_version = output
            .get("config_version")
            .and_then(|v| v.first())
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(5); // Default to version 5 if not found or invalid 
        let godot_version = output
            .get("config/features")
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_default();
        Ok(GodotConfig::new(config_version, godot_version))
    }

    fn load_project_file(&self, godot_file_path: &str) -> Result<Vec<String>> {
        let file = self
            .get_file_service()
            .read_file_cached(PathBuf::from(godot_file_path))?;
        let lines = file.split('\n').map(|s| s.to_string()).collect::<Vec<_>>();
        Ok(lines)
    }

    fn save_project_file(&self, godot_project_file_path: &str, lines: Vec<String>) -> Result<()> {
        let file_service = self.get_file_service();

        if lines.is_empty() {
            return Err(anyhow!("No content to write to the project file"));
        }
        if !file_service.file_exists(PathBuf::from(godot_project_file_path)) {
            return Err(anyhow!(
                "Godot project file not found: {}",
                godot_project_file_path
            ));
        }
        file_service.write_file(PathBuf::from(godot_project_file_path), &lines.join("\n"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
}
