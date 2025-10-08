use crate::plugin_config::PluginConfig;
use clap::{ArgMatches, Command};

pub const COMMAND_NAME: &str = "install";

pub fn configure() -> Command {
    Command::new(COMMAND_NAME).about("Installs all plugins listed in the dependency file")
}

pub async fn handle(_matches: &ArgMatches) -> anyhow::Result<()> {
    let plugin_config = PluginConfig::new();

    Ok(())
}
