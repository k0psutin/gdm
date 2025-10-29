use crate::api::asset_edit_response::AssetEditResponse;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
pub struct AssetResponse {
    pub asset_id: String,
    pub title: String,
    pub version: String,
    pub version_string: String,
    pub godot_version: String,
    pub rating: String,
    pub cost: String,
    pub description: String,
    pub download_provider: String,
    pub download_commit: String,
    pub modify_date: String,
    pub download_url: String,
}

impl From<AssetEditResponse> for AssetResponse {
    fn from(edit: AssetEditResponse) -> Self {
        let asset = edit.original;
        AssetResponse {
            asset_id: asset.asset_id.clone(),
            title: asset.title.clone(),
            version: asset.version.clone(),
            version_string: edit.version_string.clone(),
            godot_version: asset.godot_version.clone(),
            rating: asset.rating.clone(),
            cost: asset.cost.clone(),
            description: asset.description.clone(),
            download_provider: asset.download_provider.clone(),
            download_commit: edit.download_commit.unwrap_or_default().to_string(),
            modify_date: asset.modify_date.clone(),
            download_url: edit.download_url.to_string(),
        }
    }
}

impl AssetResponse {
    #[allow(unused, clippy::too_many_arguments)]
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
        assert_eq!(asset.asset_id, "456");
        assert_eq!(asset.title, "Test Asset");
        assert_eq!(asset.version_string, "0.0.1");
        assert_eq!(asset.download_url, "https://example.com/old.zip");
        assert_eq!(asset.download_commit, "commit_hash");
    }
}
