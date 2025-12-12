use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info};

use crate::config::AppConfig;
use crate::models::{Plugin, PluginSource};
use crate::services::{FileService, PluginParser};

/// Type alias for the result of plugin discovery
/// (plugins_with_cfg, all_addon_folders)
type DiscoveredPlugins = (Vec<(PathBuf, Plugin)>, Vec<PathBuf>);

/// Service for managing staged plugin installations
/// Provides a unified workflow for all installer types
pub struct StagingService {
    file_service: Arc<dyn FileService + Send + Sync>,
    parser: Arc<PluginParser>,
    cache_dir: PathBuf,
}

impl StagingService {
    pub fn new(
        file_service: Arc<dyn FileService + Send + Sync>,
        parser: Arc<PluginParser>,
        app_config: &dyn AppConfig,
    ) -> Self {
        let cache_dir = app_config.get_cache_folder_path().to_path_buf();
        Self {
            file_service,
            parser,
            cache_dir,
        }
    }

    /// Creates a staging directory for a source
    /// Returns the path to the staging directory
    pub fn create_staging_dir(&self, source_id: &str) -> Result<PathBuf> {
        let staging_dir = self.cache_dir.join(source_id);

        if self.file_service.directory_exists(&staging_dir) {
            debug!(
                "Staging directory already exists: {}",
                staging_dir.display()
            );
            self.file_service
                .remove_dir_all(&staging_dir)
                .with_context(|| {
                    format!(
                        "Failed to clean existing staging directory: {}",
                        staging_dir.display()
                    )
                })?;
        }

        self.file_service
            .create_directory(&staging_dir)
            .with_context(|| {
                format!(
                    "Failed to create staging directory: {}",
                    staging_dir.display()
                )
            })?;

        info!("Created staging directory: {}", staging_dir.display());
        Ok(staging_dir)
    }

