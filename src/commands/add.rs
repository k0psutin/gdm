use crate::services::{DefaultPluginService, PluginService};

use anyhow::{Result, bail};
use clap::Args;

#[derive(Args, Debug)]
#[command(
    about = "Add a dependency to the project. You can specify the dependency by name or asset slug, and optionally provide a version."
)]
pub struct AddArgs {
    #[arg(
        help = "Name or part of the dependency name, e.g. \"GDUnit4\" or publisher/asset slug, e.g. mikeschulze/gdunit4"
    )]
    name: Option<String>,
    #[arg(long, help = "Publisher slug of the dependency, e.g. \"mikeschulze\"")]
    publisher_slug: Option<String>,
    #[arg(long, help = "Asset slug of the dependency, e.g. \"gdunit4\"")]
    asset_slug: Option<String>,
    #[arg(long, help = "Version of the dependency, e.g. \"1.0.0\"")]
    version: Option<String>,
    #[arg(
        long,
        help = "Override the Godot version detected from project.godot (e.g., '4.2.1')"
    )]
    godot_version: Option<String>,
    #[arg(
        long,
        help = "Git URL of the dependency, e.g. \"https://github.com/user/repo.git\""
    )]
    git: Option<String>,
    #[arg(long = "ref", help = "Git reference of the dependency, e.g. \"main\"")]
    reference: Option<String>,
}

fn try_parse_plugin_name(name_opt: &Option<String>) -> (Option<String>, Option<String>) {
    if name_opt.is_none() {
        return (None, None);
    }

    let name = name_opt.as_ref().unwrap();

    if !name.contains("/") {
        return (None, None);
    }

    let split_name: Vec<&str> = name.split("/").collect();

    if split_name.len() != 2 {
        return (None, None);
    }

    let first = split_name.first().unwrap().to_string();
    let second = split_name.last().unwrap().to_string();

    (Some(first), Some(second))
}

pub async fn handle(args: &AddArgs) -> Result<()> {
    let plugin_service = DefaultPluginService::default();

    if args.git.is_some() {
        if args.publisher_slug.is_some() || args.asset_slug.is_some() || args.version.is_some() {
            bail!("--publisher-slug, --asset-slug, and --version are not used with --git");
        }
        return plugin_service
            .add_plugin_by_git_url_and_reference(args.git.as_deref(), args.reference.as_deref())
            .await;
    }

    let (parsed_publisher_slug, parsed_asset_slug) = try_parse_plugin_name(&args.name);

    let publisher_slug = if parsed_publisher_slug.is_some() {
        parsed_publisher_slug
    } else {
        args.publisher_slug.clone()
    };

    let asset_slug = if parsed_asset_slug.is_some() {
        parsed_asset_slug
    } else {
        args.asset_slug.clone()
    };

    if publisher_slug.is_some() && asset_slug.is_some() {
        return plugin_service
            .add_plugin_by_publisher_and_asset_slug_and_version_and_godot_version(
                publisher_slug.as_deref(),
                asset_slug.as_deref(),
                args.version.as_deref(),
                args.godot_version.as_deref(),
            )
            .await;
    }

    if let Some(name) = &args.name {
        if args.git.is_some() || args.publisher_slug.is_some() || args.asset_slug.is_some() {
            bail!(
                "Cannot use --git, --publisher-slug, or --asset-slug together with a name argument"
            );
        }
        return plugin_service
            .add_plugin_by_name_and_version_and_godot_version(
                name,
                args.version.as_deref(),
                args.godot_version.as_deref(),
            )
            .await;
    }

    bail!(
        "You must specify either a dependency --name, a --git URL, or both --publisher-slug and --asset-slug"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plugin_name_returns_publisher_before_asset() {
        assert_eq!(
            try_parse_plugin_name(&Some("publisher/asset".to_string())),
            (Some("publisher".to_string()), Some("asset".to_string()))
        );
    }
}
