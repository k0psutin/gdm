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
use crate::file_service::{DefaultFileService, FileService};
use crate::http_client::{DefaultHttpClient, HttpClient};

use anyhow::{Result, anyhow};
use asset_list_response::AssetListResponse;
use asset_response::AssetResponse;
use indicatif::ProgressBar;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use url::Url;

pub struct DefaultAssetStoreAPI {
    http_client: Arc<dyn HttpClient + Send + Sync>,
    app_config: DefaultAppConfig,
    extract_service: Box<dyn ExtractService + Send + Sync + 'static>,
    file_service: Arc<dyn FileService + Send + Sync + 'static>,
}

impl DefaultAssetStoreAPI {
    #[allow(dead_code)]
    pub fn new(
        http_client: Arc<dyn HttpClient + Send + Sync>,
        app_config: DefaultAppConfig,
        extract_service: Box<dyn ExtractService + Send + Sync + 'static>,
        file_service: Arc<dyn FileService + Send + Sync + 'static>,
    ) -> DefaultAssetStoreAPI {
        DefaultAssetStoreAPI {
            http_client,
            app_config,
            extract_service,
            file_service,
        }
    }
}

impl Default for DefaultAssetStoreAPI {
    fn default() -> Self {
        DefaultAssetStoreAPI {
            http_client: Arc::new(DefaultHttpClient::default()),
            app_config: DefaultAppConfig::default(),
            extract_service: Box::new(DefaultExtractService::default()),
            file_service: Arc::new(DefaultFileService),
        }
    }
}

#[async_trait::async_trait]
pub trait AssetStoreAPI: Send + Sync {
    fn get_extract_service(&self) -> &dyn ExtractService;
    fn get_http_client(&self) -> Arc<dyn HttpClient + Send + Sync>;
    fn get_file_service(&self) -> Arc<dyn FileService + Send + Sync + 'static>;
    fn get_base_url(&self) -> String;
    fn get_cache_folder_path(&self) -> &Path;

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

    async fn download_asset(&self, asset: &AssetResponse, pb_task: ProgressBar) -> Result<Asset>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
impl AssetStoreAPI for DefaultAssetStoreAPI {
    fn get_file_service(&self) -> Arc<dyn FileService + Send + Sync + 'static> {
        Arc::clone(&self.file_service)
    }

    fn get_extract_service(&self) -> &dyn ExtractService {
        &*self.extract_service
    }

    fn get_base_url(&self) -> String {
        self.app_config.get_api_base_url()
    }

    fn get_cache_folder_path(&self) -> &Path {
        self.app_config.get_cache_folder_path()
    }

