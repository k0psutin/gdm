use crate::plugin_service::{PluginService, PluginServiceImpl};

use clap::Args;

#[derive(Args)]
#[command(about = "Install all plugins with versions listed in the configuration file.")]
pub struct InstallArgs {}

pub async fn handle() -> anyhow::Result<()> {
    let plugin_service = PluginService::default();
    plugin_service.install_all_plugins().await?;    
    Ok(())
}
