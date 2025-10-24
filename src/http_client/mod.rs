use std::collections::HashMap;

use anyhow::{Result, bail};
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
    async fn get(&self, url: String, params: HashMap<String, String>) -> Result<Value> {
        let _url = Url::parse_with_params(&url, params)?;

        match reqwest::get(_url.as_str()).await {
            Ok(response) => {
                let status = response.status();
                info!("[GET] {} [{}]", _url, status);

                if !status.is_success() {
                    bail!(status);
                }

                let data = response.json().await?;
                Ok(data)
            }
            Err(e) => {
                match e.status() {
                    Some(status) => error!("[GET] {} [{}] - Error: {}", _url, status, e),
                    None => error!("[GET] {} - Error: {}", _url, e),
                }
                bail!("Failed to fetch data: {}", e)
            }
        }
    }

    async fn get_file(&self, url: String) -> Result<Response> {
        let _url = Url::parse(&url)?;

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
                bail!("Failed to fetch file: {}", e)
            }
        }
    }
}

#[async_trait::async_trait]
pub trait HttpClient: Send + Sync {
    async fn get(&self, url: String, params: HashMap<String, String>) -> Result<Value>;

    async fn get_file(&self, url: String) -> Result<Response>;
}
