use anyhow::{Result, anyhow};
use indicatif::ProgressBar;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use crate::app_config::AppConfig;
use crate::app_config::DefaultAppConfig;
use crate::file_service::{DefaultFileService, FileService};
use crate::utils::Utils;

#[cfg(not(tarpaulin_include))]
pub struct DefaultExtractService {
    file_service: Box<dyn FileService + Send + Sync + 'static>,
    app_config: DefaultAppConfig,
}

#[cfg(not(tarpaulin_include))]
impl DefaultExtractService {
    #[allow(dead_code)]
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

#[cfg(not(tarpaulin_include))]
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
    fn get_file_service(&self) -> &dyn FileService {
        &*self.file_service
    }

    fn get_app_config(&self) -> &DefaultAppConfig {
        &self.app_config
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

    fn get_root_directory_name_from_archive(
        &self,
        archive: &mut zip::ZipArchive<fs::File>,
    ) -> Result<PathBuf> {
        let mut paths = HashSet::new();
        for i in 0..archive.len() {
            let file = archive.by_index(i).unwrap();
            let path = file.mangled_name().iter().skip(1).collect::<PathBuf>();
            if file.is_dir() {
                if path == PathBuf::from("addons") || path == PathBuf::from("") {
                    continue;
                }
                paths.insert(path);
            } else if file.is_file() && path.iter().count() == 1 {
                return Err(anyhow!("Invalid archive structure: no root folder"));
            }
        }
        if paths.is_empty() {
            return Err(anyhow!("No directories found in the archive"));
        }
        let path = paths.iter().next().unwrap();
        let addons_index = path.iter().position(|p| p == "addons");
        match addons_index {
            Some(i) => Ok(path.iter().skip(i + 1).take(1).collect::<PathBuf>()),
            None => Ok(path.iter().take(1).collect::<PathBuf>()),
        }
    }

    fn get_root_dir_from_archive(&self, file_path: &Path) -> Result<PathBuf> {
        let file = fs::File::open(file_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        self.get_root_directory_name_from_archive(&mut archive)
    }

    fn extract_zip_file(
        &self,
        file_path: &Path,
        destination: &Path,
        pb_task: ProgressBar,
    ) -> anyhow::Result<()> {
        let file_service = self.get_file_service();
        let file = file_service.open(file_path)?;

        let mut archive = zip::ZipArchive::new(file)?;
        let file_count = archive.file_names().count();
        pb_task.set_length(file_count as u64);

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            pb_task.inc(1);
            let outpath = match file.enclosed_name() {
                Some(path) => {
                    self.create_extract_path(destination.to_path_buf(), path.to_path_buf())
                }
                None => continue,
            };

            let extract_path = outpath.as_path();

            if file.is_dir() {
                file_service.create_directory(extract_path).unwrap();
            } else {
                if let Some(p) = outpath.parent()
                    && !p.exists()
                {
                    file_service.create_directory(p).unwrap();
                }
                let mut outfile = file_service.create_file(extract_path).unwrap();
                io::copy(&mut file, &mut outfile).unwrap();
            }
            // Get and Set permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).unwrap();
                }
            }
        }
        pb_task.finish_and_clear();
        Ok(())
    }

    /// ExtractServices a downloaded plugin zip file to the addons folder and removes the zip file afterwards
    ///
    /// Removes the old plugin folder if it already exists
    async fn extract_plugin(&self, file_path: &Path, pb_task: ProgressBar) -> Result<PathBuf> {
        let file_service = self.get_file_service();
        let plugin_folder = self.get_root_dir_from_archive(file_path)?;
        let addon_folder = self.get_app_config().get_addon_folder_path();
        let plugin_folder_path =
            Utils::plugin_name_to_addon_folder_path(&plugin_folder, addon_folder);

        if file_service.directory_exists(&plugin_folder_path) {
            file_service.remove_dir_all(&plugin_folder_path)?;
        }

        self.extract_zip_file(file_path, Path::new(&addon_folder), pb_task)?;
        file_service.remove_file(file_path)?;
        Ok(plugin_folder)
    }
}

#[async_trait::async_trait]
pub trait ExtractService: Send + Sync + 'static {
    fn get_app_config(&self) -> &DefaultAppConfig;
    fn get_file_service(&self) -> &dyn FileService;
    fn create_extract_path(&self, root: PathBuf, path: PathBuf) -> PathBuf;

    fn get_root_directory_name_from_archive(
        &self,
        archive: &mut zip::ZipArchive<fs::File>,
    ) -> Result<PathBuf>;
    fn get_root_dir_from_archive(&self, file_path: &Path) -> Result<PathBuf>;
    fn extract_zip_file(
        &self,
        file_path: &Path,
        destination: &Path,
        pb_task: ProgressBar,
    ) -> anyhow::Result<()>;
    async fn extract_plugin(&self, file_path: &Path, pb_task: ProgressBar) -> Result<PathBuf>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_root_directory_name_from_archive_with_addons_folder() {
        let extract = DefaultExtractService::default();
        let file = fs::File::open("test/mocks/zip_files/test_with_addons_folder.zip").unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let root_dir = extract.get_root_directory_name_from_archive(&mut archive);
        assert_eq!(root_dir.unwrap(), PathBuf::from("some_plugin"));
    }

    #[test]
    fn test_get_root_directory_name_from_archive_without_addons_folder() {
        let extract = DefaultExtractService::default();
        let file = fs::File::open("test/mocks/zip_files/test_without_addons_folder.zip").unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let root_dir = extract.get_root_directory_name_from_archive(&mut archive);
        assert_eq!(root_dir.unwrap(), PathBuf::from("some_plugin"));
    }

    #[test]
    fn test_get_root_directory_name_from_archive_without_root_should_return_error() {
        let extract = DefaultExtractService::default();
        let file = fs::File::open("test/mocks/zip_files/test_without_root_folder.zip").unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let result = extract.get_root_directory_name_from_archive(&mut archive);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_zip_file() {
        let extract = DefaultExtractService::default();
        let pb_task = ProgressBar::new(5000000);
        let result = extract.extract_zip_file(
            Path::new("test/mocks/zip_files/test_with_addons_folder.zip"),
            Path::new("test/addons"),
            pb_task,
        );
        assert!(result.is_ok());
        fs::remove_dir_all("test/addons").unwrap();
    }

    #[test]
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
    fn test_create_extract_path_should_return_with_addons_folder_path_3() {
        let extract = DefaultExtractService::default();
        let path = ["zip_filename", "some_plugin", "file.txt"]
            .iter()
            .collect::<PathBuf>();
        let extract_path = extract.create_extract_path(PathBuf::from("test/addons"), path);
        assert_eq!(
            extract_path,
            ["test", "addons", "some_plugin", "file.txt"]
                .iter()
                .collect::<PathBuf>()
        );
    }

    #[test]
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
}
