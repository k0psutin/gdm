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

    /// Parse a Godot asset version string into a semantic version
    ///
    /// Godot Asset Store might use version strings like "11" or "2.0" which are not valid semantic versions.
    /// This function converts them into valid semantic versions, e.g.:
    /// ```parse_semantic_version("11") // returns semver::Version { major: 11, minor: 0, patch: 0 }```
    pub fn parse_semantic_version(version: &str) -> semver::Version {
        let number_regex = regex::Regex::new(r"^\d+$").unwrap();
        let two_part_regex = regex::Regex::new(r"^\d+\.\d+$").unwrap();

        if number_regex.is_match(version) {
            let major: u64 = version.parse().unwrap_or(0);
            return semver::Version::new(major, 0, 0);
        }

        if two_part_regex.is_match(version) {
            let new_version_string = format!("{}.0", version);
            return semver::Version::parse(&new_version_string)
                .unwrap_or(semver::Version::new(0, 0, 0));
        }

        semver::Version::parse(version).unwrap_or(semver::Version::new(0, 0, 0))
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

    #[test]
    fn test_parse_semantic_version_valid() {
        let version = "1.0.0";
        let parsed = Utils::parse_semantic_version(version);
        assert_eq!(parsed.major, 1);
        assert_eq!(parsed.minor, 0);
        assert_eq!(parsed.patch, 0);
        assert!(parsed.pre.is_empty());
        assert!(parsed.build.is_empty());
    }

    #[test]
    fn test_parse_semantic_version_with_pre_release() {
        let version = "2.1.3-alpha";
        let parsed = Utils::parse_semantic_version(version);
        assert_eq!(parsed.major, 2);
        assert_eq!(parsed.minor, 1);
        assert_eq!(parsed.patch, 3);
        assert_eq!(parsed.pre.as_str(), "alpha");
    }

    #[test]
    fn test_parse_semantic_version_with_build_metadata() {
        let version = "0.0.1+build.1";
        let parsed = Utils::parse_semantic_version(version);
        assert_eq!(parsed.major, 0);
        assert_eq!(parsed.minor, 0);
        assert_eq!(parsed.patch, 1);
        assert_eq!(parsed.build.as_str(), "build.1");
    }

    #[test]
    fn test_parse_semantic_version_godot_asset_version_integer() {
        let version = "11";
        let parsed = Utils::parse_semantic_version(version);
        assert_eq!(parsed.major, 11);
        assert_eq!(parsed.minor, 0);
        assert_eq!(parsed.patch, 0);
    }

    #[test]
    fn test_parse_semantic_version_godot_asset_version_two_parts() {
        let version = "2.0";
        let parsed = Utils::parse_semantic_version(version);
        assert_eq!(parsed.major, 2);
        assert_eq!(parsed.minor, 0);
        assert_eq!(parsed.patch, 0);
    }

    #[test]
    fn test_parse_semantic_version_invalid() {
        let version = "not_a_version";
        let parsed = Utils::parse_semantic_version(version);
        assert_eq!(parsed.major, 0);
        assert_eq!(parsed.minor, 0);
        assert_eq!(parsed.patch, 0);
    }
}
