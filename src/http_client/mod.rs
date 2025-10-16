use std::collections::HashMap;

use anyhow::{Result, anyhow};
use reqwest::Response;
use serde::de::DeserializeOwned;
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

    async fn get<T: DeserializeOwned + Send + 'static>(
        &self,
        base_url: String,
        path: String,
        params: HashMap<String, String>,
    ) -> Result<T> {
        let _url = Url::parse_with_params(&self.get_url(base_url, path), params)?;

        match reqwest::get(_url.as_str()).await {
            Ok(response) => {
                info!("[GET] {} [{}]", _url, response.status());
                let data = response.json::<T>().await?;
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
    async fn get<T: DeserializeOwned + Send + 'static>(
        &self,
        base_url: String,
        path: String,
        params: HashMap<String, String>,
    ) -> Result<T>
    where
        T: DeserializeOwned;
    async fn get_file(&self, file_url: String) -> Result<Response>;
}
