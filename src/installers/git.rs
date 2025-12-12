use crate::config::{AppConfig, DefaultAppConfig};
use crate::installers::PluginInstaller;
use crate::models::{Plugin, PluginSource};
use crate::services::{GitService, PluginParser, StagingService};
use crate::ui::OperationManager;

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tracing::error;

pub struct GitInstaller {
    git_service: Arc<dyn GitService + Send + Sync>,
    parser: Arc<PluginParser>,
    staging_service: Option<Arc<StagingService>>,
}

impl GitInstaller {
    pub fn new(git_service: Arc<dyn GitService + Send + Sync>, parser: Arc<PluginParser>) -> Self {
        Self {
            git_service,
            parser,
            staging_service: None,
        }
    }

    /// Create a new GitInstaller with staging support
    pub fn with_staging(
        git_service: Arc<dyn GitService + Send + Sync>,
        parser: Arc<PluginParser>,
        staging_service: Arc<StagingService>,
    ) -> Self {
        Self {
            git_service,
            parser,
            staging_service: Some(staging_service),
        }
    }

    /// Helper method to extract addon folders from staging directory
    fn extract_addon_folders_from_staging(&self, staging_dir: &PathBuf) -> Result<Vec<PathBuf>> {
        let addons_dir = staging_dir.join("addons");
        if !addons_dir.exists() {
            anyhow::bail!("No addons directory found at {}", addons_dir.display());
        }

        let mut addon_folders = Vec::new();
        for entry in std::fs::read_dir(&addons_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir()
                && let Some(folder_name) = path.file_name()
            {
                addon_folders.push(PathBuf::from(folder_name));
            }
        }

        Ok(addon_folders)
    }

    /// Install using the unified staging workflow
    /// This method uses StagingService for consistent plugin discovery and installation
    async fn install_with_staging(
        &self,
        plugins: Vec<Plugin>,
        operation_manager: Arc<OperationManager>,
        start_index: usize,
        total_count: usize,
        staging_service: &StagingService,
        addons_dir: &std::path::Path,
    ) -> Result<BTreeMap<String, Plugin>> {
        let mut join_set = JoinSet::new();

        for (i, plugin_def) in plugins.iter().enumerate() {
            if let Some(PluginSource::Git { url, reference }) = &plugin_def.source {
                let git_service = self.git_service.clone();
                let url = url.clone();
                let reference = if reference.is_empty() {
                    "main".to_string()
                } else {
                    reference.clone()
                };

                let global_index = start_index + i;

                let pb = operation_manager.add_progress_bar(
                    global_index,
                    total_count,
                    &url,
                    &reference,
                )?;

                pb.enable_steady_tick(Duration::from_millis(100));

                join_set.spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        pb.set_message("Cloning...");

                        // Phase 1: Clone to staging (.gdm/cache/<repo_name>/)
                        let (staging_dir, file_count) = git_service
                            .shallow_fetch_repository(&url, Some(reference.clone()))
                            .with_context(|| format!("Failed to fetch {}", url))?;

                        pb.set_message("Extracting files...");
                        pb.set_length(file_count as u64);
                        pb.set_position(0);

                        pb.finish_and_clear();

                        Ok::<(PathBuf, String, String), anyhow::Error>((
                            staging_dir,
                            url,
                            reference,
                        ))
                    })
                    .await;

                    match result {
                        Ok(inner_result) => inner_result,
                        Err(join_err) => Err(anyhow::anyhow!("Task panicked: {}", join_err)),
                    }
                });
            }
        }

        let mut staging_results = Vec::new();

        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(Ok((staging_dir, url, reference))) => {
                    staging_results.push((staging_dir, url, reference));
                }
                Ok(Err(e)) => {
                    error!("Error cloning git repository: {:#}", e);
                }
                Err(join_err) => {
                    error!("Git clone task failed to join: {}", join_err);
                }
            }
        }

        let mut installed_results = BTreeMap::new();

        // Process each staged repository using StagingService
        for (staging_dir, url, reference) in staging_results {
            // Phase 3 & 4: Discover and Analyze - find all addons and determine main plugin
            let plugin_source = PluginSource::Git {
                url: url.clone(),
                reference: reference.clone(),
            };

            let repo_name = self.git_service.extract_repo_name_from_src(&staging_dir)?;
            let (folder_name, main_plugin, addon_folders) = staging_service
                .discover_and_analyze_plugins(&staging_dir, &plugin_source, &repo_name)
                .context("Failed to discover and analyze git plugins")?;

            // Phase 5: Validate (already validated in discover_and_analyze_plugins)
            if addon_folders.is_empty() {
                anyhow::bail!("No addon folders found in staging directory");
            }

            // Phase 6: Install from staging
            staging_service
                .install_from_staging(&staging_dir, &addon_folders, addons_dir)
                .context("Failed to install git plugins from staging")?;

            // Update plugin path to point to production addons folder
            let mut main_plugin_mut = main_plugin;
            if let Some(cfg_path) = &main_plugin_mut.plugin_cfg_path {
                // E.g., ".gdm/Gut/addons/gut/plugin.cfg" -> "gut/plugin.cfg"
                if let Some(addons_idx) = cfg_path.rfind("/addons/") {
                    // Found "/addons/" in the path, extract everything after it
                    let relative_path = &cfg_path[addons_idx + "/addons/".len()..];
                    main_plugin_mut.plugin_cfg_path = Some(format!("addons/{}", relative_path));
                } else {
                    // Fallback: construct path from folder name
                    main_plugin_mut.plugin_cfg_path =
                        Some(format!("addons/{}/plugin.cfg", folder_name));
                }
            }

            // Phase 7: Cleanup - staging directory cleanup
            staging_service
                .cleanup_staging(&staging_dir)
                .context("Failed to cleanup git staging")?;

            installed_results.insert(folder_name, main_plugin_mut);
        }

        Ok(installed_results)
    }
}

