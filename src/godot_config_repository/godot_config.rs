use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct DefaultGodotConfig {
    config_version: usize,
    godot_version: String,
}

impl DefaultGodotConfig {
    pub fn new(config_version: usize, godot_version: String) -> DefaultGodotConfig {
        DefaultGodotConfig {
            config_version,
            godot_version,
        }
    }
}

impl Default for DefaultGodotConfig {
    fn default() -> Self {
        DefaultGodotConfig {
            config_version: 5,
            godot_version: "4.5".to_string(),
        }
    }
}

impl DefaultGodotConfig {
    pub fn get_config_version(&self) -> usize {
        self.config_version
    }

    pub fn get_godot_version(&self) -> Result<String> {
        if !self.godot_version.is_empty() {
            return Ok(self.godot_version.clone());
        }
        self.get_default_godot_version()
    }

    fn get_default_godot_version(&self) -> Result<String> {
        match self.get_config_version() {
            5 => Ok("4.5".to_string()),
            4 => Ok("3.6".to_string()),
            _ => bail!("Unsupported config_version: {}", self.get_config_version()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // get_config_godot_version

    #[test]
    fn test_get_config_godot_version() {
        let config = DefaultGodotConfig::new(5, "4.5".to_string());
        assert_eq!(config.get_godot_version().unwrap(), "4.5");
    }

    // get_config_version

    #[test]
    fn test_get_config_version() {
        let config = DefaultGodotConfig::new(4, "3.6".to_string());
        assert_eq!(config.get_config_version(), 4);
    }

    // get_godot_version

    #[test]
    fn test_get_godot_version_with_non_empty_version() {
        let config = DefaultGodotConfig::new(5, "4.5".to_string());
        assert_eq!(config.get_godot_version().unwrap(), "4.5");
    }

    #[test]
    fn test_get_godot_version_with_empty_version() {
        let config = DefaultGodotConfig::new(5, "".to_string());
        assert_eq!(config.get_godot_version().unwrap(), "4.5");
    }

    // get_default_godot_version

    #[test]
    fn test_get_default_godot_version_supported_versions() {
        let config_v5 = DefaultGodotConfig::new(5, "".to_string());
        assert_eq!(config_v5.get_default_godot_version().unwrap(), "4.5");

        let config_v4 = DefaultGodotConfig::new(4, "".to_string());
        assert_eq!(config_v4.get_default_godot_version().unwrap(), "3.6");
    }

    #[test]
    fn test_get_default_godot_version_unsupported_version() {
        let config = DefaultGodotConfig::new(3, "".to_string());
        let result = config.get_default_godot_version();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Unsupported config_version: 3"
        );
    }
}
