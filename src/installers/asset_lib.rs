use crate::api::{Asset, AssetResponse, AssetStoreAPI};
use crate::config::{AppConfig, DefaultAppConfig};
use crate::installers::PluginInstaller;
use crate::models::{Plugin, PluginSource};
use crate::services::{ExtractService, StagingService};
use crate::ui::OperationManager;
use crate::utils::Utils;

use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::future::try_join_all;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{error, info};

pub struct AssetLibraryInstaller {
    asset_store_api: Arc<dyn AssetStoreAPI + Send + Sync>,
    extract_service: Arc<dyn ExtractService + Send + Sync>,
    staging_service: Option<Arc<StagingService>>,
}

impl AssetLibraryInstaller {
    pub fn new(
        asset_store_api: Arc<dyn AssetStoreAPI + Send + Sync>,
        extract_service: Arc<dyn ExtractService + Send + Sync>,
    ) -> Self {
        Self {
            asset_store_api,
            extract_service,
            staging_service: None,
        }
    }

    /// Create a new AssetLibraryInstaller with staging support
    pub fn with_staging(
        asset_store_api: Arc<dyn AssetStoreAPI + Send + Sync>,
        extract_service: Arc<dyn ExtractService + Send + Sync>,
        staging_service: Arc<StagingService>,
    ) -> Self {
        Self {
            asset_store_api,
            extract_service,
            staging_service: Some(staging_service),
        }
    }

    /// Step 1: Resolve metadata (concurrently) to get download URLs
    async fn resolve_assets_metadata(&self, plugins: &[Plugin]) -> Result<Vec<AssetResponse>> {
        let mut futures = Vec::new();

        for plugin in plugins {
            if let Some(PluginSource::AssetLibrary { asset_id }) = &plugin.source {
                let api = self.asset_store_api.clone();
                let id = asset_id.clone();
                let version = plugin.get_version();

                futures.push(async move {
                    api.get_asset_by_id_and_version(&id, &version)
                        .await
                        .with_context(|| format!("Failed to fetch metadata for asset {}", id))
                });
            }
        }

        let fetched_assets: Vec<AssetResponse> = try_join_all(futures).await?;
        Ok(fetched_assets)
    }

    async fn download_assets_with_manager(
        &self,
        assets: &[AssetResponse],
        operation_manager: &OperationManager,
        start_index: usize,
        total_count: usize,
    ) -> Result<Vec<Asset>> {
        if assets.is_empty() {
            return Ok(vec![]);
        }

        let mut download_tasks = JoinSet::new();

        for (i, asset_response) in assets.iter().enumerate() {
            let asset_res = asset_response.clone();

            let global_index = start_index + i;

            // Create a progress bar for this specific download
            let pb_task = operation_manager.add_progress_bar(
                global_index,
                total_count,
                &asset_res.title,
                &asset_res.version,
            )?;

            let api = self.asset_store_api.clone();

            download_tasks.spawn(async move { api.download_asset(&asset_res, pb_task).await });
        }

        let mut successful_downloads = Vec::new();

        while let Some(res) = download_tasks.join_next().await {
            match res {
                Ok(Ok(asset)) => successful_downloads.push(asset),
                Ok(Err(e)) => error!("Failed to download asset: {:#}", e),
                Err(join_err) => error!("Download task panicked: {}", join_err),
            }
        }

        info!(
            "Downloaded {}/{} plugins successfully",
            successful_downloads.len(),
            assets.len()
        );
        Ok(successful_downloads)
    }

    async fn extract_assets_with_manager(
        &self,
        downloaded_assets: &[Asset],
        operation_manager: &OperationManager,
        start_index: usize,
        total_count: usize,
    ) -> Result<BTreeMap<String, Plugin>> {
        if downloaded_assets.is_empty() {
            return Ok(BTreeMap::new());
        }

        let mut extract_tasks = JoinSet::new();

        for (i, asset_wrapper) in downloaded_assets.iter().enumerate() {
            let asset_response = &asset_wrapper.asset_response;

            let global_index = start_index + i;

            let pb_task = operation_manager.add_progress_bar(
                global_index,
                total_count,
                &asset_response.title,
                &asset_response.version_string,
            )?;

            let extract_service = self.extract_service.clone();
            let asset_cloned = asset_wrapper.clone();

            extract_tasks
                .spawn(async move { extract_service.extract_asset(&asset_cloned, pb_task).await });
        }

        let mut installed_plugins = BTreeMap::new();

        while let Some(res) = extract_tasks.join_next().await {
            match res {
                Ok(Ok((folder, plugin))) => {
                    installed_plugins.insert(folder, plugin);
                }
                Ok(Err(e)) => error!("Failed to extract asset: {:#}", e),
                Err(join_err) => error!("Extraction task panicked: {}", join_err),
            }
        }

        info!("Extracted {} plugins successfully", installed_plugins.len());
        Ok(installed_plugins)
    }

