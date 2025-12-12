mod add;
mod install;
mod outdated;
mod remove;
mod search;
mod update;

use anyhow::Result;

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{OffLevel, Verbosity};

use crate::{
    commands::{
        add::AddArgs, install::InstallArgs, outdated::OutdatedArgs, remove::RemoveArgs,
        search::SearchArgs, update::UpdateArgs,
    },
    config::{DefaultGodotConfig, GodotConfig},
};

#[derive(Parser)]
#[command(about, version, author, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[command(flatten)]
    pub verbosity: Verbosity<OffLevel>,
}

#[derive(Subcommand)]
pub enum Commands {
    Add(AddArgs),
    Install(InstallArgs),
    Outdated(OutdatedArgs),
    Remove(RemoveArgs),
    Search(SearchArgs),
    Update(UpdateArgs),
}

pub async fn handle(command: &Commands) -> Result<()> {
    DefaultGodotConfig::default().validate_project_file()?;

    match command {
        Commands::Add(add_args) => {
            add::handle(add_args).await?;
        }
        Commands::Install(_) => {
            install::handle().await?;
        }
        Commands::Outdated(_) => {
            outdated::handle().await?;
        }
        Commands::Remove(remove_args) => {
            remove::handle(remove_args).await?;
        }
        Commands::Search(search_args) => {
            search::handle(search_args).await?;
        }
        Commands::Update(_) => {
            update::handle().await?;
        }
    }

    Ok(())
}