#[async_trait]
impl PluginInstaller for GitInstaller {
    fn can_handle(&self, source: &PluginSource) -> bool {
        matches!(source, PluginSource::Git { .. })
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
            let app_config = DefaultAppConfig::default();
            let addons_dir = app_config.get_addon_folder_path();
            return self
                .install_with_staging(
                    plugins,
                    operation_manager,
                    start_index,
                    total_count,
                    staging_service,
                    addons_dir.as_path(),
                )
                .await;
        }

        // Fallback to legacy workflow
        let mut join_set = JoinSet::new();

        for (i, plugin_def) in plugins.iter().enumerate() {
            if let Some(PluginSource::Git { url, reference }) = &plugin_def.source {
                let git_service = self.git_service.clone();
                let parser = self.parser.clone();
                let url = url.clone();
                let reference = if reference.is_empty() {
                    "main".to_string()
                } else {
                    reference.clone()
                };

                let global_index = start_index + i;

                let pb = operation_manager.add_progress_bar(
                    global_index,
                    total_count,
                    &url,
                    &reference,
                )?;

                pb.enable_steady_tick(Duration::from_millis(100));

                join_set.spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        pb.set_message("Cloning...");

                        let (src, file_count) = git_service
                            .shallow_fetch_repository(&url, Some(reference.clone()))
                            .with_context(|| format!("Failed to fetch {}", url))?;

                        pb.set_message("Extracting files...");
                        pb.set_length(file_count as u64);
                        pb.set_position(0);

                        let main_plugin_name = git_service.extract_repo_name_from_src(&src)?;

                        let addon_folders = git_service.move_downloaded_addons(&src, pb.clone())?;

                        let plugin_source = PluginSource::Git {
                            url: url.clone(),
                            reference: reference.clone(),
                        };

                        let plugins_found = parser
                            .create_plugins_from_addon_folders(&plugin_source, &addon_folders)?;

                        let (folder_name, main_plugin) = parser
                            .determine_best_main_plugin_match(&plugins_found, &main_plugin_name)?;

                        let plugin_with_sub_assets = parser.enrich_with_sub_assets(
                            &main_plugin,
                            &plugins_found,
                            &addon_folders,
                        )?;

                        pb.finish_and_clear();

                        Ok::<BTreeMap<String, Plugin>, anyhow::Error>(BTreeMap::from([(
                            folder_name,
                            plugin_with_sub_assets,
                        )]))
                    })
                    .await;

