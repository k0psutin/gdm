use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::models::{Plugin, PluginSource};
use crate::services::FileService;

/// Helper struct for parsing plugin.cfg files and managing plugin discovery
#[derive(Clone)]
pub struct PluginParser {
    file_service: Arc<dyn FileService + Send + Sync>,
}

impl Default for PluginParser {
    fn default() -> Self {
        let file_service = Arc::new(crate::services::DefaultFileService);
        Self::new(file_service)
    }
}

impl PluginParser {
    pub fn new(file_service: Arc<dyn FileService + Send + Sync>) -> Self {
        Self { file_service }
    }

    /// Parses a plugin.cfg file and creates a Plugin instance
    pub fn parse_plugin_cfg(
        &self,
        path: &Path,
        plugin_source: &PluginSource,
        base_dir: Option<&Path>,
    ) -> Result<Plugin> {
        let content = self.file_service.read_file_cached(path)?;
        let mut title = String::new();
        let mut version = String::new();

        for line in content.lines() {
            if let Some(name) = line.strip_prefix("name=") {
                title = name.trim_matches('"').to_string();
            } else if let Some(_version) = line.strip_prefix("version=") {
                version = _version.trim_matches('"').to_string();
            }
        }

        // Determine the relative plugin.cfg path if base_dir is provided
        let plugin_config_path = if let Some(base) = base_dir {
            Some(
                path.strip_prefix(base)
                    .with_context(|| {
                        format!(
                            "Failed to get relative plugin.cfg path for {:?} with base {:?}",
                            path, base
                        )
                    })?
                    .to_path_buf(),
            )
        } else {
            Some(path.to_path_buf())
        };

        Ok(Plugin::new(
            plugin_source.clone(),
            plugin_config_path,
            title,
            version,
            None,
            vec![],
        ))
    }

    /// Finds all plugin.cfg files in addon folders and creates Plugin instances
    /// with an optional base directory
    pub fn create_plugins_from_addon_folders_with_base(
        &self,
        plugin_source: &PluginSource,
        addon_folders: &[PathBuf],
        base_dir: Option<&Path>,
    ) -> Result<Vec<(PathBuf, Plugin)>> {
        let mut plugins: Vec<(PathBuf, Plugin)> = vec![];
        for folder in addon_folders {
            let path = if let Some(base) = base_dir {
                base.join("addons").join(folder)
            } else {
                PathBuf::from("addons").join(folder)
            };

            if let Some(plugin_cfg_path) = self.file_service.find_plugin_cfg_file_greedy(&path)? {
                plugins.push((
                    folder.clone(),
                    self.parse_plugin_cfg(&plugin_cfg_path, plugin_source, base_dir)?,
                ));
            }
        }
        Ok(plugins)
    }

    /// Enriches a plugin with sub-asset information
    pub fn enrich_with_sub_assets(
        &self,
        plugin: &Plugin,
        main_plugin_folder: &Path,
        addon_folders: &[PathBuf],
    ) -> Result<Plugin> {
        let sub_assets: Vec<String> = addon_folders
            .iter()
            .filter_map(|folder| {
                if folder != main_plugin_folder {
                    Some(folder.to_string_lossy().to_string())
                } else {
                    None
                }
            })
            .collect();

        Ok(Plugin::new(
            plugin.source.clone(),
            plugin.plugin_cfg_path.as_ref().map(PathBuf::from),
            plugin.title.clone(),
            plugin.get_version(),
            plugin.license.clone(),
            sub_assets,
        ))
    }

