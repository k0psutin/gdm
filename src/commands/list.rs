use crate::config::{DefaultGdmConfig, GdmConfig};
use crate::models::{Plugin, PluginSource};

use anyhow::Result;
use clap::Args;
use std::collections::BTreeMap;

const HEADERS: [&str; 3] = ["Dependency", "Version", "Source"];

#[derive(Args)]
#[command(about = "List dependencies declared in gdm.toml.")]
pub struct ListArgs {}

#[derive(Debug, PartialEq, Eq)]
struct DependencyRow {
    dependency: String,
    version: String,
    source: String,
}

pub fn handle() -> Result<()> {
    let config = DefaultGdmConfig::default();
    let dependencies = config.get_dependencies()?;
    print!("{}", render_dependencies(&dependencies));
    Ok(())
}

fn render_dependencies(plugins: &BTreeMap<String, Plugin>) -> String {
    if plugins.is_empty() {
        return "No dependencies found.\n".to_string();
    }

    let rows = plugins
        .iter()
        .map(|(key, plugin)| DependencyRow {
            dependency: key.clone(),
            version: dependency_version(plugin),
            source: dependency_source(&plugin.source),
        })
        .collect::<Vec<_>>();

    let widths = [
        std::iter::once(HEADERS[0])
            .chain(rows.iter().map(|row| row.dependency.as_str()))
            .map(str::len)
            .max()
            .unwrap_or(HEADERS[0].len()),
        std::iter::once(HEADERS[1])
            .chain(rows.iter().map(|row| row.version.as_str()))
            .map(str::len)
            .max()
            .unwrap_or(HEADERS[1].len()),
        std::iter::once(HEADERS[2])
            .chain(rows.iter().map(|row| row.source.as_str()))
            .map(str::len)
            .max()
            .unwrap_or(HEADERS[2].len()),
    ];

    let mut output = String::new();
    output.push_str(&format_row(&HEADERS, widths));
    output.push('\n');
    for row in &rows {
        let values = [
            row.dependency.as_str(),
            row.version.as_str(),
            row.source.as_str(),
        ];
        output.push_str(&format_row(&values, widths));
        output.push('\n');
    }
    output.push('\n');
    output.push_str("To remove a dependency, use: gdm remove <dependency>\n");
    output
}

fn format_row(values: &[&str; 3], widths: [usize; 3]) -> String {
    format!(
        "{:<width0$}  {:<width1$}  {}",
        values[0],
        values[1],
        values[2],
        width0 = widths[0],
        width1 = widths[1],
    )
}

fn dependency_version(plugin: &Plugin) -> String {
    match &plugin.source {
        PluginSource::Git { reference, .. } => reference.clone(),
        PluginSource::AssetStore { .. } => plugin.version.clone(),
    }
}

fn dependency_source(source: &PluginSource) -> String {
    match source {
        PluginSource::AssetStore {
            publisher_slug,
            asset_slug,
        } => format!("{publisher_slug}/{asset_slug}"),
        PluginSource::Git { url, .. } => normalize_git_source(url),
    }
}

fn normalize_git_source(url: &str) -> String {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let without_slash = without_scheme.strip_suffix('/').unwrap_or(without_scheme);
    let normalized = without_slash.strip_suffix(".git").unwrap_or(without_slash);
    normalized.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn asset_plugin(title: &str, version: &str, source: PluginSource) -> Plugin {
        Plugin::new(
            source,
            Some(PathBuf::from("addons/plugin/plugin.cfg")),
            title.to_string(),
            version.to_string(),
            None,
            vec!["sub_asset".to_string()],
        )
    }

    #[test]
    fn test_normalize_git_source_strips_only_supported_decorations() {
        assert_eq!(
            normalize_git_source("https://github.com/foo/bar.git"),
            "github.com/foo/bar"
        );
        assert_eq!(
            normalize_git_source("http://gitlab.com/foo/bar/"),
            "gitlab.com/foo/bar"
        );
        assert_eq!(
            normalize_git_source("git@github.com:foo/bar.git"),
            "git@github.com:foo/bar"
        );
        assert_eq!(
            normalize_git_source("example.com/foo/bar"),
            "example.com/foo/bar"
        );
    }

    #[test]
    fn test_render_dependencies_uses_manifest_key_and_git_reference_as_version() {
        let plugins = BTreeMap::from([
            (
                "custom-plugin".to_string(),
                asset_plugin(
                    "",
                    "",
                    PluginSource::Git {
                        url: "https://github.com/foo/bar.git".to_string(),
                        reference: "main".to_string(),
                        publisher_slug: "foo".to_string(),
                        asset_slug: "bar".to_string(),
                    },
                ),
            ),
            (
                "netfox".to_string(),
                asset_plugin(
                    "netfox",
                    "v1.35.3",
                    PluginSource::AssetStore {
                        publisher_slug: "foxssake".to_string(),
                        asset_slug: "netfox".to_string(),
                    },
                ),
            ),
        ]);

        let output = render_dependencies(&plugins);

        assert!(output.contains("custom-plugin  main     github.com/foo/bar"));
        assert!(output.lines().any(|line| line.contains("netfox")
            && line.contains("v1.35.3")
            && line.contains("foxssake/netfox")));
        assert!(!output.contains("custom-plugin  custom-plugin"));
        assert!(!output.contains("sub_asset"));
        assert!(output.contains("To remove a dependency, use: gdm remove <dependency>"));
    }

    #[test]
    fn test_render_dependencies_aligns_columns_for_different_lengths() {
        let plugins = BTreeMap::from([
            (
                "short".to_string(),
                asset_plugin(
                    "Ignored dependency title",
                    "1",
                    PluginSource::AssetStore {
                        publisher_slug: "publisher".to_string(),
                        asset_slug: "asset".to_string(),
                    },
                ),
            ),
            (
                "a-much-longer-key".to_string(),
                asset_plugin(
                    "Short",
                    "1.234.567",
                    PluginSource::AssetStore {
                        publisher_slug: "p".to_string(),
                        asset_slug: "a".to_string(),
                    },
                ),
            ),
        ]);

        let output = render_dependencies(&plugins);
        let lines = output.lines().collect::<Vec<_>>();

        let source_column = lines[0].find("Source").unwrap();
        let long_source_line = lines
            .iter()
            .find(|line| line.contains("publisher/asset"))
            .unwrap();
        let short_source_line = lines.iter().find(|line| line.ends_with("p/a")).unwrap();
        assert_eq!(
            long_source_line.find("publisher/asset").unwrap(),
            source_column
        );
        assert_eq!(short_source_line.find("p/a").unwrap(), source_column);
    }

    #[test]
    fn test_render_dependencies_empty_config() {
        assert_eq!(
            render_dependencies(&BTreeMap::new()),
            "No dependencies found.\n"
        );
    }
}
