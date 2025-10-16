use crate::plugin_service::{PluginService, PluginServiceImpl};

use clap::Args;
use tracing::debug;

#[derive(Args, Debug)]
#[command(about = "Add a plugin to the project. You can specify the plugin by name or asset ID, and optionally provide a version.")]
pub struct AddArgs {
    #[arg(help = "Name of the plugin, e.g. \"Godot Unit Testing\"")]
    name: Option<String>,
    #[arg(long, help = "Asset ID of the plugin, e.g. \"67845\"")]
    asset_id: Option<String>,
    #[arg(long, help = "Version of the plugin, e.g. \"1.0.0\"")]
    version: Option<String>,
}

pub async fn handle(args: &AddArgs) -> anyhow::Result<()> {
    debug!("Adding plugin with args: {:?}", args);
    let plugin_service = PluginService::default();
    plugin_service.add_plugin_by_id_or_name_and_version(args.asset_id.clone(), args.name.clone(), args.version.clone()).await?;
    Ok(())
}
