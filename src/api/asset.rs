use crate::api::asset_response::AssetResponse;

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Asset {
    pub file_path: PathBuf,
    pub asset_response: AssetResponse,
}

impl Asset {
    pub fn new(file_path: PathBuf, asset_response: AssetResponse) -> Asset {
        Asset {
            file_path,
            asset_response,
        }
    }
}
