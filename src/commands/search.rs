use crate::api::AssetStoreAPI;
use crate::godot_config::GodotConfig;
use clap::{ArgMatches, Command};
use std::collections::HashMap;

pub const COMMAND_NAME: &str = "search";

pub fn configure() -> Command {
    Command::new(COMMAND_NAME)
        .about("Searches for a specific plugin")
        .arg(
            clap::Arg::new("name")
                .help("Name of the plugin to search for, e.g. \"Godot Unit Testing\"")
                .required(true)
                .value_parser(clap::value_parser!(String)),
        ).arg(
            clap::Arg::new("godot-version")
                .help("The Godot version to search for, e.g. 4.5")
                .required(false)
                .long("godot-version")
                .value_parser(clap::value_parser!(String)),
        )
}

pub async fn handle(_matches: &ArgMatches) -> anyhow::Result<()> {
    
    let name = _matches.get_one::<String>("name").unwrap();
    let mut godot_version = _matches.get_one::<String>("godot-version").map(|s| s.as_str()).unwrap_or("");

    let godot_config = GodotConfig::new();
    let parsed_version = godot_config.get_godot_version()?;

    if godot_version.is_empty() && parsed_version.is_empty() {
        println!("Couldn't determine Godot version from project.godot. Please provide a version using --godot-version.");
        return Ok(());
    }

    if godot_version.is_empty() && !parsed_version.is_empty() {
        godot_version = parsed_version.as_str();
    }

    let params = HashMap::from([("filter", name.as_str()), ("godot_version", godot_version)]);
    let asset_results = AssetStoreAPI::new().search_assets(params).await?;
    match asset_results.get_result_len() {
            0 => println!("No assets found matching \"{}\"", name),
            1 => println!("Found 1 asset matching \"{}\":", name),
            n => println!("Found {} assets matching \"{}\":", n, name),
        }

    asset_results.print_info();

    if asset_results.get_result_len() == 1 {
        let asset = asset_results.get_asset_list_item_by_index(0).unwrap();
            println!("To install the plugin, use: gdm add \"{}\"", asset.get_title());
        } else {
            println!(
                "To install a plugin, use: gdm add --asset-id <asset_id> or narrow down your search"
            );
        }
    Ok(())
}