    /// Scans staging dir for addons and determines the main plugin
    /// Returns: (main_plugin_folder_name, main_plugin, all_addon_folders)
    ///
    /// This method:
    /// 1. Finds ALL addon folders under staging_dir/addons/
    /// 2. Searches each folder for plugin.cfg files
    /// 3. Matches plugin names against the expected main_plugin_name
    /// 4. Returns the best matching plugin and ALL addon folders
    pub fn discover_and_analyze_plugins(
        &self,
        staging_dir: &Path,
        source: &PluginSource,
        main_plugin_name: &str,
    ) -> Result<(String, Plugin, Vec<PathBuf>)> {
        let addons_dir = staging_dir.join("addons");

        if !self.file_service.directory_exists(&addons_dir) {
            anyhow::bail!(
                "No addons directory found in staging: {}",
                addons_dir.display()
            );
        }

        // Find ALL addon folders (with or without plugin.cfg)
        let addon_folders = self
            .file_service
            .read_dir(&addons_dir)?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    let path = e.path();
                    if path.is_dir() {
                        path.file_name().map(PathBuf::from)
                    } else {
                        None
                    }
                })
            })
            .collect::<Vec<_>>();

        debug!("Found {} addon folders in staging", addon_folders.len());

        if addon_folders.is_empty() {
            anyhow::bail!("No addon folders found in {}", addons_dir.display());
        }

        // Create plugins only for addon folders that have plugin.cfg
        let plugins_with_cfg = self.parser.create_plugins_from_addon_folders_with_base(
            source,
            &addon_folders,
            Some(staging_dir),
        )?;

        info!(
            "Found {} plugins with plugin.cfg out of {} total addon folders",
            plugins_with_cfg.len(),
            addon_folders.len()
        );

        // Determine the best matching main plugin
        let (main_folder_name, main_plugin) = self
            .parser
            .determine_best_main_plugin_match(&plugins_with_cfg, main_plugin_name)?;

        // Enrich the main plugin with sub_assets (all folders except the main one)
        let enriched_plugin =
            self.parser
                .enrich_with_sub_assets(&main_plugin, &plugins_with_cfg, &addon_folders)?;

        Ok((main_folder_name, enriched_plugin, addon_folders))
    }

    /// Scans staging dir for addons and creates Plugin objects
    /// Returns a list of (folder_name, plugin) tuples for all discovered plugins
    pub fn discover_plugins(
        &self,
        staging_dir: &Path,
        source: &PluginSource,
    ) -> Result<Vec<(PathBuf, Plugin)>> {
        let (plugins, _) = self.discover_plugins_and_folders(staging_dir, source)?;
        Ok(plugins)
    }

    /// Scans staging dir for addons and creates Plugin objects
    /// Returns both plugins (with plugin.cfg) and ALL addon folders (with or without plugin.cfg)
    /// Returns: (plugins_with_cfg, all_addon_folders)
    pub fn discover_plugins_and_folders(
        &self,
        staging_dir: &Path,
        source: &PluginSource,
    ) -> Result<DiscoveredPlugins> {
        let addons_dir = staging_dir.join("addons");

        if !self.file_service.directory_exists(&addons_dir) {
            anyhow::bail!(
                "No addons directory found in staging: {}",
                addons_dir.display()
            );
        }

        // Find all addon folders (both with and without plugin.cfg)
        let addon_folders = self
            .file_service
            .read_dir(&addons_dir)?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    let path = e.path();
                    if path.is_dir() {
                        path.file_name().map(PathBuf::from)
                    } else {
                        None
                    }
                })
            })
            .collect::<Vec<_>>();

        debug!("Found {} addon folders in staging", addon_folders.len());

        // Create plugins only for addon folders that have plugin.cfg
        // Pass staging_dir as base so it looks in staging_dir/addons/<folder> instead of cwd/addons/<folder>
        let plugins = self.parser.create_plugins_from_addon_folders_with_base(
            source,
            &addon_folders,
            Some(staging_dir),
        )?;

        info!(
            "Discovered {} plugins with plugin.cfg out of {} total addon folders",
            plugins.len(),
            addon_folders.len()
        );
        Ok((plugins, addon_folders))
    }

    /// Determines the main plugin and enriches it with sub-assets
    /// Returns (folder_name, main_plugin_with_sub_assets)
    pub fn analyze_plugins(
        &self,
        plugins: &[(PathBuf, Plugin)],
        main_plugin_name: &str,
        addon_folders: &[PathBuf],
    ) -> Result<(String, Plugin)> {
        let (folder_name, main_plugin) = self
            .parser
            .determine_best_main_plugin_match(plugins, main_plugin_name)?;

        let plugin_with_sub_assets =
            self.parser
                .enrich_with_sub_assets(&main_plugin, plugins, addon_folders)?;

        Ok((folder_name, plugin_with_sub_assets))
    }

    /// Validates plugins before installation
    pub fn validate_plugins(&self, plugins: &[(PathBuf, Plugin)]) -> Result<()> {
        if plugins.is_empty() {
            anyhow::bail!("No plugins found to validate");
        }

        for (folder, plugin) in plugins {
            // Validate that plugin has required fields
            if plugin.title.is_empty() {
                anyhow::bail!("Plugin in folder '{}' has empty title", folder.display());
            }

            // Validate plugin.cfg exists if path is specified
            if let Some(cfg_path) = &plugin.plugin_cfg_path {
                let path = PathBuf::from(cfg_path);
                if !self.file_service.file_exists(&path)? {
                    anyhow::bail!("Plugin config not found: {}", cfg_path);
                }
            }
        }

        debug!("All plugins validated successfully");
        Ok(())
    }

    /// Moves validated addons from staging to production
    /// Returns the paths that were moved
    pub fn install_from_staging(
        &self,
        staging_dir: &Path,
        addon_folders: &[PathBuf],
        target_dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        let mut moved_paths = Vec::new();
        let staging_addons_dir = staging_dir.join("addons");

        for folder in addon_folders {
            let src = staging_addons_dir.join(folder);
            let dest = target_dir.join(folder);

            if !self.file_service.directory_exists(&src) {
                anyhow::bail!("Source addon folder not found: {}", src.display());
            }

            // Remove existing destination if it exists
            if self.file_service.directory_exists(&dest) {
                debug!("Removing existing addon: {}", dest.display());
                self.file_service.remove_dir_all(&dest)?;
            }

            // Ensure parent directory exists
            if let Some(parent) = dest.parent()
                && !self.file_service.directory_exists(parent)
            {
                self.file_service.create_directory(parent)?;
            }

            // Move addon from staging to production
            debug!("Moving {} -> {}", src.display(), dest.display());
            self.file_service.rename(&src, &dest).with_context(|| {
                format!(
                    "Failed to move addon from {} to {}",
                    src.display(),
                    dest.display()
                )
            })?;

            moved_paths.push(dest);
        }

        info!("Installed {} addons from staging", moved_paths.len());
        Ok(moved_paths)
    }

    /// Cleans up staging directory
    pub fn cleanup_staging(&self, staging_dir: &Path) -> Result<()> {
        if self.file_service.directory_exists(staging_dir) {
            self.file_service
                .remove_dir_all(staging_dir)
                .with_context(|| {
                    format!(
                        "Failed to cleanup staging directory: {}",
                        staging_dir.display()
                    )
                })?;
            info!("Cleaned up staging directory: {}", staging_dir.display());
        }
        Ok(())
    }

    /// Gets the staging directory path for a source ID
    pub fn get_staging_path(&self, source_id: &str) -> PathBuf {
        self.cache_dir.join(source_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DefaultAppConfig;
    use crate::services::MockDefaultFileService;
    use semver::Version;

    fn create_test_plugin(title: &str, version: &str, source: PluginSource) -> Plugin {
        Plugin {
            source: Some(source),
            plugin_cfg_path: Some(format!("addons/{}/plugin.cfg", title.to_lowercase())),
            title: title.to_string(),
            version: Version::parse(version).unwrap_or(Version::new(1, 0, 0)),
            sub_assets: vec![],
            license: None,
        }
    }

    #[test]
    fn test_create_staging_dir() {
        let mut mock_fs = MockDefaultFileService::new();
        let app_config = DefaultAppConfig::default();
        let cache_dir = app_config.get_cache_folder_path();
        let expected_staging = cache_dir.join("test-source");
        let expected_staging_clone1 = expected_staging.clone();
        let expected_staging_clone2 = expected_staging.clone();

        mock_fs
            .expect_directory_exists()
            .withf(move |p: &Path| p == expected_staging_clone1)
            .return_once(|_| false);

        mock_fs
            .expect_create_directory()
            .withf(move |p: &Path| p == expected_staging_clone2)
            .return_once(|_| Ok(()));

        let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
        let staging_service = StagingService::new(Arc::new(mock_fs), parser, &app_config);

        let result = staging_service.create_staging_dir("test-source");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_staging);
    }

    #[test]
    fn test_create_staging_dir_removes_existing() {
        let mut mock_fs = MockDefaultFileService::new();
        let app_config = DefaultAppConfig::default();
        let cache_dir = app_config.get_cache_folder_path();
        let expected_staging = cache_dir.join("test-source");
        let expected_staging_clone1 = expected_staging.clone();
        let expected_staging_clone2 = expected_staging.clone();
        let expected_staging_clone3 = expected_staging.clone();

        // Expect directory exists check
        mock_fs
            .expect_directory_exists()
            .withf(move |p: &Path| p == expected_staging_clone1)
            .return_once(|_| true);

        // Expect removal of existing directory
        mock_fs
            .expect_remove_dir_all()
            .withf(move |p: &Path| p == expected_staging_clone2)
            .return_once(|_| Ok(()));

        // Expect creation of new directory
        mock_fs
            .expect_create_directory()
            .withf(move |p: &Path| p == expected_staging_clone3)
            .return_once(|_| Ok(()));

        let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
        let staging_service = StagingService::new(Arc::new(mock_fs), parser, &app_config);

        let result = staging_service.create_staging_dir("test-source");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_staging);
    }

    #[test]
    fn test_validate_plugins_with_valid_plugins() {
        let mut mock_fs = MockDefaultFileService::new();
        let app_config = DefaultAppConfig::default();

        // Setup file existence checks for plugin.cfg files
        mock_fs
            .expect_file_exists()
            .times(2)
            .returning(|_| Ok(true));

        let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
        let staging_service = StagingService::new(Arc::new(mock_fs), parser, &app_config);

        let source = PluginSource::Git {
            url: "https://github.com/test/plugin".to_string(),
            reference: "main".to_string(),
        };

        let plugin1 = create_test_plugin("TestPlugin", "1.0.0", source.clone());
        let plugin2 = create_test_plugin("AnotherPlugin", "2.0.0", source);

        let plugins = vec![
            (PathBuf::from("test_plugin"), plugin1),
            (PathBuf::from("another_plugin"), plugin2),
        ];

        let result = staging_service.validate_plugins(&plugins);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_plugins_with_empty_list() {
        let mock_fs = MockDefaultFileService::new();
        let mock_parser_fs = MockDefaultFileService::new();
        let app_config = DefaultAppConfig::default();
        let parser = Arc::new(PluginParser::new(Arc::new(mock_parser_fs)));
        let staging_service = StagingService::new(Arc::new(mock_fs), parser, &app_config);

        let plugins: Vec<(PathBuf, Plugin)> = vec![];

        let result = staging_service.validate_plugins(&plugins);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No plugins found"));
    }

    #[test]
    fn test_validate_plugins_with_empty_title() {
        let mock_fs = MockDefaultFileService::new();
        let mock_parser_fs = MockDefaultFileService::new();
        let app_config = DefaultAppConfig::default();
        let parser = Arc::new(PluginParser::new(Arc::new(mock_parser_fs)));
        let staging_service = StagingService::new(Arc::new(mock_fs), parser, &app_config);

        let source = PluginSource::Git {
            url: "https://github.com/test/plugin".to_string(),
            reference: "main".to_string(),
        };

        let mut plugin = create_test_plugin("TestPlugin", "1.0.0", source);
        plugin.title = "".to_string(); // Empty title

        let plugins = vec![(PathBuf::from("test_plugin"), plugin)];

        let result = staging_service.validate_plugins(&plugins);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty title"));
    }

    #[test]
    fn test_validate_plugins_with_missing_config() {
        let mut mock_fs = MockDefaultFileService::new();
        let app_config = DefaultAppConfig::default();

        // Plugin config file doesn't exist
        mock_fs
            .expect_file_exists()
            .times(1)
            .returning(|_| Ok(false));

        let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
        let staging_service = StagingService::new(Arc::new(mock_fs), parser, &app_config);

        let source = PluginSource::Git {
            url: "https://github.com/test/plugin".to_string(),
            reference: "main".to_string(),
        };

        let plugin = create_test_plugin("TestPlugin", "1.0.0", source);
        let plugins = vec![(PathBuf::from("test_plugin"), plugin)];

        let result = staging_service.validate_plugins(&plugins);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Plugin config not found")
        );
    }

    #[test]
    fn test_cleanup_staging_existing_dir() {
        let mut mock_fs = MockDefaultFileService::new();
        let app_config = DefaultAppConfig::default();
        let staging_dir = PathBuf::from("/tmp/test-staging");
        let staging_dir_clone1 = staging_dir.clone();
        let staging_dir_clone2 = staging_dir.clone();

        mock_fs
            .expect_directory_exists()
            .withf(move |p: &Path| p == staging_dir_clone1)
            .return_once(|_| true);

        mock_fs
            .expect_remove_dir_all()
            .withf(move |p: &Path| p == staging_dir_clone2)
            .return_once(|_| Ok(()));

        let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
        let staging_service = StagingService::new(Arc::new(mock_fs), parser, &app_config);

        let result = staging_service.cleanup_staging(&staging_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cleanup_staging_nonexistent_dir() {
        let mut mock_fs = MockDefaultFileService::new();
        let app_config = DefaultAppConfig::default();
        let staging_dir = PathBuf::from("/tmp/test-staging");
        let staging_dir_clone = staging_dir.clone();

        mock_fs
            .expect_directory_exists()
            .withf(move |p: &Path| p == staging_dir_clone)
            .return_once(|_| false);

        // Should not call remove_dir_all if directory doesn't exist

        let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
        let staging_service = StagingService::new(Arc::new(mock_fs), parser, &app_config);

        let result = staging_service.cleanup_staging(&staging_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_staging_path() {
        let mock_fs = MockDefaultFileService::new();
        let app_config = DefaultAppConfig::default();
        let cache_dir = app_config.get_cache_folder_path();

        let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
        let staging_service = StagingService::new(Arc::new(mock_fs), parser, &app_config);

        let result = staging_service.get_staging_path("my-source");
        assert_eq!(result, cache_dir.join("my-source"));
    }

    #[test]
    fn test_install_from_staging_success() {
        let mut mock_fs = MockDefaultFileService::new();
        let app_config = DefaultAppConfig::default();

        let staging_dir = PathBuf::from("/tmp/staging");
        let target_dir = PathBuf::from("./addons");
        let addon_folders = vec![PathBuf::from("plugin1")];

        let src = staging_dir.join("addons").join(&addon_folders[0]);
        let dest = target_dir.join(&addon_folders[0]);
        let src_clone = src.clone();
        let dest_clone = dest.clone();

        // Check if source exists
        mock_fs
            .expect_directory_exists()
            .withf(move |p: &Path| p == src_clone)
            .return_once(|_| true);

        // Check if destination exists (no)
        mock_fs
            .expect_directory_exists()
            .withf(move |p: &Path| p == dest_clone)
            .return_once(|_| false);

        // Check if parent exists
        mock_fs
            .expect_directory_exists()
            .times(1)
            .returning(|_| true);

        // Rename operation
        mock_fs.expect_rename().times(1).returning(|_, _| Ok(()));

        let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
        let staging_service = StagingService::new(Arc::new(mock_fs), parser, &app_config);

        let result =
            staging_service.install_from_staging(&staging_dir, &addon_folders, &target_dir);
        assert!(result.is_ok());
        let moved_paths = result.unwrap();
        assert_eq!(moved_paths.len(), 1);
    }

    #[test]
    fn test_install_from_staging_removes_existing_destination() {
        let mut mock_fs = MockDefaultFileService::new();
        let app_config = DefaultAppConfig::default();

        let staging_dir = PathBuf::from("/tmp/staging");
        let target_dir = PathBuf::from("./addons");
        let addon_folders = vec![PathBuf::from("plugin1")];

        let src = staging_dir.join("addons").join(&addon_folders[0]);
        let dest = target_dir.join(&addon_folders[0]);
        let src_clone = src.clone();
        let dest_clone = dest.clone();

        // Source exists
        mock_fs
            .expect_directory_exists()
            .withf(move |p: &Path| p == src_clone)
            .return_once(|_| true);

        // Destination exists (should be removed)
        mock_fs
            .expect_directory_exists()
            .withf(move |p: &Path| p == dest_clone)
            .return_once(|_| true);

        // Remove existing destination
        mock_fs
            .expect_remove_dir_all()
            .times(1)
            .returning(|_| Ok(()));

        // Parent exists
        mock_fs
            .expect_directory_exists()
            .times(1)
            .returning(|_| true);

        // Rename operation
        mock_fs.expect_rename().times(1).returning(|_, _| Ok(()));

        let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
        let staging_service = StagingService::new(Arc::new(mock_fs), parser, &app_config);

        let result =
            staging_service.install_from_staging(&staging_dir, &addon_folders, &target_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_install_from_staging_missing_source() {
        let mut mock_fs = MockDefaultFileService::new();
        let app_config = DefaultAppConfig::default();

        let staging_dir = PathBuf::from("/tmp/staging");
        let target_dir = PathBuf::from("./addons");
        let addon_folders = vec![PathBuf::from("missing_plugin")];

        let src = staging_dir.join("addons").join(&addon_folders[0]);
        let src_clone = src.clone();

        // Source doesn't exist
        mock_fs
            .expect_directory_exists()
            .withf(move |p: &Path| p == src_clone)
            .return_once(|_| false);

        let parser = Arc::new(PluginParser::new(Arc::new(MockDefaultFileService::new())));
        let staging_service = StagingService::new(Arc::new(mock_fs), parser, &app_config);

        let result =
            staging_service.install_from_staging(&staging_dir, &addon_folders, &target_dir);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Source addon folder not found")
        );
    }

    #[test]
    fn test_discover_plugins_looks_in_wrong_directory() {
        // This test demonstrates the bug: discover_plugins passes addon folder names
        // to create_plugins_from_addon_folders, which then looks for them in
        // "addons/<folder>" relative to CWD, not in staging_dir/addons/<folder>

        use std::fs;
        use temp_dir::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let staging_dir = temp_dir.path().join("staging");
        let addons_dir = staging_dir.join("addons");
        let gut_dir = addons_dir.join("gut");

        // Create the staging directory structure
        fs::create_dir_all(&gut_dir).unwrap();
        fs::write(
            gut_dir.join("plugin.cfg"),
            r#"name="Gut"
version="9.5.1""#,
        )
        .unwrap();

        let app_config = DefaultAppConfig::default();
        let file_service = Arc::new(crate::services::DefaultFileService);
        let parser = Arc::new(PluginParser::new(file_service.clone()));
        let staging_service = StagingService::new(file_service, parser, &app_config);

        let source = PluginSource::Git {
            url: "https://github.com/bitwes/Gut".to_string(),
            reference: "v9.5.1".to_string(),
        };

        // This should find the plugin in staging_dir/addons/gut
        // But currently it looks for addons/gut relative to CWD
        let result = staging_service.discover_plugins(&staging_dir, &source);

        // This will fail with the current implementation because it looks in
        // the wrong directory (CWD/addons/gut instead of staging_dir/addons/gut)
        assert!(
            result.is_ok(),
            "discover_plugins should find plugins in staging directory: {:?}",
            result.err()
        );

        let plugins = result.unwrap();
        assert_eq!(plugins.len(), 1, "Should find 1 plugin");
        assert_eq!(plugins[0].1.title, "Gut");
        assert_eq!(plugins[0].1.get_version(), "9.5.1");
    }
}
