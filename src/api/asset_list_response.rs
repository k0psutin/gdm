#[derive(serde::Deserialize)]
pub struct AssetListResponse {
    result: Vec<AssetListItem>,
}

impl AssetListResponse {
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

        println!();
        for asset in self.get_results() {
            asset.print_info();
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct AssetListItem {
    asset_id: String,
    title: String,
    author: String,
    author_id: String,
    category: String,
    category_id: String,
    godot_version: String,
    rating: String,
    cost: String,
    support_level: String,
    version: String,
    version_string: String,
    modify_date: String,
}

impl AssetListItem {
    pub fn get_title(&self) -> &str {
        &self.title
    }

    pub fn get_asset_id(&self) -> &str {
        &self.asset_id
    }

    pub fn print_info(&self) {
        println!("Title:        {}", self.title);
        println!("Author:       {} (ID: {})", self.author, self.author_id);
        println!("Category:     {} (ID: {})", self.category, self.category_id);
        println!("Godot Ver.:   {}", self.godot_version);
        println!(
            "Version:      {} (Internal: {})",
            self.version_string, self.version
        );
        println!("License:      {}", self.cost);
        println!("Rating:       {}", self.rating);
        println!("Support:      {}", self.support_level);
        println!("Asset ID:     {}", self.asset_id);
        println!("Last Updated: {}", self.modify_date);
        println!(
            "Asset URL:    https://godotengine.org/asset-library/asset/{}",
            self.asset_id
        );
        println!();
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
            author_id: "456".to_string(),
            category: "Test Category".to_string(),
            category_id: "789".to_string(),
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
}