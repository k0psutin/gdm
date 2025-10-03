use crate::api::AssetStoreAPI;
use clap::{ArgMatches, Command, value_parser};
use crate::http_client;
use crate::extract;

pub const COMMAND_NAME: &str = "add";

pub fn configure() -> Command {
    Command::new(COMMAND_NAME)
        .about("Add a new dependency")
        .arg(
            clap::Arg::new("name")
                .help("The name of the dependency to add, e.g. gut")
                .required(false)
                .value_parser(value_parser!(String)),
        )
        .arg(
            clap::Arg::new("asset-id")
                .help("The asset ID of the plugin to add, e.g. 12345")
                .required(false)
                .long("asset-id")
                .value_parser(value_parser!(String)),
        )
}

pub async fn handle(matches: &ArgMatches) -> anyhow::Result<()> {
    let name = matches.get_one::<String>("name");
    let asset_id = matches.get_one::<String>("asset-id");

    if name.is_none() && asset_id.is_none() {
        println!("Please provide either a name or an asset ID to add a dependency.");
        return Ok(());
    }

     let mut id = String::new();

        if asset_id.is_some() {
            id = asset_id.unwrap().to_string();
        } else if name.is_some() {
         let name = name.unwrap();
          let params =
            std::collections::HashMap::from([("filter", name.as_str()), ("godot_version", "4.5")]);
        let asset_results = AssetStoreAPI.search_assets(params).await?;

        if asset_results.get_result_len() != 1 {
            println!(
                "Expected to find exactly one asset matching \"{}\", but found {}. Please refine your search or use --asset-id.",
                name,
                asset_results.get_result_len()
            );
            return Ok(());
        }
        let asset = asset_results.get_asset_list_item_by_index(0).unwrap();
        id = asset.get_asset_id().to_owned();
     }

    let asset = AssetStoreAPI.fetch_asset_by_id(id.as_str()).await?;
    let download_url = asset.get_download_url();
    let file = http_client::get_file(download_url.to_string()).await?;
    extract::extract_zip_file(file, "todo");
    Ok(())
}
