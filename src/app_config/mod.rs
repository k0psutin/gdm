use serde::Deserialize;

use std::path::Path;

/// Application configuration settings
///
/// Settings are loaded from environment variables using the `dotenv` crate.

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
        dotenvy::dotenv().ok();
        let default_app_config = envy::from_env::<DefaultAppConfig>().unwrap();
        DefaultAppConfig {
            api_base_url: api_base_url.unwrap_or(default_app_config.api_base_url),
            config_file_path: config_file_path.unwrap_or(default_app_config.config_file_path),
            cache_folder_path: cache_folder_path.unwrap_or(default_app_config.cache_folder_path),
            godot_project_file_path: godot_project_file_path
                .unwrap_or(default_app_config.godot_project_file_path),
            addon_folder_path: addon_folder_path.unwrap_or(default_app_config.addon_folder_path),
        }
    }
}

impl Default for DefaultAppConfig {
    fn default() -> Self {
        dotenvy::dotenv().ok();
        envy::from_env::<DefaultAppConfig>()
            .expect("Failed to load configuration from environment variables")
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

    fn get_addon_folder_path(&self) -> &Path {
        Path::new(&self.addon_folder_path)
    }
}

impl dyn AppConfig {
    #[allow(unused)]
    fn default() -> Box<Self> {
        dotenvy::dotenv().ok();
        Box::new(
            envy::from_env::<DefaultAppConfig>()
                .expect("Failed to load configuration from environment variables"),
        )
    }
}

pub trait AppConfig: Send + Sync + 'static {
    fn get_godot_project_file_path(&self) -> &Path;
    fn get_config_file_path(&self) -> &Path;
    fn get_cache_folder_path(&self) -> &Path;
    fn get_addon_folder_path(&self) -> &Path;
}
