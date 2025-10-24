mod cache;

use anyhow::{Context, Result};
use bytes::Bytes;
use cache::Cache;
use std::{fs::File, path::Path};
use tracing::{debug, info};

use crate::file_service::cache::DefaultCache;

#[derive(Debug, Default, Clone)]
pub struct DefaultFileService;

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
impl FileService for DefaultFileService {
    fn open(&self, file_path: &Path) -> Result<File> {
        debug!("Opening file: {}", file_path.display());
        let file = File::open(file_path)
            .with_context(|| format!("Failed to open file: {}", file_path.display()))?;
        Ok(file)
    }

    fn read_file_cached(&self, file_path: &Path) -> Result<String> {
        debug!("Reading file with cache: {}", file_path.display());
        let cache = DefaultCache::new();
        let path = file_path
            .to_path_buf()
            .into_os_string()
            .into_string()
            .unwrap();
        if cache.has_key(&path) {
            debug!("Cache hit for key: {}", path);
            return Ok(cache.get(&path).unwrap().clone());
        }
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.to_str().unwrap()))?;
        cache.insert(&path, &content);
        debug!("Cache miss for key: {}", path);
        Ok(content)
    }

    fn file_exists(&self, file_path: &Path) -> Result<bool> {
        debug!("Checking if file exists: {}", file_path.display());
        file_path
            .try_exists()
            .with_context(|| format!("Failed to check if file exists: {}", file_path.display()))
    }

    fn write_file(&self, file_path: &Path, content: &str) -> Result<()> {
        debug!("Writing file: {}", file_path.display());
        std::fs::write(file_path, content)
            .with_context(|| format!("Failed to write file: {}", file_path.display()))?;
        Ok(())
    }

    fn create_file(&self, file_path: &Path) -> Result<File> {
        debug!("Creating file: {}", file_path.display());
        let file = std::fs::File::create(file_path)
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;
        Ok(file)
    }

    async fn create_file_async(&self, file_path: &Path) -> Result<tokio::fs::File> {
        debug!("Creating async file: {}", file_path.display());
        let file = tokio::fs::File::create(file_path)
            .await
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;
        Ok(file)
    }

    async fn write_all_async(&self, file: &mut tokio::fs::File, chunk: &Bytes) -> Result<()> {
        debug!("Writing all to async file");
        tokio::io::AsyncWriteExt::write_all(&mut *file, chunk)
            .await
            .with_context(|| "Failed to write to async file")?;
        Ok(())
    }

    fn create_directory(&self, dir_path: &Path) -> Result<()> {
        debug!("Creating directory: {}", dir_path.display());
        std::fs::create_dir_all(dir_path)
            .with_context(|| format!("Failed to create directory: {}", dir_path.display()))?;
        info!("Created directory: {}", dir_path.display());
        Ok(())
    }

    fn remove_dir_all(&self, dir_path: &Path) -> Result<()> {
        debug!("Removing directory: {}", dir_path.display());
        if self.directory_exists(dir_path) {
            std::fs::remove_dir_all(dir_path)
                .with_context(|| format!("Failed to remove directory: {}", dir_path.display()))?;
            info!("Removed directory: {}", dir_path.display());
        }
        Ok(())
    }

    fn directory_exists(&self, dir_path: &Path) -> bool {
        debug!("Checking if directory exists: {}", dir_path.display());
        dir_path.is_dir()
    }

    fn remove_file(&self, file_path: &Path) -> Result<()> {
        debug!("Removing file: {}", file_path.display());
        if self.file_exists(file_path)? {
            std::fs::remove_file(file_path)
                .with_context(|| format!("Failed to remove file: {}", file_path.display()))?;
            info!("Removed file: {}", file_path.display());
        }
        Ok(())
    }
}

#[async_trait::async_trait]
pub trait FileService: Send + Sync + 'static {
    fn open(&self, file_path: &Path) -> Result<File>;
    fn read_file_cached(&self, file_path: &Path) -> Result<String>;
    fn file_exists(&self, file_path: &Path) -> Result<bool>;
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
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_read_file_cached_should_cache_file() {
        let file_service = DefaultFileService;
        let test_file_path = Path::new("tests/mocks/test_2.txt");
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
}
