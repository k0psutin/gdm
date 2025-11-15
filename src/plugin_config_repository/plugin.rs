use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use semver::Version;

use crate::{api::asset_response::AssetResponse, utils::Utils};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Plugin {
    pub asset_id: String,
    #[serde(skip)]
    pub plugin_cfg_path: Option<PathBuf>,
    pub title: String,
    #[serde(
        serialize_with = "serialize_version",
        deserialize_with = "deserialize_version"
    )]
    version: Version,
    #[serde(default = "Vec::new")]
    pub sub_assets: Vec<String>,
    pub license: String,
}

fn serialize_version<S>(version: &Version, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&version.to_string())
}

fn deserialize_version<'de, D>(deserializer: D) -> Result<Version, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    Ok(Utils::parse_semantic_version(&s))
}

impl Eq for Plugin {}

impl PartialEq for Plugin {
    fn eq(&self, other: &Self) -> bool {
        self.asset_id == other.asset_id && self.version == other.version
    }
}

impl PartialOrd for Plugin {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.version.cmp(&other.version))
    }
}

impl From<AssetResponse> for Plugin {
    fn from(asset_response: AssetResponse) -> Self {
        Plugin::new(
            asset_response.asset_id,
            None,
            asset_response.title,
            asset_response.version_string,
            asset_response.cost,
            Vec::new(),
        )
    }
}

impl Plugin {
    pub fn new(
        asset_id: String,
        plugin_cfg_path: Option<PathBuf>,
        title: String,
        version: String,
        license: String,
        sub_assets: Vec<String>,
    ) -> Plugin {
        Plugin {
            asset_id,
            plugin_cfg_path,
            title,
            version: Utils::parse_semantic_version(version.as_str()),
            license,
            sub_assets,
        }
    }

    pub fn get_version(&self) -> String {
        self.version.to_string()
    }

    #[cfg(test)]
    pub fn create_mock_plugin_1() -> Plugin {
        Plugin::new(
            "54321".to_string(),
            Some("addons/awesome_plugin/plugin.cfg".into()),
            "Awesome Plugin".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
            vec![],
        )
    }

    #[cfg(test)]
    pub fn create_mock_plugin_2() -> Plugin {
        Plugin::new(
            "12345".to_string(),
            Some("addons/super_plugin/plugin.cfg".into()),
            "Super Plugin".to_string(),
            "2.1.3".to_string(),
            "MIT".to_string(),
            vec![],
        )
    }

