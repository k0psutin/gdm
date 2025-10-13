mod cache;

use anyhow::{Context, Result};
use cache::Cache;
use std::{fs::File, path::PathBuf};

pub struct FileService;

impl FileService {
    pub fn read_file_cached(file_path: &PathBuf) -> Result<String> {
        let cache = Cache::new();
        let path = file_path.to_str().unwrap();
        if cache.has_key(path) {
            return Ok(cache.get(path).unwrap().clone());
        }
        let content = std::fs::read_to_string(file_path).with_context(|| format!("Failed to read file: {}", file_path.to_str().unwrap()))?;
        cache.insert(path.to_string(), content.clone());
        Ok(content)
    }

    pub fn file_exists(file_path: &PathBuf) -> bool {
        file_path.try_exists().unwrap_or(false)
    }

    pub fn write_file(file_path: &PathBuf, content: &str) -> Result<()> {
        std::fs::write(file_path, content).with_context(|| format!("Failed to write file: {}", file_path.display()))?;
        Ok(())
    }

    pub fn create_file(file_path: &PathBuf) -> Result<File> {
        let file = File::create(file_path).with_context(|| format!("Failed to create file: {}", file_path.display()))?;
        Ok(file)
    }

    pub fn create_directory(dir_path: &PathBuf) -> Result<()> {
        std::fs::create_dir_all(dir_path).with_context(|| format!("Failed to create directory: {}", dir_path.display()))?;
        Ok(())
    }

    pub fn remove_file(file_path: &PathBuf) -> Result<()> {
        #[cfg(test)] {
            return Ok(());
        }
        if Self::file_exists(file_path) {
            std::fs::remove_file(file_path).with_context(|| format!("Failed to remove file: {}", file_path.display()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_file_cached_should_cache_file() {
        let test_file_path = PathBuf::from("test/mocks/test_1.txt");
        std::fs::write(&test_file_path, "Hello, world!").unwrap();
        let content_first_read = FileService::read_file_cached(&test_file_path).unwrap();
        assert_eq!(content_first_read, "Hello, world!");

        // Modify the file after the first read
        let modified_content = "Goodbye, world!";
        std::fs::write(&test_file_path, modified_content).unwrap();

        // Read again, should return cached content
        let content_second_read = FileService::read_file_cached(&test_file_path).unwrap();
        assert_eq!(content_second_read, "Hello, world!");

        // Clean up
        std::fs::remove_file(test_file_path).unwrap();
    }

    #[test]
    fn test_write_file_should_create_file_with_text() {
        let test_file_path = PathBuf::from("test/mocks/test_2.txt");
        FileService::write_file(&test_file_path, "Hello, world!").unwrap();
        let content_first_read = std::fs::read_to_string(&test_file_path).unwrap();
        assert_eq!(content_first_read, "Hello, world!");

        // Clean up
        std::fs::remove_file(&test_file_path).unwrap();
    }

    #[test]
    fn test_create_file_should_create_empty_file() {
        let test_file_path = PathBuf::from("test/mocks/test_3.txt");
        FileService::create_file(&test_file_path.clone()).unwrap();
        let content_first_read = std::fs::read_to_string(&test_file_path.clone()).unwrap();
        assert_eq!(content_first_read, "");

        // Clean up
        std::fs::remove_file(test_file_path).unwrap();
    }
}
