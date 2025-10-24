pub mod godot_config;

use std::collections::{BTreeSet, HashMap, HashSet};

use anyhow::{Result, bail};
use tracing::{debug, error, info};

use crate::app_config::{AppConfig, DefaultAppConfig};
use crate::file_service::{DefaultFileService, FileService};
use crate::godot_config_repository::godot_config::DefaultGodotConfig;
use crate::plugin_config_repository::plugin_config::DefaultPluginConfig;
use crate::utils::Utils;

pub struct DefaultGodotConfigRepository {
    pub file_service: Box<dyn FileService + Send + Sync + 'static>,
    pub app_config: DefaultAppConfig,
}

impl Default for DefaultGodotConfigRepository {
    fn default() -> Self {
        DefaultGodotConfigRepository {
            file_service: Box::new(DefaultFileService),
            app_config: DefaultAppConfig::default(),
        }
    }
}

impl DefaultGodotConfigRepository {
    #[allow(unused)]
    pub fn new(
        file_service: Box<dyn FileService + Send + Sync + 'static>,
        app_config: DefaultAppConfig,
    ) -> DefaultGodotConfigRepository {
        DefaultGodotConfigRepository {
            file_service,
            app_config,
        }
    }
}

#[cfg_attr(test, mockall::automock)]
impl GodotConfigRepository for DefaultGodotConfigRepository {
    fn get_godot_version_from_project(&self) -> Result<String> {
        let godot_config = self.load()?;
        let godot_version = godot_config.get_godot_version()?;
        info!(
            "Retrieved Godot version from project: {}",
            godot_version.clone()
        );
        Ok(godot_version)
    }

    fn plugin_root_folder_to_resource_path(&self, plugin_path: String) -> String {
        let addon_folder_path = self
            .app_config
            .get_addon_folder_path()
            .display()
            .to_string();
        Utils::plugin_folder_to_resource_path(format!("{}/{}", addon_folder_path, plugin_path))
    }

    fn plugins_to_packed_string_array(&self, plugins: HashSet<String>) -> String {
        let joined = BTreeSet::from_iter(plugins)
            .iter()
            .map(|s| {
                format!(
                    "\"{}\"",
                    self.plugin_root_folder_to_resource_path(s.to_string())
                )
            })
            .collect::<Vec<String>>()
            .join(", ");
        let packed_string_array = format!("PackedStringArray({})", joined);
        info!(
            "Converted plugins to PackedStringArray: {}",
            packed_string_array
        );
        packed_string_array
    }

    fn save(&self, plugin_config: DefaultPluginConfig) -> Result<()> {
        let godot_project_file_path = self.app_config.get_godot_project_file_path();
        if !self.file_service.file_exists(godot_project_file_path)? {
            bail!(
                "Godot project file not found: {}",
                godot_project_file_path.display()
            )
        }
        let lines = self.update_project_file(plugin_config)?;
        self.save_project_file(lines)
    }