    #[cfg(test)]
    pub fn create_mock_plugin_3() -> Plugin {
        Plugin::new(
            "345678".to_string(),
            None,
            "Some Library".to_string(),
            "3.3.3".to_string(),
            "MIT".to_string(),
            vec!["sub_asset1".to_string(), "sub_asset2".to_string()],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::api::asset_response::AssetResponse;
    use serde_json;

    fn setup_test_plugin() -> Plugin {
        Plugin::new(
            "123".to_string(),
            Some(PathBuf::from("path/to/plugin.cfg")),
            "Sample Plugin".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
            vec!["sub1".to_string(), "sub2".to_string()],
        )
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = setup_test_plugin();
        assert_eq!(plugin.asset_id, "123");
        assert_eq!(plugin.title, "Sample Plugin");
        assert_eq!(plugin.get_version(), "1.0.0");
        assert_eq!(plugin.license, "MIT");
        assert_eq!(plugin.sub_assets, vec!["sub1", "sub2"]);
        assert_eq!(
            plugin.plugin_cfg_path,
            Some(PathBuf::from("path/to/plugin.cfg"))
        );
    }

    #[test]
    fn test_plugin_from_asset_response() {
        let asset_response = AssetResponse::new(
            "456".to_string(),
            "Test Asset".to_string(),
            "0.0.1".to_string(),
            "0.0.1".to_string(),
            "4.5".to_string(),
            "5".to_string(),
            "MIT".to_string(),
            "A test asset".to_string(),
            "GitHub".to_string(),
            "commit_hash".to_string(),
            "2023-01-01".to_string(),
            "https://example.com/old.zip".to_string(),
        );
        let plugin = Plugin::from(asset_response.clone());
        assert_eq!(plugin.asset_id, "456");
        assert_eq!(plugin.title, "Test Asset");
        assert_eq!(plugin.get_version(), "0.0.1");
        assert_eq!(plugin.license, "MIT");
        assert_eq!(plugin.sub_assets, Vec::<String>::new());
        assert_eq!(plugin.plugin_cfg_path, None);
    }

    #[test]
    fn test_plugin_partial_eq() {
        let plugin1 = Plugin::new(
            "id1".to_string(),
            None,
            "Plugin One".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
            vec!["sub1".to_string()],
        );
        let plugin2 = Plugin::new(
            "id1".to_string(),
            None,
            "Plugin One".to_string(),
            "2.0.0".to_string(),
            "MIT".to_string(),
            vec!["sub1".to_string()],
        );
        let plugin3 = Plugin::new(
            "id2".to_string(),
            None,
            "Plugin Three".to_string(),
            "1.0.0".to_string(),
            "GPL".to_string(),
            vec!["sub2".to_string()],
        );
        let plugin4 = Plugin::new(
            "id1".to_string(),
            None,
            "Plugin One".to_string(),
            "1.3.0".to_string(),
            "MIT".to_string(),
            vec!["sub1".to_string()], // different sub_assets
        );
        let plugin5 = Plugin::new(
            "id5".to_string(),
            Some(PathBuf::from("other/path/plugin.cfg")),
            "Plugin One".to_string(),
            "1.5.0".to_string(),
            "MIT".to_string(),
            vec!["sub1".to_string()],
        );
        assert_ne!(plugin1, plugin2);
        assert_ne!(plugin1, plugin3);
        assert_ne!(plugin1, plugin4);
        assert_ne!(plugin1, plugin5);
    }

    #[test]
    fn test_plugin_partial_ord_semver_numeric_comparison() {
        let plugin_2_new = Plugin::new(
            "id2".to_string(),
            None,
            "Plugin 2".to_string(),
            "1.10.0".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_2_old = Plugin::new(
            "id2".to_string(),
            None,
            "Plugin 2".to_string(),
            "1.2.0".to_string(),
            "MIT".to_string(),
            vec![],
        );
        assert!(plugin_2_new > plugin_2_old);
    }

    #[test]
    fn test_plugin_partial_ord_semver_different_length_versions_with_same_major_should_be_same() {
        let plugin_short = Plugin::new(
            "id".to_string(),
            None,
            "Plugin Long".to_string(),
            "1.0".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_long = Plugin::new(
            "id".to_string(),
            None,
            "Plugin Long".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
            vec![],
        );
        assert!(plugin_short == plugin_long);
    }

    #[test]
    fn test_plugin_partial_ord_semver_pre_release_versions() {
        let plugin_pre = Plugin::new(
            "idPre".to_string(),
            None,
            "Plugin Pre".to_string(),
            "1.0.0-alpha".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_release = Plugin::new(
            "idRel".to_string(),
            None,
            "Plugin Release".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
            vec![],
        );
        assert!(plugin_pre < plugin_release);
    }

    #[test]
    fn test_plugin_partial_ord_semver_empty_version_string() {
        let plugin_empty = Plugin::new(
            "idE".to_string(),
            None,
            "Plugin Empty".to_string(),
            "".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_nonempty = Plugin::new(
            "idNE".to_string(),
            None,
            "Plugin NonEmpty".to_string(),
            "0.0.1".to_string(),
            "MIT".to_string(),
            vec![],
        );
        assert!(plugin_empty < plugin_nonempty);
    }

    #[test]
    fn test_plugin_partial_ord_semver_identical_versions() {
        let plugin_same1 = Plugin::new(
            "idSame1".to_string(),
            None,
            "Plugin Same".to_string(),
            "2.3.4".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_same2 = Plugin::new(
            "idSame2".to_string(),
            None,
            "Plugin Same".to_string(),
            "2.3.4".to_string(),
            "MIT".to_string(),
            vec![],
        );
        assert_eq!(
            plugin_same1.partial_cmp(&plugin_same2),
            Some(std::cmp::Ordering::Equal)
        );
    }

    #[test]
    fn test_plugin_partial_ord_version_with_letters() {
        let plugin_a = Plugin::new(
            "idA".to_string(),
            None,
            "Plugin A".to_string(),
            "1.0.0-alpha".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_b = Plugin::new(
            "idB".to_string(),
            None,
            "Plugin B".to_string(),
            "1.0.0-beta".to_string(),
            "MIT".to_string(),
            vec![],
        );
        assert!(plugin_a < plugin_b);

        let plugin_num = Plugin::new(
            "idNum".to_string(),
            None,
            "Plugin Num".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
            vec![],
        );
        assert!(plugin_num > plugin_a);
    }

    #[test]
    fn test_plugin_partial_ord_version_with_leading_zeros() {
        let plugin_leading_zero = Plugin::new(
            "idLZ".to_string(),
            None,
            "Plugin LeadingZero".to_string(),
            "01.2.3".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_normal = Plugin::new(
            "idN".to_string(),
            None,
            "Plugin Normal".to_string(),
            "1.2.3".to_string(),
            "MIT".to_string(),
            vec![],
        );
        assert!(plugin_leading_zero < plugin_normal);
    }

    #[test]
    fn test_plugin_partial_ord_version_single_vs_double_segment() {
        let plugin_leading_zero = Plugin::new(
            "idLZ".to_string(),
            None,
            "Plugin LeadingZero".to_string(),
            "1".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_normal = Plugin::new(
            "idN".to_string(),
            None,
            "Plugin Normal".to_string(),
            "1.1".to_string(),
            "MIT".to_string(),
            vec![],
        );
        assert!(plugin_leading_zero < plugin_normal);
    }

    #[test]
    fn test_plugin_partial_ord_version_three_vs_two_segment() {
        let plugin_three_segment = Plugin::new(
            "idLZ".to_string(),
            None,
            "Plugin LeadingZero".to_string(),
            "1.1.1".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_two_segment = Plugin::new(
            "idN".to_string(),
            None,
            "Plugin Normal".to_string(),
            "1.1".to_string(),
            "MIT".to_string(),
            vec![],
        );
        assert!(plugin_three_segment > plugin_two_segment);
    }

    #[test]
    fn test_plugin_serialize_to_json() {
        let plugin = Plugin {
            asset_id: "123".to_string(),
            plugin_cfg_path: None,
            title: "Test Plugin".to_string(),
            version: Version::new(1, 0, 0),
            license: "MIT".to_string(),
            sub_assets: vec!["sub1".to_string()],
        };
        let json = serde_json::to_string(&plugin).unwrap();
        // Version should be serialized as a string
        assert!(json.contains("\"version\":\"1.0.0\""));
        assert!(json.contains("\"asset_id\":\"123\""));
        assert!(json.contains("\"title\":\"Test Plugin\""));
        assert!(json.contains("\"license\":\"MIT\""));
        assert!(json.contains("\"sub_assets\":[\"sub1\"]"));
        // plugin_cfg_path is skipped in serialization
    }

    #[test]
    fn test_plugin_deserialize_from_json() {
        let json = r#"{
            "asset_id": "456",
            "title": "Deserialize Plugin",
            "version": "2.1.3",
            "license": "Apache-2.0",
            "sub_assets": ["subA", "subB"]
        }"#;
        let plugin: Plugin = serde_json::from_str(json).unwrap();
        assert_eq!(plugin.asset_id, "456");
        assert_eq!(plugin.title, "Deserialize Plugin");
        assert_eq!(plugin.version, Version::new(2, 1, 3));
        assert_eq!(plugin.license, "Apache-2.0");
        assert_eq!(plugin.sub_assets, vec!["subA", "subB"]);
        // plugin_cfg_path is None by default
    }

    #[test]
    fn test_plugin_serialize_deserialize_roundtrip() {
        let original = Plugin {
            asset_id: "789".to_string(),
            plugin_cfg_path: Some(PathBuf::from("roundtrip/plugin.cfg")),
            title: "Roundtrip Plugin".to_string(),
            version: Version::parse("3.2.1-alpha").unwrap(),
            license: "GPL-3.0".to_string(),
            sub_assets: vec!["subX".to_string()],
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Plugin = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
        assert_eq!(deserialized.version, Version::parse("3.2.1-alpha").unwrap());
        assert_eq!(deserialized.sub_assets, vec!["subX".to_string()]);
        // plugin_cfg_path is None by default
    }

    #[test]
    fn test_plugin_deserialize_v_prefix_version() {
        let json = r#"{
            "asset_id": "101",
            "title": "V Prefix Plugin",
            "version": "v7.3.4",
            "license": "BSD-2-Clause",
            "sub_assets": []
        }"#;
        let deserialized: Plugin = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized.version, Version::parse("7.3.4").unwrap());
        assert_eq!(deserialized.sub_assets, Vec::<String>::new());
        // plugin_cfg_path is None by default
    }

    #[test]
    fn test_plugin_deserialize_v_prefix_version_with_build_metadata() {
        let json = r#"{
            "asset_id": "101",
            "title": "V Prefix Plugin",
            "version": "v7.3.4 (26)",
            "license": "BSD-2-Clause",
            "sub_assets": []
        }"#;
        let deserialized: Plugin = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized.version, Version::parse("7.3.4").unwrap());
        assert_eq!(deserialized.sub_assets, Vec::<String>::new());
        // plugin_cfg_path is None by default
    }
}
