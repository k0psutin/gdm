use crate::models::{Plugin, PluginSource};
use crate::ui::OperationManager;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::Arc;

#[async_trait]
pub trait PluginInstaller: Send + Sync {
    fn can_handle(&self, source: &PluginSource) -> bool;

    async fn install(
        &self,
        plugins: Vec<Plugin>,
        operation_manager: Arc<OperationManager>,
        start_index: usize,
        total_count: usize,
    ) -> Result<BTreeMap<String, Plugin>>;
}
