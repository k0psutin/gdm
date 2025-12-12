mod asset;
mod asset_edit_list_response;
mod asset_edit_response;
mod asset_list_response;
mod asset_response;

pub use asset::Asset;
pub use asset_edit_list_response::AssetEditListResponse;
pub use asset_edit_response::AssetEditResponse;
#[cfg(test)]
pub use asset_list_response::AssetListItem;
pub use asset_list_response::AssetListResponse;
pub use asset_response::AssetResponse;

use crate::config::{AppConfig, DefaultAppConfig};
use crate::services::{DefaultFileService, DefaultHttpService, FileService, HttpService};

use anyhow::{Result, bail};
use indicatif::ProgressBar;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tracing::{error, info};
use url::Url;

pub struct DefaultAssetStoreAPI {
    pub http_service: Arc<dyn HttpService + Send + Sync>,
    pub app_config: DefaultAppConfig,
    pub file_service: Arc<dyn FileService + Send + Sync + 'static>,
}

impl DefaultAssetStoreAPI {
    #[allow(unused)]
    pub fn new(
        http_service: Arc<dyn HttpService + Send + Sync>,
        app_config: DefaultAppConfig,
        file_service: Arc<dyn FileService + Send + Sync + 'static>,
    ) -> DefaultAssetStoreAPI {
        DefaultAssetStoreAPI {
            http_service,
            app_config,
            file_service,
        }
    }

    fn get_url(&self, path: &str) -> String {
        format!("{}{}", self.app_config.api_base_url, path)
    }
}

impl Default for DefaultAssetStoreAPI {
    fn default() -> Self {
        DefaultAssetStoreAPI {
            http_service: Arc::new(DefaultHttpService::default()),
            app_config: DefaultAppConfig::default(),
            file_service: Arc::new(DefaultFileService),
        }
    }
}

#[async_trait::async_trait]
/// Trait defining the API for interacting with an asset store.
///
/// Provides methods for accessing services, retrieving assets, downloading files,
/// and managing asset edits. Implementors must be thread-safe (`Send` + `Sync`).
///
/// # Services
/// - `get_extract_service`: Returns a reference to the extract service.
/// - `http_service`: Returns an HTTP client for making requests.
/// - `get_file_service`: Returns a file service for file operations.
/// - `get_base_url`: Returns the base URL of the asset store.
/// - `get_cache_folder_path`: Returns the path to the cache folder.
///
/// # Asset Retrieval
/// - `get_asset_by_id`: Fetches an asset by its ID.
/// - `get_assets`: Fetches a list of assets based on query parameters.
/// - `get_asset_by_id_and_version`: Fetches a specific version of an asset by ID.
///
/// # Asset Edits
/// - `get_asset_edits_by_asset_id`: Retrieves a paginated list of edits for an asset.
/// - `get_asset_edit_by_edit_id`: Retrieves a specific asset edit by its edit ID.
///
/// # Downloading
/// - `download_file`: Downloads a file from a given URL.
/// - `download_asset`: Downloads an asset and reports progress via a progress bar.
pub trait AssetStoreAPI: Send + Sync {
    async fn find_asset_by_asset_name_and_version_and_godot_version(
        &self,
        name: &str,
        version: &str,
        godot_version: &str,
    ) -> Result<AssetResponse>;

    async fn find_asset_by_id_or_name_and_version(
        &self,
        asset_id: &str,
        name: &str,
        godot_version: &str,
    ) -> Result<AssetResponse>;

    /// Fetches an asset by its ID.
    async fn get_asset_by_id(&self, asset_id: &str) -> Result<AssetResponse>;

    /// Fetches a list of assets based on query parameters.
    async fn get_assets(&self, params: HashMap<String, String>) -> Result<AssetListResponse>;

    /// Fetches a specific version of an asset by ID.
    async fn get_asset_by_id_and_version(
        &self,
        asset_id: &str,
        version: &str,
    ) -> Result<AssetResponse>;

    /// Retrieves a paginated list of edits for an asset.
    async fn get_asset_edits_by_asset_id(
        &self,
        asset_id: &str,
        page: usize,
    ) -> Result<AssetEditListResponse>;

    /// Retrieves a specific asset edit by its edit ID.
    async fn get_asset_edit_by_edit_id(&self, edit_id: &str) -> Result<AssetEditResponse>;

    /// Downloads an asset and reports progress via a progress bar.
    async fn download_asset(&self, asset: &AssetResponse, pb_task: ProgressBar) -> Result<Asset>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
impl AssetStoreAPI for DefaultAssetStoreAPI {
    async fn find_asset_by_asset_name_and_version_and_godot_version(
        &self,
        name: &str,
        version: &str,
        godot_version: &str,
    ) -> Result<AssetResponse> {
        if !name.is_empty() && !version.is_empty() {
            let asset = self
                .find_asset_by_id_or_name_and_version("", name, godot_version)
                .await?;
            self.get_asset_by_id_and_version(&asset.asset_id, version)
                .await
        } else {
            error!("Asset name or version is empty");
            bail!("Both asset name and version must be provided to search by version.")
        }
    }

