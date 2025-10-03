use crate::parser;

pub struct Settings {
    pub api_base_url: &'static str,
    pub application_name: &'static str,
    pub config_file_name: &'static str,
    pub godot_version: String,
    pub godot_plugins: Vec<String>,
}

impl Settings {
    pub fn get_settings() -> anyhow::Result<Settings> {
        let godot_version = parser::Parser::get_godot_version();
        let godot_plugins = parser::Parser::get_installed_plugins();

        Ok(Settings {
            api_base_url: "https://godotengine.org/asset-library/api",
            application_name: "Godot Dependency Manager",
            config_file_name: "gdm.json",
            godot_version: godot_version.unwrap_or_default(),
            godot_plugins: godot_plugins.unwrap_or_default(),
        })
    }
}