    fn load(&self) -> Result<DefaultGodotConfig> {
        let godot_project_file_path = self.app_config.get_godot_project_file_path();
        if !self.file_service.file_exists(godot_project_file_path)? {
            bail!(
                "Godot project file not found: {}",
                godot_project_file_path.display()
            )
        }
        self.read_godot_project_file()
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
    fn update_project_file(&self, plugin_config: DefaultPluginConfig) -> Result<Vec<String>> {
        let _plugins = HashSet::from_iter(plugin_config.plugins.keys().cloned());

        let mut contents = self.load_project_file()?;

        if contents.last().unwrap() != "" {
            contents.push("".to_string());
        }

        let editor_plugins_index = contents
            .iter()
            .position(|line| line.starts_with("[editor_plugins]"));

        if _plugins.is_empty() {
            // If there are no plugins, we need to remove the [editor_plugins] section if it exists.
            if let Some(index) = editor_plugins_index {
                info!(
                    "Removing [editor_plugins] section from Godot project file: {}",
                    self.app_config.get_godot_project_file_path().display()
                );
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

        if let Some(plugin_index) = plugin_index {
            contents[plugin_index] =
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
            } else if i == contents.len() - 1 {
                let new_lines = vec![
                    "[editor_plugins]".to_string(),
                    "".to_string(),
                    format!("enabled={}", self.plugins_to_packed_string_array(_plugins)),
                    "".to_string(),
                ];
                contents.extend(new_lines);
                return Ok(contents);
            }
        }

        bail!("Failed to update plugins in Godot project file")
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
    fn read_godot_project_file(&self) -> Result<DefaultGodotConfig> {
        let contents = self.load_project_file()?;
        let mut output: HashMap<String, Vec<String>> = HashMap::new();
        output.insert("config/plugins".to_string(), vec![]);
        output.insert("config_version".to_string(), vec![]);

        for line in contents {
            if line.starts_with("config/features=") || line.starts_with("config_version") {
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
        let godot_config = DefaultGodotConfig::new(config_version, godot_version);
        info!("Parsed Godot config successfully");
        Ok(godot_config)
    }

    fn validate_project_file(&self) -> Result<()> {
        let exists = self
            .file_service
            .file_exists(self.app_config.get_godot_project_file_path())?;
        if !exists {
            error!(
                "Godot project file not found: {}",
                self.app_config.get_godot_project_file_path().display()
            );
            bail!("Godot project file not found")
        }
        info!("Godot project file validated successfully");
        Ok(())
    }

    fn load_project_file(&self) -> Result<Vec<String>> {
        debug!(
            "Loading Godot project file: {}",
            self.app_config.get_godot_project_file_path().display()
        );
        let file = self
            .file_service
            .read_file_cached(self.app_config.get_godot_project_file_path())?;
        let lines = file.split('\n').map(|s| s.to_string()).collect::<Vec<_>>();
        info!("Loaded Godot project file with {} lines", lines.len());
        Ok(lines)
    }

    fn save_project_file(&self, lines: Vec<String>) -> Result<()> {
        if lines.is_empty() {
            bail!("No content to write to the project file")
        }
        let godot_project_file_path = self.app_config.get_godot_project_file_path();
        if !self.file_service.file_exists(godot_project_file_path)? {
            bail!(
                "Godot project file not found: {}",
                godot_project_file_path.display()
            )
        }
        self.file_service
            .write_file(godot_project_file_path, &lines.join("\n"))?;
        info!(
            "Godot project file saved successfully: {}",
            godot_project_file_path.display()
        );
        Ok(())
    }
}
pub trait GodotConfigRepository {
    fn get_godot_version_from_project(&self) -> Result<String>;
    fn plugin_root_folder_to_resource_path(&self, plugin_path: String) -> String;
    fn plugins_to_packed_string_array(&self, plugins: HashSet<String>) -> String;
    fn validate_project_file(&self) -> Result<()>;
    fn save(&self, plugin_config: DefaultPluginConfig) -> Result<()>;
    fn load(&self) -> Result<DefaultGodotConfig>;
    fn update_project_file(&self, plugin_config: DefaultPluginConfig) -> Result<Vec<String>>;
    fn read_godot_project_file(&self) -> Result<DefaultGodotConfig>;
    fn load_project_file(&self) -> Result<Vec<String>>;
    fn save_project_file(&self, lines: Vec<String>) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use crate::file_service::MockDefaultFileService;

    use super::*;

    // plugin_root_folder_to_resource_path

    #[test]
    fn test_plugin_root_folder_to_resource_path() {
        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from(
                "tests/mocks/project_with_plugins_and_version.godot",
            )),
            Some(String::from("tests/mocks/addons")),
        );

        let mock_file_service = MockDefaultFileService::default();
        let repository = DefaultGodotConfigRepository::new(Box::new(mock_file_service), app_config);
        let result = repository.plugin_root_folder_to_resource_path("my_plugin".to_string());
        assert_eq!("res://tests/mocks/addons/my_plugin/plugin.cfg", result);
    }

    // plugins_to_packed_string_array

    #[test]
    fn test_plugins_to_packed_string_array() {
        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from(
                "tests/mocks/project_with_plugins_and_version.godot",
            )),
            Some(String::from("tests/mocks/addons")),
        );

        let mock_file_service = MockDefaultFileService::default();
        let repository = DefaultGodotConfigRepository::new(Box::new(mock_file_service), app_config);
        let result = repository.plugins_to_packed_string_array(HashSet::from([
            "my_plugin".to_string(),
            "another_plugin".to_string(),
        ]));
        assert_eq!(
            result,
            String::from(
                "PackedStringArray(\"res://tests/mocks/addons/another_plugin/plugin.cfg\", \"res://tests/mocks/addons/my_plugin/plugin.cfg\")"
            )
        );
    }

    // read_godot_project_file

    #[test]
    fn test_read_godot_project_file_with_config_version_5_and_plugins() {
        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from(
                "tests/mocks/project_with_plugins_and_version.godot",
            )),
            Some(String::from("tests/mocks/addons")),
        );

