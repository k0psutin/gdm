use serde_derive::Deserialize;

use std::path::{Path, PathBuf};

/// Application configuration settings
#[derive(Debug, Clone, Deserialize)]
pub struct DefaultAppConfig {
    /// API_BASE_URL environment variable
    pub api_base_url: String,
    /// CONFIG_FILE_PATH environment variable
    config_file_path: String,
    /// CACHE_FOLDER_PATH environment variable
    cache_folder_path: String,
    /// GODOT_PROJECT_FILE_PATH environment variable
    godot_project_file_path: String,
    /// ADDON_FOLDER_PATH environment variable
    addon_folder_path: String,
}

impl DefaultAppConfig {
    #[allow(unused)]
    pub fn new(
        api_base_url: Option<String>,
        config_file_path: Option<String>,
        cache_folder_path: Option<String>,
        godot_project_file_path: Option<String>,
        addon_folder_path: Option<String>,
    ) -> DefaultAppConfig {
        DefaultAppConfig {
            api_base_url: api_base_url
                .unwrap_or("https://godotengine.org/asset-library/api".to_string()),
            config_file_path: config_file_path.unwrap_or("gdm.json".to_string()),
            cache_folder_path: cache_folder_path.unwrap_or(".gdm".to_string()),
            godot_project_file_path: godot_project_file_path.unwrap_or("project.godot".to_string()),
            addon_folder_path: addon_folder_path.unwrap_or("addons".to_string()),
        }
    }
}

impl Default for DefaultAppConfig {
    fn default() -> Self {
        DefaultAppConfig {
            api_base_url: "https://godotengine.org/asset-library/api".to_string(),
            config_file_path: "gdm.json".to_string(),
            cache_folder_path: ".gdm".to_string(),
            godot_project_file_path: "project.godot".to_string(),
            addon_folder_path: "addons".to_string(),
        }
    }
}

#[cfg_attr(test, mockall::automock)]
impl AppConfig for DefaultAppConfig {
    fn get_godot_project_file_path(&self) -> &Path {
        Path::new(&self.godot_project_file_path)
    }

    fn get_config_file_path(&self) -> &Path {
        Path::new(&self.config_file_path)
    }

    fn get_cache_folder_path(&self) -> &Path {
        Path::new(&self.cache_folder_path)
    }

    fn get_addon_folder_path(&self) -> PathBuf {
        PathBuf::from(self.addon_folder_path.as_str())
    }
}

impl dyn AppConfig {
    #[allow(unused)]
    fn default() -> Box<Self> {
        Box::new(DefaultAppConfig::default())
    }
}

pub trait AppConfig: Send + Sync + 'static {
    fn get_godot_project_file_path(&self) -> &Path;
    fn get_config_file_path(&self) -> &Path;
    fn get_cache_folder_path(&self) -> &Path;
    fn get_addon_folder_path(&self) -> PathBuf;
}
