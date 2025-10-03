#[derive(Debug, serde::Deserialize)]
pub struct AssetResponse {
    asset_id: String,
    r#type: String,
    title: String,
    author: String,
    author_id: String,
    version: String,
    version_string: String,
    category: String,
    category_id: String,
    godot_version: String,
    rating: String,
    cost: String,
    description: String,
    support_level: String,
    download_provider: String,
    download_commit: String,
    browse_url: String,
    issues_url: String,
    icon_url: String,
    searchable: String,
    modify_date: String,
    download_url: String,
    previews: Vec<serde_json::Value>,
    download_hash: String,
}

impl AssetResponse {
    pub fn get_download_url(&self) -> &str {
        &self.download_url
    }
}