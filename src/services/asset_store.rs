use asset_store_client::apis::configuration::Configuration;
use asset_store_client::models::{AssetData, SearchResults};
use futures::future::{join_all, try_join_all};

use crate::config::{AppConfig, DefaultAppConfig};
use crate::models::Asset;
use crate::services::{DefaultFileService, DefaultHttpService, FileService, HttpService};

use anyhow::{Context, Result, anyhow, bail};
use asset_store_client::apis::{assets_api, releases_api, searching_api};
use indicatif::ProgressBar;
use std::collections::HashSet;
use std::future::Future;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use url::Url;

pub struct DefaultAssetStoreService {
    pub app_config: DefaultAppConfig,
    pub http_service: Arc<dyn HttpService + Send + Sync + 'static>,
    pub file_service: Arc<dyn FileService + Send + Sync + 'static>,
}

impl DefaultAssetStoreService {
    #[allow(unused)]
    pub fn new(
        app_config: DefaultAppConfig,
        http_service: Arc<dyn HttpService + Send + Sync + 'static>,
        file_service: Arc<dyn FileService + Send + Sync + 'static>,
    ) -> DefaultAssetStoreService {
        DefaultAssetStoreService {
            app_config,
            http_service,
            file_service,
        }
    }

    async fn resolve_search_assets<'a>(
        &self,
        hits: Vec<AssetData>,
        version: Option<&'a str>,
        godot_version: Option<&'a str>,
    ) -> Result<Vec<Asset>> {
        let asset_futures = hits.into_iter().filter_map(|asset| {
            let publisher_slug = asset.publisher?.slug?;
            let asset_slug = asset.slug?;

            Some(async move {
                self.get_asset(&publisher_slug, &asset_slug, version, godot_version)
                    .await
            })
        });

        if let Some(version) = version {
            let expected_missing_version = format!("Dependency with version {version} not found");
            let mut assets = Vec::new();
            for result in join_all(asset_futures).await {
                match result {
                    Ok(asset) => assets.push(asset),
                    Err(error) if error.to_string() == expected_missing_version => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(assets)
        } else {
            try_join_all(asset_futures).await
        }
    }
}

impl Default for DefaultAssetStoreService {
    fn default() -> Self {
        DefaultAssetStoreService {
            app_config: DefaultAppConfig::default(),
            http_service: Arc::new(DefaultHttpService::default()),
            file_service: Arc::new(DefaultFileService),
        }
    }
}

async fn collect_search_assets<F, Fut>(
    mut fetch_page: F,
    fetch_all_pages: bool,
) -> Result<Vec<AssetData>>
where
    F: FnMut(Option<String>, Option<i32>, Option<i32>) -> Fut,
    Fut: Future<Output = Result<SearchResults>>,
{
    let mut search_assets = Vec::new();
    let mut seen_scroll_tokens = HashSet::new();
    let mut scroll: Option<String> = None;
    let mut page = Some(1);

    loop {
        let results = fetch_page(scroll.clone(), page, Some(10)).await?;

        search_assets.extend(
            results
                .hits
                .unwrap_or_default()
                .into_iter()
                .filter_map(|hit| hit.asset.map(|asset| *asset)),
        );

        if !fetch_all_pages {
            break;
        }

        let Some(next_scroll) = results.scroll.flatten() else {
            break;
        };

        if !seen_scroll_tokens.insert(next_scroll.clone()) {
            bail!("repeated scroll token encountered while searching dependencies: {next_scroll}");
        }
        scroll = Some(next_scroll);
        page = None;
    }

    Ok(search_assets)
}

fn normalized_name(name: &str) -> String {
    name.trim().to_lowercase()
}

fn cache_path_component(value: &str) -> String {
    let value: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();

    if value.is_empty() || value == "." || value == ".." {
        "unknown".to_string()
    } else {
        value
    }
}

fn matches_search_term(asset: &AssetData, search_term: &str) -> bool {
    asset
        .name
        .as_deref()
        .is_some_and(|name| normalized_name(name).contains(search_term))
        || asset
            .slug
            .as_deref()
            .is_some_and(|slug| normalized_name(slug).contains(search_term))
}

#[async_trait::async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait AssetStoreService: Send + Sync {
    /// Fetches a list of assets based on query parameters.
    async fn search_assets<'a>(
        &self,
        name: &str,
        version: Option<&'a str>,
        godot_version: Option<&'a str>,
    ) -> Result<Vec<Asset>>;

    /// Finds dependencies whose display name or Asset Store slug contains the supplied search term.
    async fn search_assets_by_name<'a>(
        &self,
        name: &str,
        version: Option<&'a str>,
        godot_version: Option<&'a str>,
    ) -> Result<Vec<Asset>>;

    async fn get_asset<'a>(
        &self,
        publisher_slug: &str,
        asset_slug: &str,
        version: Option<&'a str>,
        godot_version: Option<&'a str>,
    ) -> Result<Asset>;

    /// Downloads an asset and reports progress via a progress bar.
    async fn download_asset(&self, asset: Asset, pb_task: ProgressBar) -> Result<Asset>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
