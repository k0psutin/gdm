pub mod asset;
pub mod asset_edit_list_response;
pub mod asset_edit_response;
pub mod asset_list_response;
pub mod asset_response;

use crate::api::asset::Asset;
use crate::api::asset_edit_list_response::AssetEditListResponse;
use crate::api::asset_edit_response::AssetEditResponse;
use crate::app_config::{AppConfig, DefaultAppConfig};
use crate::extract_service::{DefaultExtractService, ExtractService};
use crate::http_client::{DefaultHttpClient, HttpClient};

use anyhow::{Result, anyhow};
use asset_list_response::AssetListResponse;
use asset_response::AssetResponse;
use indicatif::ProgressBar;
use std::collections::HashMap;
use tokio::{fs, io};
use url::Url;

#[derive(Debug, Clone, Default)]
pub struct DefaultAssetStoreAPI {
    http_client: DefaultHttpClient,
    app_config: DefaultAppConfig,
    extract_service: DefaultExtractService,
}

impl DefaultAssetStoreAPI {
    #[allow(dead_code)]
    pub fn new(
        http_client: DefaultHttpClient,
        app_config: DefaultAppConfig,
        extract_service: DefaultExtractService,
    ) -> DefaultAssetStoreAPI {
        DefaultAssetStoreAPI {
            http_client,
            app_config,
            extract_service,
        }
    }
}

#[async_trait::async_trait]
pub trait AssetStoreAPI: Send + Sync {
    fn get_extract_service(&self) -> &DefaultExtractService;
    fn get_http_client(&self) -> &DefaultHttpClient;
    fn get_base_url(&self) -> String;

    async fn get_asset_by_id(&self, asset_id: String) -> Result<AssetResponse>;

    async fn get_assets(&self, params: HashMap<String, String>) -> Result<AssetListResponse>;

    async fn get_asset_by_id_and_version(
        &self,
        asset_id: String,
        version: String,
    ) -> Result<AssetResponse>;

    async fn get_asset_edits_by_asset_id(
        &self,
        asset_id: String,
        page: usize,
    ) -> Result<AssetEditListResponse>;

    async fn get_asset_edit_by_edit_id(&self, edit_id: String) -> Result<AssetEditResponse>;

    async fn download_file(&self, download_url: String) -> Result<reqwest::Response>;

    async fn download_asset(
        &self,
        asset: &AssetResponse,
        pb_task: ProgressBar,
        cache_folder: String,
    ) -> Result<Asset>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
impl AssetStoreAPI for DefaultAssetStoreAPI {
    fn get_extract_service(&self) -> &DefaultExtractService {
        &self.extract_service
    }

    fn get_base_url(&self) -> String {
        self.app_config.get_api_base_url()
    }

    fn get_http_client(&self) -> &DefaultHttpClient {
        &self.http_client
    }

    async fn get_asset_by_id(&self, asset_id: String) -> Result<AssetResponse> {
        match self
            .get_http_client()
            .get(
                self.get_base_url(),
                format!("/asset/{}", asset_id),
                [].into(),
            )
            .await
        {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow!("Failed to get asset by ID: {}", e)),
        }
    }

