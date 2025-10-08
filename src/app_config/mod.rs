use std::sync::OnceLock;

pub struct AppConfig {
    api_base_url: &'static str,
    application_name: &'static str,
    config_file_name: &'static str,
    cache_folder_path: &'static str,
    godot_project_file_path: &'static str,
    addon_folder_path: &'static str,
}

impl AppConfig {
    pub fn get_godot_project_file_path(&self) -> &'static str {
        self.godot_project_file_path
    }

    pub fn get_api_base_url(&self) -> &'static str {
        self.api_base_url
    }

    pub fn get_application_name(&self) -> &'static str {
        self.application_name
    }

    pub fn get_config_file_name(&self) -> &'static str {
        self.config_file_name
    }

    pub fn get_cache_folder_path(&self) -> &'static str {
        self.cache_folder_path
    }

    pub fn get_addon_folder_path(&self) -> &'static str {
        self.addon_folder_path
    }

    pub fn default(app_config: AppConfig) -> AppConfig {
        app_config
    }

    pub fn new<'a>() -> &'a AppConfig {
        static INSTANCE: OnceLock<AppConfig> = OnceLock::new();
        INSTANCE.get_or_init(|| AppConfig {
            api_base_url: "https://godotengine.org/asset-library/api",
            application_name: "Godot Dependency Manager",
            config_file_name: "gdm.json",
            cache_folder_path: ".gdm",
            godot_project_file_path: "project.godot",
            addon_folder_path: "addons",
        })
    }
}
