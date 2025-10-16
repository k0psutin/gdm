use crate::plugin_service::{self, PluginServiceImpl};

use clap::Args;

#[derive(Args)]
#[command(about = "Update all outdated plugins")]
pub struct UpdateArgs {}

pub async fn handle() -> anyhow::Result<()> {
    let plugin_service = plugin_service::PluginService::default();
    plugin_service.update_plugins().await?;
    Ok(())
}
