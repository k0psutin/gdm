mod cache;

use anyhow::{Context, Result};
use bytes::Bytes;
use cache::Cache;
use std::{fs::File, path::Path};

use crate::file_service::cache::DefaultCache;

#[derive(Debug, Default, Clone)]
pub struct DefaultFileService;

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
impl FileService for DefaultFileService {
    fn open(&self, file_path: &Path) -> Result<File> {
        let file = File::open(file_path)
            .with_context(|| format!("Failed to open file: {}", file_path.display()))?;
        Ok(file)
    }

    fn read_file_cached(&self, file_path: &Path) -> Result<String> {
        let cache = DefaultCache::new();
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
        let file = std::fs::File::create(file_path)
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;
        Ok(file)
    }

    async fn create_file_async(&self, file_path: &Path) -> Result<tokio::fs::File> {
        let file = tokio::fs::File::create(file_path)
            .await
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;
        Ok(file)
    }

    async fn write_all_async(&self, file: &mut tokio::fs::File, chunk: &Bytes) -> Result<()> {
        tokio::io::AsyncWriteExt::write_all(&mut *file, chunk)
            .await
            .with_context(|| "Failed to write to async file")?;
        Ok(())
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

#[async_trait::async_trait]
pub trait FileService: Send + Sync + 'static {
    fn open(&self, file_path: &Path) -> Result<File>;
    fn read_file_cached(&self, file_path: &Path) -> Result<String>;
    fn file_exists(&self, file_path: &Path) -> bool;
    fn write_file(&self, file_path: &Path, content: &str) -> Result<()>;
    fn create_file(&self, file_path: &Path) -> Result<File>;
    async fn create_file_async(&self, file_path: &Path) -> Result<tokio::fs::File>;
    fn create_directory(&self, dir_path: &Path) -> Result<()>;
    fn remove_dir_all(&self, dir_path: &Path) -> Result<()>;
    fn directory_exists(&self, dir_path: &Path) -> bool;
    fn remove_file(&self, file_path: &Path) -> Result<()>;
    async fn write_all_async(&self, file: &mut tokio::fs::File, chunk: &Bytes) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // open

    #[test]
    fn test_open_should_return_file() {
        let file_service = DefaultFileService;
        let test_file_path = Path::new("test/mocks/test_1.txt");
        std::fs::write(test_file_path, "Hello, world!").unwrap();
        let file = file_service.open(test_file_path).unwrap();
        assert!(file.metadata().unwrap().is_file());

        // Clean up
        std::fs::remove_file(test_file_path).unwrap();
    }

    // read_file_cached

    #[test]
    fn test_read_file_cached_should_cache_file() {
        let file_service = DefaultFileService;
        let test_file_path = Path::new("test/mocks/test_2.txt");
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

    // write_file

    #[test]
    fn test_write_file_should_create_file_with_text() {
        let file_service = DefaultFileService;
        let test_file_path = Path::new("test/mocks/test_3.txt");
        file_service
            .write_file(test_file_path, "Hello, world!")
            .unwrap();
        let content_first_read = std::fs::read_to_string(test_file_path).unwrap();
        assert_eq!(content_first_read, "Hello, world!");

        // Clean up
        std::fs::remove_file(test_file_path).unwrap();
    }

    // create_file

    #[tokio::test]
    async fn test_create_file_should_create_empty_file() {
        let file_service = DefaultFileService;
        let test_file_path = Path::new("test/mocks/test_4.txt");
        file_service.create_file(test_file_path).unwrap();
        let content_first_read = std::fs::read_to_string(test_file_path).unwrap();
        assert_eq!(content_first_read, "");

        // Clean up
        std::fs::remove_file(test_file_path).unwrap();
    }

    // file_exists

    #[test]
    fn test_file_exists_should_return_true_for_existing_file() {
        let file_service = DefaultFileService;
        let test_file_path = Path::new("test/mocks/gdm.json");
        let exists = file_service.file_exists(test_file_path);
        assert!(exists);
    }

    // create_file_async

    #[tokio::test]
    async fn test_create_file_async_should_create_empty_file() {
        let file_service = DefaultFileService;
        let test_file_path = Path::new("test/mocks/test_5.txt");
        file_service
            .create_file_async(test_file_path)
            .await
            .unwrap();
        let content_first_read = std::fs::read_to_string(test_file_path).unwrap();
        assert_eq!(content_first_read, "");
        std::fs::remove_file(test_file_path).unwrap();
    }

    // create_directory

    #[test]
    fn test_create_directory_should_create_directory() {
        let file_service = DefaultFileService;
        let test_dir_path = Path::new("test/mocks/test_dir");
        file_service.create_directory(test_dir_path).unwrap();
        assert!(test_dir_path.exists());
        std::fs::remove_dir_all(test_dir_path).unwrap();
    }

    // remove_dir_all

    #[test]
    fn test_remove_dir_all_should_remove_directory() {
        let file_service = DefaultFileService;
        let test_dir_path = Path::new("test/mocks/test_dir_to_remove/subdir");
        std::fs::create_dir_all(test_dir_path).unwrap();
        file_service
            .remove_dir_all(Path::new("test/mocks/test_dir_to_remove"))
            .unwrap();
        assert!(!test_dir_path.exists());
    }

    // directory_exists

    #[test]
    fn test_directory_exists_should_return_true_for_existing_directory() {
        let file_service = DefaultFileService;
        let test_dir_path = Path::new("test/mocks");
        let exists = file_service.directory_exists(test_dir_path);
        assert!(exists);
    }

    // remove_file

    #[test]
    fn test_remove_file_should_remove_existing_file() {
        let file_service = DefaultFileService;
        let test_file_path = Path::new("test/mocks/test_file_to_remove.txt");
        std::fs::write(test_file_path, "Hello, world!").unwrap();
        file_service.remove_file(test_file_path).unwrap();
        assert!(!test_file_path.exists());
    }

    // write_all_async

    #[tokio::test]
    async fn test_write_all_async_should_write_to_file() {
        let file_service = DefaultFileService;
        let test_file_path = Path::new("test/mocks/test_async_write.txt");
        let mut file = file_service
            .create_file_async(test_file_path)
            .await
            .unwrap();
        let content = Bytes::from("Hello, async world!");
        file_service
            .write_all_async(&mut file, &content)
            .await
            .unwrap();
        let read_content = std::fs::read_to_string(test_file_path).unwrap();
        assert_eq!(read_content, "Hello, async world!");
        std::fs::remove_file(test_file_path).unwrap();
    }
}