    // ===== NEW UNIFIED STAGING WORKFLOW METHODS =====

    /// Extract assets to staging directories (Phase 2 of unified workflow)
    async fn extract_to_staging_with_manager(
        &self,
        downloaded_assets: &[Asset],
        operation_manager: &OperationManager,
        start_index: usize,
        total_count: usize,
        cache_dir: PathBuf,
    ) -> Result<Vec<(String, PathBuf)>> {
        if downloaded_assets.is_empty() {
            return Ok(vec![]);
        }

        let mut extract_tasks = JoinSet::new();

        for (i, asset_wrapper) in downloaded_assets.iter().enumerate() {
            let asset_response = &asset_wrapper.asset_response;
            let asset_id = asset_response.asset_id.clone();
            let global_index = start_index + i;

            let pb_task = operation_manager.add_progress_bar(
                global_index,
                total_count,
                &asset_response.title,
                &asset_response.version_string,
            )?;

            let extract_service = self.extract_service.clone();
            let asset_cloned = asset_wrapper.clone();

            // Create staging directory for this asset
            let staging_dir = cache_dir.join(&asset_id);

            extract_tasks.spawn(async move {
                extract_service
                    .extract_asset_to_staging(&asset_cloned, &staging_dir, pb_task)
                    .await
                    .map(|path| (asset_id, path))
            });
        }

        let mut staging_paths = Vec::new();

        while let Some(res) = extract_tasks.join_next().await {
            match res {
                Ok(Ok((asset_id, path))) => {
                    staging_paths.push((asset_id, path));
                }
                Ok(Err(e)) => error!("Failed to extract asset to staging: {:#}", e),
                Err(join_err) => error!("Extraction task panicked: {}", join_err),
            }
        }

        info!(
            "Extracted {} assets to staging successfully",
            staging_paths.len()
        );
        Ok(staging_paths)
    }

    /// Install plugins using the unified staging workflow
    /// This is the new implementation that follows the 7-phase approach
    async fn install_with_staging(
        &self,
        plugins: Vec<Plugin>,
        operation_manager: Arc<OperationManager>,
        start_index: usize,
        total_count: usize,
        staging_service: &StagingService,
        app_config: &dyn AppConfig,
    ) -> Result<BTreeMap<String, Plugin>> {
        // Phase 1: Resolve metadata
        let assets_metadata = self.resolve_assets_metadata(&plugins).await?;

        if assets_metadata.is_empty() {
            return Ok(BTreeMap::new());
        }

        // Phase 1 continued: Download assets
        let downloaded_files = self
            .download_assets_with_manager(
                &assets_metadata,
                &operation_manager,
                start_index,
                total_count,
            )
            .await?;

        // Phase 2: Extract to staging
        let cache_dir = app_config.get_cache_folder_path().to_path_buf();
        let staging_paths = self
            .extract_to_staging_with_manager(
                &downloaded_files,
                &operation_manager,
                start_index,
                total_count,
                cache_dir,
            )
            .await?;

        let mut installed_plugins = BTreeMap::new();

        // Process each staged asset
        for (asset_id, staging_dir) in staging_paths {
            // Find the asset metadata
            let asset_metadata = downloaded_files
                .iter()
                .find(|a| a.asset_response.asset_id == asset_id)
                .context("Asset metadata not found")?;

            // Phase 3 & 4: Discover and Analyze - find all addons and determine main plugin
            let plugin_source = PluginSource::AssetLibrary {
                asset_id: asset_id.clone(),
            };

            let main_plugin_name = &asset_metadata.asset_response.title;
            let (folder_name, mut main_plugin, addon_folders) = staging_service
                .discover_and_analyze_plugins(&staging_dir, &plugin_source, main_plugin_name)
                .context("Failed to discover and analyze plugins")?;

            // Phase 5: Validate (already validated in discover_and_analyze_plugins)
            // Just check we have addon folders
            if addon_folders.is_empty() {
                anyhow::bail!("No addon folders found in staging directory");
            }

            // Phase 6: Install from staging
            let addons_dir = app_config.get_addon_folder_path();
            staging_service
                .install_from_staging(&staging_dir, &addon_folders, &addons_dir)
                .context("Failed to install from staging")?;

            // Update plugin with AssetLibrary metadata (overrides plugin.cfg data)
            // For AssetLibrary plugins, we trust the API metadata over plugin.cfg
            main_plugin.title = asset_metadata.asset_response.title.clone();
            main_plugin.version =
                Utils::parse_semantic_version(&asset_metadata.asset_response.version_string);
            main_plugin.license = Some(asset_metadata.asset_response.cost.clone());

            // Update plugin path to point to production addons folder
            // The folder_name is the addon folder (e.g., "gut")
            // The plugin.cfg is typically at addons/<folder>/plugin.cfg or deeper
            if let Some(cfg_path) = &main_plugin.plugin_cfg_path {
                // Extract the relative path from the staging path
                // E.g., ".gdm/1709/addons/gut/plugin.cfg" -> "gut/plugin.cfg"
                if let Some(addons_idx) = cfg_path.rfind("/addons/") {
                    // Found "/addons/" in the path, extract everything after it
                    let relative_path = &cfg_path[addons_idx + "/addons/".len()..];
                    main_plugin.plugin_cfg_path = Some(format!("addons/{}", relative_path));
                } else {
                    // Fallback: construct path from folder name
                    main_plugin.plugin_cfg_path =
                        Some(format!("addons/{}/plugin.cfg", folder_name));
                }
            }

            // Phase 7: Cleanup staging
            staging_service
                .cleanup_staging(&staging_dir)
                .context("Failed to cleanup staging")?;

            installed_plugins.insert(folder_name, main_plugin);
        }

        info!(
            "Installed {} plugins using staging workflow",
            installed_plugins.len()
        );
        Ok(installed_plugins)
    }
}

