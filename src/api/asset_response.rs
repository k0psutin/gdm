use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
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

impl AssetResponse {
    pub fn get_download_url(&self) -> &str {
        &self.download_url
    }

    pub fn get_title(&self) -> &str {
        &self.title
    }

    pub fn get_asset_id(&self) -> &str {
        &self.asset_id
    }

    pub fn get_version(&self) -> &str {
        &self.version_string
    }

    pub fn get_download_commit(&self) -> &str {
        &self.download_commit
    }
}
