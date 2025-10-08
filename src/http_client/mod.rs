use std::collections::HashMap;

use reqwest::{Response};
use serde::{de::DeserializeOwned};
use url::Url;
use anyhow::{Result, anyhow};

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

pub async fn get_file(url: String) -> Result<Response> {
    let _url = Url::parse(&url)?;

    match reqwest::get(_url.as_str()).await {
        Ok(response) => Ok(response),
        Err(e) => Err(anyhow!("Failed to fetch file: {}", e)),
    }
} 

#[cfg(test)]
mod tests {
    struct TestResources {
        _temp_file: String,
    }

    impl Drop for TestResources {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self._temp_file);
        }
    }

    use super::*;

    

    #[test]
    fn test_get_file() {
        let url = "https://github.com/levinzonr/godot-asset-placer/archive/bf54218c5f3bb1d37f4121242197de4f40459b68.zip".to_string();
        let result = tokio::runtime::Runtime::new().unwrap().block_on(get_file(url));
      
        assert!(result.is_ok());
    }
}