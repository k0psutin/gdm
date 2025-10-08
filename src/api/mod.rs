pub mod asset_response;
pub mod asset_list_response;
pub mod asset_edit_list_response;
pub mod asset_edit_response;

use std::collections::HashMap;
use asset_response::AssetResponse;
use asset_list_response::AssetListResponse;

use crate::api::asset_edit_list_response::AssetEditListResponse;
use crate::api::asset_edit_response::AssetEditResponse;
use crate::app_config::AppConfig;
use crate::http_client;

pub struct AssetStoreAPI {
    api_base_url: String,
}

impl AssetStoreAPI {
    pub fn new() -> AssetStoreAPI {
        AssetStoreAPI {
            api_base_url: AppConfig::new().get_api_base_url().to_string(),
        }
    }
    
    fn default(api_base_url: String) -> AssetStoreAPI {
        AssetStoreAPI { api_base_url }
    }

    pub async fn fetch_asset_by_id(&self, asset_id: &str) -> anyhow::Result<AssetResponse> {
        match http_client::get( format!("{}/asset/{}", self.api_base_url, asset_id), [].into()).await {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow::anyhow!("Failed to fetch asset by ID: {}", e)),
        }
    }

    pub async fn search_assets(&self, params: HashMap<&str, &str>) -> anyhow::Result<AssetListResponse> {
        match http_client::get( format!("{}/asset", self.api_base_url), params).await {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow::anyhow!("Failed to fetch assets: {}", e)),
        }
    }

    pub async fn search_asset_by_id_and_version(&self, asset_id: &str, version: &str) -> anyhow::Result<AssetEditResponse> {
        let mut page = 0 as u8;
        loop {
            let edits_response = self.fetch_asset_edits_by_asset_id(asset_id, page).await?;
            for edit in edits_response.get_results().iter() {
                if edit.get_version_string() == version {
                    let edit_result = self.fetch_asset_edit_by_edit_id(edit.get_edit_id()).await?;
                    return Ok(edit_result);
                }
            }
            if page == edits_response.get_pages() - 1 {
                break; 
            }
            page += 1;
        }
        Err(anyhow::anyhow!("No asset found for asset_id: {} with version: {}", asset_id, version))
    }

    pub async fn fetch_asset_edits_by_asset_id(&self, asset_id: &str, page: u8) -> anyhow::Result<AssetEditListResponse> {
        match http_client::get( format!("{}/asset/edit", self.api_base_url), [("asset", asset_id), ("status", "new accepted"), ("page", &page.to_string())].into()).await {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow::anyhow!("Failed to fetch assets: {}", e)),
        }
    }

    pub async fn fetch_asset_edit_by_edit_id(&self, edit_id: &str) -> anyhow::Result<AssetEditResponse> {
        match http_client::get( format!("{}/asset/edit/{}", self.api_base_url, edit_id), [].into()).await {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow::anyhow!("Failed to fetch assets: {}", e)),
        }
    }

    pub async fn download_asset(&self, download_url: &str) -> anyhow::Result<reqwest::Response> {
        match http_client::get_file(download_url.to_string()).await {
            Ok(response) => Ok(response),
            Err(e) => Err(anyhow::anyhow!("Failed to download asset: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_api() -> AssetStoreAPI {
        AssetStoreAPI::default("https://godotengine.org/asset-library/api".to_string())
    }

    #[tokio::test]
    async fn test_fetch_asset_by_id() {
        let api = setup_test_api();
        let asset_id = "1709";
        let result = api.fetch_asset_by_id(asset_id).await;
        assert!(result.is_ok());
        let asset = result.unwrap();
        assert_eq!(asset.get_asset_id(), asset_id);
    }

    #[tokio::test]
    async fn test_search_assets_should_return_empty_list() {
        let api = setup_test_api();
        let params = HashMap::from([("filter", "some_filter"), ("godot_version", "4.5")]);
        let result = api.search_assets(params).await;
        assert!(result.is_ok());
        let asset_list = result.unwrap();
        assert!(asset_list.get_results().is_empty());
    }

    #[tokio::test]
    async fn test_search_assets_should_return_asset_list() {
        let api = setup_test_api();
        let params = HashMap::from([("filter", "Godot Unit Testing"), ("godot_version", "4.5")]);
        let result = api.search_assets(params).await;
        assert!(result.is_ok());
        let asset_list = result.unwrap();
        assert!(!asset_list.get_results().is_empty());
        assert_eq!(asset_list.get_result_len(), 1);
        let asset = asset_list.get_asset_list_item_by_index(0).unwrap();
        assert_eq!(asset.get_asset_id(), "1709");
    }

    #[tokio::test]
    async fn test_fetch_asset_edits_by_asset_id_should_return_asset_edit_list_when_page_is_zero() {
        let api = setup_test_api();
        let asset_id = "1709";
        let result = api.fetch_asset_edits_by_asset_id(asset_id, 0).await;
        assert!(result.is_ok());
        let edit_list = result.unwrap();
        assert!(!edit_list.get_results().is_empty());
        let edit_list_item = edit_list.get_asset_edit_list_item_by_index(0).unwrap();
        assert_eq!(edit_list_item.get_asset_id(), asset_id);
    }

    #[tokio::test]
    async fn test_fetch_asset_edit_by_edit_id_should_return_asset_edit() {
        let api = setup_test_api();
        let edit_id = "18531";
        let result = api.fetch_asset_edit_by_edit_id(edit_id).await;
        assert!(result.is_ok());
        let edit = result.unwrap();
        assert_eq!(edit.get_asset_id(), "1709");
    }

    #[tokio::test]
    async fn test_search_asset_by_id_and_version_should_return_newer_version() {
        let api = setup_test_api();
        let edit_id = "1709";
        let version = "9.5.0";
        let result = api.search_asset_by_id_and_version(edit_id, version).await;
        assert!(result.is_ok());
        let edit = result.unwrap();
        assert_eq!(edit.get_asset_id(), "1709");
        assert_eq!(edit.get_version_string(), version);
    }

    #[tokio::test]
    async fn test_search_asset_by_id_and_version_should_return_older_version() {
        let api = setup_test_api();
        let edit_id = "1709";
        let version = "9.4.0";
        let result = api.search_asset_by_id_and_version(edit_id, version).await;
        assert!(result.is_ok());
        let edit = result.unwrap();
        println!("{:?}", edit);
        assert_eq!(edit.get_asset_id(), "1709");
        assert_eq!(edit.get_version_string(), version);
    }
}
