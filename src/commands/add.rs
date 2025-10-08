use clap::{ArgMatches, Command, value_parser};
use crate::plugin_service::PluginService;

pub const COMMAND_NAME: &str = "add";

pub fn configure() -> Command {
    Command::new(COMMAND_NAME)
        .about("Add a new dependency")
        .arg(
            clap::Arg::new("name")
                .help("The name of the dependency to add, e.g. gut")
                .required(false)
                .value_parser(value_parser!(String)),
        )
        .arg(
            clap::Arg::new("asset-id")
                .help("The asset ID of the plugin to add, e.g. 12345")
                .required(false)
                .long("asset-id")
                .value_parser(value_parser!(String)),
        )
}

pub async fn handle(matches: &ArgMatches) -> anyhow::Result<()> {
    let name = matches.get_one::<String>("name");
    let asset_id = matches.get_one::<String>("asset-id");

    let plugin_service = PluginService::new();
    plugin_service.add_plugin_by_id_or_name(asset_id, name).await?;
    Ok(())
}
