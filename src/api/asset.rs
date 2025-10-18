use crate::api::asset_response::AssetResponse;

use std::path::PathBuf;

#[cfg(not(tarpaulin_include))]
#[derive(Debug, Clone)]
pub struct Asset {
    root_folder: String,
    file_path: PathBuf,
    asset_response: AssetResponse,
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

    pub fn get_root_folder(&self) -> String {
        self.root_folder.clone()
    }

    pub fn get_file_path(&self) -> &PathBuf {
        &self.file_path
    }

    pub fn get_asset_response(&self) -> &AssetResponse {
        &self.asset_response
    }
}
