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
use dotenv::dotenv;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let result = commands::handle(&cli.command).await;

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
