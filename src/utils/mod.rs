pub struct Utils;

use std::path::{Path, PathBuf};

impl Utils {
    /// Convert a plugin folder path to a Godot resource path
    ///
    /// Godot resource path is in format: res://addons/plugin_name/plugin.cfg
    ///
    /// e.g. ```plugin_folder_to_resource_path("some_plugin") // returns "res://addons/some_plugin/plugin.cfg"```
    pub fn plugin_folder_to_resource_path(plugin_root_folder: String) -> String {
        format!("res://{}/plugin.cfg", plugin_root_folder)
    }

    pub fn plugin_name_to_addon_folder_path(addon_folder: &Path, plugin_name: &Path) -> PathBuf {
        addon_folder.join(plugin_name)
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
        let plugin_name = Path::new("some_plugin");
        let addon_folder_path =
            Utils::plugin_name_to_addon_folder_path(Path::new("addons"), plugin_name);
        assert_eq!(addon_folder_path, PathBuf::from("addons/some_plugin"));
    }

    #[test]
    fn test_plugin_name_to_addon_folder_path_two_levels() {
        let plugin_name = Path::new("some_folder/some_plugin");
        let addon_folder_path =
            Utils::plugin_name_to_addon_folder_path(Path::new("addons"), plugin_name);
        assert_eq!(
            addon_folder_path,
            PathBuf::from("addons/some_folder/some_plugin")
        );
    }
}