        let repository =
            DefaultGodotConfigRepository::new(Box::new(DefaultFileService), app_config);
        let result = repository.read_godot_project_file();
        assert!(result.is_ok());
        let godot_config = result.unwrap();
        assert_eq!(godot_config.get_config_version(), 5);
        assert_eq!(godot_config.get_godot_version().unwrap(), "4.5");
    }

    #[test]
    fn test_read_godot_project_file_with_config_version_4_and_no_plugins() {
        let project_file_path_string = String::from("tests/mocks/project_with_old_config.godot");
        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(project_file_path_string.clone()),
            Some(String::from("tests/mocks/addons")),
        );

        let repository =
            DefaultGodotConfigRepository::new(Box::new(DefaultFileService), app_config);
        let result = repository.read_godot_project_file();
        assert!(result.is_ok());
        let godot_config = result.unwrap();
        assert_eq!(godot_config.get_config_version(), 4);
        assert_eq!(godot_config.get_godot_version().unwrap(), "3.6");
    }

    // load

    #[test]
    fn test_load_should_return_error_if_file_not_found() {
        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from("non_existent_file.godot")),
            Some(String::from("tests/mocks/addons")),
        );
        let repository =
            DefaultGodotConfigRepository::new(Box::new(DefaultFileService), app_config);
        let result = repository.load();
        assert!(result.is_err());
    }

    #[test]
    fn test_load_should_not_return_error_if_file_exists() {
        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from(
                "tests/mocks/project_with_plugins_and_version.godot",
            )),
            Some(String::from("tests/mocks/addons")),
        );
        let repository =
            DefaultGodotConfigRepository::new(Box::new(DefaultFileService), app_config);
        let result = repository.load();
        assert!(result.is_ok());
    }

    // load_project_file

    #[test]
    fn test_load_project_file_should_return_error_if_file_not_found() {
        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from("non_existent_file.godot")),
            Some(String::from("tests/mocks/addons")),
        );
        let repository =
            DefaultGodotConfigRepository::new(Box::new(DefaultFileService), app_config);
        let result = repository.load_project_file();
        assert!(result.is_err());
    }

    #[test]
    fn test_load_project_file_should_not_return_error_if_file_exists() {
        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from(
                "tests/mocks/project_with_plugins_and_version.godot",
            )),
            Some(String::from("tests/mocks/addons")),
        );
        let repository =
            DefaultGodotConfigRepository::new(Box::new(DefaultFileService), app_config);
        let result = repository.load_project_file();
        assert!(result.is_ok());
    }

    // update_project_file

    #[test]
    fn test_update_project_file_should_add_editor_plugins_section_when_it_is_missing() {
        use crate::plugin_config_repository::plugin::Plugin;
        use std::collections::BTreeMap;

        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from("tests/mocks/project.godot")),
            Some(String::from("addons")),
        );

        pub const MOCK_PROJECT_GODOT: &str = r#"
