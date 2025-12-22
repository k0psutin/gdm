use anyhow::{Result, bail};
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
    app_config: Box<dyn AppConfig>,
    parser: Arc<PluginParser>,
    installers: Vec<Box<dyn PluginInstaller>>,
}

impl Default for DefaultInstallService {
    fn default() -> Self {
        let file_service = Arc::new(DefaultFileService);
        let app_config = Box::new(DefaultAppConfig::default());
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
        app_config: Box<dyn AppConfig>,
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

            self.file_service.rename(&src, &dest)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MockDefaultAppConfig;
    use crate::installers::PluginInstaller;
    use crate::services::MockDefaultFileService;
    use anyhow::{Context, anyhow};

    // Mock installer for testing
    struct MockPluginInstaller {
        can_handle_result: bool,
        plugins: Vec<Plugin>,
        should_fail: bool,
        error_message: Option<String>,
    }

    impl MockPluginInstaller {
        fn new(can_handle: bool) -> Self {
            Self {
                can_handle_result: can_handle,
                plugins: Vec::new(),
                should_fail: false,
                error_message: None,
            }
        }

        fn with_plugin(mut self, plugin: Plugin) -> Self {
            self.plugins.push(plugin);
            self
        }

        fn with_failure(mut self, error_msg: &str) -> Self {
            self.should_fail = true;
            self.error_message = Some(error_msg.to_string());
            self
        }
    }

    #[async_trait]
    impl PluginInstaller for MockPluginInstaller {
        fn can_handle(&self, _source: Option<PluginSource>) -> bool {
            self.can_handle_result
        }

        async fn install(
            &self,
            _index: usize,
            _total: usize,
            _install_service: &dyn InstallService,
            plugin: &Plugin,
            _operation_manager: Arc<OperationManager>,
        ) -> Result<(String, Plugin)> {
            if self.should_fail {
                return Err(anyhow!(
                    self.error_message
                        .clone()
                        .unwrap_or_else(|| "Installation failed".to_string())
                ));
            }

            // Find the plugin in our list that matches the requested plugin's title
            let found_plugin = self
                .plugins
                .iter()
                .find(|p| p.title == plugin.title)
                .ok_or_else(|| anyhow!("Plugin '{}' not found in mock installer", plugin.title))?;

            // Return the plugin's title as the key and the plugin itself
            Ok((found_plugin.title.clone(), found_plugin.clone()))
        }
    }

    fn create_test_plugin(title: &str, version: &str, source: Option<PluginSource>) -> Plugin {
        Plugin {
            source,
            plugin_cfg_path: Some(format!("addons/{}/plugin.cfg", title)),
            title: title.to_string(),
            version: version.to_string(),
            sub_assets: vec![],
            license: Some("MIT".to_string()),
        }
    }

    mod discover_and_analyze_plugins_tests {
        use super::*;

        #[test]
        fn test_discover_fails_when_no_addons_directory() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mock_app_config = MockDefaultAppConfig::new();
            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));

