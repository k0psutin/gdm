use crate::api::asset_edit_response::AssetEditResponse;

use serde::{Deserialize, Serialize};

#[cfg(not(tarpaulin_include))]
#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
pub struct AssetResponse {
    asset_id: String,
    title: String,
    version: String,
    version_string: String,
    godot_version: String,
    rating: String,
    cost: String,
    description: String,
    download_provider: String,
    download_commit: String,
    modify_date: String,
    download_url: String,
}

impl From<AssetEditResponse> for AssetResponse {
    fn from(edit: AssetEditResponse) -> Self {
        let asset = edit.get_original();
        AssetResponse {
            asset_id: asset.asset_id.clone(),
            title: asset.title.clone(),
            version: asset.version.clone(),
            version_string: edit.get_version_string().to_string(),
            godot_version: asset.godot_version.clone(),
            rating: asset.rating.clone(),
            cost: asset.cost.clone(),
            description: asset.description.clone(),
            download_provider: asset.download_provider.clone(),
            download_commit: edit.get_download_commit().unwrap_or_default().to_string(),
            modify_date: asset.modify_date.clone(),
            download_url: edit.get_download_url().to_string(),
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl AssetResponse {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        asset_id: String,
        title: String,
        version: String,
        version_string: String,
        godot_version: String,
        rating: String,
        cost: String,
        description: String,
        download_provider: String,
        download_commit: String,
        modify_date: String,
        download_url: String,
    ) -> AssetResponse {
        AssetResponse {
            asset_id,
            title,
            version,
            version_string,
            godot_version,
            rating,
            cost,
            description,
            download_provider,
            download_commit,
            modify_date,
            download_url,
        }
    }

    pub fn get_download_url(&self) -> String {
        self.download_url.clone()
    }

    pub fn get_title(&self) -> String {
        self.title.clone()
    }

    pub fn get_asset_id(&self) -> String {
        self.asset_id.clone()
    }

    pub fn get_version_string(&self) -> String {
        self.version_string.clone()
    }

    pub fn get_download_commit(&self) -> String {
        self.download_commit.clone()
    }

    pub fn get_license(&self) -> String {
        self.cost.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::api::asset_edit_response::AssetEditResponse;

    fn setup_test_asset_response() -> AssetResponse {
        let edit = AssetEditResponse::new(
            "123".to_string(),
            "456".to_string(),
            Some("4.0".to_string()),
            "0.0.1".to_string(),
            Some("commit_hash".to_string()),
            "".to_string(),
            "author_name".to_string(),
            "https://example.com/old.zip".to_string(),
            AssetResponse {
                asset_id: "456".to_string(),
                title: "Test Asset".to_string(),
                version: "11".to_string(),
                version_string: "1.0.0".to_string(),
                godot_version: "4.0".to_string(),
                rating: "5".to_string(),
                cost: "Free".to_string(),
                description: "A test asset".to_string(),
                download_provider: "github".to_string(),
                download_commit: "commit_hash".to_string(),
                modify_date: "2023-10-01".to_string(),
                download_url: "https://example.com/new.zip".to_string(),
            },
        );
        AssetResponse::from(edit)
    }

    #[test]
    fn test_asset_response_from_asset_edit_response() {
        let asset = setup_test_asset_response();
        assert_eq!(asset.get_asset_id(), "456");
        assert_eq!(asset.get_title(), "Test Asset");
        assert_eq!(asset.get_version_string(), "0.0.1");
        assert_eq!(asset.get_download_url(), "https://example.com/old.zip");
        assert_eq!(asset.get_download_commit(), "commit_hash");
    }
}
