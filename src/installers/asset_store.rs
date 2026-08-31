use crate::config::{AppConfig, DefaultAppConfig};
use crate::installers::PluginInstaller;
use crate::models::{Asset, Plugin, PluginSource};
use crate::services::{
    AssetStoreService, DefaultAssetStoreService, DefaultExtractService, ExtractService,
    InstallService,
};
use crate::ui::OperationManager;

use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

pub struct AssetStoreInstaller {
    asset_store_service: Arc<dyn AssetStoreService + Send + Sync>,
    extract_service: Arc<dyn ExtractService + Send + Sync>,
    app_config: DefaultAppConfig,
}

impl Default for AssetStoreInstaller {
    fn default() -> Self {
        let asset_store_service = Arc::new(DefaultAssetStoreService::default());
        let extract_service = Arc::new(DefaultExtractService::default());

        let app_config = DefaultAppConfig::default();

        Self {
            asset_store_service,
            extract_service,
            app_config,
        }
    }
}

impl AssetStoreInstaller {
    #[allow(unused)]
    pub fn new(
        asset_store_service: Arc<dyn AssetStoreService + Send + Sync>,
        extract_service: Arc<dyn ExtractService + Send + Sync>,
        app_config: DefaultAppConfig,
    ) -> Self {
        Self {
            asset_store_service,
            extract_service,
            app_config,
        }
    }

    async fn resolve_asset_metadata(&self, plugin: &Plugin) -> Result<Asset> {
        if let PluginSource::AssetStore {
            asset_slug,
            publisher_slug,
        } = &plugin.source
        {
            let asset_version = plugin.get_version();

            self.asset_store_service
                .get_asset(publisher_slug, asset_slug, Some(&asset_version), None)
                .await
        } else {
            anyhow::bail!("Dependency is not from the Godot Asset Library")
        }
    }

    async fn download_asset_with_manager(
        &self,
        asset: &Asset,
        index: usize,
        total: usize,
        operation_manager: &OperationManager,
    ) -> Result<Asset> {
        let pb_task =
            operation_manager.add_progress_bar(index, total, &asset.title, &asset.version)?;

        self.asset_store_service
            .download_asset(asset.clone(), pb_task)
            .await
    }

    async fn extract_to_cache_with_manager(
        &self,
        downloaded_asset: &Asset,
        index: usize,
        total: usize,
        operation_manager: &OperationManager,
    ) -> Result<PathBuf> {
        let cache_dir = self.app_config.get_cache_folder_path();

        let pb_task = operation_manager.add_progress_bar(
            index,
            total,
            &downloaded_asset.title,
            &downloaded_asset.version,
        )?;

        let extract_service = self.extract_service.clone();
        let asset_cloned = downloaded_asset.clone();

        let tmp_dir = cache_dir
            .join("staging")
            .join(cache_path_component(&downloaded_asset.publisher_slug))
            .join(cache_path_component(&downloaded_asset.asset_slug))
            .join(cache_path_component(&downloaded_asset.version));

        extract_service
            .extract_asset_to_cache(&asset_cloned, &tmp_dir, pb_task)
            .await
    }
}

fn cache_path_component(value: &str) -> String {
    let value: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();

    if value.is_empty() || value == "." || value == ".." {
        "unknown".to_string()
    } else {
        value
    }
}

#[async_trait]
impl PluginInstaller for AssetStoreInstaller {
    fn can_handle(&self, source: &PluginSource) -> bool {
        matches!(source, PluginSource::AssetStore { .. })
    }

