pub struct Utils;

use regex::Regex;
use semver::Version;
use std::path::{Path, PathBuf};

impl Utils {
    pub fn plugin_name_to_addon_folder_path(addon_folder: &Path, plugin_name: &Path) -> PathBuf {
        addon_folder.join(plugin_name)
    }

    /// Parse a Godot asset version string into a semantic version
    ///
    /// Godot Asset Store might use version strings like "11" or "2.0" which are not valid semantic versions.
    /// This function converts them into valid semantic versions, e.g.:
    /// ```parse_semantic_version("11") // returns Version { major: 11, minor: 0, patch: 0 }```
    pub fn parse_semantic_version(version: &str) -> Version {
        let number_regex = Regex::new(r"^\d+$").unwrap();
        let two_part_regex = Regex::new(r"^\d+\.\d+$").unwrap();

        if number_regex.is_match(version) {
            let major: u64 = version.parse().unwrap_or(0);
            return Version::new(major, 0, 0);
        }

        if two_part_regex.is_match(version) {
            let new_version_string = format!("{}.0", version);
            return Version::parse(&new_version_string).unwrap_or(Version::new(0, 0, 0));
        }

        let parsed_version = Version::parse(version);

        if let Ok(v) = parsed_version {
            return v;
        }

        // If we don't have a valid semantic version yet, try to extract the first three-part version we can find
        let three_part_regex = Regex::new(r"\d+\.\d+\.\d+").unwrap();

        if let Some(captures) = three_part_regex.captures(version) {
            let semver_str = captures.get(0);
            if let Some(semver) = semver_str {
                return Version::parse(semver.as_str()).unwrap_or(Version::new(0, 0, 0));
            }
        }

        // Unable to parse version, return default 0.0.0
        Version::new(0, 0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_parse_semantic_version_with_prefix() {
        let version = "v1.2.3 (26)";
        let parsed = Utils::parse_semantic_version(version);
        assert_eq!(parsed.major, 1);
        assert_eq!(parsed.minor, 2);
        assert_eq!(parsed.patch, 3);
        assert_eq!(parsed.build.as_str(), "");
    }

    #[test]
    fn test_parse_semantic_version_with_prefix_with_space() {
        let version = "v 1.2.3 (26)";
        let parsed = Utils::parse_semantic_version(version);
        assert_eq!(parsed.major, 1);
        assert_eq!(parsed.minor, 2);
        assert_eq!(parsed.patch, 3);
        assert_eq!(parsed.build.as_str(), "");
    }

    #[test]
    fn test_parse_semantic_version_with_other_build_metadata() {
        let version = "1.2.3 (26)";
        let parsed = Utils::parse_semantic_version(version);
        assert_eq!(parsed.major, 1);
        assert_eq!(parsed.minor, 2);
        assert_eq!(parsed.patch, 3);
        assert_eq!(parsed.build.as_str(), "");
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
