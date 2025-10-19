use std::collections::HashMap;

use anyhow::{Result, anyhow};
use reqwest::Response;
use serde_json::Value;
use tracing::{error, info};
use url::Url;

#[derive(Debug, Clone)]
pub struct DefaultHttpClient {}

impl DefaultHttpClient {
    pub fn new() -> DefaultHttpClient {
        DefaultHttpClient {}
    }
}

impl Default for DefaultHttpClient {
    fn default() -> Self {
        DefaultHttpClient::new()
    }
}

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
impl HttpClient for DefaultHttpClient {
    fn get_url(&self, base_url: String, path: String) -> String {
        format!("{}{}", base_url, path)
    }

    #[cfg(not(tarpaulin_include))]
    async fn get(
        &self,
        base_url: String,
        path: String,
        params: HashMap<String, String>,
    ) -> Result<Value> {
        let _url = Url::parse_with_params(&self.get_url(base_url, path), params)?;

        match reqwest::get(_url.as_str()).await {
            Ok(response) => {
                info!("[GET] {} [{}]", _url, response.status());
                let data = response.json().await?;
                Ok(data)
            }
            Err(e) => {
                match e.status() {
                    Some(status) => error!("[GET] {} [{}] - Error: {}", _url, status, e),
                    None => error!("[GET] {} - Error: {}", _url, e),
                }
                Err(anyhow!("Failed to fetch data: {}", e))
            }
        }
    }

    #[cfg(not(tarpaulin_include))]
    async fn get_file(&self, file_url: String) -> Result<Response> {
        let _url = Url::parse(&file_url)?;

        match reqwest::get(_url.as_str()).await {
            Ok(response) => {
                info!("[GET] {} [{}]", _url, response.status());
                Ok(response)
            }
            Err(e) => {
                match e.status() {
                    Some(status) => error!("[GET] {} [{}] - Error: {}", _url, status, e),
                    None => error!("[GET] {} - Error: {}", _url, e),
                }
                Err(anyhow!("Failed to fetch file: {}", e))
            }
        }
    }
}

#[async_trait::async_trait]
pub trait HttpClient: Send + Sync {
    fn get_url(&self, base_url: String, path: String) -> String;

    async fn get(
        &self,
        base_url: String,
        path: String,
        params: HashMap<String, String>,
    ) -> Result<Value>;

    async fn get_file(&self, file_url: String) -> Result<Response>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_get_url() {
        let http_client = DefaultHttpClient::new();
        let base_url = "https://api.example.com/";
        let path = "endpoint";
        let full_url = http_client.get_url(base_url.to_string(), path.to_string());
        assert_eq!(full_url, "https://api.example.com/endpoint");
    }
}
