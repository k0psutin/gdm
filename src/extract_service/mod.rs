use crate::app_config::AppConfig;
use crate::plugin_config_repository::plugin::Plugin;
use anyhow::{Result, bail};
use indicatif::ProgressBar;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use crate::api::asset::Asset;
use crate::app_config::DefaultAppConfig;
use crate::file_service::{DefaultFileService, FileService};

pub struct DefaultExtractService {
    pub file_service: Box<dyn FileService + Send + Sync + 'static>,
    pub app_config: DefaultAppConfig,
}
impl DefaultExtractService {
    #[allow(unused)]
    pub fn new(
        file_service: Box<dyn FileService + Send + Sync + 'static>,
        app_config: DefaultAppConfig,
    ) -> Self {
        DefaultExtractService {
            file_service,
            app_config,
        }
    }
}

impl Default for DefaultExtractService {
    fn default() -> Self {
        DefaultExtractService {
            file_service: Box::new(DefaultFileService),
            app_config: DefaultAppConfig::default(),
        }
    }
}

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
impl ExtractService for DefaultExtractService {
    fn open_archive_file(&self, file_path: &Path) -> Result<zip::ZipArchive<fs::File>> {
        let file = self.file_service.open(file_path)?;
        let archive = zip::ZipArchive::new(file)?;
        Ok(archive)
    }

    fn create_extract_path(&self, root: PathBuf, path: PathBuf) -> PathBuf {
        let index = path.iter().skip(1).position(|p| p == "addons");
        match index {
            Some(i) => {
                let components: Vec<_> = path.iter().skip(i + 2).collect();
                let mut new_path = root;
                new_path.extend(components);
                new_path
            }
            None => {
                let components: Vec<_> = path.iter().skip(1).collect();
                let mut new_path = root;
                new_path.extend(components);
                new_path
            }
        }
    }

