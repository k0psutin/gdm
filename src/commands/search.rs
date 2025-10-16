use crate::plugin_service::{DefaultPluginService, PluginService};

use anyhow::Result;
use clap::Args;

#[derive(Args)]
#[command(
    about = "Search for plugins by name. If godot version can't be determined from the project, it can be provided with --godot-version"
)]
pub struct SearchArgs {
    #[arg(help = "Name or part of the name of the plugin, e.g. \"Godot Unit Testing\"")]
    name: String,
    #[arg(
        long,
        help = "Specify the Godot version if it can't be determined from the project, e.g. --godot-version 4.5"
    )]
    godot_version: Option<String>,
}

pub async fn handle(args: &SearchArgs) -> Result<()> {
    let plugin_service = DefaultPluginService::default();
    plugin_service
        .search_assets_by_name_or_version(
            args.name.clone(),
            args.godot_version.clone().unwrap_or_default(),
        )
        .await?;

    Ok(())
}
