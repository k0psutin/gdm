use clap::{value_parser, ArgMatches, Arg, Command};
use crate::plugin_service::PluginService;

pub const COMMAND_NAME: &str = "remove";

pub fn configure() -> Command {
    Command::new(COMMAND_NAME).about("Remove a dependency").arg(
        Arg::new("name")
            .help("The name of the dependency to remove, e.g. gut. Matches the name in the dependency file.")
            .required(true)
            .value_parser(value_parser!(String)),
    )
}

pub async fn handle(matches: &ArgMatches) -> anyhow::Result<()> {
    let name = matches.get_one::<String>("name").unwrap();
    let plugin_service = PluginService::new();
    plugin_service.remove_plugin_by_name(name).await?;
    Ok(())
}