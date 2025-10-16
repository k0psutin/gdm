use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Plugin {
    asset_id: String,
    title: String,
    version: String,
    license: String,
}

impl Plugin {
    pub fn new(asset_id: String, title: String, version: String, license: String) -> Plugin {
        Plugin {
            asset_id,
            title,
            version,
            license,
        }
    }

    pub fn get_asset_id(&self) -> String {
        self.asset_id.clone()
    }

    pub fn get_title(&self) -> String {
        self.title.clone()
    }

    pub fn get_version(&self) -> String {
        self.version.clone()
    }

    pub fn get_license(&self) -> String {
        self.license.clone()
    }
}