    async fn get_assets(&self, params: HashMap<String, String>) -> Result<AssetListResponse> {
        match self
            .get_http_client()
            .get(self.get_base_url(), String::from("/asset"), params)
            .await
        {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow!("Failed to get assets: {}", e)),
        }
    }

    async fn get_asset_by_id_and_version(
        &self,
        asset_id: String,
        version: String,
    ) -> Result<AssetResponse> {
        let mut page = 0;
        loop {
            let edits_response = self
                .get_asset_edits_by_asset_id(asset_id.clone(), page)
                .await?;
            for edit in edits_response.get_results().iter() {
                if edit.get_version_string() == version {
                    let edit_result = self
                        .get_asset_edit_by_edit_id(edit.get_edit_id().to_string())
                        .await?;
                    let asset_response = AssetResponse::from(edit_result);
                    return Ok(asset_response);
                }
            }
            if page == edits_response.get_pages() - 1 {
                break;
            }
            page += 1;
        }
        Err(anyhow!(
            "No asset found for asset_id: {} with version: {}",
            asset_id,
            version
        ))
    }

    async fn get_asset_edits_by_asset_id(
        &self,
        asset_id: String,
        page: usize,
    ) -> Result<AssetEditListResponse> {
        let params = HashMap::from([
            ("asset".to_string(), asset_id),
            ("status".to_string(), "new accepted".to_string()),
            ("page".to_string(), page.to_string()),
        ]);
        match self
            .get_http_client()
            .get(self.get_base_url(), String::from("/asset/edit"), params)
            .await
        {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow!("Failed to get assets: {}", e)),
        }
    }

    async fn get_asset_edit_by_edit_id(&self, edit_id: String) -> Result<AssetEditResponse> {
        match self
            .get_http_client()
            .get(
                self.get_base_url(),
                format!("/asset/edit/{}", edit_id),
                [].into(),
            )
            .await
        {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow!("Failed to get assets: {}", e)),
        }
    }

    async fn download_file(&self, download_url: String) -> Result<reqwest::Response> {
        match self.get_http_client().get_file(download_url).await {
            Ok(response) => Ok(response),
            Err(e) => Err(anyhow!("Failed to download file: {}", e)),
        }
    }

    /// Downloads a plugin from the Asset Store and returns a Asset struct
    ///
    /// Downloaded files are saved to the cache folder defined in the AppConfig
    async fn download_asset(
        &self,
        asset: &AssetResponse,
        pb_task: ProgressBar,
        cache_folder: String,
    ) -> Result<Asset> {
        let download_url = asset.get_download_url();

        let url = Url::parse(&download_url)?;

        let filename = url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .unwrap_or("temp_file.zip");
        let filepath = format!("{}/{}", cache_folder, filename);

        if !fs::try_exists(cache_folder.clone()).await? {
            fs::create_dir(cache_folder.clone()).await?;
        }

        if fs::try_exists(&filepath).await? {
            fs::remove_file(&filepath).await?;
        }

        let mut res = self.download_file(download_url).await?;

        pb_task.set_length(100);

        let mut file = fs::File::create(&filepath).await?;

        while let Some(chunk) = res.chunk().await? {
            pb_task.inc(chunk.len() as u64);
            let result = io::AsyncWriteExt::write_all(&mut file, &chunk).await;
            result?;
        }

        pb_task.finish_and_clear();

        match res.error_for_status() {
            Ok(_) => {
                let root_folder = self
                    .get_extract_service()
                    .get_root_dir_from_archive(&filepath)?;
                Ok(Asset::new(root_folder, filepath, asset.clone()))
            }
            Err(e) => Err(anyhow!("Failed to fetch file: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_api() -> DefaultAssetStoreAPI {
        DefaultAssetStoreAPI::default()
    }

    #[tokio::test]
    async fn test_get_asset_by_id() {
        let api = setup_test_api();
        let asset_id = String::from("1709");
        let result = api.get_asset_by_id(asset_id.clone()).await;
        assert!(result.is_ok());
        let asset = result.unwrap();
        assert_eq!(asset.get_asset_id(), asset_id);
    }

    #[tokio::test]
    async fn test_search_assets_should_return_empty_list() {
        let api = setup_test_api();
        let params = HashMap::from([
            ("filter".to_string(), "some_filter".to_string()),
            ("godot_version".to_string(), "4.5".to_string()),
        ]);
        let result = api.get_assets(params).await;
        assert!(result.is_ok());
        let asset_list = result.unwrap();
        assert!(asset_list.get_results().is_empty());
    }

    #[tokio::test]
    async fn test_search_assets_should_return_asset_list() {
        let api = setup_test_api();
        let params = HashMap::from([
            ("filter".to_string(), "Godot Unit Testing".to_string()),
            ("godot_version".to_string(), "4.5".to_string()),
        ]);
        let result = api.get_assets(params).await;
        assert!(result.is_ok());
        let asset_list = result.unwrap();
        assert!(!asset_list.get_results().is_empty());
        assert_eq!(asset_list.get_result_len(), 1);
        let asset = asset_list.get_asset_list_item_by_index(0).unwrap();
        assert_eq!(asset.get_asset_id(), "1709");
    }

    #[tokio::test]
    async fn test_get_asset_edits_by_asset_id_should_return_asset_edit_list_when_page_is_zero() {
        let api = setup_test_api();
        let asset_id = "1709".to_string();
        let result = api.get_asset_edits_by_asset_id(asset_id.clone(), 0).await;
        assert!(result.is_ok());
        let edit_list = result.unwrap();
        assert!(!edit_list.get_results().is_empty());
        let edit_list_item = edit_list.get_results().first().unwrap();
        assert_eq!(edit_list_item.get_asset_id(), asset_id);
    }

    #[tokio::test]
    async fn test_get_asset_edit_by_edit_id_should_return_asset_edit() {
        let api = setup_test_api();
        let edit_id = "18531".to_string();
        let result = api.get_asset_edit_by_edit_id(edit_id).await;
        assert!(result.is_ok());
        let edit = result.unwrap();
        assert_eq!(edit.get_asset_id(), "1709");
    }

    #[tokio::test]
    async fn test_search_asset_by_id_and_version_should_return_newer_version() {
        let api = setup_test_api();
        let edit_id = "1709".to_string();
        let version = "9.5.0".to_string();
        let result = api
            .get_asset_by_id_and_version(edit_id, version.clone())
            .await;
        assert!(result.is_ok());
        let edit = result.unwrap();
        assert_eq!(edit.get_asset_id(), "1709");
        assert_eq!(edit.get_version_string(), version);
    }

    #[tokio::test]
    async fn test_search_asset_by_id_and_version_should_return_older_version() {
        let api = setup_test_api();
        let edit_id = "1709".to_string();
        let version = "9.4.0".to_string();
        let result = api
            .get_asset_by_id_and_version(edit_id, version.clone())
            .await;
        assert!(result.is_ok());
        let edit = result.unwrap();
        assert_eq!(edit.get_asset_id(), "1709");
        assert_eq!(edit.get_version_string(), version);
    }

    #[tokio::test]
    async fn test_search_asset_by_id_and_version_should_return_err_if_no_version_found() {
        let api = setup_test_api();
        let edit_id = "1709".to_string();
        let version = "0.0.1".to_string();
        let result = api.get_asset_by_id_and_version(edit_id, version).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_download_file_should_return_response() {
        let api = setup_test_api();
        let download_url = String::from("some_uri");
        let result = api.download_file(download_url).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
    }
}
