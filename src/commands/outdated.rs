use clap::{ArgMatches, Command};
use crate::plugin_service;

pub const COMMAND_NAME: &str = "outdated";

pub fn configure() -> Command {
    Command::new(COMMAND_NAME)
        .about("Checks for outdated plugins")
}

pub async fn handle(_matches: &ArgMatches) -> anyhow::Result<()> {
    let plugin_service = plugin_service::PluginService::default();
    plugin_service.check_outdated_plugins().await?;
    Ok(())
}
