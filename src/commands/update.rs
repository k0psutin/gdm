use crate::services::{DefaultPluginService, PluginService};

use anyhow::Result;
use clap::Args;

#[derive(Args)]
#[command(about = "Update all outdated dependencies")]
pub struct UpdateArgs {}

pub async fn handle() -> Result<()> {
    let plugin_service = DefaultPluginService::default();
    plugin_service.update_plugins().await?;
    Ok(())
}
