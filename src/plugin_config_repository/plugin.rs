use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Plugin {
    asset_id: String,
    title: String,
    version: String,
    license: String,
}

impl Plugin {
    pub fn new(asset_id: String, title: String, version: String, license: String) -> Plugin {
        Plugin {
            asset_id,
            title,
            version,
            license,
        }
    }

    pub fn get_asset_id(&self) -> String {
        self.asset_id.clone()
    }

    pub fn get_title(&self) -> String {
        self.title.clone()
    }

    pub fn get_version(&self) -> String {
        self.version.clone()
    }

    #[allow(dead_code)]
    pub fn get_license(&self) -> String {
        self.license.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::api::asset_response::AssetResponse;

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
        assert_eq!(plugin.get_asset_id(), "123");
        assert_eq!(plugin.get_title(), "Sample Plugin");
        assert_eq!(plugin.get_version(), "1.0.0");
        assert_eq!(plugin.get_license(), "MIT");
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
        assert_eq!(plugin.get_asset_id(), "456");
        assert_eq!(plugin.get_title(), "Test Asset");
        assert_eq!(plugin.get_version(), "0.0.1");
        assert_eq!(plugin.get_license(), "MIT");
    }
}
