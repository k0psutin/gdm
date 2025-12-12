use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::models::{Plugin, PluginSource};
use crate::services::FileService;

/// Helper struct for parsing plugin.cfg files and managing plugin discovery
#[derive(Clone)]
pub struct PluginParser {
    file_service: Arc<dyn FileService + Send + Sync>,
}

impl PluginParser {
    pub fn new(file_service: Arc<dyn FileService + Send + Sync>) -> Self {
        Self { file_service }
    }

    /// Parses a plugin.cfg file and creates a Plugin instance
    pub fn parse_plugin_cfg(&self, path: &Path, plugin_source: PluginSource) -> Result<Plugin> {
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

        Ok(Plugin::new(
            Some(plugin_source),
            Some(path.to_path_buf()),
            title,
            version,
            None,
            vec![],
        ))
    }

    /// Finds all plugin.cfg files in addon folders and creates Plugin instances
    pub fn create_plugins_from_addon_folders(
        &self,
        plugin_source: &PluginSource,
        addon_folders: &[PathBuf],
    ) -> Result<Vec<(PathBuf, Plugin)>> {
        let mut plugins: Vec<(PathBuf, Plugin)> = vec![];
        for folder in addon_folders {
            let path = PathBuf::from("addons").join(folder);
            if let Some(plugin_cfg_path) = self.file_service.find_plugin_cfg_file_greedy(&path)? {
                plugins.push((
                    folder.clone(),
                    self.parse_plugin_cfg(&plugin_cfg_path, plugin_source.clone())?,
                ));
            }
        }
        Ok(plugins)
    }

    /// Enriches a plugin with sub-asset information
    pub fn enrich_with_sub_assets(
        &self,
        plugin: &Plugin,
        plugins: &[(PathBuf, Plugin)],
        addon_folders: &[PathBuf],
    ) -> Result<Plugin> {
        let main_plugin_folder = plugins
            .iter()
            .find(|(_, p)| p.title == plugin.title)
            .map(|(path, _)| path)
            .with_context(|| format!("Main plugin folder not found for {}", plugin.title))?;

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

    /// Determines the best matching plugin from a list based on name similarity
    /// Uses Jaro similarity to compare both folder names and plugin titles
    /// Returns the folder name and the plugin
    pub fn determine_best_main_plugin_match(
        &self,
        plugins: &[(PathBuf, Plugin)],
        main_plugin_name: &str,
    ) -> Result<(String, Plugin)> {
        let best_match = plugins.iter().fold(
            (PathBuf::new(), Plugin::default(), 0.0),
            |mut best, (path, plugin)| {
                let folder_name = path.to_string_lossy().to_string();
                let jaro_filename = strsim::jaro(
                    &folder_name.to_lowercase(),
                    &main_plugin_name.to_lowercase(),
                );
                let jaro_title =
                    strsim::jaro(&folder_name.to_lowercase(), &plugin.title.to_lowercase());
                let max_jaro = jaro_filename.max(jaro_title);
                if max_jaro > best.2 {
                    best = (PathBuf::from(path), plugin.clone(), max_jaro);
                }
                best
            },
        );
        Ok((best_match.0.to_string_lossy().to_string(), best_match.1))
    }
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

    #[test]
    fn test_parse_plugin_cfg_basic() {
        let content = r#"name="Test Plugin"
version="1.0.0""#;

        let mut files = HashMap::new();
        files.insert("addons/test/plugin.cfg".to_string(), content.to_string());

        let mock_service = create_mock_file_service_with_files(files);
        let parser = PluginParser::new(Arc::new(mock_service));

        let result = parser.parse_plugin_cfg(
            Path::new("addons/test/plugin.cfg"),
            PluginSource::AssetLibrary {
                asset_id: "123".to_string(),
            },
        );

        assert!(result.is_ok());
        let plugin = result.unwrap();
        assert_eq!(plugin.title, "Test Plugin");
        assert_eq!(plugin.get_version(), "1.0.0");
    }

    #[test]
    fn test_determine_best_main_plugin_match_exact() {
        let mock_service = MockDefaultFileService::new();
        let parser = PluginParser::new(Arc::new(mock_service));

        let plugins = vec![(
            PathBuf::from("gut"),
            Plugin::new(
                Some(PluginSource::Git {
                    url: "https://example.com/gut".to_string(),
                    reference: "main".to_string(),
                }),
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
                Some(PluginSource::Git {
                    url: "https://example.com/gut".to_string(),
                    reference: "main".to_string(),
                }),
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
    fn test_enrich_with_sub_assets() {
        let mock_service = MockDefaultFileService::new();
        let parser = PluginParser::new(Arc::new(mock_service));

        let main_plugin = Plugin::new(
            Some(PluginSource::AssetLibrary {
                asset_id: "123".to_string(),
            }),
            Some(PathBuf::from("addons/main/plugin.cfg")),
            "Main Plugin".to_string(),
            "1.0.0".to_string(),
            None,
            vec![],
        );

        let plugins = vec![
            (PathBuf::from("main"), main_plugin.clone()),
            (
                PathBuf::from("sub1"),
                Plugin::new(
                    Some(PluginSource::AssetLibrary {
                        asset_id: "123".to_string(),
                    }),
                    Some(PathBuf::from("addons/sub1/plugin.cfg")),
                    "Sub Plugin 1".to_string(),
                    "1.0.0".to_string(),
                    None,
                    vec![],
                ),
            ),
        ];

        let addon_folders = vec![PathBuf::from("main"), PathBuf::from("sub1")];

        let result = parser.enrich_with_sub_assets(&main_plugin, &plugins, &addon_folders);
        assert!(result.is_ok());
        let enriched = result.unwrap();
        assert_eq!(enriched.sub_assets, vec!["sub1"]);
    }
}