    fn extract_zip_file(
        &self,
        file_path: &Path,
        destination: &Path,
        pb_task: ProgressBar,
    ) -> Result<()> {
        let mut archive = self.open_archive_file(file_path)?;

        let file_count = archive.file_names().count();
        pb_task.set_length(file_count as u64);

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            pb_task.inc(1);
            let outpath = match file.enclosed_name() {
                Some(path) => self.create_extract_path(destination.to_path_buf(), path),
                None => continue,
            };

            let extract_path = outpath.as_path();

            if !file.is_dir() && extract_path.is_dir() {
                // If we have a file that is outside the expected structure, skip it
                // E.g., .zip file contains file /some-file.txt at root level.
                // See [create_extract_path] TODO comment
                continue;
            }

            if file.is_dir() {
                self.file_service.create_directory(extract_path)?;
            } else {
                if let Some(p) = outpath.parent()
                    && !p.exists()
                {
                    self.file_service.create_directory(p)?;
                }
                let mut outfile = self.file_service.create_file(extract_path)?;
                io::copy(&mut file, &mut outfile)?;
            }
            // Get and Set permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                }
            }
        }
        pb_task.finish_and_clear();
        Ok(())
    }

    fn create_plugin_from_asset_archive(&self, asset: &Asset) -> Result<(PathBuf, Plugin)> {
        let mut archive = self.open_archive_file(&asset.file_path)?;
        let addons_folder = PathBuf::from(self.app_config.get_addon_folder_path());
        let mut paths = HashSet::new();
        let mut plugin_cfgs = HashSet::new();

        for i in 0..archive.len() {
            let file = archive.by_index(i).unwrap();
            let outpath = match file.enclosed_name() {
                Some(path) => self.create_extract_path(addons_folder.clone(), path),
                None => continue,
            };

            if file.is_dir() {
                if outpath == addons_folder || outpath.as_os_str().is_empty() {
                    continue;
                }
                paths.insert(outpath);
            } else if file.name().ends_with("plugin.cfg") {
                plugin_cfgs.insert(outpath);
            }
        }
        if paths.is_empty() {
            bail!("No directories found in the archive")
        }

        let filtered_paths = paths
            .iter()
            .map(|p| {
                let index = p.iter().position(|comp| comp == "addons");
                if let Some(index) = index {
                    p.iter()
                        .by_ref()
                        .skip(index + 1)
                        .take(1)
                        .collect::<PathBuf>()
                } else {
                    p.iter().by_ref().skip(1).take(1).collect::<PathBuf>()
                }
            })
            .collect::<HashSet<_>>();

        let filename = &asset
            .file_path
            .file_stem()
            .unwrap_or_default()
            .display()
            .to_string();
        let title = &asset.asset_response.title;

        let best_match = filtered_paths
            .iter()
            .fold((PathBuf::new(), 0.0), |mut best, path| {
                let folder_name = path.to_string_lossy().to_string();
                let jaro_filename =
                    strsim::jaro(&folder_name.to_lowercase(), &filename.to_lowercase());
                let jaro_title = strsim::jaro(&folder_name.to_lowercase(), &title.to_lowercase());
                let max_jaro = jaro_filename.max(jaro_title);
                if max_jaro > best.1 {
                    best = (PathBuf::from(path), max_jaro);
                }
                best
            });

        let main_plugin_folder = best_match.0.clone();

        let sub_addons = filtered_paths
            .iter()
            .filter(|p| {
                let folder_name = p.to_string_lossy().to_string();
                folder_name != main_plugin_folder
            })
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<String>>();

        // Check if best match has a plugin.cfg file
        let filtered_plugin_cfg_paths = paths
            .iter()
            .map(|p| {
                let index = p.iter().position(|comp| comp == "addons");
                if let Some(index) = index {
                    p.iter()
                        .by_ref()
                        .skip(index + 1)
                        .take(1)
                        .collect::<PathBuf>()
                } else {
                    p.iter().by_ref().skip(1).take(1).collect::<PathBuf>()
                }
            })
            .collect::<HashSet<_>>();

        let has_plugin_cfg = filtered_plugin_cfg_paths.contains(&main_plugin_folder);

        Ok((
            main_plugin_folder,
            Plugin::new(
                asset.asset_response.asset_id.clone(),
                asset.asset_response.title.clone(),
                asset.asset_response.version.clone(),
                asset.asset_response.cost.clone(),
                sub_addons,
                has_plugin_cfg,
            ),
        ))
    }

    /// Extracts a downloaded plugin zip file to the Godot project's addons folder and removes the zip file afterwards.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the plugin zip file to extract.
    /// * `pb_task` - ProgressBar instance for reporting extraction progress.
    ///
    /// # Returns
    ///
    /// Returns a tuple containing the plugin folder name (as a String) and the constructed `Plugin` struct.
    ///
    /// # Side Effects
    ///
    /// - Removes the old plugin folder if it already exists in the addons directory.
    /// - Extracts the contents of the zip file into the addons directory.
    /// - Deletes the original zip file after extraction.
    ///
    /// # Panics
    ///
    /// Returns an error if extraction fails, the zip file is invalid, or the plugin folder cannot be determined.
    async fn extract_asset(&self, asset: &Asset, pb_task: ProgressBar) -> Result<(String, Plugin)> {
        let (main_plugin_folder, plugin) = self.create_plugin_from_asset_archive(asset)?;
        let plugin_folder = self
            .app_config
            .get_addon_folder_path()
            .join(&main_plugin_folder);

        if self.file_service.directory_exists(&plugin_folder) {
            self.file_service.remove_dir_all(&plugin_folder)?;
        }

        self.extract_zip_file(&asset.file_path, &plugin_folder, pb_task)?;
        self.file_service.remove_file(&asset.file_path)?;

        let plugin_name = main_plugin_folder.to_string_lossy().to_string();

        Ok((plugin_name, plugin))
    }
}

#[async_trait::async_trait]
pub trait ExtractService: Send + Sync + 'static {
    fn open_archive_file(&self, file_path: &Path) -> Result<zip::ZipArchive<fs::File>>;

    fn create_extract_path(&self, root: PathBuf, path: PathBuf) -> PathBuf;

    fn create_plugin_from_asset_archive(&self, asset: &Asset) -> Result<(PathBuf, Plugin)>;

    fn extract_zip_file(
        &self,
        file_path: &Path,
        destination: &Path,
        pb_task: ProgressBar,
    ) -> Result<()>;

    async fn extract_asset(&self, asset: &Asset, pb_task: ProgressBar) -> Result<(String, Plugin)>;
}