    fn get_http_client(&self) -> Arc<dyn HttpClient + Send + Sync> {
        Arc::clone(&self.http_client)
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
            Ok(data) => Ok(serde_json::from_value(data)?),
            Err(e) => Err(anyhow!("Failed to get asset by ID: {}", e)),
        }
    }

    async fn get_assets(&self, params: HashMap<String, String>) -> Result<AssetListResponse> {
        match self
            .get_http_client()
            .get(self.get_base_url(), String::from("/asset"), params)
            .await
        {
            Ok(data) => Ok(serde_json::from_value(data)?),
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
            Ok(data) => Ok(serde_json::from_value(data)?),
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
            Ok(data) => Ok(serde_json::from_value(data)?),
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
    async fn download_asset(&self, asset: &AssetResponse, pb_task: ProgressBar) -> Result<Asset> {
        let cache_folder = self.get_cache_folder_path();
        let download_url = asset.get_download_url();
        let file_service = self.get_file_service();

        let url = Url::parse(&download_url)?;

        let filename = url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .unwrap_or("temp_file.zip");
        let filepath = cache_folder.join(filename);

        if !file_service.directory_exists(cache_folder) {
            file_service.create_directory(cache_folder)?;
        }

        if file_service.file_exists(&filepath) {
            file_service.remove_file(&filepath)?;
        }

        let mut res = self.download_file(download_url).await?;

        pb_task.set_length(100);

        let mut file = file_service.create_file_async(&filepath).await?;

        while let Some(chunk) = res.chunk().await? {
            pb_task.inc(chunk.len() as u64);
            file_service.write_all_async(&mut file, &chunk).await?;
        }

        pb_task.finish_and_clear();

        match res.error_for_status() {
            Ok(_) => {
                let root_folder = self
                    .get_extract_service()
                    .get_root_dir_from_archive(&filepath)?;
                Ok(Asset::new(
                    root_folder.display().to_string(),
                    filepath,
                    asset.clone(),
                ))
            }
            Err(e) => Err(anyhow!("Failed to fetch file: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        extract_service::MockDefaultExtractService, file_service::MockDefaultFileService,
        http_client::MockDefaultHttpClient,
    };

    use super::*;
    use mockall::predicate::*;

    fn setup_test_api() -> DefaultAssetStoreAPI {
        DefaultAssetStoreAPI::default()
    }

    // get_asset_by_id

    #[tokio::test]
    async fn test_get_asset_by_id() {
        let api = setup_test_api();
        let asset_id = String::from("1709");
        let result = api.get_asset_by_id(asset_id.clone()).await;
        assert!(result.is_ok());
        let asset = result.unwrap();
        assert_eq!(asset.get_asset_id(), asset_id);
    }

    // get_assets

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

    // get_asset_edits_by_asset_id

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

    // get_asset_edit_by_edit_id

    #[tokio::test]
    async fn test_get_asset_edit_by_edit_id_should_return_asset_edit() {
        let api = setup_test_api();
        let edit_id = "18531".to_string();
        let result = api.get_asset_edit_by_edit_id(edit_id).await;
        assert!(result.is_ok());
        let edit = result.unwrap();
        assert_eq!(edit.get_asset_id(), "1709");
    }

    // get_asset_by_id_and_version

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

    // download_file

    #[tokio::test]
    async fn test_download_file_should_return_error() {
        let api = setup_test_api();
        let download_url = String::from("some_uri");
        let result = api.download_file(download_url).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_download_file_should_return_response() {
        let api = setup_test_api();
        let download_url = String::from("https://httpbin.io/bytes/1024");
        let result = api.download_file(download_url).await;
        assert!(result.is_ok());
    }

    // download_asset
    #[tokio::test]
    async fn test_download_asset_should_download_to_cache_folder() {
        let mut mock_http_client = MockDefaultHttpClient::new();
        mock_http_client.expect_get_file().returning(|_url| {
            let http_response = http::Response::builder().status(200).body("ok").unwrap();
            let something = reqwest::Response::from(http_response);
            Ok(something)
        });

        let mut mock_file_service = MockDefaultFileService::new();
        let mut mock_extract_service = MockDefaultExtractService::new();

        mock_file_service
            .expect_directory_exists()
            .with(eq(Path::new("test/mocks/cache")))
            .returning(|_path| true);

        mock_file_service
            .expect_file_exists()
            .with(eq(Path::new("test/mocks/cache/asset.zip")))
            .returning(|_path| false);

        mock_file_service
            .expect_create_file_async()
            .with(eq(Path::new("test/mocks/cache\\asset.zip")))
            .returning(|_path| {
                // Create a temp file and open it as tokio::fs::File
                std::fs::create_dir_all("test/mocks/cache").unwrap();
                let file = std::fs::File::create("test/mocks/cache/asset.zip").unwrap();
                Ok(tokio::fs::File::from_std(file))
            });
        mock_file_service
            .expect_write_all_async()
            .returning(|_file, _chunk| Ok(()));

        mock_extract_service
            .expect_get_root_dir_from_archive()
            .with(eq(Path::new("test/mocks/cache/asset.zip")))
            .returning(|_path| Ok(Path::new("test/mocks/addons/asset").to_path_buf()));

        let api = DefaultAssetStoreAPI::new(
            Arc::new(mock_http_client),
            DefaultAppConfig::new(
                Some(String::from("http://mock")),
                Some(String::from("test/mocks/gdm.json")),
                Some(String::from("test/mocks/cache")),
                Some(String::from(
                    "test/mocks/project_with_plugins_and_version.godot",
                )),
                Some(String::from("test/mocks/addons")),
            ),
            Box::new(mock_extract_service),
            Arc::new(mock_file_service),
        );

        let mock_asset = AssetResponse::new(
            "1234".to_string(),
            "Mock Asset".to_string(),
            "11".to_string(),
            "1.1.1".to_string(),
            "4.5".to_string(),
            "5.0".to_string(),
            "MIT".to_string(),
            "Some description.".to_string(),
            "github".to_string(),
            "commit_hash".to_string(),
            "2023-10-01".to_string(),
            "https://some-url-with.com/asset.zip".to_string(),
        );

        let pb_task = ProgressBar::no_length();
        let result = api.download_asset(&mock_asset, pb_task).await;
        assert!(result.is_ok());
        std::fs::remove_dir_all("test/mocks/cache").unwrap();
    }
}
