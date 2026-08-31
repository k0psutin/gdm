use anyhow::{Result, bail};
use reqwest::Response;
use tracing::{error, info};
use url::Url;

#[derive(Debug, Clone)]
pub struct DefaultHttpService {}

impl DefaultHttpService {
    pub fn new() -> DefaultHttpService {
        DefaultHttpService {}
    }
}

impl Default for DefaultHttpService {
    fn default() -> Self {
        DefaultHttpService::new()
    }
}

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
impl HttpService for DefaultHttpService {
    async fn get(&self, url: String) -> Result<Response> {
        let _url = Url::parse(&url)?;

        match reqwest::get(_url.as_str()).await {
            Ok(response) => {
                let status = response.status();
                info!("[GET] {} [{}]", _url, status.as_u16());

                if !status.is_success() {
                    bail!(status);
                }

                Ok(response)
            }
            Err(e) => {
                match e.status() {
                    Some(status) => error!("[GET] {} [{}] - Error: {}", _url, status, e),
                    None => error!("[GET] {} - Error: {}", _url, e),
                }
                bail!("Failed to fetch: {}", e)
            }
        }
    }
}

#[async_trait::async_trait]
pub trait HttpService: Send + Sync {
    async fn get(&self, url: String) -> Result<Response>;
}