                    match result {
                        Ok(inner_result) => inner_result,
                        Err(join_err) => Err(anyhow::anyhow!("Task panicked: {}", join_err)),
                    }
                });
            }
        }

        let mut installed_results = BTreeMap::new();

        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(Ok(map)) => {
                    installed_results.extend(map);
                }
                Ok(Err(e)) => {
                    error!("Error installing git plugin: {:#}", e);
                }
                Err(join_err) => {
                    error!("Git installation task failed to join: {}", join_err);
                }
            }
        }

        Ok(installed_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DefaultAppConfig;
    use crate::services::{MockDefaultFileService, MockDefaultGitService, PluginParser};
    use semver::Version;

    fn create_test_plugin(title: &str, version: &str) -> Plugin {
        Plugin {
            source: Some(PluginSource::Git {
                url: "https://github.com/test/repo".to_string(),
                reference: "main".to_string(),
            }),
            plugin_cfg_path: Some(format!("addons/{}/plugin.cfg", title.to_lowercase())),
            title: title.to_string(),
            version: Version::parse(version).unwrap_or(Version::new(1, 0, 0)),
            sub_assets: vec![],
            license: None,
        }
    }

    #[test]
    fn test_git_installer_can_handle_git_source() {
        let mock_git = MockDefaultGitService::new();
        let mock_fs = MockDefaultFileService::new();
        let parser = Arc::new(PluginParser::new(Arc::new(mock_fs)));
        let installer = GitInstaller::new(Arc::new(mock_git), parser);

        let git_source = PluginSource::Git {
            url: "https://github.com/test/repo".to_string(),
            reference: "main".to_string(),
        };

        assert!(installer.can_handle(&git_source));
    }

    #[test]
    fn test_git_installer_cannot_handle_asset_library_source() {
        let mock_git = MockDefaultGitService::new();
        let mock_fs = MockDefaultFileService::new();
        let parser = Arc::new(PluginParser::new(Arc::new(mock_fs)));
        let installer = GitInstaller::new(Arc::new(mock_git), parser);

        let asset_source = PluginSource::AssetLibrary {
            asset_id: "123".to_string(),
        };

        assert!(!installer.can_handle(&asset_source));
    }

    #[test]
    fn test_with_staging_constructor() {
        let mock_git = MockDefaultGitService::new();
        let mock_fs = MockDefaultFileService::new();
        let app_config = DefaultAppConfig::default();

        let parser = Arc::new(PluginParser::new(Arc::new(mock_fs)));
        let mock_fs2 = MockDefaultFileService::new();
        let staging_service = Arc::new(StagingService::new(
            Arc::new(mock_fs2),
            parser.clone(),
            &app_config,
        ));

        let installer =
            GitInstaller::with_staging(Arc::new(mock_git), parser, staging_service.clone());

        assert!(installer.staging_service.is_some());
    }

    #[test]
    fn test_extract_addon_folders_from_staging_success() {
        let mock_git = MockDefaultGitService::new();
        let mock_fs = MockDefaultFileService::new();
        let parser = Arc::new(PluginParser::new(Arc::new(mock_fs)));
        let installer = GitInstaller::new(Arc::new(mock_git), parser);

        // Create a temporary staging directory
        let temp_staging = std::env::temp_dir().join("test_staging");
        let addons_dir = temp_staging.join("addons");
        std::fs::create_dir_all(&addons_dir).unwrap();
        std::fs::create_dir_all(addons_dir.join("plugin1")).unwrap();
        std::fs::create_dir_all(addons_dir.join("plugin2")).unwrap();

        let result = installer.extract_addon_folders_from_staging(&temp_staging);
        assert!(result.is_ok());

        let folders = result.unwrap();
        assert_eq!(folders.len(), 2);
        assert!(folders.contains(&PathBuf::from("plugin1")));
        assert!(folders.contains(&PathBuf::from("plugin2")));

        // Cleanup
        std::fs::remove_dir_all(temp_staging).ok();
    }

    #[test]
    fn test_extract_addon_folders_from_staging_no_addons_dir() {
        let mock_git = MockDefaultGitService::new();
        let mock_fs = MockDefaultFileService::new();
        let parser = Arc::new(PluginParser::new(Arc::new(mock_fs)));
        let installer = GitInstaller::new(Arc::new(mock_git), parser);

        // Create staging without addons directory
        let temp_staging = std::env::temp_dir().join("test_staging_no_addons");
        std::fs::create_dir_all(&temp_staging).unwrap();

        let result = installer.extract_addon_folders_from_staging(&temp_staging);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No addons directory found")
        );

        // Cleanup
        std::fs::remove_dir_all(temp_staging).ok();
    }

    #[test]
    fn test_extract_addon_folders_from_staging_empty_addons() {
        let mock_git = MockDefaultGitService::new();
        let mock_fs = MockDefaultFileService::new();
        let parser = Arc::new(PluginParser::new(Arc::new(mock_fs)));
        let installer = GitInstaller::new(Arc::new(mock_git), parser);

        // Create staging with empty addons directory
        let temp_staging = std::env::temp_dir().join("test_staging_empty");
        let addons_dir = temp_staging.join("addons");
        std::fs::create_dir_all(&addons_dir).unwrap();

        let result = installer.extract_addon_folders_from_staging(&temp_staging);
        assert!(result.is_ok());

        let folders = result.unwrap();
        assert_eq!(folders.len(), 0);

        // Cleanup
        std::fs::remove_dir_all(temp_staging).ok();
    }

    #[test]
    fn test_extract_addon_folders_from_staging_ignores_files() {
        let mock_git = MockDefaultGitService::new();
        let mock_fs = MockDefaultFileService::new();
        let parser = Arc::new(PluginParser::new(Arc::new(mock_fs)));
        let installer = GitInstaller::new(Arc::new(mock_git), parser);

        // Create staging with files and directories
        let temp_staging = std::env::temp_dir().join("test_staging_mixed");
        let addons_dir = temp_staging.join("addons");
        std::fs::create_dir_all(&addons_dir).unwrap();
        std::fs::create_dir_all(addons_dir.join("plugin1")).unwrap();
        std::fs::write(addons_dir.join("README.md"), "test").unwrap();
        std::fs::write(addons_dir.join("file.txt"), "test").unwrap();

        let result = installer.extract_addon_folders_from_staging(&temp_staging);
        assert!(result.is_ok());

        let folders = result.unwrap();
        assert_eq!(folders.len(), 1); // Only the directory, not files
        assert!(folders.contains(&PathBuf::from("plugin1")));

        // Cleanup
        std::fs::remove_dir_all(temp_staging).ok();
    }
}
