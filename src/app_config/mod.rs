use dotenv::dotenv;

use std::path::Path;

/// Application configuration settings
///
/// Settings are loaded from environment variables using the `dotenv` crate.
#[cfg(not(tarpaulin_include))]
#[derive(Debug, Clone)]
pub struct DefaultAppConfig {
    api_base_url: String,
    config_file_path: String,
    cache_folder_path: String,
    godot_project_file_path: String,
    addon_folder_path: String,
}

#[cfg(not(tarpaulin_include))]
impl DefaultAppConfig {
    #[allow(dead_code)]
    pub fn new(
        api_base_url: Option<String>,
        config_file_path: Option<String>,
        cache_folder_path: Option<String>,
        godot_project_file_path: Option<String>,
        addon_folder_path: Option<String>,
    ) -> DefaultAppConfig {
        dotenv().ok();

        let api_base_url = api_base_url.unwrap_or_else(|| dotenv::var("API_BASE_URL").unwrap());
        let config_file_path =
            config_file_path.unwrap_or_else(|| dotenv::var("CONFIG_FILE_PATH").unwrap());
        let cache_folder_path =
            cache_folder_path.unwrap_or_else(|| dotenv::var("CACHE_FOLDER_PATH").unwrap());
        let godot_project_file_path = godot_project_file_path
            .unwrap_or_else(|| dotenv::var("GODOT_PROJECT_FILE_PATH").unwrap());
        let addon_folder_path =
            addon_folder_path.unwrap_or_else(|| dotenv::var("ADDON_FOLDER_PATH").unwrap());
        DefaultAppConfig {
            api_base_url,
            config_file_path,
            cache_folder_path,
            godot_project_file_path,
            addon_folder_path,
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl Default for DefaultAppConfig {
    fn default() -> Self {
        dotenv().ok();

        let api_base_url = dotenv::var("API_BASE_URL").unwrap();
        let config_file_path = dotenv::var("CONFIG_FILE_PATH").unwrap();
        let cache_folder_path = dotenv::var("CACHE_FOLDER_PATH").unwrap();
        let godot_project_file_path = dotenv::var("GODOT_PROJECT_FILE_PATH").unwrap();
        let addon_folder_path = dotenv::var("ADDON_FOLDER_PATH").unwrap();
        DefaultAppConfig {
            api_base_url,
            config_file_path,
            cache_folder_path,
            godot_project_file_path,
            addon_folder_path,
        }
    }
}

#[cfg_attr(test, mockall::automock)]
impl AppConfig for DefaultAppConfig {
    fn get_godot_project_file_path(&self) -> &Path {
        Path::new(&self.godot_project_file_path)
    }

    fn get_api_base_url(&self) -> String {
        self.api_base_url.clone()
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
    #[allow(dead_code)]
    fn default() -> Box<Self> {
        dotenv().ok();

        let api_base_url = dotenv::var("API_BASE_URL").unwrap();
        let config_file_path = dotenv::var("CONFIG_FILE_PATH").unwrap();
        let cache_folder_path = dotenv::var("CACHE_FOLDER_PATH").unwrap();
        let godot_project_file_path = dotenv::var("GODOT_PROJECT_FILE_PATH").unwrap();
        let addon_folder_path = dotenv::var("ADDON_FOLDER_PATH").unwrap();
        Box::new(DefaultAppConfig {
            api_base_url,
            config_file_path,
            cache_folder_path,
            godot_project_file_path,
            addon_folder_path,
        })
    }
}

pub trait AppConfig: Send + Sync + 'static {
    fn get_godot_project_file_path(&self) -> &Path;
    fn get_api_base_url(&self) -> String;
    fn get_config_file_path(&self) -> &Path;
    fn get_cache_folder_path(&self) -> &Path;
    fn get_addon_folder_path(&self) -> &Path;
}
