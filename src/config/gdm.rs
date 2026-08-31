use crate::config::{AppConfig, DefaultAppConfig};
use crate::models::Plugin;
use crate::services::{DefaultFileService, FileService};

use anyhow::{Context, Result};
use serde_derive::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct GdmManifest {
    #[serde(default)]
    pub dependencies: BTreeMap<String, Plugin>,
}

impl GdmManifest {
    pub fn new(dependencies: BTreeMap<String, Plugin>) -> Self {
        Self { dependencies }
    }

    pub fn dependency_by_asset_slug(&self, asset_slug: &str) -> Option<Plugin> {
        self.dependencies
            .values()
            .find(|dependency| dependency.source.asset_slug() == asset_slug)
            .cloned()
    }

    pub fn dependency_by_config_key(&self, key: &str) -> Option<Plugin> {
        self.dependencies.get(key).cloned()
    }

    pub fn with_dependencies(&self, dependencies: &BTreeMap<String, Plugin>) -> Self {
        let mut updated = self.dependencies.clone();
        updated.extend(dependencies.clone());
        Self::new(updated)
    }

    pub fn without_dependencies(&self, dependency_keys: &HashSet<String>) -> Self {
        let mut updated = self.dependencies.clone();
        for dependency_key in dependency_keys {
            updated.remove(dependency_key);
        }
        Self::new(updated)
    }

    pub fn dependencies_with_config(&self) -> BTreeMap<String, Plugin> {
        self.dependencies
            .iter()
            .filter(|(_, dependency)| dependency.plugin_cfg_path.is_some())
            .map(|(key, dependency)| (key.clone(), dependency.clone()))
            .collect()
    }
}

pub struct DefaultGdmConfig {
    pub app_config: DefaultAppConfig,
    pub file_service: Arc<dyn FileService + Send + Sync + 'static>,
}

impl Default for DefaultGdmConfig {
    fn default() -> Self {
        Self {
            file_service: Arc::new(DefaultFileService),
            app_config: DefaultAppConfig::default(),
        }
    }
}

impl DefaultGdmConfig {
    #[allow(unused)]
    pub fn new(
        app_config: DefaultAppConfig,
        file_service: Arc<dyn FileService + Send + Sync + 'static>,
    ) -> Self {
        Self {
            app_config,
            file_service,
        }
    }
}

#[cfg_attr(test, mockall::automock)]
impl GdmConfig for DefaultGdmConfig {
    fn add_dependencies(&self, dependencies: &BTreeMap<String, Plugin>) -> Result<GdmManifest> {
        debug!("Adding dependencies: {:?}", dependencies.keys());
        let manifest = self.load()?;
        let updated_manifest = manifest.with_dependencies(dependencies);
        self.save(&updated_manifest)?;
        info!(
            "Added dependencies {:?}",
            updated_manifest.dependencies.keys()
        );
        Ok(updated_manifest)
    }

    fn remove_dependencies(&self, dependency_keys: HashSet<String>) -> Result<GdmManifest> {
        debug!("Removing dependencies: {:?}", dependency_keys);
        let manifest = self.load()?;
        let updated_manifest = manifest.without_dependencies(&dependency_keys);
        self.save(&updated_manifest)?;
        info!(
            "Removed dependencies {:?}",
            updated_manifest.dependencies.keys()
        );
        Ok(updated_manifest)
    }

    fn get_dependency_by_asset_slug(&self, asset_slug: &str) -> Result<Option<Plugin>> {
        Ok(self.load()?.dependency_by_asset_slug(asset_slug))
    }

    fn get_dependency_by_config_key(&self, key: &str) -> Result<Option<Plugin>> {
        Ok(self.load()?.dependency_by_config_key(key))
    }

    fn get_dependencies(&self) -> Result<BTreeMap<String, Plugin>> {
        Ok(self.load()?.dependencies)
    }

    fn has_dependencies(&self) -> Result<bool> {
        Ok(!self.get_dependencies()?.is_empty())
    }

