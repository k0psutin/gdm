use clap::{ArgMatches, Command};
use crate::plugin_service::PluginService;

pub const COMMAND_NAME: &str = "search";

pub fn configure() -> Command {
    Command::new(COMMAND_NAME)
        .about("Searches for a specific plugin")
        .arg(
            clap::Arg::new("name")
                .help("Name of the plugin to search for, e.g. \"Godot Unit Testing\"")
                .required(true)
                .value_parser(clap::value_parser!(String)),
        ).arg(
            clap::Arg::new("godot-version")
                .help("The Godot version to search for, e.g. 4.5")
                .required(false)
                .long("godot-version")
                .value_parser(clap::value_parser!(String)),
        )
}

pub async fn handle(_matches: &ArgMatches) -> anyhow::Result<()> {
    
    let name = _matches.get_one::<String>("name").unwrap();
    let godot_version = _matches.get_one::<String>("godot-version").map(|s| s.as_str()).unwrap_or("");

    let plugin_service = PluginService::default();
    plugin_service.search_assets_by_name_or_version(name, godot_version).await?;
    
    Ok(())
}
