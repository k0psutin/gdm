use serde::{Deserialize};

#[derive(Debug, Deserialize)]
pub struct AssetEditListResponse {
    result: Vec<AssetEditListItem>,
    pages: usize,
}

impl AssetEditListResponse {
    pub fn get_pages(&self) -> usize {
        self.pages
    }

    pub fn get_results(&self) -> &Vec<AssetEditListItem> {
        &self.result
    }
}

#[derive(Debug, Deserialize)]
pub struct AssetEditListItem {
    edit_id: String,
    asset_id: String,
    version_string: String,
}

impl AssetEditListItem {
    pub fn new(
        edit_id: String,
        asset_id: String,
        version_string: String,
    ) -> AssetEditListItem {
        AssetEditListItem {
            edit_id,
            asset_id,
            version_string,
        }
    }

    pub fn get_edit_id(&self) -> &str {
        &self.edit_id
    }

    pub fn get_asset_id(&self) -> &str {
        &self.asset_id
    }

    pub fn get_version_string(&self) -> &str {
        &self.version_string
    }
}