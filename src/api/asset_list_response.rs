use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(serde::Deserialize, Debug)]
pub struct AssetListResponse {
    result: Vec<AssetListItem>,
}

impl AssetListResponse {
    pub fn new(result: Vec<AssetListItem>) -> AssetListResponse {
        AssetListResponse { result }
    }

    pub fn get_result_len(&self) -> usize {
        self.result.len()
    }

    pub fn get_results(&self) -> &Vec<AssetListItem> {
        &self.result
    }

    pub fn get_asset_list_item_by_index(&self, index: usize) -> Option<&AssetListItem> {
        self.result.get(index)
    }

    pub fn print_info(&self) {
        if self.get_results().is_empty() {
            return;
        }

        for asset in self.get_results() {
            println!();
            println!("{}", asset);
            println!();
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct AssetListItem {
    asset_id: String,
    title: String,
    author: String,
    category: String,
    godot_version: String,
    rating: String,
    cost: String,
    support_level: String,
    version: String,
    version_string: String,
    modify_date: String,
}

impl Display for AssetListItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
"Asset ID: {}
Title: {}
Author: {}
Category: {}
Godot Ver.: {}
Version: {} ({})
License: {}
Rating: {}
Support: {}
Last Updated: {}
Asset URL: https://godotengine.org/asset-library/asset/{}",
            self.asset_id,
            self.title,
            self.author,
            self.category,
            self.godot_version,
            self.version_string,
            self.version,
            self.cost,
            self.rating,
            self.support_level,
            self.modify_date,
            self.asset_id
        )
    }
}

impl AssetListItem {
    pub fn get_title(&self) -> &str {
        &self.title
    }

    pub fn get_asset_id(&self) -> &str {
        &self.asset_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_asset_list_item() -> AssetListItem {
        AssetListItem {
            asset_id: "123".to_string(),
            title: "Test Asset".to_string(),
            author: "Test Author".to_string(),
            category: "Test Category".to_string(),
            godot_version: "3.3".to_string(),
            rating: "5".to_string(),
            cost: "Free".to_string(),
            support_level: "Community".to_string(),
            version: "1.0".to_string(),
            version_string: "1.0".to_string(),
            modify_date: "2023-01-01".to_string(),
        }
    }

    #[test]
    fn test_should_return_asset_id() {
        let asset = setup_asset_list_item();
        assert_eq!(asset.get_asset_id(), "123");
    }

    #[test]
    fn test_should_return_title() {
        let asset = setup_asset_list_item();
        assert_eq!(asset.get_title(), "Test Asset");
    }

    #[test]
    fn test_asset_list_item_display() {
        let asset = setup_asset_list_item();
        let display_output = format!("{}", asset);
        let expected = "\
    Asset ID: 123
    Title: Test Asset
    Author: Test Author
    Category: Test Category
    Godot Ver.: 3.3
    Version: 1.0 (1.0)
    License: Free
    Rating: 5
    Support: Community
    Last Updated: 2023-01-01
    Asset URL: https://godotengine.org/asset-library/asset/123";
        assert_eq!(display_output, expected);
    }
}