    async fn find_asset_by_id_or_name_and_version(
        &self,
        asset_id: &str,
        name: &str,
        godot_version: &str,
    ) -> Result<AssetResponse> {
        if !asset_id.is_empty() {
            let asset = self.get_asset_by_id(asset_id).await?;
            Ok(asset)
        } else if !name.is_empty() {
            let params = HashMap::from([
                ("filter".to_string(), name.to_string()),
                ("godot_version".to_string(), godot_version.to_string()),
            ]);
            let asset_results = self.get_assets(params).await?;

            if asset_results.result.len() != 1 {
                bail!(
                    "Expected to find exactly one asset matching \"{}\", but found {}. Please refine your search or use --asset-id.",
                    name,
                    asset_results.result.len()
                )
            }
            let asset = asset_results.result.first().unwrap();
            let asset = self.get_asset_by_id(&asset.asset_id).await?;

            info!("Found asset: {}", asset.title);
            Ok(asset)
        } else {
            bail!("No name or asset ID provided")
        }
    }

    async fn get_asset_by_id(&self, asset_id: &str) -> Result<AssetResponse> {
        match self
            .http_service
            .get(self.get_url(&format!("/asset/{}", asset_id)), [].into())
            .await
        {
            Ok(data) => Ok(serde_json::from_value(data)?),
            Err(e) => {
                error!("Failed to get asset by ID '{}': {}", asset_id, e);
                bail!("No asset found with ID '{}'", asset_id)
            }
        }
    }

    async fn get_assets(&self, params: HashMap<String, String>) -> Result<AssetListResponse> {
        match self
            .http_service
            .get(self.get_url("/asset"), params.clone())
            .await
        {
            Ok(data) => Ok(serde_json::from_value(data)?),
            Err(e) => {
                error!("Failed to get assets with params {:?}: {}", params, e);
                bail!("Failed to get assets")
            }
        }
    }

    async fn get_asset_by_id_and_version(
        &self,
        asset_id: &str,
        version: &str,
    ) -> Result<AssetResponse> {
        if asset_id.is_empty() || version.is_empty() {
            bail!("Both asset ID and version must be provided to search by version.")
        }
        let mut page = 0;
        loop {
            let edits_response = self.get_asset_edits_by_asset_id(asset_id, page).await?;
            if edits_response.result.is_empty() {
                break;
            }
            for edit in edits_response.result.iter() {
                if edit.version_string == version && edit.asset_id == asset_id {
                    let edit_result = self.get_asset_edit_by_edit_id(&edit.edit_id).await?;
                    let asset_response = AssetResponse::from(edit_result);
                    return Ok(asset_response);
                }
            }
            if page == edits_response.pages - 1 {
                break;
            }
            page += 1;
        }
        bail!(
            "No asset found for asset_id: {} with version: {}",
            asset_id,
            version
        )
    }

    async fn get_asset_edits_by_asset_id(
        &self,
        asset_id: &str,
        page: usize,
    ) -> Result<AssetEditListResponse> {
        let params = HashMap::from([
            ("asset".to_string(), asset_id.to_string()),
            ("status".to_string(), "new accepted".to_string()),
            ("page".to_string(), page.to_string()),
        ]);
        match self
            .http_service
            .get(self.get_url("/asset/edit"), params)
            .await
        {
            Ok(data) => Ok(serde_json::from_value(data)?),
            Err(e) => {
                error!("Failed to get asset edits for asset ID {}: {}", asset_id, e);
                bail!("Failed to get asset edits for asset ID {}", asset_id)
            }
        }
    }

    async fn get_asset_edit_by_edit_id(&self, edit_id: &str) -> Result<AssetEditResponse> {
        match self
            .http_service
            .get(self.get_url(&format!("/asset/edit/{}", edit_id)), [].into())
            .await
        {
            Ok(data) => Ok(serde_json::from_value(data)?),
            Err(e) => {
                error!("Failed to get asset edit by edit ID {}: {}", edit_id, e); // TODO check how could I disable error! from loggin without -v 
                bail!("Failed to get asset edit by edit ID {}", edit_id)
            }
        }
    }