; Engine configuration file.
; It's best edited using the editor UI and not directly,
; since the parameters that go here are not all obvious.
;
; Format:
;   [section] ; section goes between []
;   param=value ; assign values to parameters

config_version=5

[application]

config/name="Test Project"
config/features=PackedStringArray("4.5")

[rendering]

renderer/rendering_method="gl_compatibility"

"#;

        pub const EXPECTED_PROJECT_GODOT: &str = r#"
; Engine configuration file.
; It's best edited using the editor UI and not directly,
; since the parameters that go here are not all obvious.
;
; Format:
;   [section] ; section goes between []
;   param=value ; assign values to parameters

config_version=5

[application]

config/name="Test Project"
config/features=PackedStringArray("4.5")

[editor_plugins]

enabled=PackedStringArray("res://addons/test_plugin/plugin.cfg")

[rendering]

renderer/rendering_method="gl_compatibility"

"#;

        let mut mock_file_service = MockDefaultFileService::default();
        mock_file_service
            .expect_read_file_cached()
            .returning(|_| Ok(String::from(MOCK_PROJECT_GODOT)));

        let repository = DefaultGodotConfigRepository::new(Box::new(mock_file_service), app_config);

        let mut plugins = BTreeMap::new();
        plugins.insert(
            "test_plugin".to_string(),
            Plugin::new(
                "1".to_string(),
                "Test Plugin".to_string(),
                "1.0.0".to_string(),
                "MIT".to_string(),
            ),
        );
        let plugin_config = DefaultPluginConfig::new(plugins);

        let result = repository.update_project_file(plugin_config);
        assert!(result.is_ok());
        let lines = result.unwrap();

        assert_eq!(lines.join("\n").trim(), EXPECTED_PROJECT_GODOT.trim());
    }

    #[test]
    fn test_update_project_file_should_add_editor_plugins_section_when_it_is_missing_in_simple_config()
     {
        use crate::plugin_config_repository::plugin::Plugin;
        use std::collections::BTreeMap;

        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from("tests/mocks/project.godot")),
            Some(String::from("addons")),
        );

        pub const MOCK_PROJECT_GODOT: &str = r#"
; Engine configuration file.
; It's best edited using the editor UI and not directly,
; since the parameters that go here are not all obvious.
;
; Format:
;   [section] ; section goes between []
;   param=value ; assign values to parameters

config_version=5

[application]

config/name="Test Project"
config/features=PackedStringArray("4.5")
"#;

        pub const EXPECTED_PROJECT_GODOT: &str = r#"
; Engine configuration file.
; It's best edited using the editor UI and not directly,
; since the parameters that go here are not all obvious.
;
; Format:
;   [section] ; section goes between []
;   param=value ; assign values to parameters

config_version=5

[application]

config/name="Test Project"
config/features=PackedStringArray("4.5")

[editor_plugins]

enabled=PackedStringArray("res://addons/test_plugin/plugin.cfg")

