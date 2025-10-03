pub struct Parser;

use std::fs::File;
use std::io::{self, BufRead};
use std::collections::HashMap;
use lazy_static::lazy_static;

// Maybe we could transform project.godot into TOML and use a TOML parser?
// e.g. comments, replace ; with #,
// surround all keys with '' single quotes
// transform PackedStringArray(...) into a TOML array [...]

lazy_static! {
    static ref GODOT_CONFIG: HashMap<String, Vec<String>> = Parser::convert_godot_project_file_to_hashmap().unwrap();
}

impl Parser {
    fn get_config() -> &'static HashMap<String, Vec<String>> {
        &GODOT_CONFIG
    }

    pub fn get_godot_version() -> anyhow::Result<String> {
        let config = Self::get_config();
        if let Some(version_vec) = config.get("config/features") {
            if !version_vec.is_empty() {
                return Ok(version_vec[0].clone());
            }
        }
        Err(anyhow::anyhow!("Godot version not found in project.godot"))
    }

    pub fn get_installed_plugins() -> anyhow::Result<Vec<String>> {
        let config = Self::get_config();
        if let Some(plugins_vec) = config.get("config/plugins") {
            return Ok(plugins_vec.clone());
        }
        Err(anyhow::anyhow!("Installed plugins not found in project.godot"))
    }

    fn convert_godot_project_file_to_hashmap() -> anyhow::Result<HashMap<String, Vec<String>>> {
        let contents = Self::read_godot_project_file();
        let mut output: HashMap<String, Vec<String>> = HashMap::new();

        if contents.is_err() {
            eprintln!("Failed to open project.godot file");
            return Err(anyhow::anyhow!("Failed to open project.godot file"));
        }

        for line in contents.unwrap() {
            if let Ok(line) = line {
                if line.starts_with("config/features=") || line.starts_with("enabled=") {
                    let parts: Vec<&str> = line.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        let key = parts[0].trim().to_string();
                        let mut value = parts[1].trim().to_string();
                    if value.starts_with("PackedStringArray") {
                            value = value.replace("PackedStringArray(", "").replace(")", "");
                            let parts: Vec<String> = value.split(',').map(|s| s.replace('"', "").trim().to_string()).collect();
                            output.insert(key, parts);
                        } 
                    }
                }
            }
        }

        return Ok(output);
    }

    fn read_godot_project_file() -> io::Result<io::Lines<io::BufReader<File>>> {
        let file = File::open("project.godot").expect("Unable to read project.godot");
        Ok(io::BufReader::new(file).lines())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_godot_project_file() {
        let file = Parser::read_godot_project_file();
        assert!(file.is_ok());
    }

    #[test]
    fn test_convert_godot_project_file_to_hashmap() {
        let content = Parser::convert_godot_project_file_to_hashmap();
        
        assert!(content.is_ok());
        
        let map = content.unwrap();
        println!(
            "Converted HashMap content:\n{:?}",
            map
        );
        assert!(!map.is_empty());
    }
}
