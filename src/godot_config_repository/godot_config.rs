use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GodotConfig {
    config_version: usize,
    godot_version: String,
}

impl GodotConfig {
    pub fn new(config_version: usize, godot_version: String) -> GodotConfig {
        GodotConfig {
            config_version,
            godot_version,
        }
    }
}

impl Default for GodotConfig {
    fn default() -> Self {
        GodotConfig {
            config_version: 5,
            godot_version: "4.5".to_string(),
        }
    }
}

#[cfg_attr(test, mockall::automock)]
impl GodotConfigImpl for GodotConfig {
    fn get_config_godot_version(&self) -> String {
        self.godot_version.clone()
    }

    fn get_config_version(&self) -> usize {
        self.config_version
    }
}

pub trait GodotConfigImpl {
    fn get_config_godot_version(&self) -> String;
    fn get_config_version(&self) -> usize;

    fn get_godot_version(&self) -> Result<String> {
        if !self.get_config_godot_version().is_empty() {
            return Ok(self.get_config_godot_version());
        }
        self.get_default_godot_version()
    }

    fn get_default_godot_version(&self) -> Result<String> {
        match self.get_config_version() {
            5 => Ok("4.5".to_string()),
            4 => Ok("3.6".to_string()),
            _ => Err(anyhow!(
                "Unsupported config_version: {}",
                self.get_config_version()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
