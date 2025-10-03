mod asset_response;
mod asset_list_response;

use std::collections::HashMap;
use asset_response::AssetResponse;
use asset_list_response::AssetListResponse;

use crate::settings::Settings;
use crate::http_client;

pub struct AssetStoreAPI;

impl AssetStoreAPI {
    fn get_base_url() -> &'static str {
        let settings = Settings::get_settings().unwrap();
        settings.api_base_url
    }

    pub async fn fetch_asset_by_id(&self, asset_id: &str) -> anyhow::Result<AssetResponse> {
        match http_client::get( format!("{}/asset/{}", Self::get_base_url(), asset_id), [].into()).await {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow::anyhow!("Failed to fetch asset by ID: {}", e)),
        }
    }

    pub async fn search_assets(&self, params: HashMap<&str, &str>) -> anyhow::Result<AssetListResponse> {
        match http_client::get( format!("{}/asset", Self::get_base_url()), params).await {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow::anyhow!("Failed to fetch assets: {}", e)),
        }
    }
}