#[cfg(test)]
mod tests {
    fn make_mock_asset<P: Into<std::path::PathBuf>>(zip_path: P, title: &str) -> Asset {
        use crate::api::asset_response::AssetResponse;
        Asset {
            file_path: zip_path.into(),
            asset_response: AssetResponse {
                asset_id: "test_id".to_string(),
                title: title.to_string(),
                version: "1.0.0".to_string(),
                version_string: "1.0.0".to_string(),
                godot_version: "4.0".to_string(),
                rating: "5".to_string(),
                cost: "Free".to_string(),
                description: "Test plugin asset".to_string(),
                download_provider: "local".to_string(),
                download_commit: "".to_string(),
                modify_date: "2023-01-01".to_string(),
                download_url: "".to_string(),
            },
        }
    }
    use crate::file_service::MockDefaultFileService;

    use super::*;
    use serial_test::serial;

    // create_plugin_from_asset_archive

    #[test]
    #[serial]
    fn test_create_plugin_from_asset_archive_with_no_directories_should_error() {
        let extract = DefaultExtractService::default();
        let asset = make_mock_asset(
            "tests/mocks/zip_files/test_without_root_folder.zip",
            "NoDirs",
        );
        let result = extract.create_plugin_from_asset_archive(&asset);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_create_plugin_from_asset_archive_with_single_plugin_folder() {
        let extract = DefaultExtractService::default();
        let asset = make_mock_asset(
            "tests/mocks/zip_files/test_with_addons_folder.zip",
            "SomePlugin",
        );
        let result = extract.create_plugin_from_asset_archive(&asset);
        assert!(result.is_ok());
        let (main_folder, plugin) = result.unwrap();
        assert_eq!(main_folder, PathBuf::from("some_plugin"));
        assert_eq!(plugin.title, asset.asset_response.title);
        assert_eq!(plugin.asset_id, asset.asset_response.asset_id);
        assert_eq!(plugin.get_version(), asset.asset_response.version);
        assert_eq!(plugin.license, asset.asset_response.cost);
        assert_eq!(plugin.sub_assets.len(), 0);
        assert!(plugin.has_plugin_cfg);
    }

    #[test]
    #[serial]
    fn test_create_plugin_from_asset_archive_with_multiple_plugin_folders() {
        let extract = DefaultExtractService::default();
        let asset = make_mock_asset(
            "tests/mocks/zip_files/test_with_addons_folder_with_subaddons.zip",
            "MultiPlugin",
        );
        let result = extract.create_plugin_from_asset_archive(&asset);
        assert!(result.is_ok());
        let (_main_folder, plugin) = result.unwrap();
        // Should have at least one sub_asset
        assert_eq!(plugin.sub_assets.len(), 1);
        assert_eq!(plugin.sub_assets[0], "some_plugin");
    }

    #[test]
    #[serial]
    fn test_create_plugin_from_asset_archive_with_plugin_cfg_in_subfolder() {
        let extract = DefaultExtractService::default();
        let asset = make_mock_asset(
            "tests/mocks/zip_files/test_with_addons_folder_with_extra_addons_files.zip",
            "PluginCfgSubfolder",
        );
        let result = extract.create_plugin_from_asset_archive(&asset);
        assert!(result.is_ok());
        let (_main_folder, plugin) = result.unwrap();
        assert!(plugin.has_plugin_cfg);
    }

    #[test]
    #[serial]
    fn test_create_plugin_from_asset_archive_with_files_at_root_or_addons() {
        let extract = DefaultExtractService::default();
        let asset = make_mock_asset(
            "tests/mocks/zip_files/test_with_addons_folder_with_root_files.zip",
            "RootFiles",
        );
        let result = extract.create_plugin_from_asset_archive(&asset);
        assert!(result.is_ok());
        let (main_folder, _) = result.unwrap();
        assert_eq!(main_folder.to_string_lossy(), "some_plugin");
    }

    // extract_zip_file

    #[test]
    #[serial]
    fn test_extract_zip_file_with_addons_folder() {
        let extract = DefaultExtractService::default();
        let pb_task = ProgressBar::new(5000000);
        let result = extract.extract_zip_file(
            Path::new("tests/mocks/zip_files/test_with_addons_folder.zip"),
            Path::new("tests/addons"),
            pb_task,
        );
        fs::remove_dir_all("tests/addons").unwrap();
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_extract_zip_file_with_extra_addons_files() {
        let extract = DefaultExtractService::default();
        let pb_task = ProgressBar::new(5000000);
        let result = extract.extract_zip_file(
            Path::new("tests/mocks/zip_files/test_with_addons_folder_with_extra_addons_files.zip"),
            Path::new("tests/addons"),
            pb_task,
        );
        fs::remove_dir_all("tests/addons").unwrap();
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_extract_zip_file_with_root_files() {
        let extract = DefaultExtractService::default();
        let pb_task = ProgressBar::new(5000000);
        let result = extract.extract_zip_file(
            Path::new("tests/mocks/zip_files/test_with_addons_folder_with_root_files.zip"),
            Path::new("tests/addons"),
            pb_task,
        );
        fs::remove_dir_all("tests/addons").unwrap();
        assert!(result.is_ok());
    }

    // create_extract_path

    #[test]
    #[serial]
    fn test_create_extract_path_should_return_with_addons_folder_path_2() {
        let extract = DefaultExtractService::default();
        let path = ["zip_filename", "some_plugin", "file.txt"]
            .iter()
            .collect::<PathBuf>();
        let extract_path = extract.create_extract_path(PathBuf::from("addons"), path);
        assert_eq!(
            extract_path,
            ["addons", "some_plugin", "file.txt"]
                .iter()
                .collect::<PathBuf>()
        );
    }

    #[test]
    #[serial]
    fn test_create_extract_path_should_return_with_addons_folder_path_3() {
        let extract = DefaultExtractService::default();
        let path = ["zip_filename", "some_plugin", "file.txt"]
            .iter()
            .collect::<PathBuf>();
        let extract_path = extract.create_extract_path(PathBuf::from("tests/addons"), path);
        assert_eq!(
            extract_path,
            ["tests", "addons", "some_plugin", "file.txt"]
                .iter()
                .collect::<PathBuf>()
        );
    }

    #[test]
    #[serial]
    fn test_create_extract_path_should_not_modify_existing_folder_path() {
        let extract = DefaultExtractService::default();
        let path = ["zip_filename", "addons", "some_plugin", "test.txt"]
            .iter()
            .collect::<PathBuf>();
        let extract_path = extract.create_extract_path(PathBuf::from("addons"), path);
        assert_eq!(
            extract_path,
            ["addons", "some_plugin", "test.txt"]
                .iter()
                .collect::<PathBuf>()
        );
    }

    #[test]
    #[serial]
    fn test_create_extract_path_should_modify_existing_path() {
        let extract = DefaultExtractService::default();
        let path = [
            "zip_filename",
            "some_folder",
            "addons",
            "some_plugin",
            "test.txt",
        ]
        .iter()
        .collect::<PathBuf>();
        let extract_path = extract.create_extract_path(PathBuf::from("addons"), path);
        assert_eq!(
            extract_path,
            ["addons", "some_plugin", "test.txt"]
                .iter()
                .collect::<PathBuf>()
        );
    }

    // extract_plugin

    #[tokio::test]
    #[serial]
    async fn test_extract_plugin() {
        let mut mock_file_service = MockDefaultFileService::new();
        mock_file_service
            .expect_directory_exists()
            .returning(|_: &Path| true);
        mock_file_service.expect_open().returning(|path: &Path| {
            let file = fs::File::open(path).unwrap();
            Ok(file)
        });
        mock_file_service
            .expect_create_file()
            .returning(|path: &Path| {
                let file = fs::File::create(path).unwrap();
                Ok(file)
            });
        mock_file_service
            .expect_create_directory()
            .returning(|path: &Path| {
                fs::create_dir_all(path).unwrap();
                Ok(())
            });
        mock_file_service
            .expect_remove_dir_all()
            .returning(|_: &Path| Ok(()));
        mock_file_service
            .expect_file_exists()
            .returning(|_: &Path| Ok(false));
        mock_file_service
            .expect_remove_file()
            .returning(|_: &Path| Ok(()));

        let extract = DefaultExtractService::new(
            Box::new(mock_file_service),
            DefaultAppConfig::new(
                None,
                Some("tests/config/config.toml".to_string()),
                Some("tests/cache".to_string()),
                Some("tests/project/project.godot".to_string()),
                Some("tests/addons".to_string()),
            ),
        );
        let pb_task = ProgressBar::no_length();
        let asset = make_mock_asset(
            "tests/mocks/zip_files/test_with_addons_folder.zip",
            "SomePlugin",
        );
        let result = extract.extract_asset(&asset, pb_task).await;
        assert!(result.is_ok());
        let (_plugin_folder, _plugin) = result.unwrap();
        assert_eq!(_plugin_folder, PathBuf::from("some_plugin"));
        fs::remove_dir_all("tests/addons").unwrap();
    }
}
