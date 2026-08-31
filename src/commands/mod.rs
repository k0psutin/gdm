mod add;
mod install;
mod list;
mod outdated;
mod remove;
mod search;
mod update;

use anyhow::Result;

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{OffLevel, Verbosity};

use crate::{
    commands::{
        add::AddArgs, install::InstallArgs, list::ListArgs, outdated::OutdatedArgs,
        remove::RemoveArgs, search::SearchArgs, update::UpdateArgs,
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
    List(ListArgs),
    Outdated(OutdatedArgs),
    Remove(RemoveArgs),
    Search(SearchArgs),
    Update(UpdateArgs),
}

pub async fn handle(command: &Commands) -> Result<()> {
    if !matches!(command, Commands::List(_)) {
        DefaultGodotConfig::default().validate_project_file()?;
    }

    match command {
        Commands::Add(add_args) => {
            add::handle(add_args).await?;
        }
        Commands::Install(_) => {
            install::handle().await?;
        }
        Commands::List(_) => {
            list::handle()?;
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
