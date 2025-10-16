use std::collections::HashMap;

use anyhow::{Result, anyhow};
use reqwest::Response;
use serde::de::{DeserializeOwned};
use serde_json;
use url::Url;
use tracing::{debug, error, info};

#[derive(Debug, Clone)]
pub struct HttpClient {}

impl HttpClient {
    pub fn new() -> HttpClient {
        HttpClient {}
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        HttpClient::new()
    }
}

impl HttpClientImpl for HttpClient {}

pub trait HttpClientImpl {
    fn get_url(&self, base_url: String, path: String) -> String {
        format!("{}{}", base_url, path)
    }

    async fn get<T>(&self, base_url: String, path: String, params: HashMap<&str, &str>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let _url = Url::parse_with_params(&self.get_url(base_url, path), params)?;

        match reqwest::get(_url.as_str()).await {
            Ok(response) => {
                info!("[GET] {} [{}]", _url, response.status());
                let body = response.text().await?;
                debug!("Response Body: {}", body);
                let data = serde_json::from_str::<T>(&body)?;
                Ok(data)
            }
            Err(e) => {
                match e.status() {
                    Some(status) => error!("[GET] {} [{}] - Error: {}", _url, status, e),
                    None => error!("[GET] {} - Error: {}", _url, e),
                }
                Err(anyhow!("Failed to fetch data: {}", e))
            },
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
            },
        }
    }
}
