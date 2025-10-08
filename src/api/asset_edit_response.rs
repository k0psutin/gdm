use serde::{Deserialize, Serialize};

use crate::api::asset_response::AssetResponse;

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetEditResponse {
    edit_id: String,
    asset_id: String,
    godot_version: Option<String>,
    version_string: String,
    download_commit: Option<String>,
    status: String,
    reason: String,
    author: String,
    original: OriginalAsset,
    download_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OriginalAsset {
    asset_id: String,
    title: String,
    version: String,
    version_string: String,
    category_id: String,
    godot_version: String,
    rating: String,
    cost: String,
    description: String,
    download_provider: String,
    download_commit: String,
    modify_date: String,
    download_url: String,
}

impl AssetEditResponse {
    pub fn get_asset_id(&self) -> &str {
        &self.asset_id
    }

    pub fn get_version_string(&self) -> &str {
        &self.version_string
    }

    pub fn get_download_url(&self) -> &str {
        &self.download_url
    }

    pub fn get_download_commit(&self) -> Option<String> {
        self.download_commit.clone()
    }
}