    /// Determines the best matching plugin from a list using exact normalized names.
    /// Returns the folder name and the plugin
    pub fn determine_best_main_plugin_match(
        &self,
        plugins: &[(PathBuf, Plugin)],
        main_plugin_name: &str,
    ) -> Result<(String, Plugin)> {
        let candidates: Vec<_> = plugins
            .iter()
            .filter(|(_, plugin)| plugin.plugin_cfg_path.is_some())
            .collect();

        if candidates.is_empty() {
            bail!("No plugin.cfg found in the addon folders")
        }

        let requested_name = normalize_identifier(main_plugin_name);
        let folder_matches: Vec<_> = candidates
            .iter()
            .copied()
            .filter(|(path, _)| {
                path.file_name()
                    .map(|name| normalize_identifier(&name.to_string_lossy()) == requested_name)
                    .unwrap_or(false)
            })
            .collect();

        let selected = if !folder_matches.is_empty() {
            folder_matches
        } else {
            candidates
                .iter()
                .copied()
                .filter(|(_, plugin)| normalize_identifier(&plugin.title) == requested_name)
                .collect()
        };

        let selected = if selected.len() == 1 {
            selected[0]
        } else if selected.len() > 1 {
            bail!(ambiguous_plugin_match_error(main_plugin_name, &selected))
        } else if candidates.len() == 1 {
            candidates[0]
        } else {
            bail!(ambiguous_plugin_match_error(main_plugin_name, &candidates))
        };

        let (folder, plugin) = selected;

        Ok((folder.to_string_lossy().to_string(), plugin.clone()))
    }
}

fn normalize_identifier(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .filter(|character| !matches!(character, '-' | '_' | '.'))
        .collect()
}

