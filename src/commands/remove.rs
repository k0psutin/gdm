use crate::services::{DefaultPluginService, PluginService};

use anyhow::Result;
use clap::Args;

#[derive(Args)]
#[command(
    about = "Remove a dependency by name. Use exact name of the addon folder, e.g. \"gdUnit4\""
)]
pub struct RemoveArgs {
    #[arg(help = "Name of the dependency to remove, e.g. \"gut\"")]
    name: String,
}

pub async fn handle(args: &RemoveArgs) -> Result<()> {
    let plugin_service = DefaultPluginService::default();
    plugin_service
        .remove_plugin_by_config_key(&args.name)
        .await?;
    Ok(())
}
