pub mod asset_store;
pub mod git;

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub use asset_store::AssetStoreInstaller;
pub use git::GitInstaller;

use crate::{
    models::{Plugin, PluginSource},
    services::InstallService,
    ui::OperationManager,
};

#[async_trait]
pub trait PluginInstaller: Send + Sync {
    fn can_handle(&self, source: &PluginSource) -> bool;

    async fn install(
        &self,
        index: usize,
        total: usize,
        install_service: &dyn InstallService,
        plugin: &Plugin,
        operation_manager: Arc<OperationManager>,
    ) -> Result<(String, Plugin)>;
}
