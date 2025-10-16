use dotenv::dotenv;

/// Application configuration settings
///
/// Settings are loaded from environment variables using the `dotenv` crate.
#[derive(Debug, Clone)]
pub struct AppConfig {
    api_base_url: String,
    config_file_path: String,
    cache_folder_path: String,
    godot_project_file_path: String,
    addon_folder_path: String,
}

impl AppConfig {
    pub fn new(
        api_base_url: Option<String>,
        config_file_path: Option<String>,
        cache_folder_path: Option<String>,
        godot_project_file_path: Option<String>,
        addon_folder_path: Option<String>,
    ) -> AppConfig {
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
        AppConfig {
            api_base_url,
            config_file_path,
            cache_folder_path,
            godot_project_file_path,
            addon_folder_path,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        dotenv().ok();

        let api_base_url = dotenv::var("API_BASE_URL").unwrap();
        let config_file_path = dotenv::var("CONFIG_FILE_PATH").unwrap();
        let cache_folder_path = dotenv::var("CACHE_FOLDER_PATH").unwrap();
        let godot_project_file_path = dotenv::var("GODOT_PROJECT_FILE_PATH").unwrap();
        let addon_folder_path = dotenv::var("ADDON_FOLDER_PATH").unwrap();
        AppConfig {
            api_base_url,
            config_file_path,
            cache_folder_path,
            godot_project_file_path,
            addon_folder_path,
        }
    }
}

#[cfg_attr(test, mockall::automock)]
impl AppConfigImpl for AppConfig {
    fn get_godot_project_file_path(&self) -> String {
        self.godot_project_file_path.clone()
    }

    fn get_api_base_url(&self) -> String {
        self.api_base_url.clone()
    }

    fn get_config_file_path(&self) -> String {
        self.config_file_path.clone()
    }

    fn get_cache_folder_path(&self) -> String {
        self.cache_folder_path.clone()
    }

    fn get_addon_folder_path(&self) -> String {
        self.addon_folder_path.clone()
    }
}

impl dyn AppConfigImpl {
    fn default() -> Box<Self> {
        dotenv().ok();

        let api_base_url = dotenv::var("API_BASE_URL").unwrap();
        let config_file_path = dotenv::var("CONFIG_FILE_PATH").unwrap();
        let cache_folder_path = dotenv::var("CACHE_FOLDER_PATH").unwrap();
        let godot_project_file_path = dotenv::var("GODOT_PROJECT_FILE_PATH").unwrap();
        let addon_folder_path = dotenv::var("ADDON_FOLDER_PATH").unwrap();
        Box::new(AppConfig {
            api_base_url,
            config_file_path,
            cache_folder_path,
            godot_project_file_path,
            addon_folder_path,
        })
    }
}

pub trait AppConfigImpl {
    fn get_godot_project_file_path(&self) -> String;
    fn get_api_base_url(&self) -> String;
    fn get_config_file_path(&self) -> String;
    fn get_cache_folder_path(&self) -> String;
    fn get_addon_folder_path(&self) -> String;
}
