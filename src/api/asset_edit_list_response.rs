use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AssetEditListResponse {
    pub result: Vec<AssetEditListItem>,
    pub pages: usize,
}

#[derive(Debug, Deserialize)]
pub struct AssetEditListItem {
    pub edit_id: String,
    pub asset_id: String,
    pub version_string: String,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_edit_list_item_new() {
        let edit_id = String::from("edit_1");
        let asset_id = String::from("asset_1");
        let version_string = String::from("1.0.0");

        let item = AssetEditListItem::new(edit_id, asset_id, version_string);

        assert_eq!(item.edit_id, "edit_1");
        assert_eq!(item.asset_id, "asset_1");
        assert_eq!(item.version_string, "1.0.0");
    }
}
