mod api;
mod commands;
mod plugin_config;
mod app_config;
mod godot_config;
mod extract;
mod http_client;
mod plugin_service;
mod file_service;
mod utils;

use clap::Command;
use crate::app_config::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
        let application_name = AppConfig::new().get_application_name();
        let mut command = Command::new(application_name)
            .version("0.1.0")
            .about("A CLI tool to manage Godot dependencies");
        command = commands::configure(command);

        let matches = command.get_matches();
        let result = commands::handle(&matches).await;

        return match result {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("{}", e);
                Ok(())
            }
        };
}
