use clap::{ArgMatches, Command};
use crate::plugin_service::PluginService;

pub const COMMAND_NAME: &str = "install";

pub fn configure() -> Command {
    Command::new(COMMAND_NAME).about("Installs all plugins listed in the dependency file")
}

pub async fn handle(_matches: &ArgMatches) -> anyhow::Result<()> {
    let plugin_service = PluginService::new();
    plugin_service.install_plugins().await?;    
    Ok(())
}
