use anyhow::{Result, anyhow};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::PathBuf;
use zip;

fn create_extract_path(root: PathBuf, path: PathBuf) -> PathBuf {
    let index = path.iter().skip(1).position(|p| p == "addons");
    match index {
        Some(i) => {
            let components: Vec<_> = path.iter().skip(i + 2).collect();
            let mut new_path = PathBuf::from(root);
            new_path.extend(components);
            new_path
        }
        None => {
            let components: Vec<_> = path.iter().skip(1).collect();
            let mut new_path = PathBuf::from(root);
            new_path.extend(components);
            new_path
        }
    }
}

fn get_root_directory_name_from_archive(
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
        } else if file.is_file() {
            if path.iter().count() == 1 {
                return Err(anyhow!("Invalid archive structure: no root folder"));
            }
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

pub fn extract_zip_file(file_path: &str, destination: &str) -> anyhow::Result<String> {
    let file = fs::File::open(file_path).unwrap();

    let mut archive = zip::ZipArchive::new(file).unwrap();
    let plugin_root_dir: PathBuf = get_root_directory_name_from_archive(&mut archive)?;

    let extract_progress_bar: ProgressBar = ProgressBar::new(archive.len() as u64);
    extract_progress_bar.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7} {msg}",
        )
        .unwrap()
        .progress_chars("#>-"),
    );
    let file_name_progress_bar = ProgressBar::new(archive.len() as u64);
    file_name_progress_bar
        .set_style(ProgressStyle::with_template("{spinner:.green} {wide_msg}").unwrap());

    let mp = MultiProgress::new();
    let extract_progress_bar: ProgressBar = mp.add(extract_progress_bar);
    let file_progress_bar: ProgressBar = mp.add(file_name_progress_bar);

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => create_extract_path(PathBuf::from(destination), path),
            None => continue,
        };

        if file.is_dir() {
            fs::create_dir_all(&outpath).unwrap();
        } else {
            file_progress_bar.set_message(format!("File: {}", outpath.display()));
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
        file_progress_bar.inc(1);
        extract_progress_bar.inc(1);
        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }
    extract_progress_bar.finish_with_message("done");
    file_progress_bar.finish_and_clear();
    Ok(plugin_root_dir.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_root_directory_name_from_archive_with_addons_folder() {
        let file = fs::File::open("test/mocks/zip_files/test_with_addons_folder.zip").unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let root_dir = get_root_directory_name_from_archive(&mut archive);
        assert_eq!(root_dir.unwrap(), PathBuf::from("some_plugin"));
    }

    #[test]
    fn test_get_root_directory_name_from_archive_without_addons_folder() {
        let file = fs::File::open("test/mocks/zip_files/test_without_addons_folder.zip").unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let root_dir = get_root_directory_name_from_archive(&mut archive);
        assert_eq!(root_dir.unwrap(), PathBuf::from("some_plugin"));
    }

    #[test]
    fn test_get_root_directory_name_from_archive_without_root_should_return_error() {
        let file = fs::File::open("test/mocks/zip_files/test_without_root_folder.zip").unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let result = get_root_directory_name_from_archive(&mut archive);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_zip_file() {
        let result = extract_zip_file("test/mocks/zip_files/test_with_addons_folder.zip", "test/addons");
        assert!(result.is_ok());
        fs::remove_dir_all("test/addons").unwrap();
    }

    #[test]
    fn test_create_extract_path_should_return_with_addons_folder_path_2() {
        let path = PathBuf::from(
            ["zip_filename", "some_plugin", "file.txt"]
                .iter()
                .collect::<PathBuf>(),
        );
        let extract_path = create_extract_path(PathBuf::from("addons"), path);
        assert_eq!(
            extract_path,
            PathBuf::from(
                ["addons", "some_plugin", "file.txt"]
                    .iter()
                    .collect::<PathBuf>()
            )
        );
    }

        #[test]
    fn test_create_extract_path_should_return_with_addons_folder_path_3() {
        let path = PathBuf::from(
            ["zip_filename", "some_plugin", "file.txt"]
                .iter()
                .collect::<PathBuf>(),
        );
        let extract_path = create_extract_path(PathBuf::from("test/addons"), path);
        assert_eq!(
            extract_path,
            PathBuf::from(
                ["test", "addons", "some_plugin", "file.txt"]
                    .iter()
                    .collect::<PathBuf>()
            )
        );
    }

    #[test]
    fn test_create_extract_path_should_not_modify_existing_folder_path() {
        let path = PathBuf::from(
            ["zip_filename", "addons", "some_plugin", "test.txt"]
                .iter()
                .collect::<PathBuf>(),
        );
        let extract_path = create_extract_path(PathBuf::from("addons"), path);
        assert_eq!(
            extract_path,
            PathBuf::from(
                ["addons", "some_plugin", "test.txt"]
                    .iter()
                    .collect::<PathBuf>()
            )
        );
    }

    #[test]
    fn test_create_extract_path_should_modify_existing_path() {
        let path = PathBuf::from(
            [
                "zip_filename",
                "some_folder",
                "addons",
                "some_plugin",
                "test.txt",
            ]
            .iter()
            .collect::<PathBuf>(),
        );
        let extract_path = create_extract_path(PathBuf::from("addons"), path);
        assert_eq!(
            extract_path,
            PathBuf::from(
                ["addons", "some_plugin", "test.txt"]
                    .iter()
                    .collect::<PathBuf>()
            )
        );
    }
}