    async fn install(
        &self,
        index: usize,
        total: usize,
        install_service: &dyn InstallService,
        plugin: &Plugin,
        operation_manager: Arc<OperationManager>,
    ) -> Result<(String, Plugin)> {
        let asset = self.resolve_asset_metadata(plugin).await?;

        let downloaded_file = self
            .download_asset_with_manager(&asset, index, total, &operation_manager)
            .await?;

        let staging_dir = self
            .extract_to_cache_with_manager(&downloaded_file, index, total, &operation_manager)
            .await?;

        let plugin_source = PluginSource::AssetStore {
            asset_slug: asset.asset_slug.clone(),
            publisher_slug: asset.publisher_slug.clone(),
        };

        let (main_folder_name, discovered_plugin, folders_to_move) = install_service
            .discover_and_analyze_plugins(
                &plugin_source,
                &staging_dir,
                &asset.asset_slug.clone(),
            )?;

        install_service.install_from_cache(&staging_dir, &folders_to_move)?;

        let mut plugin = Plugin::from(asset);
        plugin.plugin_cfg_path = discovered_plugin.plugin_cfg_path;
        plugin.sub_assets = discovered_plugin.sub_assets;

        Ok((main_folder_name, plugin.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{
        MockDefaultAssetStoreService, MockDefaultExtractService, MockDefaultInstallService,
    };
    use crate::ui::{Operation, OperationManager};

    fn test_asset() -> Asset {
        Asset {
            file_path: PathBuf::from("/cache/license-manager.zip"),
            publisher_slug: "kenyoni".to_string(),
            asset_slug: "license-manager".to_string(),
            license: "MIT".to_string(),
            score: 5,
            version: "1.0.0".to_string(),
            title: "License Manager".to_string(),
            description: "Description".to_string(),
            size: 100.0,
            download_url: "https://example.com/license-manager.zip".to_string(),
        }
    }

    #[tokio::test]
    async fn test_install_preserves_discovered_sub_assets() {
        let asset = test_asset();
        let source = PluginSource::AssetStore {
            publisher_slug: asset.publisher_slug.clone(),
            asset_slug: asset.asset_slug.clone(),
        };
        let plugin = Plugin::new(
            source.clone(),
            None,
            asset.title.clone(),
            asset.version.clone(),
            Some(asset.license.clone()),
            vec![],
        );
        let discovered_plugin = Plugin::new(
            source,
            Some(PathBuf::from("addons/licenses/plugin.cfg")),
            asset.title.clone(),
            asset.version.clone(),
            Some(asset.license.clone()),
            vec!["license-assets".to_string()],
        );

        let mut asset_store_service = MockDefaultAssetStoreService::default();
        asset_store_service.expect_get_asset().times(1).returning({
            let asset = asset.clone();
            move |_, _, _, _| Ok(asset.clone())
        });
        asset_store_service
            .expect_download_asset()
            .times(1)
            .returning(|asset, _| Ok(asset));

        let mut extract_service = MockDefaultExtractService::default();
        extract_service
            .expect_extract_asset_to_cache()
            .times(1)
            .withf(|_, path, _| {
                path == std::path::Path::new(".gdm/staging/kenyoni/license-manager/1.0.0")
            })
            .returning(|_, _, _| Ok(PathBuf::from("/cache/license-manager")));

        let mut install_service = MockDefaultInstallService::default();
        install_service
            .expect_discover_and_analyze_plugins()
            .times(1)
            .returning({
                let discovered_plugin = discovered_plugin.clone();
                move |_, _, _| {
                    Ok((
                        "licenses".to_string(),
                        discovered_plugin.clone(),
                        vec![PathBuf::from("licenses"), PathBuf::from("license-assets")],
                    ))
                }
            });
        install_service
            .expect_install_from_cache()
            .times(1)
            .returning(|_, _| Ok(vec![]));

        let installer = AssetStoreInstaller::new(
            Arc::new(asset_store_service),
            Arc::new(extract_service),
            DefaultAppConfig::default(),
        );
        let operation_manager = Arc::new(OperationManager::new(Operation::Install).unwrap());

        let (_, installed_plugin) = installer
            .install(0, 1, &install_service, &plugin, operation_manager)
            .await
            .unwrap();

        assert_eq!(installed_plugin.sub_assets, vec!["license-assets"]);
        assert_eq!(
            installed_plugin.plugin_cfg_path,
            Some("addons/licenses/plugin.cfg".to_string())
        );
    }
}
