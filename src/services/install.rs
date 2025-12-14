use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use strsim::jaro;
use tracing::{debug, warn};

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
        staging_dir: &Path,
        expected_name: &str,
    ) -> Result<(String, Plugin, Vec<PathBuf>)> {
        let addons_dir = staging_dir.join("addons");

        if !self.file_service.directory_exists(&addons_dir) {
            bail!(
                "No 'addons' directory found in staging: {}",
                staging_dir.display()
            );
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
            bail!("No folders found inside {}/addons", staging_dir.display());
        }

        let parsed_plugins = self
            .parser
            .create_plugins_from_addon_folders(source, &addon_folders)
            .unwrap_or_else(|e| {
                warn!(
                    "Failed to parse plugin configs: {}. Will create minimal plugins.",
                    e
                );
                Vec::new()
            });

        let (main_plugin_folder, mut main_plugin) = if !parsed_plugins.is_empty() {
            self.parser
                .determine_best_main_plugin_match(&parsed_plugins, expected_name)?
        } else {
            warn!("No valid plugin.cfg files found. Using folder name matching.");
            let best_folder = addon_folders
                .iter()
                .max_by(|a, b| {
                    let score_a = jaro(
                        &a.to_string_lossy().to_lowercase(),
                        &expected_name.to_lowercase(),
                    );
                    let score_b = jaro(
                        &b.to_string_lossy().to_lowercase(),
                        &expected_name.to_lowercase(),
                    );
                    score_a
                        .partial_cmp(&score_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .cloned()
                .unwrap_or_else(|| addon_folders[0].clone());

            let folder_name = best_folder.to_string_lossy().to_string();
            (
                folder_name.clone(),
                Plugin {
                    source: Some(source.clone()),
                    plugin_cfg_path: None,
                    title: folder_name,
                    ..Plugin::default()
                },
            )
        };

        let sub_assets: Vec<String> = addon_folders
            .iter()
            .filter_map(|f| {
                let folder_name = f.to_string_lossy().to_string();
                if folder_name != main_plugin_folder {
                    Some(folder_name)
                } else {
                    None
                }
            })
            .collect();

        main_plugin.sub_assets = sub_assets;

        debug!(
            "Discovered main plugin '{}' with {} sub-assets (plugin.cfg: {})",
            main_plugin.title,
            main_plugin.sub_assets.len(),
            if main_plugin.plugin_cfg_path.is_some() {
                "found"
            } else {
                "not found"
            }
        );

        Ok((main_plugin_folder, main_plugin, addon_folders))
    }

    fn install_from_cache(
        &self,
        staging_dir: &Path,
        addon_folders: &[PathBuf],
    ) -> Result<Vec<PathBuf>> {
        let project_addons_dir = self.app_config.get_addon_folder_path();
        let staging_addons_dir = staging_dir.join("addons");
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
                .find(|inst| inst.can_handle(plugin.source.clone()))
                .expect("No installer found for plugin source");

            installed_plugins.push(installer.install(
                idx,
                plugins.len(),
                self,
                plugin,
                operation_manager.clone(),
            ));
        }

        let installed_plugins = futures::future::try_join_all(installed_plugins).await?;

        self.cleanup_cache()?;

        Ok(installed_plugins.into_iter().collect())
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
