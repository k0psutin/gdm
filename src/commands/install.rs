use crate::services::{DefaultPluginService, PluginService};

use anyhow::Result;
use clap::Args;

#[derive(Args)]
#[command(about = "Install all plugins with versions listed in the configuration file.")]
pub struct InstallArgs {}

pub async fn handle() -> Result<()> {
    let plugin_service = DefaultPluginService::default();
    plugin_service.install_all_plugins().await?;
    Ok(())
}
