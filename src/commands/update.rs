use clap::{ArgMatches, Command};
use crate::plugin_service;

pub const COMMAND_NAME: &str = "update";

pub fn configure() -> Command {
    Command::new(COMMAND_NAME)
        .about("Updates outdated plugins")
}

pub async fn handle(_matches: &ArgMatches) -> anyhow::Result<()> {
    let plugin_service = plugin_service::PluginService::default();
    plugin_service.update_plugins().await?;
    Ok(())
}
