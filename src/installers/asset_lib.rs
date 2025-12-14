use crate::api::{Asset, AssetResponse, AssetStoreAPI};
use crate::config::{AppConfig, DefaultAppConfig};
use crate::installers::PluginInstaller;
use crate::models::{Plugin, PluginSource};
use crate::services::{ExtractService, InstallService};
use crate::ui::OperationManager;
use crate::utils::Utils;

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

pub struct AssetLibraryInstaller {
    asset_store_api: Arc<dyn AssetStoreAPI + Send + Sync>,
    extract_service: Arc<dyn ExtractService + Send + Sync>,
    app_config: DefaultAppConfig,
}

impl Default for AssetLibraryInstaller {
    fn default() -> Self {
        let asset_store_api = Arc::new(crate::api::DefaultAssetStoreAPI::default());
        let extract_service = Arc::new(crate::services::DefaultExtractService::default());

        let app_config = DefaultAppConfig::default();

        Self {
            asset_store_api,
            extract_service,
            app_config,
        }
    }
}

impl AssetLibraryInstaller {
    pub fn new(
        asset_store_api: Arc<dyn AssetStoreAPI + Send + Sync>,
        extract_service: Arc<dyn ExtractService + Send + Sync>,
        app_config: DefaultAppConfig,
    ) -> Self {
        Self {
            asset_store_api,
            extract_service,
            app_config,
        }
    }

    async fn resolve_asset_metadata(&self, plugin: &Plugin) -> Result<AssetResponse> {
        if let Some(PluginSource::AssetLibrary { asset_id }) = &plugin.source {
            let api = self.asset_store_api.clone();
            let version = plugin.get_version();

            api.get_asset_by_id_and_version(asset_id, &version)
                .await
                .with_context(|| format!("Failed to fetch metadata for asset {}", asset_id))
        } else {
            anyhow::bail!("Plugin is not from asset library")
        }
    }

    async fn download_asset_with_manager(
        &self,
        asset: &AssetResponse,
        index: usize,
        total: usize,
        operation_manager: &OperationManager,
    ) -> Result<Asset> {
        let pb_task =
            operation_manager.add_progress_bar(index, total, &asset.title, &asset.version)?;

        let api = self.asset_store_api.clone();

        api.download_asset(asset, pb_task).await
    }

    async fn extract_to_cache_with_manager(
        &self,
        downloaded_asset: &Asset,
        index: usize,
        total: usize,
        operation_manager: &OperationManager,
    ) -> Result<(String, PathBuf)> {
        let cache_dir = self.app_config.get_cache_folder_path();

        let asset_response = &downloaded_asset.asset_response;
        let asset_id = asset_response.asset_id.clone();

        let pb_task = operation_manager.add_progress_bar(
            index,
            total,
            &asset_response.title,
            &asset_response.version_string,
        )?;

        let extract_service = self.extract_service.clone();
        let asset_cloned = downloaded_asset.clone();

        let tmp_dir = cache_dir.join(&asset_id);

        extract_service
            .extract_asset_to_staging(&asset_cloned, &tmp_dir, pb_task)
            .await
            .map(|path| (asset_id, path))
    }
}

#[async_trait]
impl PluginInstaller for AssetLibraryInstaller {
    fn can_handle(&self, source: Option<PluginSource>) -> bool {
        matches!(source, Some(PluginSource::AssetLibrary { .. }))
    }

    async fn install(
        &self,
        index: usize,
        total: usize,
        install_service: &dyn InstallService,
        plugin: &Plugin,
        operation_manager: Arc<OperationManager>,
    ) -> Result<(String, Plugin)> {
        let asset_metadata = self.resolve_asset_metadata(plugin).await?;

        let downloaded_file = self
            .download_asset_with_manager(&asset_metadata, index, total, &operation_manager)
            .await?;

        let path = self
            .extract_to_cache_with_manager(&downloaded_file, index, total, &operation_manager)
            .await?;

        let (asset_id, staging_dir) = path;
        let metadata = &downloaded_file.asset_response;

        let plugin_source = PluginSource::AssetLibrary {
            asset_id: asset_id.clone(),
        };

        let (main_folder_name, mut plugin, folders_to_move) = install_service
            .discover_and_analyze_plugins(&plugin_source, &staging_dir, &metadata.title)?;

        install_service.install_from_cache(&staging_dir, &folders_to_move)?;

        plugin.title = metadata.title.clone();
        plugin.version = Utils::parse_semantic_version(&metadata.version_string);

        Ok((main_folder_name, plugin))
    }
}
