use clap::{value_parser, ArgMatches, Arg, Command};

pub const COMMAND_NAME: &str = "remove";

pub fn configure() -> Command {
    Command::new(COMMAND_NAME).about("Remove a dependency").arg(
        Arg::new("name")
            .help("The name of the dependency to remove, e.g. gut")
            .required(true)
            .value_parser(value_parser!(String)),
    )
}

pub async fn handle(matches: &ArgMatches) -> anyhow::Result<()> {
    let name = matches.get_one::<String>("name").unwrap();
    println!("Removing a dependency: {}", name);

    Ok(())
}