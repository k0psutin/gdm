use crate::api::{Asset, AssetResponse, AssetStoreAPI};
use crate::installers::PluginInstaller;
use crate::models::{Plugin, PluginSource};
use crate::services::ExtractService;
use crate::ui::OperationManager;

use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::future::try_join_all;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{error, info};

pub struct AssetLibraryInstaller {
    asset_store_api: Arc<dyn AssetStoreAPI + Send + Sync>,
    extract_service: Arc<dyn ExtractService + Send + Sync>,
}

impl AssetLibraryInstaller {
    pub fn new(
        asset_store_api: Arc<dyn AssetStoreAPI + Send + Sync>,
        extract_service: Arc<dyn ExtractService + Send + Sync>,
    ) -> Self {
        Self {
            asset_store_api,
            extract_service,
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
