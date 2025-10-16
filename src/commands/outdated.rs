use crate::plugin_service::{self, PluginServiceImpl};

use clap::Args;

#[derive(Args)]
#[command(about = "Show outdated plugins")]
pub struct OutdatedArgs {}

pub async fn handle() -> anyhow::Result<()> {
    let plugin_service = plugin_service::PluginService::default();
    plugin_service.check_outdated_plugins().await?;
    Ok(())
}
