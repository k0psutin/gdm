use std::collections::HashMap;

use anyhow::{Result, anyhow};

use std::fs::File;
use std::io::{self, BufRead};
use std::sync::OnceLock;

/// Singleton Parser for Godot project files (project.godot)
/// Parses project.godot file and gathers plugins, config_version, and godot_version
pub struct Parser {
    parsed_project: HashMap<String, Vec<String>>,
}

impl Parser {
    pub fn new<'a>(godot_project_file_path: &str) -> &'a Parser {
        static INSTANCE: OnceLock<Parser> = OnceLock::new();
        INSTANCE.get_or_init(|| Parser {
            parsed_project: Self::convert_godot_project_file_to_hashmap(godot_project_file_path)
                .unwrap_or_default(),
        })
    }

    pub fn get_parsed_project(&self) -> &HashMap<String, Vec<String>> {
        &self.parsed_project
    }

    fn plugin_vec_to_packed_string_array(plugins: &Vec<String>) -> String {
        let joined = plugins
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<String>>()
            .join(", ");
        format!("PackedStringArray({})", joined)
    }

    fn save_project_file(godot_project_file_path: &str, lines: Vec<String>) -> Result<()> {
        std::fs::write(godot_project_file_path, lines.join("\n"))?;
        Ok(())
    }

