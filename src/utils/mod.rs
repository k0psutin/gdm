use crate::app_config::AppConfig;

pub struct Utils;

impl Utils {
    pub fn plugin_folder_to_resource_path(plugin_path: String) -> String {
        format!("res://{}/plugin.cfg", plugin_path)
    }

    pub fn plugin_name_to_addon_folder_path(plugin_name: String) -> String {
        let addon_folder = AppConfig::default().get_addon_folder_path().to_string();
        format!("{}/{}", addon_folder, plugin_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_folder_to_resource_path() {
        let path = String::from("addons/gut");
        let resource_path = Utils::plugin_folder_to_resource_path(path);
        assert_eq!(resource_path, "res://addons/gut/plugin.cfg");
    }
}