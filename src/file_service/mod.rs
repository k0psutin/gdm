mod cache;

use anyhow::{Context, Result};
use cache::Cache;
use std::{fs::File, path::PathBuf};

use crate::file_service::cache::CacheImpl;

#[derive(Debug, Clone, Default)]
pub struct FileService;

#[cfg_attr(test, mockall::automock)]
impl FileServiceInternal for FileService {}

pub trait FileServiceInternal {
    fn open(&self, file_path: PathBuf) -> Result<File> {
        let file = File::open(file_path.clone())
            .with_context(|| format!("Failed to open file: {}", file_path.display()))?;
        Ok(file)
    }

    fn read_file_cached(&self, file_path: PathBuf) -> Result<String> {
        let cache = Cache::new();
        let path = file_path.clone().into_os_string().into_string().unwrap();
        if cache.has_key(path.clone()) {
            return Ok(cache.get(path).unwrap().clone());
        }
        let content = std::fs::read_to_string(file_path.clone())
            .with_context(|| format!("Failed to read file: {}", file_path.to_str().unwrap()))?;
        cache.insert(path.to_string(), content.clone());
        Ok(content)
    }

    fn file_exists(&self, file_path: PathBuf) -> bool {
        file_path.try_exists().unwrap_or(false)
    }

    fn write_file(&self, file_path: PathBuf, content: &str) -> Result<()> {
        std::fs::write(file_path.clone(), content)
            .with_context(|| format!("Failed to write file: {}", file_path.display()))?;
        Ok(())
    }

    fn create_file(&self, file_path: PathBuf) -> Result<File> {
        let file = File::create(file_path.clone())
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;
        Ok(file)
    }

    fn create_directory(&self, dir_path: PathBuf) -> Result<()> {
        std::fs::create_dir_all(dir_path.clone())
            .with_context(|| format!("Failed to create directory: {}", dir_path.display()))?;
        Ok(())
    }

    fn remove_dir_all(&self, dir_path: PathBuf) -> Result<()> {
        if self.directory_exists(dir_path.clone()) {
            std::fs::remove_dir_all(dir_path.clone())
                .with_context(|| format!("Failed to remove directory: {}", dir_path.display()))?;
        }
        Ok(())
    }

    fn directory_exists(&self, dir_path: PathBuf) -> bool {
        dir_path.is_dir()
    }

    fn remove_file(&self, file_path: PathBuf) -> Result<()> {
        if self.file_exists(file_path.clone()) {
            std::fs::remove_file(file_path.clone())
                .with_context(|| format!("Failed to remove file: {}", file_path.display()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_file_cached_should_cache_file() {
        let file_service = FileService;
        let test_file_path = PathBuf::from("test/mocks/test_1.txt");
        std::fs::write(&test_file_path, "Hello, world!").unwrap();
        let content_first_read = file_service
            .read_file_cached(test_file_path.clone())
            .unwrap();
        assert_eq!(content_first_read, "Hello, world!");

        // Modify the file after the first read
        let modified_content = "Goodbye, world!";
        std::fs::write(&test_file_path, modified_content).unwrap();

        // Read again, should return cached content
        let content_second_read = file_service
            .read_file_cached(test_file_path.clone())
            .unwrap();
        assert_eq!(content_second_read, "Hello, world!");

        // Clean up
        std::fs::remove_file(test_file_path).unwrap();
    }

    #[test]
    fn test_write_file_should_create_file_with_text() {
        let file_service = FileService;
        let test_file_path = PathBuf::from("test/mocks/test_2.txt");
        file_service
            .write_file(test_file_path.clone(), "Hello, world!")
            .unwrap();
        let content_first_read = std::fs::read_to_string(test_file_path.clone()).unwrap();
        assert_eq!(content_first_read, "Hello, world!");

        // Clean up
        std::fs::remove_file(test_file_path).unwrap();
    }

    #[test]
    fn test_create_file_should_create_empty_file() {
        let file_service = FileService;
        let test_file_path = PathBuf::from("test/mocks/test_3.txt");
        file_service.create_file(test_file_path.clone()).unwrap();
        let content_first_read = std::fs::read_to_string(&test_file_path.clone()).unwrap();
        assert_eq!(content_first_read, "");

        // Clean up
        std::fs::remove_file(test_file_path).unwrap();
    }
}