    pub fn update_plugins(
        &self,
        godot_project_file_path: &str,
        plugins: Vec<String>,
    ) -> Result<()> {
        let updated_lines = self.update_plugins_to_project_file(godot_project_file_path, plugins)?;
        Self::save_project_file(godot_project_file_path, updated_lines.clone())?;
        Ok(())
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
    fn update_plugins_to_project_file(
        &self,
        godot_project_file_path: &str,
        plugins: Vec<String>,
    ) -> Result<Vec<String>> {
        let contents = Self::read_godot_project_file(godot_project_file_path)?;
        let mut updated_lines: Vec<String> = contents
            .into_iter()
            .map(|l| l.unwrap_or_default())
            .collect();
        if updated_lines.last().unwrap() != "" {
            updated_lines.push("".to_string());
        }

        let editor_plugins_index = updated_lines
            .iter()
            .position(|line| line.starts_with("[editor_plugins]"));

        if plugins.is_empty() {
            // If there are no plugins, we need to remove the [editor_plugins] section if it exists.
            if let Some(index) = editor_plugins_index {
                for _ in 0..4 {
                    updated_lines.remove(index);
                }
            }
            return Ok(updated_lines);
        }

        let plugin_index = match editor_plugins_index {
            Some(index) => updated_lines
                .iter()
                .skip(index + 1)
                .position(|line| line.starts_with("enabled="))
                .map(|i| i + index + 1),
            None => None,
        };

        if plugin_index.is_some() {
            updated_lines[plugin_index.unwrap()] = format!(
                "enabled={}",
                Self::plugin_vec_to_packed_string_array(&plugins)
            );
            return Ok(updated_lines);
        }

        // If [editor_plugins] section doesn't exists, we need to add it to the project file.
        // I _think_ it should be added alphabetically, but I'm not 100% sure.
        for i in 0..updated_lines.len() {
            let line = &updated_lines[i];
            if line.starts_with("[")
                && line.ends_with("]")
                && line.to_lowercase().cmp(&"[editor_plugins]".to_string())
                    == std::cmp::Ordering::Greater
            {
                let new_lines = vec![
                    "[editor_plugins]".to_string(),
                    "".to_string(),
                    format!(
                        "enabled={}",
                        Self::plugin_vec_to_packed_string_array(&plugins)
                    ),
                    "".to_string(),
                ];
                updated_lines.splice(i..i, new_lines);
                return Ok(updated_lines);
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
    fn convert_godot_project_file_to_hashmap(
        godot_project_file_path: &str,
    ) -> Result<HashMap<String, Vec<String>>> {
        let contents = Self::read_godot_project_file(godot_project_file_path)?;
        let mut output: HashMap<String, Vec<String>> = HashMap::new();
        output.insert("config/plugins".to_string(), vec![]);
        output.insert("config_version".to_string(), vec![]);

        for line in contents {
            if let Ok(line) = line {
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
        }

        return Ok(output);
    }

    fn read_godot_project_file(godot_file_path: &str) -> Result<io::Lines<io::BufReader<File>>> {
        let file = File::open(godot_file_path)?;
        Ok(io::BufReader::new(file).lines())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_parser_new_creates_instance() {
        let parser = Parser::new("test/mocks/project.godot");
        assert!(parser.get_parsed_project().is_empty());
    }

    #[test]
    fn test_convert_godot_project_file_to_hashmap() {
        let result = Parser::convert_godot_project_file_to_hashmap(
            "test/mocks/project_with_plugins_and_version.godot",
        );
        assert!(result.is_ok());
        let config_map = result.unwrap();
        assert_eq!(
            config_map.get("config_version").unwrap(),
            &vec!["5".to_string()]
        );
        assert_eq!(
            config_map.get("enabled").unwrap(),
            &vec!["res://addons/test_plugin/plugin.cfg".to_string()]
        );
    }

    #[test]
    fn test_convert_godot_project_file_to_hashmap_no_file() {
        let result = Parser::convert_godot_project_file_to_hashmap("non_existent_file.godot");
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_vec_to_packed_string_array() {
        let plugins = vec![
            "res://addons/gd_flow/plugin.cfg".to_string(),
            "res://addons/gut/plugin.cfg".to_string(),
        ];
        let packed_string = Parser::plugin_vec_to_packed_string_array(&plugins);
        assert_eq!(
            packed_string,
            "PackedStringArray(\"res://addons/gd_flow/plugin.cfg\", \"res://addons/gut/plugin.cfg\")"
        );
    }

    #[test]
    fn test_update_plugins_existing_editor_plugins_section() {
        let parser = Parser::new("test/mocks/project_with_plugins_and_version.godot");
        let new_plugins = vec![
            "res://addons/test_plugin/plugin.cfg".to_string(),
            "res://addons/test_plugin_2/plugin.cfg".to_string(),
        ];
        let result = parser.update_plugins_to_project_file(
            "test/mocks/project_with_plugins_and_version.godot",
            new_plugins.clone(),
        );
        assert!(result.is_ok());
        let expected = fs::read_to_string(
            "test/mocks/project_with_plugins_and_version_new_plugin_expected.godot",
        )
        .unwrap();
        let updated_lines = result.unwrap();
        let updated_content = updated_lines.join("\n");
        assert_eq!(updated_content, expected);
    }

    #[test]
    fn test_update_plugins_should_create_editor_plugins_section() {
        let parser = Parser::new("test/mocks/project_without_plugins.godot");
        let new_plugins = vec!["res://addons/test_plugin/plugin.cfg".to_string()];
        let result = parser.update_plugins_to_project_file(
            "test/mocks/project_without_plugins.godot",
            new_plugins.clone(),
        );
        let expected =
            fs::read_to_string("test/mocks/project_without_plugins_expected.godot").unwrap();
        assert!(result.is_ok());
        let updated_lines = result.unwrap();
        let updated_content = updated_lines.join("\n");
        assert_eq!(updated_content, expected);
    }

    #[test]
    fn test_update_plugins_should_remove_editor_plugins_section_if_there_are_no_plugins() {
        let parser = Parser::new("test/mocks/project_with_plugins_and_version.godot");
        let new_plugins = vec![];
        let result = parser.update_plugins_to_project_file(
            "test/mocks/project_with_plugins_and_version.godot",
            new_plugins.clone(),
        );
        let expected = fs::read_to_string(
            "test/mocks/project_with_plugins_and_version_plugins_removed_expected.godot",
        )
        .unwrap();
        assert!(result.is_ok());
        let updated_lines = result.unwrap();
        let updated_content = updated_lines.join("\n");
        assert_eq!(updated_content, expected);
    }
}
