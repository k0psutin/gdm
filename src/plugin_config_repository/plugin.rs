use serde::{Deserialize, Serialize};

use semver::Version;

use crate::{api::asset_response::AssetResponse, utils::Utils};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Plugin {
    pub asset_id: String,
    pub title: String,
    #[serde(
        serialize_with = "serialize_version",
        deserialize_with = "deserialize_version"
    )]
    version: Version,
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

impl PartialOrd for Plugin {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.version.cmp(&other.version))
    }
}

impl From<AssetResponse> for Plugin {
    fn from(asset_response: AssetResponse) -> Self {
        Plugin::new(
            asset_response.asset_id,
            asset_response.title,
            asset_response.version_string,
            asset_response.cost, // TODO check if serde_json can map this directly to license
        )
    }
}

impl Plugin {
    pub fn new(asset_id: String, title: String, version: String, license: String) -> Plugin {
        Plugin {
            asset_id,
            title,
            version: Utils::parse_semantic_version(version.as_str()),
            license,
        }
    }

    pub fn get_version(&self) -> String {
        self.version.to_string()
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
            "Sample Plugin".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
        )
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = setup_test_plugin();
        assert_eq!(plugin.asset_id, "123");
        assert_eq!(plugin.title, "Sample Plugin");
        assert_eq!(plugin.get_version(), "1.0.0");
        assert_eq!(plugin.license, "MIT");
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
    }

    #[test]
    fn test_plugin_partial_eq() {
        let plugin1 = Plugin::new(
            "id1".to_string(),
            "Plugin One".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
        );
        let plugin2 = Plugin::new(
            "id1".to_string(),
            "Plugin One".to_string(),
            "2.0.0".to_string(),
            "MIT".to_string(),
        );
        let plugin3 = Plugin::new(
            "id2".to_string(),
            "Plugin Three".to_string(),
            "1.0.0".to_string(),
            "GPL".to_string(),
        );
        assert_ne!(plugin1, plugin2);
        assert_ne!(plugin1, plugin3);
    }

    #[test]
    fn test_plugin_partial_ord_semver_numeric_comparison() {
        let plugin_2_new = Plugin::new(
            "id2".to_string(),
            "Plugin 2".to_string(),
            "1.10.0".to_string(),
            "MIT".to_string(),
        );
        let plugin_2_old = Plugin::new(
            "id2".to_string(),
            "Plugin 2".to_string(),
            "1.2.0".to_string(),
            "MIT".to_string(),
        );
        assert!(plugin_2_new > plugin_2_old);
    }

    #[test]
    fn test_plugin_partial_ord_semver_different_length_versions_with_same_major_should_be_same() {
        let plugin_short = Plugin::new(
            "id".to_string(),
            "Plugin Long".to_string(),
            "1.0".to_string(),
            "MIT".to_string(),
        );
        let plugin_long = Plugin::new(
            "id".to_string(),
            "Plugin Long".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
        );
        assert!(plugin_short == plugin_long);
    }

    #[test]
    fn test_plugin_partial_ord_semver_pre_release_versions() {
        let plugin_pre = Plugin::new(
            "idPre".to_string(),
            "Plugin Pre".to_string(),
            "1.0.0-alpha".to_string(),
            "MIT".to_string(),
        );
        let plugin_release = Plugin::new(
            "idRel".to_string(),
            "Plugin Release".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
        );
        assert!(plugin_pre < plugin_release);
    }

    #[test]
    fn test_plugin_partial_ord_semver_empty_version_string() {
        let plugin_empty = Plugin::new(
            "idE".to_string(),
            "Plugin Empty".to_string(),
            "".to_string(),
            "MIT".to_string(),
        );
        let plugin_nonempty = Plugin::new(
            "idNE".to_string(),
            "Plugin NonEmpty".to_string(),
            "0.0.1".to_string(),
            "MIT".to_string(),
        );
        assert!(plugin_empty < plugin_nonempty);
    }

    #[test]
    fn test_plugin_partial_ord_semver_identical_versions() {
        let plugin_same1 = Plugin::new(
            "idSame1".to_string(),
            "Plugin Same".to_string(),
            "2.3.4".to_string(),
            "MIT".to_string(),
        );
        let plugin_same2 = Plugin::new(
            "idSame2".to_string(),
            "Plugin Same".to_string(),
            "2.3.4".to_string(),
            "MIT".to_string(),
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
            "Plugin A".to_string(),
            "1.0.0-alpha".to_string(),
            "MIT".to_string(),
        );
        let plugin_b = Plugin::new(
            "idB".to_string(),
            "Plugin B".to_string(),
            "1.0.0-beta".to_string(),
            "MIT".to_string(),
        );
        assert!(plugin_a < plugin_b);

        let plugin_num = Plugin::new(
            "idNum".to_string(),
            "Plugin Num".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
        );
        assert!(plugin_num > plugin_a);
    }

    #[test]
    fn test_plugin_partial_ord_version_with_leading_zeros() {
        let plugin_leading_zero = Plugin::new(
            "idLZ".to_string(),
            "Plugin LeadingZero".to_string(),
            "01.2.3".to_string(),
            "MIT".to_string(),
        );
        let plugin_normal = Plugin::new(
            "idN".to_string(),
            "Plugin Normal".to_string(),
            "1.2.3".to_string(),
            "MIT".to_string(),
        );
        assert!(plugin_leading_zero < plugin_normal);
    }

    #[test]
    fn test_plugin_partial_ord_version_single_vs_double_segment() {
        let plugin_leading_zero = Plugin::new(
            "idLZ".to_string(),
            "Plugin LeadingZero".to_string(),
            "1".to_string(),
            "MIT".to_string(),
        );
        let plugin_normal = Plugin::new(
            "idN".to_string(),
            "Plugin Normal".to_string(),
            "1.1".to_string(),
            "MIT".to_string(),
        );
        assert!(plugin_leading_zero < plugin_normal);
    }

    #[test]
    fn test_plugin_partial_ord_version_three_vs_two_segment() {
        let plugin_three_segment = Plugin::new(
            "idLZ".to_string(),
            "Plugin LeadingZero".to_string(),
            "1.1.1".to_string(),
            "MIT".to_string(),
        );
        let plugin_two_segment = Plugin::new(
            "idN".to_string(),
            "Plugin Normal".to_string(),
            "1.1".to_string(),
            "MIT".to_string(),
        );
        assert!(plugin_three_segment > plugin_two_segment);
    }

    #[test]
    fn test_plugin_serialize_to_json() {
        let plugin = Plugin {
            asset_id: "123".to_string(),
            title: "Test Plugin".to_string(),
            version: Version::new(1, 0, 0),
            license: "MIT".to_string(),
        };
        let json = serde_json::to_string(&plugin).unwrap();
        // Version should be serialized as a string
        assert!(json.contains("\"version\":\"1.0.0\""));
        assert!(json.contains("\"asset_id\":\"123\""));
        assert!(json.contains("\"title\":\"Test Plugin\""));
        assert!(json.contains("\"license\":\"MIT\""));
    }

    #[test]
    fn test_plugin_deserialize_from_json() {
        let json = r#"{
            "asset_id": "456",
            "title": "Deserialize Plugin",
            "version": "2.1.3",
            "license": "Apache-2.0"
        }"#;
        let plugin: Plugin = serde_json::from_str(json).unwrap();
        assert_eq!(plugin.asset_id, "456");
        assert_eq!(plugin.title, "Deserialize Plugin");
        assert_eq!(plugin.version, Version::new(2, 1, 3));
        assert_eq!(plugin.license, "Apache-2.0");
    }

    #[test]
    fn test_plugin_serialize_deserialize_roundtrip() {
        let original = Plugin {
            asset_id: "789".to_string(),
            title: "Roundtrip Plugin".to_string(),
            version: Version::parse("3.2.1-alpha").unwrap(),
            license: "GPL-3.0".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Plugin = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
        assert_eq!(deserialized.version, Version::parse("3.2.1-alpha").unwrap());
    }
}
