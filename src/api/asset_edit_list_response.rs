use serde::Deserialize;

#[cfg(not(tarpaulin_include))]
#[derive(Debug, Deserialize)]
pub struct AssetEditListResponse {
    pub result: Vec<AssetEditListItem>,
    pub pages: usize,
}

#[cfg(not(tarpaulin_include))]
#[derive(Debug, Deserialize)]
pub struct AssetEditListItem {
    pub edit_id: String,
    pub asset_id: String,
    pub version_string: String,
}

#[cfg(not(tarpaulin_include))]
impl AssetEditListItem {
    #[allow(unused)]
    pub fn new(edit_id: String, asset_id: String, version_string: String) -> AssetEditListItem {
        AssetEditListItem {
            edit_id,
            asset_id,
            version_string,
        }
    }
}