fn ambiguous_plugin_match_error(requested_name: &str, candidates: &[&(PathBuf, Plugin)]) -> String {
    let folders = candidates
        .iter()
        .map(|(path, _)| path.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Ambiguous main dependency match for '{}'; candidates: {}",
        requested_name, folders
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::MockDefaultFileService;
    use std::collections::HashMap;

    fn create_mock_file_service_with_files(
        files: HashMap<String, String>,
    ) -> MockDefaultFileService {
        let mut mock = MockDefaultFileService::new();

        // Setup file existence checks
        for path in files.keys() {
            let path_clone = path.clone();
            mock.expect_file_exists()
                .withf(move |p: &Path| p.to_str() == Some(&path_clone))
                .returning(|_| Ok(true));
        }

        // Setup file reading
        for (path, content) in files.iter() {
            let path_clone = path.clone();
            let content_clone = content.clone();
            mock.expect_read_file_cached()
                .withf(move |p: &Path| p.to_str() == Some(&path_clone))
                .returning(move |_| Ok(content_clone.clone()));
        }

        mock
    }

    // parse_plugin_cfg tests

    #[test]
    fn test_parse_plugin_cfg_should_return_correct_plugin_without_base_dir() {
        let content = r#"name="Test Plugin"
version="1.0.0""#;

        let mut files = HashMap::new();
        files.insert("addons/test/plugin.cfg".to_string(), content.to_string());

        let mock_service = create_mock_file_service_with_files(files);
        let parser = PluginParser::new(Arc::new(mock_service));

        let result = parser.parse_plugin_cfg(
            Path::new("addons/test/plugin.cfg"),
            &PluginSource::AssetStore {
                asset_slug: "123".to_string(),
                publisher_slug: "123_publisher".to_string(),
            },
            None,
        );

        assert!(result.is_ok());
        let plugin = result.unwrap();
        assert_eq!(plugin.title, "Test Plugin");
        assert_eq!(plugin.get_version(), "1.0.0");
        assert_eq!(
            plugin.plugin_cfg_path,
            Some("addons/test/plugin.cfg".into())
        );
    }

    #[test]
    fn test_parse_plugin_cfg_should_return_correct_plugin_with_base_dir() {
        let content = r#"name="Test Plugin"
version="1.0.0""#;

        let mut files = HashMap::new();
        files.insert(
            ".gdm/test_plugin/addons/test/plugin.cfg".to_string(),
            content.to_string(),
        );

        let mock_service = create_mock_file_service_with_files(files);
        let parser = PluginParser::new(Arc::new(mock_service));

        let result = parser.parse_plugin_cfg(
            Path::new(".gdm/test_plugin/addons/test/plugin.cfg"),
            &PluginSource::AssetStore {
                asset_slug: "123".to_string(),
                publisher_slug: "123_publisher".to_string(),
            },
            Some(Path::new(".gdm/test_plugin")),
        );

        assert!(result.is_ok());
        let plugin = result.unwrap();
        assert_eq!(plugin.title, "Test Plugin");
        assert_eq!(plugin.get_version(), "1.0.0");
        assert_eq!(
            plugin.plugin_cfg_path,
            Some("addons/test/plugin.cfg".into()),
            "Should store relative path"
        );
    }

    #[test]
    fn test_parse_plugin_cfg_handles_malformed_lines_without_panicking() {
        let parser = PluginParser::default();
        let plugin = parser
            .parse_plugin_cfg(
                Path::new("tests/mocks/plugin_cfg/malformed_plugin.cfg"),
                &PluginSource::AssetStore {
                    asset_slug: "asset".to_string(),
                    publisher_slug: "publisher".to_string(),
                },
                None,
            )
            .unwrap();

        assert_eq!(plugin.title, "Broken Fixture");
        assert_eq!(plugin.get_version(), "1.0.0");
    }

    // determine_best_main_plugin_match tests

    #[test]
    fn test_determine_best_main_plugin_match_exact() {
        let mock_service = MockDefaultFileService::new();
        let parser = PluginParser::new(Arc::new(mock_service));

        let plugins = vec![(
            PathBuf::from("gut"),
            Plugin::new(
                PluginSource::Git {
                    url: "https://example.com/gut".to_string(),
                    reference: "main".to_string(),
                    publisher_slug: "example".to_string(),
                    asset_slug: "gut".to_string(),
                },
                Some(PathBuf::from("addons/gut/plugin.cfg")),
                "Gut".to_string(),
                "9.5.1".to_string(),
                None,
                vec![],
            ),
        )];

        let result = parser.determine_best_main_plugin_match(&plugins, "gut");
        assert!(result.is_ok());
        let (folder_name, plugin) = result.unwrap();
        assert_eq!(folder_name, "gut");
        assert_eq!(plugin.title, "Gut");
    }

    #[test]
    fn test_determine_best_main_plugin_match_similar() {
        let mock_service = MockDefaultFileService::new();
        let parser = PluginParser::new(Arc::new(mock_service));

        let plugins = vec![(
            PathBuf::from("godot_unit_test"),
            Plugin::new(
                PluginSource::Git {
                    url: "https://example.com/gut".to_string(),
                    reference: "main".to_string(),
                    publisher_slug: "example".to_string(),
                    asset_slug: "gut".to_string(),
                },
                Some(PathBuf::from("addons/godot_unit_test/plugin.cfg")),
                "GUT - Godot Unit Testing".to_string(),
                "9.5.1".to_string(),
                None,
                vec![],
            ),
        )];

        let result = parser.determine_best_main_plugin_match(&plugins, "Gut");
        assert!(result.is_ok());
        let (folder_name, plugin) = result.unwrap();
        assert_eq!(folder_name, "godot_unit_test");
        assert_eq!(plugin.title, "GUT - Godot Unit Testing");
    }

    #[test]
    fn test_determine_best_main_plugin_match_preserves_actual_folder_name() {
        let mock_service = MockDefaultFileService::new();
        let parser = PluginParser::new(Arc::new(mock_service));

        let plugins = vec![(
            PathBuf::from("licenses"),
            Plugin::new(
                PluginSource::AssetStore {
                    asset_slug: "license-manager".to_string(),
                    publisher_slug: "kenyoni".to_string(),
                },
                Some(PathBuf::from("addons/licenses/plugin.cfg")),
                "License Manager".to_string(),
                "1.0.0".to_string(),
                Some("MIT".to_string()),
                vec![],
            ),
        )];

        let (folder_name, _) = parser
            .determine_best_main_plugin_match(&plugins, "license-manager")
            .unwrap();

        assert_eq!(folder_name, "licenses");
    }

    #[test]
    fn test_determine_best_main_plugin_match_normalizes_slug_and_folder_separators() {
        let mock_service = MockDefaultFileService::new();
        let parser = PluginParser::new(Arc::new(mock_service));

        let plugins = vec![
            (
                PathBuf::from("demo.plugin"),
                Plugin::new(
                    PluginSource::AssetStore {
                        asset_slug: "demo-plugin".to_string(),
                        publisher_slug: "mock-publisher".to_string(),
                    },
                    Some(PathBuf::from("addons/demo.plugin/plugin.cfg")),
                    "demo.plugin".to_string(),
                    "1.0.0".to_string(),
                    Some("MIT".to_string()),
                    vec![],
                ),
            ),
            (
                PathBuf::from("demo"),
                Plugin::new(
                    PluginSource::AssetStore {
                        asset_slug: "demo-plugin".to_string(),
                        publisher_slug: "mock-publisher".to_string(),
                    },
                    Some(PathBuf::from("addons/demo/plugin.cfg")),
                    "demo".to_string(),
                    "1.0.0".to_string(),
                    Some("MIT".to_string()),
                    vec![],
                ),
            ),
        ];

        let (folder_name, plugin) = parser
            .determine_best_main_plugin_match(&plugins, "demo-plugin")
            .unwrap();

        assert_eq!(folder_name, "demo.plugin");
        assert_eq!(plugin.title, "demo.plugin");
    }

    #[test]
    fn test_determine_best_main_plugin_match_uses_plugin_title_when_folder_does_not_match() {
        let mock_service = MockDefaultFileService::new();
        let parser = PluginParser::new(Arc::new(mock_service));

        let plugin = |folder: &str, title: &str| {
            (
                PathBuf::from(folder),
                Plugin::new(
                    PluginSource::AssetStore {
                        asset_slug: "asset".to_string(),
                        publisher_slug: "publisher".to_string(),
                    },
                    Some(format!("addons/{folder}/plugin.cfg").into()),
                    title.to_string(),
                    "1.0.0".to_string(),
                    Some("MIT".to_string()),
                    vec![],
                ),
            )
        };
        let plugins = vec![
            plugin("unrelated", "Target Plugin"),
            plugin("other", "Other"),
        ];

        let (folder_name, selected) = parser
            .determine_best_main_plugin_match(&plugins, "target_plugin")
            .unwrap();

        assert_eq!(folder_name, "unrelated");
        assert_eq!(selected.title, "Target Plugin");
    }

    #[test]
    fn test_determine_best_main_plugin_match_rejects_ambiguous_candidates() {
        let mock_service = MockDefaultFileService::new();
        let parser = PluginParser::new(Arc::new(mock_service));

        let plugin = |folder: &str| {
            (
                PathBuf::from(folder),
                Plugin::new(
                    PluginSource::AssetStore {
                        asset_slug: "asset".to_string(),
                        publisher_slug: "publisher".to_string(),
                    },
                    Some(format!("addons/{folder}/plugin.cfg").into()),
                    "Plugin".to_string(),
                    "1.0.0".to_string(),
                    Some("MIT".to_string()),
                    vec![],
                ),
            )
        };
        let plugins = vec![plugin("first"), plugin("second")];

        let error = parser
            .determine_best_main_plugin_match(&plugins, "missing")
            .unwrap_err();

        assert!(error.to_string().contains("Ambiguous"));
        assert!(error.to_string().contains("first"));
        assert!(error.to_string().contains("second"));
    }

    #[test]
    fn test_determine_best_main_plugin_match_fails_without_plugin_config() {
        let mock_service = MockDefaultFileService::new();
        let parser = PluginParser::new(Arc::new(mock_service));

        let plugins = vec![(
            PathBuf::from("licenses"),
            Plugin::new(
                PluginSource::AssetStore {
                    asset_slug: "license-manager".to_string(),
                    publisher_slug: "kenyoni".to_string(),
                },
                None,
                "licenses".to_string(),
                "0.0.0".to_string(),
                None,
                vec![],
            ),
        )];

        let error = parser
            .determine_best_main_plugin_match(&plugins, "license-manager")
            .unwrap_err();

        assert!(error.to_string().contains("plugin.cfg"));
    }

    #[test]
    fn test_enrich_with_sub_assets() {
        let mock_service = MockDefaultFileService::new();
        let parser = PluginParser::new(Arc::new(mock_service));

        let main_plugin = Plugin::new(
            PluginSource::AssetStore {
                asset_slug: "123".to_string(),
                publisher_slug: "123_publisher".to_string(),
            },
            Some(PathBuf::from("addons/main/plugin.cfg")),
            "Main Plugin".to_string(),
            "1.0.0".to_string(),
            None,
            vec![],
        );

        let addon_folders = vec![PathBuf::from("main"), PathBuf::from("sub1")];

        let result = parser.enrich_with_sub_assets(&main_plugin, Path::new("main"), &addon_folders);
        assert!(result.is_ok());
        let enriched = result.unwrap();
        assert_eq!(enriched.sub_assets, vec!["sub1"]);
    }

    #[test]
    fn test_enrich_with_sub_assets_uses_selected_folder_when_titles_duplicate() {
        let mock_service = MockDefaultFileService::new();
        let parser = PluginParser::new(Arc::new(mock_service));

        let source = PluginSource::AssetStore {
            asset_slug: "asset".to_string(),
            publisher_slug: "publisher".to_string(),
        };
        let second = Plugin::new(
            source,
            Some(PathBuf::from("addons/second/plugin.cfg")),
            "Duplicate".to_string(),
            "1.0.0".to_string(),
            None,
            vec![],
        );

        let enriched = parser
            .enrich_with_sub_assets(
                &second,
                Path::new("second"),
                &[PathBuf::from("first"), PathBuf::from("second")],
            )
            .unwrap();

        assert_eq!(enriched.sub_assets, vec!["first"]);
    }

    // Missing plugin.cfg behavior tests

    #[test]
    fn test_create_plugins_from_addon_folders_returns_no_candidates_without_plugin_cfg() {
        let mut mock_service = MockDefaultFileService::new();

        // Mock that no plugin.cfg file is found
        mock_service
            .expect_find_plugin_cfg_file_greedy()
            .returning(|_| Ok(None));

        let parser = PluginParser::new(Arc::new(mock_service));

        let plugin_source = PluginSource::Git {
            url: "https://github.com/example/plugin".to_string(),
            reference: "main".to_string(),
            publisher_slug: "example".to_string(),
            asset_slug: "plugin".to_string(),
        };

        let addon_folders = vec![PathBuf::from("my_plugin")];

        let result = parser.create_plugins_from_addon_folders_with_base(
            &plugin_source,
            &addon_folders,
            None,
        );

        let plugins = result.unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_create_plugins_from_addon_folders_returns_no_candidates_when_all_cfgs_are_missing() {
        let mut mock_service = MockDefaultFileService::new();

        // Mock that no plugin.cfg files are found for any folder
        mock_service
            .expect_find_plugin_cfg_file_greedy()
            .returning(|_| Ok(None));

        let parser = PluginParser::new(Arc::new(mock_service));

        let plugin_source = PluginSource::AssetStore {
            asset_slug: "456".to_string(),
            publisher_slug: "456_publisher".to_string(),
        };

        let addon_folders = vec![
            PathBuf::from("plugin_a"),
            PathBuf::from("plugin_b"),
            PathBuf::from("plugin_c"),
        ];

        let result = parser.create_plugins_from_addon_folders_with_base(
            &plugin_source,
            &addon_folders,
            None,
        );

        let plugins = result.unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_create_plugins_from_addon_folders_returns_no_candidates_with_base_dir() {
        let mut mock_service = MockDefaultFileService::new();

        // Mock that no plugin.cfg file is found
        mock_service
            .expect_find_plugin_cfg_file_greedy()
            .returning(|_| Ok(None));

        let parser = PluginParser::new(Arc::new(mock_service));

        let plugin_source = PluginSource::Git {
            url: "https://github.com/example/plugin".to_string(),
            reference: "v1.0.0".to_string(),
            publisher_slug: "example".to_string(),
            asset_slug: "plugin".to_string(),
        };

        let addon_folders = vec![PathBuf::from("test_addon")];
        let base_dir = Some(Path::new(".gdm/cache/test"));

        let result = parser.create_plugins_from_addon_folders_with_base(
            &plugin_source,
            &addon_folders,
            base_dir,
        );

        let plugins = result.unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_create_plugins_from_addon_folders_no_fallback_when_plugin_cfg_found() {
        let mut mock_service = MockDefaultFileService::new();

        let plugin_cfg_content = r#"name="Actual Plugin Name"
version="2.1.3""#;

        // Mock finding a plugin.cfg file
        mock_service
            .expect_find_plugin_cfg_file_greedy()
            .returning(|_| Ok(Some(PathBuf::from("addons/real_plugin/plugin.cfg"))));

        // Mock reading the plugin.cfg file
        mock_service
            .expect_read_file_cached()
            .returning(move |_| Ok(plugin_cfg_content.to_string()));

        let parser = PluginParser::new(Arc::new(mock_service));

        let plugin_source = PluginSource::AssetStore {
            asset_slug: "789".to_string(),
            publisher_slug: "789_publisher".to_string(),
        };

        let addon_folders = vec![PathBuf::from("real_plugin")];

        let result = parser.create_plugins_from_addon_folders_with_base(
            &plugin_source,
            &addon_folders,
            None,
        );

        assert!(result.is_ok());
        let plugins = result.unwrap();

        // Should have 1 real plugin (not fallback)
        assert_eq!(plugins.len(), 1);

        let (folder, plugin) = &plugins[0];
        assert_eq!(folder, &PathBuf::from("real_plugin"));
        assert_eq!(plugin.title, "Actual Plugin Name"); // From plugin.cfg, not folder name
        assert_eq!(plugin.get_version(), "2.1.3"); // From plugin.cfg, not "0.0.0"
        assert_eq!(
            plugin.plugin_cfg_path,
            Some("addons/real_plugin/plugin.cfg".to_string())
        );
    }

    #[test]
    fn test_create_plugins_from_addon_folders_mixed_found_and_missing() {
        let mut mock_service = MockDefaultFileService::new();

        let plugin_cfg_content = r#"name="Found Plugin"
version="1.5.0""#;

        // Mock: first folder has plugin.cfg, second doesn't
        mock_service
            .expect_find_plugin_cfg_file_greedy()
            .returning(move |path| {
                if path.to_str().unwrap().contains("has_config") {
                    Ok(Some(PathBuf::from("addons/has_config/plugin.cfg")))
                } else {
                    Ok(None)
                }
            });

        // Mock reading the found plugin.cfg file
        mock_service
            .expect_read_file_cached()
            .returning(move |_| Ok(plugin_cfg_content.to_string()));

        let parser = PluginParser::new(Arc::new(mock_service));

        let plugin_source = PluginSource::Git {
            url: "https://github.com/test/mixed".to_string(),
            reference: "main".to_string(),
            publisher_slug: "test".to_string(),
            asset_slug: "mixed".to_string(),
        };

        let addon_folders = vec![PathBuf::from("has_config")];

        let result = parser.create_plugins_from_addon_folders_with_base(
            &plugin_source,
            &addon_folders,
            None,
        );

        assert!(result.is_ok());
        let plugins = result.unwrap();

        // When at least one plugin.cfg is found, no fallback is used
        assert_eq!(plugins.len(), 1);

        // The found plugin should have actual values from plugin.cfg
        let (_, plugin) = &plugins[0];
        assert_eq!(plugin.title, "Found Plugin");
        assert_eq!(plugin.get_version(), "1.5.0");
    }

    #[test]
    fn test_missing_plugin_cfg_with_special_characters_in_folder_name_returns_no_candidates() {
        let mut mock_service = MockDefaultFileService::new();

        mock_service
            .expect_find_plugin_cfg_file_greedy()
            .returning(|_| Ok(None));

        let parser = PluginParser::new(Arc::new(mock_service));

        let plugin_source = PluginSource::AssetStore {
            asset_slug: "999".to_string(),
            publisher_slug: "999_publisher".to_string(),
        };

        let addon_folders = vec![PathBuf::from("plugin-with-dashes_and_underscores")];

        let result = parser.create_plugins_from_addon_folders_with_base(
            &plugin_source,
            &addon_folders,
            None,
        );

        let plugins = result.unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_empty_addon_folders_return_no_candidates() {
        let mut mock_service = MockDefaultFileService::new();

        mock_service.expect_find_plugin_cfg_file_greedy().times(0); // Should not be called with empty folders

        let parser = PluginParser::new(Arc::new(mock_service));

        let plugin_source = PluginSource::Git {
            url: "https://github.com/test/empty".to_string(),
            reference: "main".to_string(),
            publisher_slug: "test".to_string(),
            asset_slug: "empty".to_string(),
        };

        let addon_folders: Vec<PathBuf> = vec![];

        let result = parser.create_plugins_from_addon_folders_with_base(
            &plugin_source,
            &addon_folders,
            None,
        );

        assert!(result.is_ok());
        let plugins = result.unwrap();

        // Should return empty vec, not fallback plugins
        assert_eq!(plugins.len(), 0);
    }
}
