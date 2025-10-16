mod add;
mod install;
mod outdated;
mod remove;
mod search;
mod update;

use anyhow::Result;

use clap::{Parser, Subcommand};

use crate::commands::{
    add::AddArgs, install::InstallArgs, outdated::OutdatedArgs, remove::RemoveArgs,
    search::SearchArgs, update::UpdateArgs,
};

#[derive(Parser)]
#[command(about, version, author, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
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