    /// Downloads a plugin from the Asset Store and returns a Asset struct
    ///
    /// Downloaded files are saved to the cache folder defined in the AppConfig
    async fn download_asset(&self, asset: &AssetResponse, pb_task: ProgressBar) -> Result<Asset> {
        let cache_folder = self.app_config.get_cache_folder_path();
        let download_url = &asset.download_url;

        let url = Url::parse(download_url)?;

        let filename = url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .unwrap_or("temp_file.zip");
        let filepath = cache_folder.join(filename);

        if !self.file_service.directory_exists(cache_folder) {
            self.file_service.create_directory(cache_folder)?;
        }

        if self.file_service.file_exists(&filepath)? {
            self.file_service.remove_file(&filepath)?;
        }

        let mut res = self.http_service.get_file(download_url.to_string()).await?;

        pb_task.set_length(100);

        let mut file = self.file_service.create_file_async(&filepath).await?;

        while let Some(chunk) = res.chunk().await? {
            pb_task.inc(chunk.len() as u64);
            self.file_service.write_all_async(&mut file, &chunk).await?;
        }

        file.flush().await?;
        pb_task.finish_and_clear();

        match res.error_for_status() {
            Ok(_) => Ok(Asset::new(filepath, asset.clone())),
            Err(e) => bail!("Failed to fetch file: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::services::{MockDefaultFileService, MockDefaultHttpService};

    use super::*;
    use mockall::predicate::*;

    fn setup_test_api() -> DefaultAssetStoreAPI {
        DefaultAssetStoreAPI::default()
    }

    // get_asset_by_id

    #[tokio::test]
    async fn test_get_asset_by_id() {
        let api = setup_test_api();
        let asset_id = "1709";
        let result = api.get_asset_by_id(asset_id).await;
        assert!(result.is_ok());
        let asset = result.unwrap();
        assert_eq!(asset.asset_id, asset_id);
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
        assert!(asset_list.result.is_empty());
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
        assert!(!asset_list.result.is_empty());
        assert_eq!(asset_list.result.len(), 1);
        let asset = asset_list.result.first().unwrap();
        assert_eq!(asset.asset_id, "1709");
    }

    // get_asset_edits_by_asset_id

    #[tokio::test]
    async fn test_get_asset_edits_by_asset_id_should_return_asset_edit_list_when_page_is_zero() {
        let api = setup_test_api();
        let asset_id = "1709";
        let result = api.get_asset_edits_by_asset_id(asset_id, 0).await;
        assert!(result.is_ok());
        let edit_list = result.unwrap();
        assert!(!edit_list.result.is_empty());
        let edit_list_item = edit_list.result.first().unwrap();
        assert_eq!(edit_list_item.asset_id, asset_id);
    }

    // get_asset_edit_by_edit_id

    #[tokio::test]
    async fn test_get_asset_edit_by_edit_id_should_return_asset_edit() {
        let api = setup_test_api();
        let edit_id = "18531";
        let result = api.get_asset_edit_by_edit_id(edit_id).await;
        assert!(result.is_ok());
        let edit = result.unwrap();
        assert_eq!(edit.asset_id, "1709");
    }

    // get_asset_by_id_and_version

    #[tokio::test]
    async fn test_search_asset_by_id_and_version_should_return_newer_version() {
        let api = setup_test_api();
        let edit_id = "1709";
        let version = "9.5.0";
        let result = api.get_asset_by_id_and_version(edit_id, version).await;
        assert!(result.is_ok());
        let edit = result.unwrap();
        assert_eq!(edit.asset_id, "1709");
        assert_eq!(edit.version_string, version);
    }

    #[tokio::test]
    async fn test_search_asset_by_id_and_version_should_return_older_version() {
        let api = setup_test_api();
        let edit_id = "1709";
        let version = "9.4.0";
        let result = api.get_asset_by_id_and_version(edit_id, version).await;
        assert!(result.is_ok());
        let edit = result.unwrap();
        assert_eq!(edit.asset_id, "1709");
        assert_eq!(edit.version_string, version);
    }

    #[tokio::test]
    async fn test_search_asset_by_id_and_version_should_return_err_if_no_version_found() {
        let api = setup_test_api();
        let edit_id = "1709";
        let version = "0.0.1";
        let result = api.get_asset_by_id_and_version(edit_id, version).await;
        assert!(result.is_err());
    }

    // download_asset
    #[tokio::test]
    async fn test_download_asset_should_download_to_cache_folder() {
        let mut mock_http_service = MockDefaultHttpService::new();
        mock_http_service.expect_get_file().returning(|_url| {
            let http_response = http::Response::builder().status(200).body("ok").unwrap();
            let something = reqwest::Response::from(http_response);
            Ok(something)
        });

        let mut mock_file_service = MockDefaultFileService::new();

        mock_file_service
            .expect_directory_exists()
            .with(eq(PathBuf::from("tests/mocks/cache")))
            .returning(|_path| true);

        mock_file_service
            .expect_file_exists()
            .with(eq(PathBuf::from("tests/mocks/cache/asset.zip")))
            .returning(|_path| Ok(false));

        mock_file_service
            .expect_create_file_async()
            .with(eq(PathBuf::from("tests/mocks/cache/asset.zip")))
            .returning(|_path| {
                // Create a temp file and open it as tokio::fs::File
                std::fs::create_dir_all("tests/mocks/cache").unwrap();
                let file = std::fs::File::create("tests/mocks/cache/asset.zip").unwrap();
                Ok(tokio::fs::File::from_std(file))
            });
        mock_file_service
            .expect_write_all_async()
            .returning(|_file, _chunk| Ok(()));

        let api = DefaultAssetStoreAPI::new(
            Arc::new(mock_http_service),
            DefaultAppConfig::new(
                Some(String::from("http://mock")),
                Some(String::from("tests/mocks/gdm.json")),
                Some(String::from("tests/mocks/cache")),
                Some(String::from(
                    "tests/mocks/project_with_plugins_and_version.godot",
                )),
                Some(String::from("tests/mocks/addons")),
            ),
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
        std::fs::remove_dir_all("tests/mocks/cache").unwrap();
    }
}
