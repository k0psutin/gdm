use crate::plugin_service::{PluginService, PluginServiceImpl};

use clap::Args;

#[derive(Args)]
#[command(about = "Remove a plugin by name. Use the exact name as listed in the configuration file, e.g. \"gut\"")]
pub struct RemoveArgs {
    #[arg(help = "Name of the plugin to remove, e.g. \"gut\"")]
    name: String,
}

pub async fn handle(args: &RemoveArgs) -> anyhow::Result<()> {
    let plugin_service = PluginService::default();
    plugin_service.remove_plugin_by_name(&args.name).await?;
    Ok(())
}