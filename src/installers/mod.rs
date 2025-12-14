pub mod asset_lib;
pub mod git;

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub use asset_lib::AssetLibraryInstaller;
pub use git::GitInstaller;

use crate::{
    models::{Plugin, PluginSource},
    services::InstallService,
    ui::OperationManager,
};

#[async_trait]
pub trait PluginInstaller: Send + Sync {
    fn can_handle(&self, source: Option<PluginSource>) -> bool;

    async fn install(
        &self,
        index: usize,
        total: usize,
        install_service: &dyn InstallService,
        plugin: &Plugin,
        operation_manager: Arc<OperationManager>,
    ) -> Result<(String, Plugin)>;
}