"#;

        let mut mock_file_service = MockDefaultFileService::default();
        mock_file_service
            .expect_read_file_cached()
            .returning(|_| Ok(String::from(MOCK_PROJECT_GODOT)));

        let repository = DefaultGodotConfigRepository::new(Box::new(mock_file_service), app_config);

        let mut plugins = BTreeMap::new();
        plugins.insert(
            "test_plugin".to_string(),
            Plugin::new(
                "1".to_string(),
                "Test Plugin".to_string(),
                "1.0.0".to_string(),
                "MIT".to_string(),
            ),
        );
        let plugin_config = DefaultPluginConfig::new(plugins);

        let result = repository.update_project_file(plugin_config);
        assert!(result.is_ok());
        let lines = result.unwrap();

        assert_eq!(lines.join("\n").trim(), EXPECTED_PROJECT_GODOT.trim());
    }

    #[test]
    fn test_update_project_file_should_update_existing_editor_plugins_section() {
        use crate::plugin_config_repository::plugin::Plugin;
        use std::collections::BTreeMap;

        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from("tests/mocks/project.godot")),
            Some(String::from("addons")),
        );

        let mut mock_file_service = MockDefaultFileService::default();
        mock_file_service.expect_read_file_cached().returning(|_| {
            Ok(String::from(
                "config_version=5\n\
                    [application]\n\
                    config/name=\"Test\"\n\
                    [editor_plugins]\n\
                    \n\
                    enabled=PackedStringArray(\"res://addons/old_plugin/plugin.cfg\")\n\
                    \n\
                    [rendering]\n\
                    renderer/rendering_method=\"gl_compatibility\"\n",
            ))
        });

        let repository = DefaultGodotConfigRepository::new(Box::new(mock_file_service), app_config);

        let mut plugins = BTreeMap::new();
        plugins.insert(
            "new_plugin".to_string(),
            Plugin::new(
                "1".to_string(),
                "New Plugin".to_string(),
                "1.0.0".to_string(),
                "MIT".to_string(),
            ),
        );
        let plugin_config = DefaultPluginConfig::new(plugins);

        let result = repository.update_project_file(plugin_config);
        assert!(result.is_ok());
        let lines = result.unwrap();

        // Check that enabled line was updated
        let enabled_line = lines
            .iter()
            .find(|line| line.starts_with("enabled="))
            .unwrap();
        assert!(enabled_line.contains("new_plugin"));
        assert!(!enabled_line.contains("old_plugin"));
    }

    #[test]
    fn test_update_project_file_should_remove_editor_plugins_section_when_no_plugins() {
        use std::collections::BTreeMap;

        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from("tests/mocks/project.godot")),
            Some(String::from("addons")),
        );

        let mut mock_file_service = MockDefaultFileService::default();
        mock_file_service.expect_read_file_cached().returning(|_| {
            Ok(String::from(
                "config_version=5\n\
                    [application]\n\
                    config/name=\"Test\"\n\
                    [editor_plugins]\n\
                    \n\
                    enabled=PackedStringArray(\"res://addons/test_plugin/plugin.cfg\")\n\
                    \n\
                    [rendering]\n\
                    renderer/rendering_method=\"gl_compatibility\"\n",
            ))
        });

        let repository = DefaultGodotConfigRepository::new(Box::new(mock_file_service), app_config);

        let plugin_config = DefaultPluginConfig::new(BTreeMap::new());

        let result = repository.update_project_file(plugin_config);
        assert!(result.is_ok());
        let lines = result.unwrap();

        // Check that [editor_plugins] section was removed
        assert!(!lines.iter().any(|line| line == "[editor_plugins]"));
        assert!(!lines.iter().any(|line| line.starts_with("enabled=")));
    }

    #[test]
    fn test_update_project_file_should_add_empty_line_at_end_if_missing() {
        use std::collections::BTreeMap;

        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from("tests/mocks/project.godot")),
            Some(String::from("addons")),
        );

        let mut mock_file_service = MockDefaultFileService::default();
        mock_file_service.expect_read_file_cached().returning(|_| {
            Ok(String::from(
                "config_version=5\n\
                    [application]\n\
                    config/name=\"Test\"",
            ))
        });

        let repository = DefaultGodotConfigRepository::new(Box::new(mock_file_service), app_config);

        let plugin_config = DefaultPluginConfig::new(BTreeMap::new());

        let result = repository.update_project_file(plugin_config);
        assert!(result.is_ok());
        let lines = result.unwrap();

        // Check that empty line was added
        assert_eq!(lines.last().unwrap(), "");
    }

    // save_project_file

    #[test]
    fn test_save_project_file_should_write_lines_to_file() {
        use std::path::Path;

        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from("tests/mocks/project.godot")),
            Some(String::from("addons")),
        );

        let mut mock_file_service = MockDefaultFileService::default();
        mock_file_service
            .expect_file_exists()
            .returning(|_| Ok(true));
        mock_file_service
            .expect_write_file()
            .withf(|path: &Path, content: &str| {
                path.to_str().unwrap() == "tests/mocks/project.godot"
                    && content == "line1\nline2\nline3"
            })
            .times(1)
            .returning(|_, _| Ok(()));

        let repository = DefaultGodotConfigRepository::new(Box::new(mock_file_service), app_config);

        let lines = vec![
            "line1".to_string(),
            "line2".to_string(),
            "line3".to_string(),
        ];
        let result = repository.save_project_file(lines);
        assert!(result.is_ok());
    }

    #[test]
    fn test_save_project_file_should_return_error_when_file_not_found() {
        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from("tests/mocks/project.godot")),
            Some(String::from("addons")),
        );

        let mut mock_file_service = MockDefaultFileService::default();
        mock_file_service
            .expect_file_exists()
            .returning(|_| Ok(false));

        let repository = DefaultGodotConfigRepository::new(Box::new(mock_file_service), app_config);

        let lines = vec!["line1".to_string()];
        let result = repository.save_project_file(lines);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Godot project file not found")
        );
    }

    #[test]
    fn test_save_project_file_should_return_error_when_no_content() {
        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from("tests/mocks/project.godot")),
            Some(String::from("addons")),
        );

        let mock_file_service = MockDefaultFileService::default();
        let repository = DefaultGodotConfigRepository::new(Box::new(mock_file_service), app_config);

        let lines = vec![];
        let result = repository.save_project_file(lines);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No content to write")
        );
    }

    // save

    #[test]
    fn test_save_should_update_and_save_project_file() {
        use crate::plugin_config_repository::plugin::Plugin;
        use std::collections::BTreeMap;
        use std::path::Path;

        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from("tests/mocks/project.godot")),
            Some(String::from("addons")),
        );

        let mut mock_file_service = MockDefaultFileService::default();
        mock_file_service
            .expect_file_exists()
            .returning(|_| Ok(true));
        mock_file_service.expect_read_file_cached().returning(|_| {
            Ok(String::from(
                "config_version=5\n\
                    [application]\n\
                    config/name=\"Test\"\n\
                    [rendering]\n\
                    renderer/rendering_method=\"gl_compatibility\"\n",
            ))
        });
        mock_file_service
            .expect_write_file()
            .withf(|path: &Path, content: &str| {
                path.to_str().unwrap() == "tests/mocks/project.godot"
                    && content.contains("[editor_plugins]")
                    && content.contains("enabled=PackedStringArray")
            })
            .times(1)
            .returning(|_, _| Ok(()));

        let repository = DefaultGodotConfigRepository::new(Box::new(mock_file_service), app_config);

        let mut plugins = BTreeMap::new();
        plugins.insert(
            "test_plugin".to_string(),
            Plugin::new(
                "1".to_string(),
                "Test Plugin".to_string(),
                "1.0.0".to_string(),
                "MIT".to_string(),
            ),
        );
        let plugin_config = DefaultPluginConfig::new(plugins);

        let result = repository.save(plugin_config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_save_should_return_error_when_file_not_found() {
        use std::collections::BTreeMap;

        let app_config = DefaultAppConfig::new(
            None,
            None,
            None,
            Some(String::from("tests/mocks/project.godot")),
            Some(String::from("addons")),
        );

        let mut mock_file_service = MockDefaultFileService::default();
        mock_file_service
            .expect_file_exists()
            .returning(|_| Ok(false));

        let repository = DefaultGodotConfigRepository::new(Box::new(mock_file_service), app_config);

        let plugin_config = DefaultPluginConfig::new(BTreeMap::new());

        let result = repository.save(plugin_config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Godot project file not found")
        );
    }
}
