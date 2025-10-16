use crate::plugin_service::{DefaultPluginService, PluginService};

use anyhow::Result;
use clap::Args;

#[derive(Args)]
#[command(about = "Show outdated plugins")]
pub struct OutdatedArgs {}

pub async fn handle() -> Result<()> {
    let plugin_service = DefaultPluginService::default();
    plugin_service.check_outdated_plugins().await?;
    Ok(())
}
