use anyhow::{Result, anyhow};
use indicatif::ProgressBar;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::app_config::AppConfig;
use crate::app_config::AppConfigImpl;
use crate::file_service::FileService;
use crate::file_service::FileServiceInternal;
use crate::utils::Utils;

#[derive(Clone, Default)]
pub struct ExtractService {
    file_service: FileService,
    app_config: AppConfig,
}

impl ExtractService {
    pub fn new(file_service: FileService, app_config: AppConfig) -> Self {
        ExtractService {
            file_service,
            app_config,
        }
    }
}

#[cfg_attr(test, mockall::automock)]
impl ExtractServiceImpl for ExtractService {
    fn get_file_service(&self) -> &FileService {
        &self.file_service
    }

    fn get_app_config(&self) -> &AppConfig {
        &self.app_config
    }
}

pub trait ExtractServiceImpl {
    fn get_app_config(&self) -> &AppConfig;
    fn get_file_service(&self) -> &FileService;

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

    fn get_root_dir_from_archive(&self, file_path: &str) -> anyhow::Result<String> {
        let file = fs::File::open(file_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let plugin_root_dir: PathBuf = self.get_root_directory_name_from_archive(&mut archive)?;
        Ok(plugin_root_dir.display().to_string())
    }

    fn extract_zip_file(
        &self,
        file_path: String,
        destination: String,
        pb_task: ProgressBar,
    ) -> anyhow::Result<()> {
        let file_service = self.get_file_service();
        let file = file_service.open(PathBuf::from(file_path))?;
        let _destination = PathBuf::from(&destination);

        let mut archive = zip::ZipArchive::new(file)?;
        let file_count = archive.file_names().count();
        pb_task.set_length(file_count as u64);

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            pb_task.inc(1);
            let outpath = match file.enclosed_name() {
                Some(path) => self.create_extract_path(_destination.clone(), path),
                None => continue,
            };

            if file.is_dir() {
                file_service.create_directory(outpath).unwrap();
            } else {
                if let Some(p) = outpath.parent()
                    && !p.exists()
                {
                    file_service.create_directory(PathBuf::from(p)).unwrap();
                }
                let mut outfile = file_service.create_file(outpath).unwrap();
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
    async fn extract_plugin(
        &self,
        file_path: String,
        addon_folder_path: String,
        pb_task: ProgressBar,
    ) -> Result<String> {
        let file_service = self.get_file_service();
        let plugin_folder = self.get_root_dir_from_archive(&file_path)?;
        let addon_folder = self.get_app_config().get_addon_folder_path();
        let plugin_folder_path =
            Utils::plugin_name_to_addon_folder_path(plugin_folder.clone(), addon_folder);

        if file_service.directory_exists(PathBuf::from(&format!(
            "{}/{}",
            &addon_folder_path, &plugin_folder_path
        ))) {
            file_service.remove_dir_all(PathBuf::from(&format!(
                "{}/{}",
                &addon_folder_path, &plugin_folder_path
            )))?;
        }

        self.extract_zip_file(file_path.clone(), addon_folder_path, pb_task)?;
        file_service.remove_file(PathBuf::from(file_path))?;
        Ok(plugin_folder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_root_directory_name_from_archive_with_addons_folder() {
        let extract = ExtractService::default();
        let file = fs::File::open("test/mocks/zip_files/test_with_addons_folder.zip").unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let root_dir = extract.get_root_directory_name_from_archive(&mut archive);
        assert_eq!(root_dir.unwrap(), PathBuf::from("some_plugin"));
    }

    #[test]
    fn test_get_root_directory_name_from_archive_without_addons_folder() {
        let extract = ExtractService::default();
        let file = fs::File::open("test/mocks/zip_files/test_without_addons_folder.zip").unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let root_dir = extract.get_root_directory_name_from_archive(&mut archive);
        assert_eq!(root_dir.unwrap(), PathBuf::from("some_plugin"));
    }

    #[test]
    fn test_get_root_directory_name_from_archive_without_root_should_return_error() {
        let extract = ExtractService::default();
        let file = fs::File::open("test/mocks/zip_files/test_without_root_folder.zip").unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let result = extract.get_root_directory_name_from_archive(&mut archive);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_zip_file() {
        let extract = ExtractService::default();
        let pb_task = ProgressBar::new(5000000);
        let result = extract.extract_zip_file(
            String::from("test/mocks/zip_files/test_with_addons_folder.zip"),
            String::from("test/addons"),
            pb_task,
        );
        assert!(result.is_ok());
        fs::remove_dir_all("test/addons").unwrap();
    }

    #[test]
    fn test_create_extract_path_should_return_with_addons_folder_path_2() {
        let extract = ExtractService::default();
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
        let extract = ExtractService::default();
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
        let extract = ExtractService::default();
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
        let extract = ExtractService::default();
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