impl AssetStoreService for DefaultAssetStoreService {
    async fn search_assets<'a>(
        &self,
        name: &str,
        version: Option<&'a str>,
        godot_version: Option<&'a str>,
    ) -> Result<Vec<Asset>> {
        let configuration = Configuration::default();
        let query = name.trim().to_string();
        let godot_version = godot_version.map(str::to_string);
        let godot_version_for_search = godot_version.clone();

        let hits = collect_search_assets(
            move |scroll, page, batch_size| {
                let configuration = configuration.clone();
                let query = query.clone();
                let godot_version = godot_version_for_search.clone();
                async move {
                    searching_api::api_v1_search_query_get(
                        &configuration,
                        &query,
                        Some(false), // No featured
                        Some(true),  // Require release
                        Some(true),  // Stable versions only
                        godot_version.as_deref(),
                        Some("relevance"),
                        None,
                        scroll.as_deref(),
                        page,
                        batch_size,
                    )
                    .await
                    .map_err(|error| anyhow!("Failed to search dependencies: {error}"))
                }
            },
            false,
        )
        .await?;

        self.resolve_search_assets(hits, version, godot_version.as_deref())
            .await
    }

    async fn search_assets_by_name<'a>(
        &self,
        name: &str,
        version: Option<&'a str>,
        godot_version: Option<&'a str>,
    ) -> Result<Vec<Asset>> {
        let configuration = Configuration::default();
        let query = name.trim().to_string();
        let search_term = normalized_name(&query);
        if search_term.is_empty() {
            return Ok(Vec::new());
        }
        let godot_version_owned = godot_version.map(str::to_string);

        let hits = collect_search_assets(
            move |scroll, page, batch_size| {
                let configuration = configuration.clone();
                let query = query.clone();
                let godot_version = godot_version_owned.clone();
                async move {
                    searching_api::api_v1_search_query_get(
                        &configuration,
                        &query,
                        Some(false), // No featured
                        Some(true),  // Require release
                        Some(true),  // Stable versions only
                        godot_version.as_deref(),
                        Some("relevance"),
                        None,
                        scroll.as_deref(),
                        page,
                        batch_size,
                    )
                    .await
                    .map_err(|error| anyhow!("Failed to search dependencies: {error}"))
                }
            },
            true,
        )
        .await?;

        let matching_hits = hits
            .into_iter()
            .filter(|asset| matches_search_term(asset, &search_term))
            .collect();

        self.resolve_search_assets(matching_hits, version, godot_version)
            .await
    }

    async fn get_asset<'a>(
        &self,
        publisher_slug: &str,
        asset_slug: &str,
        version: Option<&'a str>,
        godot_version: Option<&'a str>,
    ) -> Result<Asset> {
        let asset_data_detailed = assets_api::api_v1_assets_publisher_slug_asset_slug_get(
            &Configuration::default(),
            publisher_slug,
            asset_slug,
        )
        .await
        .with_context(|| "Failed to get dependency")?;

        let release_datas = releases_api::api_v1_releases_publisher_slug_asset_slug_get(
            &Configuration::default(),
            publisher_slug,
            asset_slug,
            Some(true),
            godot_version,
        )
        .await
        .with_context(|| "Failed to get releases")?;

        if release_datas.is_empty() {
            let publisher = asset_data_detailed.publisher.unwrap_or_default();
            bail!(
                "No releases found for {}/{}",
                publisher.slug.unwrap_or("Unknown publisher".to_string()),
                asset_data_detailed
                    .slug
                    .unwrap_or("Unknown dependency".to_string()),
            );
        }

        if let Some(version) = version {
            let release_data = release_datas
                .iter()
                .find(|release| release.version.as_deref() == Some(version))
                .with_context(|| format!("Dependency with version {version} not found"))?;

            return Asset::new(release_data.clone(), asset_data_detailed)
                .context("Failed to construct dependency from release data");
        }

        let release_data = release_datas
            .first()
            .with_context(|| "Unexpected error with release data")?;

        Asset::new(release_data.clone(), asset_data_detailed)
            .context("Failed to construct dependency from release data")
    }

    /// Downloads a plugin from the Asset Store and returns a Asset struct
    ///
    /// Downloaded files are saved to the cache folder defined in the AppConfig
    async fn download_asset(&self, asset: Asset, pb_task: ProgressBar) -> Result<Asset> {
        let cache_folder = self.app_config.get_cache_folder_path();

        let download_url = asset.download_url.clone();

        let url = Url::parse(&download_url)?;

        let filename = url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .map(cache_path_component)
            .filter(|filename| filename != "." && filename != "..")
            .unwrap_or_else(|| "temp_file.zip".to_string());
        let asset_cache_dir = cache_folder
            .join("assets")
            .join(cache_path_component(&asset.publisher_slug))
            .join(cache_path_component(&asset.asset_slug))
            .join(cache_path_component(&asset.version));

        if !self.file_service.directory_exists(cache_folder) {
            self.file_service.create_directory(cache_folder)?;
        }

        if !self.file_service.directory_exists(&asset_cache_dir) {
            self.file_service.create_directory(&asset_cache_dir)?;
        }

        let file_path = asset_cache_dir.join(filename);

        if self.file_service.file_exists(&file_path)? {
            self.file_service.remove_file(&file_path)?;
        }

        let mut res = self.http_service.get(download_url.to_string()).await?;

        // Prefer actual response content-length if available
        if let Some(content_len) = res.content_length() {
            pb_task.set_length(content_len);
        } else if let Some(size_kb) = Some(asset.size) {
            // Do we get release size from asset data detailed?
            if size_kb.is_finite() && size_kb >= 0.0 {
                pb_task.set_length((size_kb * 1024.0).round() as u64);
            }
        }

        let mut file = self.file_service.create_file_async(&file_path).await?;

        while let Some(chunk) = res.chunk().await? {
            pb_task.inc(chunk.len() as u64);
            self.file_service.write_all_async(&mut file, &chunk).await?;
        }

        file.flush().await?;
        pb_task.finish_and_clear();

        match res.error_for_status() {
            Ok(_) => Ok(asset.with_file_path(file_path)),
            Err(e) => bail!("Failed to fetch file: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asset_store_client::models::{PublisherData, SearchResultHit};
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    fn search_asset(name: &str, publisher_slug: &str, asset_slug: &str) -> AssetData {
        AssetData {
            slug: Some(asset_slug.to_string()),
            publisher: Some(Box::new(PublisherData {
                slug: Some(publisher_slug.to_string()),
                ..Default::default()
            })),
            name: Some(name.to_string()),
            ..Default::default()
        }
    }

    fn search_page(assets: Vec<AssetData>, scroll: Option<&str>) -> SearchResults {
        SearchResults {
            hits: Some(
                assets
                    .into_iter()
                    .map(|asset| SearchResultHit {
                        asset: Some(Box::new(asset)),
                        ..Default::default()
                    })
                    .collect(),
            ),
            scroll: Some(scroll.map(str::to_string)),
            ..Default::default()
        }
    }

    #[test]
    fn test_matches_search_term_is_case_insensitive_and_trims_whitespace() {
        let asset = search_asset("GdUnit4", "mikeschulze", "gdunit4");
        let search_term = normalized_name("  gdunit4  ");

        assert!(matches_search_term(&asset, &search_term));
    }

    #[test]
    fn test_matches_search_term_matches_partial_title_or_slug() {
        let asset = search_asset("GdUnit4 - Unit Testing Framework", "mikeschulze", "gdunit4");
        assert!(matches_search_term(&asset, "unit4"));
        assert!(matches_search_term(&asset, "testing"));
        assert!(matches_search_term(&asset, "gdunit"));
    }

    #[test]
    fn test_matches_search_term_rejects_unrelated_terms() {
        let asset = search_asset("GdUnit4 - Unit Testing Framework", "mikeschulze", "gdunit4");

        assert!(!matches_search_term(&asset, "dialogue"));
    }

    #[tokio::test]
    async fn test_search_assets_by_name_rejects_empty_terms_without_searching() {
        let service = DefaultAssetStoreService::default();

        assert!(
            service
                .search_assets_by_name("  ", None, None)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn test_collect_search_assets_follows_scroll_pages() {
        let pages = VecDeque::from([
            Ok(search_page(
                vec![search_asset("First", "publisher", "first")],
                Some("next-page"),
            )),
            Ok(search_page(
                vec![search_asset("Second", "publisher", "second")],
                None,
            )),
        ]);
        let calls = Arc::new(Mutex::new(Vec::new()));
        let calls_for_fetch = Arc::clone(&calls);
        let mut pages = pages;

        let assets = collect_search_assets(
            move |scroll, page, batch_size| {
                calls_for_fetch
                    .lock()
                    .unwrap()
                    .push((scroll.clone(), page, batch_size));
                let result = pages.pop_front().expect("unexpected search page request");
                async move { result }
            },
            true,
        )
        .await
        .unwrap();

        assert_eq!(assets.len(), 2);
        assert_eq!(
            *calls.lock().unwrap(),
            vec![
                (None, Some(1), Some(10)),
                (Some("next-page".to_string()), None, Some(10)),
            ]
        );
    }

    #[tokio::test]
    async fn test_collect_search_assets_rejects_repeated_scroll_token() {
        let calls = Arc::new(Mutex::new(0));
        let calls_for_fetch = Arc::clone(&calls);

        let result = collect_search_assets(
            move |_scroll, _page, _batch_size| {
                let mut calls = calls_for_fetch.lock().unwrap();
                *calls += 1;
                let call = *calls;
                drop(calls);

                async move {
                    match call {
                        1 => Ok(search_page(
                            vec![search_asset("First", "publisher", "first")],
                            Some("same-page"),
                        )),
                        2 => Ok(search_page(
                            vec![search_asset("Second", "publisher", "second")],
                            Some("same-page"),
                        )),
                        _ => Err(anyhow!("unexpected third page request")),
                    }
                }
            },
            true,
        )
        .await;

        let error = result.unwrap_err();
        assert!(error.to_string().contains("repeated scroll token"));
    }
}