    fn load(&self) -> Result<GdmManifest> {
        let config_file_path = self.app_config.get_config_file_path();

        if !self.file_service.file_exists(config_file_path)? {
            return Ok(GdmManifest::default());
        }

        let content = self.file_service.read_file_cached(config_file_path)?;
        toml::from_str(&content).with_context(|| {
            format!(
                "Failed to parse dependency manifest: {}",
                config_file_path.display()
            )
        })
    }

    fn save(&self, manifest: &GdmManifest) -> Result<()> {
        let config_file_path = self.app_config.get_config_file_path();
        let content = toml::to_string_pretty(manifest).with_context(|| {
            format!(
                "Failed to serialize dependency manifest to TOML: {}",
                config_file_path.display()
            )
        })?;

        self.file_service.write_file(config_file_path, &content)?;
        info!(
            "Saved dependency manifest with dependencies: {:?}",
            manifest.dependencies.keys()
        );
        Ok(())
    }
}

pub trait GdmConfig {
    fn add_dependencies(&self, dependencies: &BTreeMap<String, Plugin>) -> Result<GdmManifest>;
    fn get_dependency_by_asset_slug(&self, asset_slug: &str) -> Result<Option<Plugin>>;
    fn get_dependency_by_config_key(&self, key: &str) -> Result<Option<Plugin>>;
    fn get_dependencies(&self) -> Result<BTreeMap<String, Plugin>>;
    fn has_dependencies(&self) -> Result<bool>;
    fn load(&self) -> Result<GdmManifest>;
    fn remove_dependencies(&self, dependency_keys: HashSet<String>) -> Result<GdmManifest>;
    fn save(&self, manifest: &GdmManifest) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use std::path::Path;

    use crate::config::DefaultAppConfig;
    use crate::models::Plugin;
    use crate::services::{DefaultFileService, MockDefaultFileService};

    fn setup_test_dependency_map() -> BTreeMap<String, Plugin> {
        BTreeMap::from([
            ("plugin_1".to_string(), Plugin::create_mock_plugin_1()),
            ("plugin_2".to_string(), Plugin::create_mock_plugin_2()),
        ])
    }

    fn setup_test_manifest() -> GdmManifest {
        GdmManifest::new(setup_test_dependency_map())
    }

    #[test]
    fn test_manifest_operations() {
        let manifest = setup_test_manifest();
        assert_eq!(manifest.dependencies.len(), 2);
        assert_eq!(
            manifest.dependency_by_config_key("plugin_1"),
            Some(Plugin::create_mock_plugin_1())
        );
        assert_eq!(manifest.dependency_by_config_key("asset_54321"), None);
        assert_eq!(
            manifest.dependency_by_asset_slug("asset_54321"),
            Some(Plugin::create_mock_plugin_1())
        );

        let added = manifest.with_dependencies(&BTreeMap::from([(
            "plugin_3".to_string(),
            Plugin::create_mock_plugin_3(),
        )]));
        assert_eq!(added.dependencies.len(), 3);

        let removed = added.without_dependencies(&HashSet::from(["plugin_1".to_string()]));
        assert!(!removed.dependencies.contains_key("plugin_1"));
        assert!(removed.dependencies.contains_key("plugin_2"));

        assert_eq!(manifest.dependencies_with_config().len(), 2);
    }

    #[test]
    fn test_load_non_existent_file_returns_default_manifest() {
        let config = DefaultGdmConfig::new(
            DefaultAppConfig::new(
                Some(String::from("tests/mocks/non_existent_file.toml")),
                None,
                None,
                None,
            ),
            Arc::new(DefaultFileService),
        );

        assert_eq!(config.load().unwrap(), GdmManifest::default());
    }

    #[test]
    fn test_load_toml_manifest() {
        let config = DefaultGdmConfig::new(
            DefaultAppConfig::new(Some(String::from("tests/mocks/gdm.toml")), None, None, None),
            Arc::new(DefaultFileService),
        );

        let manifest = config.load().unwrap();
        assert_eq!(manifest, setup_test_manifest());
    }

