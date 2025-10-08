use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AssetEditListResponse {
    result: Vec<AssetEditListItem>,
    page: u8,
    pages: u8,
}

impl AssetEditListResponse {
    pub fn get_page(&self) -> u8 {
        self.page
    }
    
    pub fn get_pages(&self) -> u8 {
        self.pages
    }

    pub fn get_result_len(&self) -> usize {
        self.result.len()
    }

    pub fn get_asset_edit_list_item_by_index(&self, index: usize) -> Option<&AssetEditListItem> {
        self.result.get(index)
    }

    pub fn get_results(&self) -> &Vec<AssetEditListItem> {
        &self.result
    }
}

#[derive(Debug, Deserialize)]
pub struct AssetEditListItem {
    edit_id: String,
    asset_id: String,
    user_id: String,
    submit_date: String,
    modify_date: String,
    title: String,
    description: String,
    godot_version: String,
    version_string: String,
    cost: String,
    browse_url: String,
    icon_url: String,
    category: Option<String>,
    support_level: String,
    status: String,
    reason: String,
    author: String,
}

impl AssetEditListItem {
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