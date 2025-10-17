mod cache;

use anyhow::{Context, Result};
use cache::Cache;
use std::{fs::File, path::Path};

use crate::file_service::cache::CacheImpl;

#[derive(Debug, Default, Clone)]
pub struct DefaultFileService;

#[cfg_attr(test, mockall::automock)]
impl FileService for DefaultFileService {
    fn open(&self, file_path: &Path) -> Result<File> {
        let file = File::open(file_path)
            .with_context(|| format!("Failed to open file: {}", file_path.display()))?;
        Ok(file)
    }

    fn read_file_cached(&self, file_path: &Path) -> Result<String> {
        let cache = Cache::new();
        let path = file_path
            .to_path_buf()
            .into_os_string()
            .into_string()
            .unwrap();
        if cache.has_key(&path) {
            return Ok(cache.get(&path).unwrap().clone());
        }
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.to_str().unwrap()))?;
        cache.insert(&path, &content);
        Ok(content)
    }

    fn file_exists(&self, file_path: &Path) -> bool {
        file_path.try_exists().unwrap_or(false)
    }

    fn write_file(&self, file_path: &Path, content: &str) -> Result<()> {
        std::fs::write(file_path, content)
            .with_context(|| format!("Failed to write file: {}", file_path.display()))?;
        Ok(())
    }

    fn create_file(&self, file_path: &Path) -> Result<File> {
        let file = File::create(file_path)
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;
        Ok(file)
    }

    fn create_directory(&self, dir_path: &Path) -> Result<()> {
        std::fs::create_dir_all(dir_path)
            .with_context(|| format!("Failed to create directory: {}", dir_path.display()))?;
        Ok(())
    }

    fn remove_dir_all(&self, dir_path: &Path) -> Result<()> {
        if self.directory_exists(dir_path) {
            std::fs::remove_dir_all(dir_path)
                .with_context(|| format!("Failed to remove directory: {}", dir_path.display()))?;
        }
        Ok(())
    }

    fn directory_exists(&self, dir_path: &Path) -> bool {
        dir_path.is_dir()
    }

    fn remove_file(&self, file_path: &Path) -> Result<()> {
        if self.file_exists(file_path) {
            std::fs::remove_file(file_path)
                .with_context(|| format!("Failed to remove file: {}", file_path.display()))?;
        }
        Ok(())
    }
}

pub trait FileService: Send + Sync + 'static {
    fn open(&self, file_path: &Path) -> Result<File>;
    fn read_file_cached(&self, file_path: &Path) -> Result<String>;
    fn file_exists(&self, file_path: &Path) -> bool;
    fn write_file(&self, file_path: &Path, content: &str) -> Result<()>;
    fn create_file(&self, file_path: &Path) -> Result<File>;
    fn create_directory(&self, dir_path: &Path) -> Result<()>;
    fn remove_dir_all(&self, dir_path: &Path) -> Result<()>;
    fn directory_exists(&self, dir_path: &Path) -> bool;
    fn remove_file(&self, file_path: &Path) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_file_cached_should_cache_file() {
        let file_service = DefaultFileService;
        let test_file_path = Path::new("test/mocks/test_1.txt");
        std::fs::write(test_file_path, "Hello, world!").unwrap();
        let content_first_read = file_service.read_file_cached(test_file_path).unwrap();
        assert_eq!(content_first_read, "Hello, world!");

        // Modify the file after the first read
        let modified_content = "Goodbye, world!";
        std::fs::write(test_file_path, modified_content).unwrap();

        // Read again, should return cached content
        let content_second_read = file_service.read_file_cached(test_file_path).unwrap();
        assert_eq!(content_second_read, "Hello, world!");

        // Clean up
        std::fs::remove_file(test_file_path).unwrap();
    }

    #[test]
    fn test_write_file_should_create_file_with_text() {
        let file_service = DefaultFileService;
        let test_file_path = Path::new("test/mocks/test_2.txt");
        file_service
            .write_file(test_file_path, "Hello, world!")
            .unwrap();
        let content_first_read = std::fs::read_to_string(test_file_path).unwrap();
        assert_eq!(content_first_read, "Hello, world!");

        // Clean up
        std::fs::remove_file(test_file_path).unwrap();
    }

    #[test]
    fn test_create_file_should_create_empty_file() {
        let file_service = DefaultFileService;
        let test_file_path = Path::new("test/mocks/test_3.txt");
        file_service.create_file(test_file_path).unwrap();
        let content_first_read = std::fs::read_to_string(test_file_path).unwrap();
        assert_eq!(content_first_read, "");

        // Clean up
        std::fs::remove_file(test_file_path).unwrap();
    }
}
