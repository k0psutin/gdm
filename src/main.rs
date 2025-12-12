#![forbid(unsafe_code)]

mod api;
mod commands;
mod config;
mod installers;
mod models;
mod services;
mod ui;
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
