use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(serde::Deserialize, Debug)]
pub struct AssetListResponse {
    pub result: Vec<AssetListItem>,
}

impl AssetListResponse {
    #[allow(unused)]
    pub fn new(result: Vec<AssetListItem>) -> AssetListResponse {
        AssetListResponse { result }
    }

    pub fn print_info(&self) {
        if self.result.is_empty() {
            return;
        }

        for asset in &self.result {
            println!();
            println!("{}", asset);
            println!();
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct AssetListItem {
    pub asset_id: String,
    pub title: String,
    pub author: String,
    pub category: String,
    pub godot_version: String,
    pub rating: String,
    pub cost: String,
    pub support_level: String,
    pub version: String,
    pub version_string: String,
    pub modify_date: String,
}

impl AssetListItem {
    #[allow(unused, clippy::too_many_arguments)]
    pub fn new(
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
    ) -> AssetListItem {
        AssetListItem {
            asset_id,
            title,
            author,
            category,
            godot_version,
            rating,
            cost,
            support_level,
            version,
            version_string,
            modify_date,
        }
    }
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
        assert_eq!(asset.asset_id, "123");
    }

    #[test]
    fn test_should_return_title() {
        let asset = setup_asset_list_item();
        assert_eq!(asset.title, "Test Asset");
    }

    #[test]
    fn test_asset_list_item_display() {
        let asset = setup_asset_list_item();
        let display_output = format!("{}", asset);
        let expected = "Asset ID: 123
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
