use clap::{ArgMatches, Command};
use crate::plugin_config::read_config;
use crate::api;
use std::collections::HashMap;

pub const COMMAND_NAME: &str = "install";

pub fn configure() -> Command {
    Command::new(COMMAND_NAME).about("Installs all plugins listed in the dependency file")
}

pub async fn handle(_matches: &ArgMatches) -> anyhow::Result<()> {
    let result = read_config();
    
    match result {
        Ok(config) => {
            // Proceed with installation using the config
            for key in config.plugins.keys() {
                println!("Plugin: {} from {:?}", key, config.plugins[key]);
                let title = &config.plugins[key].title;
                println!("Searching for asset with title: {}", title);
                let params = HashMap::from([("filter", title.as_str())]);
                let api = api::AssetStoreAPI;
                let result = api.search_assets(params).await?;
                result.print_info();
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Error reading config: {}", e);
            Err(e)
        }
    }
}