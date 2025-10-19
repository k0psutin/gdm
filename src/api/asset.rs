use crate::api::asset_response::AssetResponse;

use std::path::PathBuf;

#[cfg(not(tarpaulin_include))]
#[derive(Debug, Clone)]
pub struct Asset {
    pub root_folder: String,
    pub file_path: PathBuf,
    pub asset_response: AssetResponse,
}

#[cfg(not(tarpaulin_include))]
impl Asset {
    pub fn new(root_folder: String, file_path: PathBuf, asset_response: AssetResponse) -> Asset {
        Asset {
            root_folder,
            file_path,
            asset_response,
        }
    }
}
