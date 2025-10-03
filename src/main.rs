mod api;
mod commands;
mod plugin_config;
mod settings;
mod parser;
mod extract;
mod http_client;


use settings::Settings;
use clap::Command;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
        let settings = Settings::get_settings()?;    
        let mut command = Command::new(settings.application_name)
            .version("0.1.0")
            .about("A CLI tool to manage Godot dependencies");
        command = commands::configure(command);

        let matches = command.get_matches();
        commands::handle(&matches).await?;

        Ok(())
}
