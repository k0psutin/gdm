use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{models::Asset, utils::Utils};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum PluginSource {
    AssetStore {
        publisher_slug: String,
        asset_slug: String,
    },
    Git {
        url: String,
        reference: String,
        #[serde(skip)]
        publisher_slug: String,
        #[serde(skip)]
        asset_slug: String,
    }, // Optionally store git URL and ref
}

impl PartialEq for PluginSource {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                PluginSource::AssetStore {
                    publisher_slug: slug1,
                    asset_slug: asset1,
                },
                PluginSource::AssetStore {
                    publisher_slug: slug2,
                    asset_slug: asset2,
                },
            ) => slug1 == slug2 && asset1 == asset2,
            (
                PluginSource::Git {
                    url: url1,
                    reference: ref1,
                    ..
                },
                PluginSource::Git {
                    url: url2,
                    reference: ref2,
                    ..
                },
            ) => url1 == url2 && ref1 == ref2,
            _ => false,
        }
    }
}

impl Eq for PluginSource {}

impl PluginSource {
    pub fn asset_slug(&self) -> &str {
        match self {
            PluginSource::AssetStore { asset_slug, .. } | PluginSource::Git { asset_slug, .. } => {
                asset_slug
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Plugin {
    pub source: PluginSource,
    /// Path to the plugin.cfg file within the Godot project, using Unix-style separators.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_cfg_path: Option<String>,
    pub title: String,
    pub version: String,
    #[serde(default = "Vec::new")]
    pub sub_assets: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

impl Eq for Plugin {}

impl PartialEq for Plugin {
    fn eq(&self, other: &Self) -> bool {
        let other_version = Utils::parse_semantic_version(&other.version);
        let self_version = Utils::parse_semantic_version(&self.version);
        self.source == other.source && self_version == other_version
    }
}

impl PartialOrd for Plugin {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let other_version = Utils::parse_semantic_version(&other.version);
        let self_version = Utils::parse_semantic_version(&self.version);
        Some(self_version.cmp(&other_version))
    }
}

impl From<Asset> for Plugin {
    fn from(asset: Asset) -> Self {
        Plugin::new(
            PluginSource::AssetStore {
                publisher_slug: asset.publisher_slug.clone(),
                asset_slug: asset.asset_slug.clone(),
            },
            None,
            asset.title,
            asset.version,
            Some(asset.license),
            Vec::new(),
        )
    }
}

impl Plugin {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
        source: PluginSource,
        plugin_cfg_path: Option<PathBuf>,
        title: String,
        version: String,
        license: Option<String>,
        sub_assets: Vec<String>,
    ) -> Plugin {
        // Convert PathBuf to Unix-style string path
        let _plugin_cfg_path = plugin_cfg_path.map(|p| {
            p.iter()
                .map(|s| s.to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
        });
        Plugin {
            source,
            plugin_cfg_path: _plugin_cfg_path,
            title,
            version,
            license,
            sub_assets,
        }
    }

    #[cfg(test)]
    pub fn new_asset_store_plugin(
        asset_slug: String,
        publisher_slug: String,
        plugin_cfg_path: Option<PathBuf>,
        title: String,
        version: String,
        license: String,
        sub_assets: Vec<String>,
    ) -> Plugin {
        Plugin::new(
            PluginSource::AssetStore {
                asset_slug: asset_slug.clone(),
                publisher_slug: publisher_slug.clone(),
            },
            plugin_cfg_path,
            title.to_string(),
            version.to_string(),
            Some(license.to_string()),
            sub_assets,
        )
    }

    pub fn get_version(&self) -> String {
        self.version.to_string()
    }

    #[cfg(test)]
    pub fn create_mock_plugin_1() -> Plugin {
        Plugin::new(
            PluginSource::AssetStore {
                asset_slug: "asset_54321".to_string(),
                publisher_slug: "publisher_12345".to_string(),
            },
            Some("addons/asset_54321/plugin.cfg".into()),
            "Awesome Plugin".to_string(),
            "1.0.0".to_string(),
            Some("MIT".to_string()),
            vec![],
        )
    }

    #[cfg(test)]
    pub fn create_mock_plugin_2() -> Plugin {
        Plugin::new(
            PluginSource::AssetStore {
                asset_slug: "asset_3344332".to_string(),
                publisher_slug: "publisher_3344332".to_string(),
            },
            Some("addons/asset_3344332/plugin.cfg".into()),
            "Super Plugin".to_string(),
            "2.1.3".to_string(),
            Some("MIT".to_string()),
            vec![],
        )
    }

    #[cfg(test)]
    pub fn create_mock_plugin_3() -> Plugin {
        Plugin::new(
            PluginSource::AssetStore {
                asset_slug: "asset_345678".to_string(),
                publisher_slug: "publisher_876543".to_string(),
            },
            None,
            "Some Library".to_string(),
            "3.3.3".to_string(),
            Some("MIT".to_string()),
            vec!["sub_asset1".to_string(), "sub_asset2".to_string()],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_plugin() -> Plugin {
        Plugin::new(
            PluginSource::AssetStore {
                asset_slug: "123".to_string(),
                publisher_slug: "321".to_string(),
            },
            Some(PathBuf::from("path/to/plugin.cfg")),
            "Sample Plugin".to_string(),
            "1.0.0".to_string(),
            Some("MIT".to_string()),
            vec!["sub1".to_string(), "sub2".to_string()],
        )
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = setup_test_plugin();
        assert_eq!(
            plugin.source,
            PluginSource::AssetStore {
                asset_slug: "123".to_string(),
                publisher_slug: "321".to_string(),
            },
        );
        assert_eq!(plugin.title, "Sample Plugin");
        assert_eq!(plugin.get_version(), "1.0.0");
        assert_eq!(plugin.license, Some("MIT".to_string()));
        assert_eq!(plugin.sub_assets, vec!["sub1", "sub2"]);
        assert_eq!(
            plugin.plugin_cfg_path,
            Some("path/to/plugin.cfg".to_string())
        );
    }

    #[test]
    fn test_git_source_equality_survives_deserialization() {
        let source = PluginSource::Git {
            url: "https://github.com/SomeUser/MyPlugin.git".to_string(),
            reference: "main".to_string(),
            publisher_slug: "someuser".to_string(),
            asset_slug: "myplugin".to_string(),
        };

        let serialized = toml::to_string(&source).unwrap();
        let deserialized: PluginSource = toml::from_str(&serialized).unwrap();

        assert_eq!(source, deserialized);
    }

    #[test]
    fn test_plugin_partial_eq() {
        let plugin1 = Plugin::new_asset_store_plugin(
            "id1".to_string(),
            "publisher1".to_string(),
            None,
            "Plugin One".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
            vec!["sub1".to_string()],
        );
        let plugin2 = Plugin::new_asset_store_plugin(
            "id1".to_string(),
            "publisher1".to_string(),
            None,
            "Plugin One".to_string(),
            "2.0.0".to_string(),
            "MIT".to_string(),
            vec!["sub1".to_string()],
        );
        let plugin3 = Plugin::new_asset_store_plugin(
            "id2".to_string(),
            "publisher2".to_string(),
            None,
            "Plugin Three".to_string(),
            "1.0.0".to_string(),
            "GPL".to_string(),
            vec!["sub2".to_string()],
        );
        let plugin4 = Plugin::new_asset_store_plugin(
            "id1".to_string(),
            "publisher1".to_string(),
            None,
            "Plugin One".to_string(),
            "1.3.0".to_string(),
            "MIT".to_string(),
            vec!["sub1".to_string()], // different sub_assets
        );
        let plugin5 = Plugin::new_asset_store_plugin(
            "id5".to_string(),
            "publisher5".to_string(),
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
        let plugin_2_new = Plugin::new_asset_store_plugin(
            "id2".to_string(),
            "publisher_id2".to_string(),
            None,
            "Plugin 2".to_string(),
            "1.10.0".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_2_old = Plugin::new_asset_store_plugin(
            "id2".to_string(),
            "publisher_id2".to_string(),
            None,
            "Plugin 2".to_string(),
            "1.2.0".to_string(),
            "MIT".to_string(),
            vec![],
        );
        assert!(plugin_2_new > plugin_2_old);
    }

    #[test]
    fn test_plugin_partial_ord_semver_pre_release_versions() {
        let plugin_pre = Plugin::new_asset_store_plugin(
            "idPre".to_string(),
            "idPre_publisher".to_string(),
            None,
            "Plugin Pre".to_string(),
            "1.0.0-alpha".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_release = Plugin::new_asset_store_plugin(
            "idRel".to_string(),
            "idRel_publisher".to_string(),
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
        let plugin_empty = Plugin::new_asset_store_plugin(
            "idE".to_string(),
            "idE_publisher".to_string(),
            None,
            "Plugin Empty".to_string(),
            "".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_nonempty = Plugin::new_asset_store_plugin(
            "idNE".to_string(),
            "idNE_publisher".to_string(),
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
        let plugin_same1 = Plugin::new_asset_store_plugin(
            "idSame1".to_string(),
            "idSame1_publisher".to_string(),
            None,
            "Plugin Same".to_string(),
            "2.3.4".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_same2 = Plugin::new_asset_store_plugin(
            "idSame2".to_string(),
            "idSame2_publisher".to_string(),
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
        let plugin_a = Plugin::new_asset_store_plugin(
            "idA".to_string(),
            "idA_publisher".to_string(),
            None,
            "Plugin A".to_string(),
            "1.0.0-alpha".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_b = Plugin::new_asset_store_plugin(
            "idB".to_string(),
            "idB_publisher".to_string(),
            None,
            "Plugin B".to_string(),
            "1.0.0-beta".to_string(),
            "MIT".to_string(),
            vec![],
        );
        assert!(plugin_a < plugin_b);

        let plugin_num = Plugin::new_asset_store_plugin(
            "idNum".to_string(),
            "idNum_publisher".to_string(),
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
        let plugin_leading_zero = Plugin::new_asset_store_plugin(
            "idLZ".to_string(),
            "idLZ_publisher".to_string(),
            None,
            "Plugin LeadingZero".to_string(),
            "01.2.3".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_normal = Plugin::new_asset_store_plugin(
            "idN".to_string(),
            "idN_publisher".to_string(),
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
        let plugin_leading_zero = Plugin::new_asset_store_plugin(
            "idLZ".to_string(),
            "idLZ_publisher".to_string(),
            None,
            "Plugin LeadingZero".to_string(),
            "1".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_normal = Plugin::new_asset_store_plugin(
            "idN".to_string(),
            "idN_publisher".to_string(),
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
        let plugin_three_segment = Plugin::new_asset_store_plugin(
            "idLZ".to_string(),
            "idLZ_publisher".to_string(),
            None,
            "Plugin LeadingZero".to_string(),
            "1.1.1".to_string(),
            "MIT".to_string(),
            vec![],
        );
        let plugin_two_segment = Plugin::new_asset_store_plugin(
            "idN".to_string(),
            "idN_publisher".to_string(),
            None,
            "Plugin Normal".to_string(),
            "1.1".to_string(),
            "MIT".to_string(),
            vec![],
        );
        assert!(plugin_three_segment > plugin_two_segment);
    }

    #[test]
    fn test_plugin_serialize_to_toml() {
        let plugin = Plugin::new_asset_store_plugin(
            "123".to_string(),
            "publisher_123".to_string(),
            None,
            "Test Plugin".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
            vec!["sub1".to_string()],
        );
        let toml = toml::to_string(&plugin).unwrap();
        assert!(toml.contains("version = \"1.0.0\""));
        assert!(toml.contains("asset_slug = \"123\""));
        assert!(toml.contains("publisher_slug = \"publisher_123\""));
        assert!(toml.contains("title = \"Test Plugin\""));
        assert!(toml.contains("license = \"MIT\""));
        assert!(toml.contains("sub1"));
        // plugin_cfg_path is skipped in serialization
    }

    #[test]
    fn test_plugin_serialize_deserialize_roundtrip() {
        let original = Plugin::new_asset_store_plugin(
            "789".to_string(),
            "789_publisher".to_string(),
            Some(PathBuf::from("roundtrip/plugin.cfg")),
            "Roundtrip Plugin".to_string(),
            "3.2.1-alpha".to_string(),
            "GPL-3.0".to_string(),
            vec!["subX".to_string()],
        );
        let serialized = toml::to_string(&original).unwrap();
        let deserialized: Plugin = toml::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
        assert_eq!(deserialized.version, "3.2.1-alpha");
        assert_eq!(deserialized.sub_assets, vec!["subX".to_string()]);
        // plugin_cfg_path is None by default
    }
}
