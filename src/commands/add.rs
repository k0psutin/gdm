use crate::services::{DefaultPluginService, PluginService};

use anyhow::Result;
use clap::Args;

#[derive(Args, Debug)]
#[command(
    about = "Add a plugin to the project. You can specify the plugin by name or asset ID, and optionally provide a version."
)]
pub struct AddArgs {
    #[arg(help = "Name of the plugin, e.g. \"Godot Unit Testing\"")]
    name: Option<String>,
    #[arg(long, help = "Asset ID of the plugin, e.g. \"67845\"")]
    asset_id: Option<String>,
    #[arg(long, help = "Version of the plugin, e.g. \"1.0.0\"")]
    version: Option<String>,
    #[arg(
        long,
        help = "Git URL of the plugin, e.g. \"https://github.com/user/repo.git\""
    )]
    git: Option<String>,
    #[arg(long = "ref", help = "Git reference of the plugin, e.g. \"main\"")]
    reference: Option<String>,
}

pub async fn handle(args: &AddArgs) -> Result<()> {
    let plugin_service = DefaultPluginService::default();
    plugin_service
        .add_plugin(
            args.asset_id.clone(),
            args.name.clone(),
            args.version.clone(),
            args.git.clone(),
            args.reference.clone(),
        )
        .await?;
    Ok(())
}
