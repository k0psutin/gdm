use serde::{Deserialize, Serialize};

use crate::api::asset_response::AssetResponse;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetEditResponse {
    edit_id: String,
    asset_id: String,
    godot_version: Option<String>,
    version_string: String,
    download_commit: Option<String>,
    status: String,
    reason: String,
    author: String,
    download_url: String,
    original: AssetResponse,
}

impl AssetEditResponse {
    pub fn new(
        edit_id: String,
        asset_id: String,
        godot_version: Option<String>,
        version_string: String,
        download_commit: Option<String>,
        status: String,
        reason: String,
        author: String,
        download_url: String,
        original: AssetResponse,
    ) -> AssetEditResponse {
        AssetEditResponse {
            edit_id,
            asset_id,
            godot_version,
            version_string,
            download_commit,
            status,
            reason,
            author,
            original,
            download_url,
        }
    }
    pub fn get_original(&self) -> &AssetResponse {
        &self.original
    }

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