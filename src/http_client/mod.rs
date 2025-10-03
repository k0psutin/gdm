use std::collections::HashMap;

use reqwest;
use serde::{de::DeserializeOwned};
use url::Url;
use bytes::Bytes;

pub async fn get<T>(url: String, params: HashMap<&str, &str>) -> anyhow::Result<T> 
where
    T: DeserializeOwned,
{
    let _url = Url::parse_with_params(&url, params)?;
    match reqwest::get(_url.as_str()).await {
        Ok(response) => {
            let data = response.json::<T>().await?;
            Ok(data)
        }
        Err(e) => Err(anyhow::anyhow!("Failed to fetch: {}", e)),
    }
}   

pub async fn get_file(url: String) -> anyhow::Result<Bytes> {
    let _url = Url::parse(&url)?;
    match reqwest::get(_url.as_str()).await {
        Ok(response) => {
            let data = response.bytes().await?;
            Ok(data)
        }
        Err(e) => Err(anyhow::anyhow!("Failed to fetch: {}", e)),
    }
}   