#![forbid(unsafe_code)]

mod api;
mod app_config;
mod commands;
mod extract_service;
mod file_service;
mod godot_config_repository;
mod http_client;
mod plugin_config_repository;
mod plugin_service;
mod utils;

use crate::commands::Cli;
use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_max_level(cli.verbosity)
        .init();

    let result = commands::handle(&cli.command).await;

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