            let cache_dir = PathBuf::from("/cache");
            let addons_dir = cache_dir.join("addons");

            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(addons_dir.clone()))
                .times(1)
                .returning(|_| false);

            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![],
            );

            let source = PluginSource::AssetLibrary {
                asset_id: "123".to_string(),
            };

            let result = service.discover_and_analyze_plugins(&source, &cache_dir, "test-plugin");

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("No 'addons' directory found")
            );
        }

        #[test]
        fn test_discover_fails_when_addons_directory_is_empty() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mock_app_config = MockDefaultAppConfig::new();

            let cache_dir = PathBuf::from("/cache");
            let addons_dir = cache_dir.join("addons");

            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(addons_dir.clone()))
                .times(1)
                .returning(|_| true);

            // Return empty iterator for read_dir
            mock_file_service
                .expect_read_dir()
                .with(mockall::predicate::eq(addons_dir.clone()))
                .times(1)
                .returning(|_path| {
                    // Create a temporary empty directory for testing
                    let temp_dir = std::env::temp_dir().join("test_empty_addons");
                    std::fs::create_dir_all(&temp_dir).ok();
                    let result = std::fs::read_dir(&temp_dir);
                    std::fs::remove_dir_all(&temp_dir).ok();
                    result.context("Failed to read directory")
                });

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![],
            );

            let source = PluginSource::AssetLibrary {
                asset_id: "123".to_string(),
            };

            let result = service.discover_and_analyze_plugins(&source, &cache_dir, "test-plugin");

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("No folders found inside")
            );
        }

        #[test]
        fn test_discover_succeeds_with_valid_addon_structure() {
            // This test would need a more complex setup with actual file system or
            // a more sophisticated mocking strategy. For now, documenting the expected behavior:
            // 1. Cache dir exists with addons/ subdirectory
            // 2. Addons directory contains plugin folders
            // 3. Parser can create plugins from those folders
            // 4. Best match is determined based on plugin name
            // 5. Plugin is enriched with sub-assets
        }
    }

    mod install_from_cache_tests {
        use std::slice;

        use super::*;

        #[test]
        fn test_install_from_cache_creates_parent_directory_if_missing() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mut mock_app_config = MockDefaultAppConfig::new();

            let project_addons = PathBuf::from("/project/addons");
            let cache_dir = PathBuf::from("/cache");
            let staging_addons = cache_dir.join("addons");
            let addon_folder = PathBuf::from("test_addon");

            let project_addons_clone = project_addons.clone();
            mock_app_config
                .expect_get_addon_folder_path()
                .returning(move || project_addons_clone.clone());

            let src = staging_addons.join(&addon_folder);
            let dest = project_addons.join(&addon_folder);
            let parent = dest.parent().unwrap().to_path_buf();

            // Destination doesn't exist
            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(dest.clone()))
                .times(1)
                .returning(|_| false);

            // Parent doesn't exist
            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(parent.clone()))
                .times(1)
                .returning(|_| false);

            // Create parent directory
            mock_file_service
                .expect_create_directory()
                .with(mockall::predicate::eq(parent.clone()))
                .times(1)
                .returning(|_| Ok(()));

            // Rename succeeds
            mock_file_service
                .expect_rename()
                .with(
                    mockall::predicate::eq(src.clone()),
                    mockall::predicate::eq(dest.clone()),
                )
                .times(1)
                .returning(|_, _| Ok(()));

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![],
            );

            let result = service.install_from_cache(&cache_dir, slice::from_ref(&addon_folder));

            assert!(result.is_ok());
            let installed = result.unwrap();
            assert_eq!(installed.len(), 1);
            assert_eq!(installed[0], dest);
        }

        #[test]
        fn test_install_from_cache_removes_existing_installation() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mut mock_app_config = MockDefaultAppConfig::new();

            let project_addons = PathBuf::from("/project/addons");
            let cache_dir = PathBuf::from("/cache");
            let staging_addons = cache_dir.join("addons");
            let addon_folder = PathBuf::from("test_addon");

            let project_addons_clone = project_addons.clone();
            mock_app_config
                .expect_get_addon_folder_path()
                .returning(move || project_addons_clone.clone());

            let src = staging_addons.join(&addon_folder);
            let dest = project_addons.join(&addon_folder);
            let parent = dest.parent().unwrap().to_path_buf();

            // Destination exists - should be removed
            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(dest.clone()))
                .times(1)
                .returning(|_| true);

            // Remove existing installation
            mock_file_service
                .expect_remove_dir_all()
                .with(mockall::predicate::eq(dest.clone()))
                .times(1)
                .returning(|_| Ok(()));

            // Parent exists
            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(parent.clone()))
                .times(1)
                .returning(|_| true);

            // Rename succeeds
            mock_file_service
                .expect_rename()
                .with(
                    mockall::predicate::eq(src.clone()),
                    mockall::predicate::eq(dest.clone()),
                )
                .times(1)
                .returning(|_, _| Ok(()));

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![],
            );

            let result = service.install_from_cache(&cache_dir, slice::from_ref(&addon_folder));

            assert!(result.is_ok());
        }

        #[test]
        fn test_install_from_cache_handles_rename_failure() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mut mock_app_config = MockDefaultAppConfig::new();

            let project_addons = PathBuf::from("/project/addons");
            let cache_dir = PathBuf::from("/cache");
            let staging_addons = cache_dir.join("addons");
            let addon_folder = PathBuf::from("test_addon");

            let project_addons_clone = project_addons.clone();
            mock_app_config
                .expect_get_addon_folder_path()
                .returning(move || project_addons_clone.clone());

            let src = staging_addons.join(&addon_folder);
            let dest = project_addons.join(&addon_folder);
            let parent = dest.parent().unwrap().to_path_buf();

            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(dest.clone()))
                .times(1)
                .returning(|_| false);

            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(parent.clone()))
                .times(1)
                .returning(|_| true);

            // Rename fails
            mock_file_service
                .expect_rename()
                .with(
                    mockall::predicate::eq(src.clone()),
                    mockall::predicate::eq(dest.clone()),
                )
                .times(1)
                .returning(|_, _| Err(anyhow!("Failed to move")));

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![],
            );

            let result = service.install_from_cache(&cache_dir, slice::from_ref(&addon_folder));

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Failed to move"));
        }

        #[test]
        fn test_install_from_cache_handles_multiple_addons() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mut mock_app_config = MockDefaultAppConfig::new();

            let project_addons = PathBuf::from("/project/addons");
            let cache_dir = PathBuf::from("/cache");
            let staging_addons = cache_dir.join("addons");
            let addon_folders = vec![
                PathBuf::from("addon1"),
                PathBuf::from("addon2"),
                PathBuf::from("addon3"),
            ];

            let project_addons_clone = project_addons.clone();
            mock_app_config
                .expect_get_addon_folder_path()
                .returning(move || project_addons_clone.clone());

            for addon_folder in &addon_folders {
                let src = staging_addons.join(addon_folder);
                let dest = project_addons.join(addon_folder);
                let parent = dest.parent().unwrap().to_path_buf();

                mock_file_service
                    .expect_directory_exists()
                    .with(mockall::predicate::eq(dest.clone()))
                    .times(1)
                    .returning(|_| false);

                mock_file_service
                    .expect_directory_exists()
                    .with(mockall::predicate::eq(parent.clone()))
                    .times(1)
                    .returning(|_| true);

                mock_file_service
                    .expect_rename()
                    .with(
                        mockall::predicate::eq(src.clone()),
                        mockall::predicate::eq(dest.clone()),
                    )
                    .times(1)
                    .returning(|_, _| Ok(()));
            }

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![],
            );

            let result = service.install_from_cache(&cache_dir, &addon_folders);

            assert!(result.is_ok());
            let installed = result.unwrap();
            assert_eq!(installed.len(), 3);
        }

        #[test]
        fn test_install_from_cache_with_empty_addon_list() {
            let mock_file_service = MockDefaultFileService::new();
            let mut mock_app_config = MockDefaultAppConfig::new();
            let cache_dir = PathBuf::from("/cache");

            // Even with empty addon list, get_addon_folder_path is called once
            mock_app_config
                .expect_get_addon_folder_path()
                .times(1)
                .returning(|| PathBuf::from("/project/addons"));

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![],
            );

            let result = service.install_from_cache(&cache_dir, &[]);

            assert!(result.is_ok());
            let installed = result.unwrap();
            assert_eq!(installed.len(), 0);
        }
    }

    mod cleanup_cache_tests {
        use super::*;

        #[test]
        fn test_cleanup_cache_removes_cache_directory() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mut mock_app_config = MockDefaultAppConfig::new();

            let cache_dir = PathBuf::from("/cache");

            mock_app_config
                .expect_get_cache_folder_path()
                .return_const(PathBuf::from("/cache"));

            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(cache_dir.clone()))
                .times(1)
                .returning(|_| true);

            mock_file_service
                .expect_remove_dir_all()
                .with(mockall::predicate::eq(cache_dir.clone()))
                .times(1)
                .returning(|_| Ok(()));

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![],
            );

            let result = service.cleanup_cache();

            assert!(result.is_ok());
        }

        #[test]
        fn test_cleanup_cache_succeeds_when_cache_does_not_exist() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mut mock_app_config = MockDefaultAppConfig::new();

            let cache_dir = PathBuf::from("/cache");

            mock_app_config
                .expect_get_cache_folder_path()
                .return_const(PathBuf::from("/cache"));

            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(cache_dir.clone()))
                .times(1)
                .returning(|_| false);

            // Should not call remove_dir_all since directory doesn't exist
            mock_file_service.expect_remove_dir_all().times(0);

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![],
            );

            let result = service.cleanup_cache();

            assert!(result.is_ok());
        }

        #[test]
        fn test_cleanup_cache_handles_removal_failure() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mut mock_app_config = MockDefaultAppConfig::new();

            let cache_dir = PathBuf::from("/cache");

            mock_app_config
                .expect_get_cache_folder_path()
                .return_const(PathBuf::from("/cache"));

            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(cache_dir.clone()))
                .times(1)
                .returning(|_| true);

            mock_file_service
                .expect_remove_dir_all()
                .with(mockall::predicate::eq(cache_dir.clone()))
                .times(1)
                .returning(|_| Err(anyhow!("Permission denied")));

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![],
            );

            let result = service.cleanup_cache();

            assert!(result.is_err());
        }
    }

    mod install_tests {
        use super::*;

        #[tokio::test]
        async fn test_install_with_empty_plugin_list() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mut mock_app_config = MockDefaultAppConfig::new();

            mock_app_config
                .expect_get_cache_folder_path()
                .return_const(PathBuf::from("/cache"));

            mock_file_service
                .expect_directory_exists()
                .returning(|_| false);

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![],
            );

            let operation_manager =
                Arc::new(OperationManager::new(crate::ui::Operation::Install).unwrap());
            let result = service.install(&[], operation_manager).await;

            assert!(result.is_ok());
            let installed = result.unwrap();
            assert_eq!(installed.len(), 0);
        }

        #[tokio::test]
        async fn test_install_with_no_matching_installer() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mut mock_app_config = MockDefaultAppConfig::new();

            mock_app_config
                .expect_get_cache_folder_path()
                .return_const(PathBuf::from("/cache"));

            mock_file_service
                .expect_directory_exists()
                .returning(|_| false);

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));

            // No installers provided
            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![],
            );

            let plugin = create_test_plugin(
                "test-plugin",
                "1.0.0",
                Some(PluginSource::AssetLibrary {
                    asset_id: "123".to_string(),
                }),
            );

            let operation_manager =
                Arc::new(OperationManager::new(crate::ui::Operation::Install).unwrap());
            let result = service.install(&[plugin], operation_manager).await;

            assert!(result.is_ok());
            let installed = result.unwrap();
            assert_eq!(installed.len(), 0); // No plugins installed since no installer matched
        }

        #[tokio::test]
        async fn test_install_with_matching_installer() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mut mock_app_config = MockDefaultAppConfig::new();

            let cache_dir = PathBuf::from("/cache");
            mock_app_config
                .expect_get_cache_folder_path()
                .return_const(PathBuf::from("/cache"));

            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(cache_dir.clone()))
                .returning(|_| false);

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));

            let plugin = create_test_plugin(
                "test-plugin",
                "1.0.0",
                Some(PluginSource::AssetLibrary {
                    asset_id: "123".to_string(),
                }),
            );

            let mock_installer = MockPluginInstaller::new(true).with_plugin(plugin.clone());

            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![Box::new(mock_installer)],
            );

            let operation_manager =
                Arc::new(OperationManager::new(crate::ui::Operation::Install).unwrap());
            let result = service.install(&[plugin], operation_manager).await;

            assert!(result.is_ok());
            let installed = result.unwrap();
            assert_eq!(installed.len(), 1);
            assert!(installed.contains_key("test-plugin"));
        }

        #[tokio::test]
        async fn test_install_with_multiple_plugins_same_key_collision() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mut mock_app_config = MockDefaultAppConfig::new();

            let cache_dir = PathBuf::from("/cache");
            mock_app_config
                .expect_get_cache_folder_path()
                .return_const(PathBuf::from("/cache"));

            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(cache_dir.clone()))
                .returning(|_| false);

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));

            let plugin1 = create_test_plugin(
                "plugin1",
                "1.0.0",
                Some(PluginSource::AssetLibrary {
                    asset_id: "123".to_string(),
                }),
            );
            let plugin2 = create_test_plugin(
                "plugin2",
                "2.0.0",
                Some(PluginSource::AssetLibrary {
                    asset_id: "456".to_string(),
                }),
            );

            // Mock installer handles both plugins
            let mock_installer = MockPluginInstaller::new(true)
                .with_plugin(plugin1.clone())
                .with_plugin(plugin2.clone());

            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![Box::new(mock_installer)],
            );

            let operation_manager =
                Arc::new(OperationManager::new(crate::ui::Operation::Install).unwrap());
            let result = service
                .install(&[plugin1, plugin2], operation_manager)
                .await;

            assert!(result.is_ok());
            let installed = result.unwrap();
            // Both plugins should now be installed
            assert_eq!(installed.len(), 2);
            assert!(installed.contains_key("plugin1"));
            assert!(installed.contains_key("plugin2"));
        }

        #[tokio::test]
        async fn test_install_handles_installer_failure() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mut mock_app_config = MockDefaultAppConfig::new();

            let cache_dir = PathBuf::from("/cache");
            mock_app_config
                .expect_get_cache_folder_path()
                .return_const(PathBuf::from("/cache"));

            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(cache_dir.clone()))
                .returning(|_| false);

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));

            let mock_installer = MockPluginInstaller::new(true).with_failure("Installation failed");

            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![Box::new(mock_installer)],
            );

            let plugin = create_test_plugin(
                "test-plugin",
                "1.0.0",
                Some(PluginSource::AssetLibrary {
                    asset_id: "123".to_string(),
                }),
            );

            let operation_manager =
                Arc::new(OperationManager::new(crate::ui::Operation::Install).unwrap());
            let result = service.install(&[plugin], operation_manager).await;

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Installation failed")
            );
        }

        #[tokio::test]
        async fn test_install_cleans_up_cache_after_success() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mut mock_app_config = MockDefaultAppConfig::new();

            let cache_dir = PathBuf::from("/cache");

            mock_app_config
                .expect_get_cache_folder_path()
                .times(1)
                .return_const(PathBuf::from("/cache"));

            // First call for cleanup check, second for actual cleanup
            let cache_clone = cache_dir.clone();
            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(cache_dir.clone()))
                .times(1)
                .returning(move |_| true);

            mock_file_service
                .expect_remove_dir_all()
                .with(mockall::predicate::eq(cache_clone.clone()))
                .times(1)
                .returning(|_| Ok(()));

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));

            let plugin = create_test_plugin(
                "test-plugin",
                "1.0.0",
                Some(PluginSource::AssetLibrary {
                    asset_id: "123".to_string(),
                }),
            );

            let mock_installer = MockPluginInstaller::new(true).with_plugin(plugin.clone());

            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![Box::new(mock_installer)],
            );

            let operation_manager =
                Arc::new(OperationManager::new(crate::ui::Operation::Install).unwrap());
            let result = service.install(&[plugin], operation_manager).await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_install_with_git_source() {
            let mut mock_file_service = MockDefaultFileService::new();
            let mut mock_app_config = MockDefaultAppConfig::new();

            let cache_dir = PathBuf::from("/cache");
            mock_app_config
                .expect_get_cache_folder_path()
                .return_const(PathBuf::from("/cache"));

            mock_file_service
                .expect_directory_exists()
                .with(mockall::predicate::eq(cache_dir.clone()))
                .returning(|_| false);

            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));

            let plugin = create_test_plugin(
                "test-plugin",
                "1.0.0",
                Some(PluginSource::Git {
                    url: "https://github.com/test/plugin.git".to_string(),
                    reference: "main".to_string(),
                }),
            );

            let mock_installer = MockPluginInstaller::new(true).with_plugin(plugin.clone());

            let service = DefaultInstallService::new(
                Arc::new(mock_file_service),
                Box::new(mock_app_config),
                parser,
                vec![Box::new(mock_installer)],
            );

            let operation_manager =
                Arc::new(OperationManager::new(crate::ui::Operation::Install).unwrap());
            let result = service.install(&[plugin], operation_manager).await;

            assert!(result.is_ok());
            let installed = result.unwrap();
            assert_eq!(installed.len(), 1);
        }
    }

    mod default_install_service_tests {
        use super::*;

        #[test]
        fn test_default_install_service_creation() {
            let service = DefaultInstallService::default();
            // Just verify it can be created
            assert_eq!(service.installers.len(), 2); // AssetLibrary and Git installers
        }

        #[test]
        fn test_new_install_service_with_custom_config() {
            let mock_file_service = Arc::new(MockDefaultFileService::new());
            let mock_app_config = MockDefaultAppConfig::new();
            let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));

            let service = DefaultInstallService::new(
                mock_file_service,
                Box::new(mock_app_config),
                parser,
                vec![],
            );

            assert_eq!(service.installers.len(), 0);
        }
    }
}
