mod add;
mod remove;
mod install;
mod search;
mod outdated;

use clap::{ArgMatches, Command};
use anyhow::{Result};

pub fn configure(command: Command) -> Command {
    command.subcommand(add::configure())
    .subcommand(remove::configure())
    .subcommand(install::configure())
    .subcommand(search::configure())
    .subcommand(outdated::configure())
    .arg_required_else_help(true)
}

pub async fn handle(matches: &ArgMatches) -> Result<()> {
    if let Some((cmd, _matches)) = matches.subcommand() {
        match cmd {
            add::COMMAND_NAME => { add::handle(_matches).await?; },
            remove::COMMAND_NAME => { remove::handle(_matches).await?; },
            install::COMMAND_NAME => { install::handle(_matches).await?; },
            search::COMMAND_NAME => { search::handle(_matches).await?; },
            outdated::COMMAND_NAME => { outdated::handle(_matches).await?; },
            &_ => {}
        }
    }

    Ok(())
}