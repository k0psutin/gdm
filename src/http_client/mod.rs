use std::collections::HashMap;

use reqwest::{Response};
use serde::{de::DeserializeOwned};
use url::Url;
use anyhow::{Result, anyhow};

pub struct HttpClient;

#[cfg(not(tarpaulin_include))]
pub async fn get<T>(url: String, params: HashMap<&str, &str>) -> Result<T> 
where
    T: DeserializeOwned,
{
    let _url = Url::parse_with_params(&url, params)?;
    match reqwest::get(_url.as_str()).await {
        Ok(response) => {
            let data = response.json::<T>().await?;
            Ok(data)
        }
        Err(e) => Err(anyhow!("Failed to fetch: {}", e)),
    }
}   

#[cfg(not(tarpaulin_include))]
pub async fn get_file(url: String) -> Result<Response> {
    let _url = Url::parse(&url)?;

    match reqwest::get(_url.as_str()).await {
        Ok(response) => Ok(response),
        Err(e) => Err(anyhow!("Failed to fetch file: {}", e)),
    }
} 