    #[test]
    fn test_manifest_toml_round_trip_preserves_dependency_data() {
        let git_dependency = Plugin {
            source: crate::models::PluginSource::Git {
                url: "git@github.com:foo/bar.git".to_string(),
                reference: "a83f10c".to_string(),
                publisher_slug: "foo".to_string(),
                asset_slug: "bar".to_string(),
            },
            plugin_cfg_path: Some("addons/bar/plugin.cfg".to_string()),
            title: "Bar dependency".to_string(),
            version: String::new(),
            sub_assets: vec!["bar.extras".to_string()],
            license: None,
        };
        let manifest = GdmManifest::new(BTreeMap::from([
            ("asset".to_string(), Plugin::create_mock_plugin_1()),
            ("git".to_string(), git_dependency),
        ]));

        let serialized = toml::to_string_pretty(&manifest).unwrap();
        let deserialized: GdmManifest = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.dependencies.len(), 2);
        assert_eq!(
            deserialized.dependencies["asset"].source,
            manifest.dependencies["asset"].source
        );
        assert_eq!(
            deserialized.dependencies["asset"].license,
            Some("MIT".to_string())
        );
        assert_eq!(
            deserialized.dependencies["git"].plugin_cfg_path,
            Some("addons/bar/plugin.cfg".to_string())
        );
        assert_eq!(
            deserialized.dependencies["git"].sub_assets,
            vec!["bar.extras".to_string()]
        );
        assert_eq!(
            deserialized.dependencies["git"].source,
            crate::models::PluginSource::Git {
                url: "git@github.com:foo/bar.git".to_string(),
                reference: "a83f10c".to_string(),
                publisher_slug: String::new(),
                asset_slug: String::new(),
            }
        );
    }

    #[test]
    fn test_load_empty_file_returns_default_manifest() {
        let mut file_service = MockDefaultFileService::new();
        let path = Path::new("gdm.toml");
        file_service
            .expect_file_exists()
            .with(eq(path))
            .returning(|_| Ok(true));
        file_service
            .expect_read_file_cached()
            .with(eq(path))
            .returning(|_| Ok(String::new()));

        let config = DefaultGdmConfig::new(
            DefaultAppConfig::new(Some(path.to_string_lossy().into_owned()), None, None, None),
            Arc::new(file_service),
        );

        assert_eq!(config.load().unwrap(), GdmManifest::default());
    }

    #[test]
    fn test_load_malformed_toml_returns_error() {
        let config = DefaultGdmConfig::new(
            DefaultAppConfig::new(
                Some(String::from("tests/mocks/gdm_malformed.toml")),
                None,
                None,
                None,
            ),
            Arc::new(DefaultFileService),
        );

        let error = config.load().unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Failed to parse dependency manifest")
        );
    }

    #[test]
    fn test_load_rejects_legacy_plugins_table() {
        let mut file_service = MockDefaultFileService::new();
        let path = Path::new("gdm.toml");
        file_service
            .expect_file_exists()
            .with(eq(path))
            .returning(|_| Ok(true));
        file_service
            .expect_read_file_cached()
            .with(eq(path))
            .returning(|_| Ok("[plugins]\n".to_string()));

        let config = DefaultGdmConfig::new(
            DefaultAppConfig::new(Some(path.to_string_lossy().into_owned()), None, None, None),
            Arc::new(file_service),
        );

        assert!(config.load().is_err());
    }

    #[test]
    fn test_save_writes_toml_manifest() {
        let path = Path::new("tests/mocks/gdm.toml");
        let mut file_service = MockDefaultFileService::new();
        file_service
            .expect_write_file()
            .with(
                eq(path),
                function(|content: &str| content.contains("[dependencies.")),
            )
            .returning(|_, _| Ok(()));

        let config = DefaultGdmConfig::new(
            DefaultAppConfig::new(Some(path.to_string_lossy().into_owned()), None, None, None),
            Arc::new(file_service),
        );

        config.save(&setup_test_manifest()).unwrap();
    }
}
