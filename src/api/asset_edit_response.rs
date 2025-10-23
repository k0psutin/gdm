use serde::{Deserialize, Serialize};

use crate::api::asset_response::AssetResponse;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetEditResponse {
    pub edit_id: String,
    pub asset_id: String,
    pub godot_version: Option<String>,
    pub version_string: String,
    pub download_commit: Option<String>,
    pub status: String,
    pub author: String,
    pub download_url: String,
    pub original: AssetResponse,
}


impl AssetEditResponse {
    #[allow(unused, clippy::too_many_arguments)]
    pub fn new(
        edit_id: String,
        asset_id: String,
        godot_version: Option<String>,
        version_string: String,
        download_commit: Option<String>,
        status: String,
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
            author,
            original,
            download_url,
        }
    }
}
