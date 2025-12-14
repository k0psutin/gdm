use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;

use crate::config::{AppConfig, DefaultAppConfig};
use crate::installers::{AssetLibraryInstaller, GitInstaller, PluginInstaller};
use crate::models::{Plugin, PluginSource};
use crate::services::{DefaultFileService, FileService, PluginParser};
use crate::ui::OperationManager;

/// Service for managing staged plugin installations
/// Provides a unified workflow for all installer types
pub struct DefaultInstallService {
    file_service: Arc<dyn FileService + Send + Sync>,
    app_config: DefaultAppConfig,
    parser: Arc<PluginParser>,
    installers: Vec<Box<dyn PluginInstaller>>,
}

impl Default for DefaultInstallService {
    fn default() -> Self {
        let file_service = Arc::new(DefaultFileService);
        let app_config = DefaultAppConfig::default();
        let parser = Arc::new(PluginParser::new(file_service.clone()));
        let asset_installer = AssetLibraryInstaller::default();
        let git_installer = GitInstaller::default();
        let installers: Vec<Box<dyn PluginInstaller>> =
            vec![Box::new(asset_installer), Box::new(git_installer)];
        Self::new(file_service, app_config, parser, installers)
    }
}

impl DefaultInstallService {
    pub fn new(
        file_service: Arc<dyn FileService + Send + Sync>,
        app_config: DefaultAppConfig,
        parser: Arc<PluginParser>,
        installers: Vec<Box<dyn PluginInstaller>>,
    ) -> Self {
        Self {
            file_service,
            app_config,
            parser,
            installers,
        }
    }
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
impl InstallService for DefaultInstallService {
    fn discover_and_analyze_plugins(
        &self,
        source: &PluginSource,
        cache_dir: &Path,
        expected_name: &str,
    ) -> Result<(String, Plugin, Vec<PathBuf>)> {
        let addons_dir = cache_dir.join("addons");

        if !self.file_service.directory_exists(&addons_dir) {
            bail!("No 'addons' directory found at: {}", cache_dir.display());
        }

        let addon_folders: Vec<PathBuf> = self
            .file_service
            .read_dir(&addons_dir)?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    let path = e.path();
                    if path.is_dir() {
                        path.file_name()
                            .map(|n| PathBuf::from(n.to_string_lossy().to_string()))
                    } else {
                        None
                    }
                })
            })
            .collect();

        if addon_folders.is_empty() {
            bail!("No folders found inside {}/addons", cache_dir.display());
        }

        let parsed_plugins = self.parser.create_plugins_from_addon_folders_with_base(
            source,
            &addon_folders,
            Some(cache_dir),
        )?;

        let (main_plugin_folder, best_main_plugin) = self
            .parser
            .determine_best_main_plugin_match(&parsed_plugins, expected_name)?;

        let plugin = self.parser.enrich_with_sub_assets(
            &best_main_plugin,
            &parsed_plugins,
            &addon_folders,
        )?;

        debug!(
            "Discovered main plugin '{}' with {} sub-assets (plugin.cfg: {})",
            plugin.title,
            plugin.sub_assets.len(),
            if plugin.plugin_cfg_path.is_some() {
                "found"
            } else {
                "not found"
            }
        );

        Ok((main_plugin_folder, plugin, addon_folders))
    }

    fn install_from_cache(
        &self,
        cache_dir: &Path,
        addon_folders: &[PathBuf],
    ) -> Result<Vec<PathBuf>> {
        let project_addons_dir = self.app_config.get_addon_folder_path();
        let staging_addons_dir = cache_dir.join("addons");
        let mut installed_paths = Vec::new();

        for folder in addon_folders {
            let src = staging_addons_dir.join(folder);
            let dest = project_addons_dir.join(folder);

            if self.file_service.directory_exists(&dest) {
                debug!("Removing existing installation: {}", dest.display());
                self.file_service.remove_dir_all(&dest)?;
            }

            if let Some(parent) = dest.parent()
                && !self.file_service.directory_exists(parent)
            {
                self.file_service.create_directory(parent)?;
            }

            self.file_service.rename(&src, &dest).with_context(|| {
                format!("Failed to move {} to {}", src.display(), dest.display())
            })?;

            installed_paths.push(dest);
        }

        Ok(installed_paths)
    }

    fn cleanup_cache(&self) -> Result<()> {
        let dir = self.app_config.get_cache_folder_path();
        if self.file_service.directory_exists(dir) {
            self.file_service.remove_dir_all(dir)?;
            debug!("Cleaned up cache: {}", dir.display());
        }
        Ok(())
    }

    async fn install(
        &self,
        plugins: &[Plugin],
        operation_manager: Arc<OperationManager>,
    ) -> Result<BTreeMap<String, Plugin>> {
        let mut installed_plugins = Vec::new();

        for (idx, plugin) in plugins.iter().enumerate() {
            let installer = self
                .installers
                .iter()
                .find(|inst| inst.can_handle(plugin.source.clone()));

            if let Some(installer) = installer {
                let future =
                    installer.install(idx, plugins.len(), self, plugin, operation_manager.clone());
                installed_plugins.push(future);
            }
        }

        let results = futures::future::try_join_all(installed_plugins).await?;

        self.cleanup_cache()?;

        let installed_plugins: BTreeMap<String, Plugin> = results.into_iter().collect();

        Ok(installed_plugins)
    }
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait InstallService: Send + Sync {
    fn discover_and_analyze_plugins(
        &self,
        source: &PluginSource,
        asset_dir: &Path,
        main_plugin_name: &str,
    ) -> Result<(String, Plugin, Vec<PathBuf>)>;

    fn install_from_cache(
        &self,
        asset_dir: &Path,
        addon_folders: &[PathBuf],
    ) -> Result<Vec<PathBuf>>;

    fn cleanup_cache(&self) -> Result<()>;

    async fn install(
        &self,
        plugins: &[Plugin],
        operation_manager: Arc<OperationManager>,
    ) -> Result<BTreeMap<String, Plugin>>;
}
