use anyhow::{Context, Result};
use asset_store_client::models::{AssetData, AssetDataDetailed, ReleaseData};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Asset {
    pub file_path: PathBuf,
    pub publisher_slug: String,
    pub asset_slug: String,
    pub license: String,
    pub score: i32,
    pub version: String,
    pub title: String,
    pub description: String,
    /// Size in kB
    pub size: f32,
    pub download_url: String,
}

impl Asset {
    pub fn new(release_data: ReleaseData, asset_response: AssetDataDetailed) -> Result<Asset> {
        let publisher = asset_response
            .publisher
            .context("Asset response is missing publisher data")?;
        let publisher_slug = publisher
            .slug
            .context("Asset response is missing publisher slug")?;
        let asset_slug = asset_response
            .slug
            .context("Asset response is missing asset slug")?;
        let version = release_data
            .version
            .context("Release data is missing version")?;
        let download_url = release_data
            .download_url
            .context("Release data is missing download URL")?;

        Ok(Asset {
            file_path: PathBuf::from(""),
            publisher_slug,
            asset_slug,
            score: asset_response.reviews_score.unwrap_or_default(),
            license: asset_response
                .license_type
                .unwrap_or(String::from("Unknown")),
            version,
            title: asset_response.name.unwrap_or_default(),
            description: asset_response.description.unwrap_or_default(),
            size: release_data.size.unwrap_or_default() * 1000.0,
            download_url,
        })
    }

    pub fn with_file_path(mut self, file_path: PathBuf) -> Self {
        self.file_path = file_path;
        self
    }
}

impl TryFrom<AssetData> for Asset {
    type Error = anyhow::Error;

    fn try_from(asset: AssetData) -> Result<Self> {
        let publisher = asset
            .publisher
            .context("Asset data is missing publisher data")?;
        let publisher_slug = publisher
            .slug
            .context("Asset data is missing publisher slug")?;
        let asset_slug = asset.slug.context("Asset data is missing asset slug")?;
        let title = asset.name.context("Asset data is missing name")?;
        let description = asset
            .description
            .context("Asset data is missing description")?;

        Ok(Asset {
            file_path: PathBuf::from(""),
            publisher_slug,
            asset_slug,
            license: asset.license_type.unwrap_or(String::from("Unknown")),
            version: String::new(),
            title,
            description,
            score: asset.reviews_score.unwrap_or_default(),
            size: 0.0,
            download_url: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asset_store_client::models::PublisherData;

    fn asset_data() -> AssetData {
        AssetData {
            publisher: Some(Box::new(PublisherData {
                slug: Some("publisher".to_string()),
                ..Default::default()
            })),
            slug: Some("asset".to_string()),
            name: Some("Asset".to_string()),
            description: Some("Description".to_string()),
            license_type: Some("MIT".to_string()),
            ..Default::default()
        }
    }

    fn detailed_asset_data() -> AssetDataDetailed {
        AssetDataDetailed {
            publisher: Some(Box::new(PublisherData {
                slug: Some("publisher".to_string()),
                ..Default::default()
            })),
            slug: Some("asset".to_string()),
            name: Some("Asset".to_string()),
            description: Some("Description".to_string()),
            license_type: Some("MIT".to_string()),
            ..Default::default()
        }
    }

    fn release_data() -> ReleaseData {
        ReleaseData {
            version: Some("1.2.3".to_string()),
            download_url: Some("https://example.com/asset.zip".to_string()),
            size: Some(1.0),
            ..Default::default()
        }
    }

    #[test]
    fn test_asset_new_returns_asset_for_complete_metadata() {
        let asset = Asset::new(release_data(), detailed_asset_data()).unwrap();

        assert_eq!(asset.publisher_slug, "publisher");
        assert_eq!(asset.asset_slug, "asset");
        assert_eq!(asset.version, "1.2.3");
        assert_eq!(asset.download_url, "https://example.com/asset.zip");
    }

    #[test]
    fn test_asset_new_rejects_missing_publisher() {
        let mut detailed = detailed_asset_data();
        detailed.publisher = None;

        assert!(Asset::new(release_data(), detailed).is_err());
    }

    #[test]
    fn test_asset_new_rejects_missing_publisher_slug() {
        let mut detailed = detailed_asset_data();
        detailed.publisher.as_mut().unwrap().slug = None;

        assert!(Asset::new(release_data(), detailed).is_err());
    }

    #[test]
    fn test_asset_new_rejects_missing_asset_slug() {
        let mut detailed = detailed_asset_data();
        detailed.slug = None;

        assert!(Asset::new(release_data(), detailed).is_err());
    }

    #[test]
    fn test_asset_new_rejects_missing_release_version() {
        let mut release = release_data();
        release.version = None;

        assert!(Asset::new(release, detailed_asset_data()).is_err());
    }

    #[test]
    fn test_asset_new_rejects_missing_download_url() {
        let mut release = release_data();
        release.download_url = None;

        assert!(Asset::new(release, detailed_asset_data()).is_err());
    }

    #[test]
    fn test_asset_try_from_rejects_missing_required_fields() {
        let mut asset = asset_data();
        asset.publisher = None;

        let result = Asset::try_from(asset);

        assert!(result.is_err());
    }
}
