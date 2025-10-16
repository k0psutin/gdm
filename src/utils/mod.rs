use anyhow::{Ok, Result, anyhow};

pub struct Utils;

impl Utils {
    /// Convert a plugin folder path to a Godot resource path
    ///
    /// Godot resource path is in format: res://addons/plugin_name/plugin.cfg
    ///
    /// e.g. ```plugin_folder_to_resource_path("some_plugin") // returns "res://addons/some_plugin/plugin.cfg"```
    pub fn plugin_folder_to_resource_path(plugin_root_folder: String) -> String {
        format!("res://{}/plugin.cfg", plugin_root_folder)
    }

    /// Convert a Godot resource path to a plugin folder path
    ///
    /// Godot resource path is in format: res://addons/plugin_name/plugin.cfg,
    /// or it might be res://addons/some/other/path/plugin.cfg
    pub fn resource_path_to_plugin_folder(resource_path: String) -> Result<String> {
        let _path = resource_path
            .replace("res://", "")
            .replace("/plugin.cfg", "");
        let paths = _path.split('/');
        if paths.clone().count() < 2 {
            return Err(anyhow!("Invalid resource path: {}", resource_path.clone()));
        }
        Ok(paths.skip(1).collect::<Vec<&str>>().join("/"))
    }

    pub fn plugin_name_to_addon_folder_path(plugin_name: String, addon_folder: String) -> String {
        format!("{}/{}", addon_folder, plugin_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_folder_to_resource_path() {
        let path = String::from("addons/some_plugin");
        let resource_path = Utils::plugin_folder_to_resource_path(path);
        assert_eq!(resource_path, "res://addons/some_plugin/plugin.cfg");
    }

    #[test]
    fn test_plugin_name_to_addon_folder_path() {
        let plugin_name = String::from("some_plugin");
        let addon_folder_path =
            Utils::plugin_name_to_addon_folder_path(plugin_name, String::from("addons"));
        assert_eq!(addon_folder_path, "addons/some_plugin");
    }

    #[test]
    fn test_plugin_name_to_addon_folder_path_two_levels() {
        let plugin_name = String::from("some_folder/some_plugin");
        let addon_folder_path =
            Utils::plugin_name_to_addon_folder_path(plugin_name, String::from("addons"));
        assert_eq!(addon_folder_path, "addons/some_folder/some_plugin");
    }

    #[test]
    fn test_resource_path_to_plugin_folder_one_level_depth() {
        let resource_path = String::from("res://addons/some_plugin/plugin.cfg");
        let plugin_folder = Utils::resource_path_to_plugin_folder(resource_path).unwrap();
        assert_eq!(plugin_folder, "some_plugin");
    }
}