#[async_trait]
impl PluginInstaller for AssetLibraryInstaller {
    fn can_handle(&self, source: &PluginSource) -> bool {
        matches!(source, PluginSource::AssetLibrary { .. })
    }

    async fn install(
        &self,
        plugins: Vec<Plugin>,
        operation_manager: Arc<OperationManager>,
        start_index: usize,
        total_count: usize,
    ) -> Result<BTreeMap<String, Plugin>> {
        // Use staging workflow if available
        if let Some(staging_service) = &self.staging_service {
            return self
                .install_with_staging(
                    plugins,
                    operation_manager,
                    start_index,
                    total_count,
                    staging_service,
                    &DefaultAppConfig::default(),
                )
                .await;
        }

        // Fallback to legacy workflow
        let assets_metadata = self.resolve_assets_metadata(&plugins).await?;

        if assets_metadata.is_empty() {
            return Ok(BTreeMap::new());
        }

        let downloaded_files = self
            .download_assets_with_manager(
                &assets_metadata,
                &operation_manager,
                start_index,
                total_count,
            )
            .await?;

        let installed_plugins = self
            .extract_assets_with_manager(
                &downloaded_files,
                &operation_manager,
                start_index,
                total_count,
            )
            .await?;

        Ok(installed_plugins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{AssetResponse, MockAssetStoreAPI};
    use crate::config::DefaultAppConfig;
    use crate::services::{MockDefaultExtractService, MockDefaultFileService, PluginParser};
    use semver::Version;

    // Note: MockAssetStoreAPI is not exported, so we test with real API or skip API-dependent tests
    // These tests focus on the new staging-related functionality that can be tested without mocks

    fn create_test_asset_response(asset_id: &str, title: &str, version: &str) -> AssetResponse {
        AssetResponse {
            asset_id: asset_id.to_string(),
            title: title.to_string(),
            version: "1".to_string(),
            version_string: version.to_string(),
            godot_version: "4.0".to_string(),
            rating: "5".to_string(),
            cost: "MIT".to_string(),
            description: "Test asset".to_string(),
            download_provider: "GitHub".to_string(),
            download_commit: "abc123".to_string(),
            modify_date: "2024-01-01".to_string(),
            download_url: "https://example.com/download".to_string(),
        }
    }

    fn create_test_plugin(asset_id: &str, title: &str, version: &str) -> Plugin {
        Plugin {
            source: Some(PluginSource::AssetLibrary {
                asset_id: asset_id.to_string(),
            }),
            plugin_cfg_path: Some(format!("addons/{}/plugin.cfg", title.to_lowercase())),
            title: title.to_string(),
            version: Version::parse(version).unwrap_or(Version::new(1, 0, 0)),
            sub_assets: vec![],
            license: Some("MIT".to_string()),
        }
    }

    #[test]
    fn test_asset_library_installer_can_handle_asset_library_source() {
        let mock_api = MockAssetStoreAPI::new();
        let mock_extract = MockDefaultExtractService::new();
        let installer = AssetLibraryInstaller::new(Arc::new(mock_api), Arc::new(mock_extract));

        let asset_source = PluginSource::AssetLibrary {
            asset_id: "123".to_string(),
        };

        assert!(installer.can_handle(&asset_source));
    }

    #[test]
    fn test_asset_library_installer_cannot_handle_git_source() {
        let mock_api = MockAssetStoreAPI::new();
        let mock_extract = MockDefaultExtractService::new();
        let installer = AssetLibraryInstaller::new(Arc::new(mock_api), Arc::new(mock_extract));

        let git_source = PluginSource::Git {
            url: "https://github.com/test/repo".to_string(),
            reference: "main".to_string(),
        };

        assert!(!installer.can_handle(&git_source));
    }

    #[test]
    fn test_with_staging_constructor() {
        let mock_api = MockAssetStoreAPI::new();
        let mock_extract = MockDefaultExtractService::new();
        let mock_fs = MockDefaultFileService::new();
        let app_config = DefaultAppConfig::default();

        let parser = Arc::new(PluginParser::new(Arc::new(mock_fs)));
        let mock_fs2 = MockDefaultFileService::new();
        let staging_service =
            Arc::new(StagingService::new(Arc::new(mock_fs2), parser, &app_config));

        let installer = AssetLibraryInstaller::with_staging(
            Arc::new(mock_api),
            Arc::new(mock_extract),
            staging_service.clone(),
        );

        assert!(installer.staging_service.is_some());
    }

    #[tokio::test]
    async fn test_resolve_assets_metadata_empty_plugins() {
        let mock_api = MockAssetStoreAPI::new();
        let mock_extract = MockDefaultExtractService::new();
        let installer = AssetLibraryInstaller::new(Arc::new(mock_api), Arc::new(mock_extract));

        let plugins: Vec<Plugin> = vec![];
        let result = installer.resolve_assets_metadata(&plugins).await;

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.len(), 0);
    }

    #[tokio::test]
    async fn test_resolve_assets_metadata_with_plugins() {
        let mut mock_api = MockAssetStoreAPI::new();

        // Mock API response
        let asset_response = create_test_asset_response("123", "TestPlugin", "1.0.0");
        let asset_response_clone = asset_response.clone();

        mock_api
            .expect_get_asset_by_id_and_version()
            .times(1)
            .returning(move |_, _| {
                let response = asset_response_clone.clone();
                Box::pin(async move { Ok(response) })
            });

        let mock_extract = MockDefaultExtractService::new();
        let installer = AssetLibraryInstaller::new(Arc::new(mock_api), Arc::new(mock_extract));

        let plugins = vec![create_test_plugin("123", "TestPlugin", "1.0.0")];
        let result = installer.resolve_assets_metadata(&plugins).await;

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].asset_id, "123");
        assert_eq!(metadata[0].title, "TestPlugin");
    }

    #[tokio::test]
    async fn test_resolve_assets_metadata_skips_non_asset_library_plugins() {
        let mock_api = MockAssetStoreAPI::new();
        let mock_extract = MockDefaultExtractService::new();
        let installer = AssetLibraryInstaller::new(Arc::new(mock_api), Arc::new(mock_extract));

        let mut plugin = create_test_plugin("123", "TestPlugin", "1.0.0");
        plugin.source = Some(PluginSource::Git {
            url: "https://github.com/test/repo".to_string(),
            reference: "main".to_string(),
        });

        let plugins = vec![plugin];
        let result = installer.resolve_assets_metadata(&plugins).await;

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.len(), 0); // Git source should be skipped
    }

    #[tokio::test]
    async fn test_download_assets_with_manager_empty_assets() {
        let mock_api = MockAssetStoreAPI::new();
        let mock_extract = MockDefaultExtractService::new();
        let installer = AssetLibraryInstaller::new(Arc::new(mock_api), Arc::new(mock_extract));

        let operation_manager =
            Arc::new(OperationManager::new(crate::ui::Operation::Install).unwrap());
        let assets: Vec<AssetResponse> = vec![];

        let result = installer
            .download_assets_with_manager(&assets, &operation_manager, 0, 1)
            .await;

        assert!(result.is_ok());
        let downloaded = result.unwrap();
        assert_eq!(downloaded.len(), 0);
    }

    #[tokio::test]
    async fn test_extract_assets_with_manager_empty_assets() {
        let mock_api = MockAssetStoreAPI::new();
        let mock_extract = MockDefaultExtractService::new();
        let installer = AssetLibraryInstaller::new(Arc::new(mock_api), Arc::new(mock_extract));

        let operation_manager =
            Arc::new(OperationManager::new(crate::ui::Operation::Install).unwrap());
        let downloaded: Vec<Asset> = vec![];

        let result = installer
            .extract_assets_with_manager(&downloaded, &operation_manager, 0, 1)
            .await;

        assert!(result.is_ok());
        let installed = result.unwrap();
        assert_eq!(installed.len(), 0);
    }
}
