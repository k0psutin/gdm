mod asset_store;
mod extract;
mod file;
mod git;
mod http;
mod install;
mod plugin;
mod plugin_parser;

#[cfg(test)]
pub use asset_store::MockDefaultAssetStoreService;
pub use asset_store::{AssetStoreService, DefaultAssetStoreService};
pub use extract::{DefaultExtractService, ExtractService};
pub use file::{DefaultFileService, FileService};
pub use git::{DefaultGitService, GitService};
pub use http::{DefaultHttpService, HttpService};
pub use install::{DefaultInstallService, InstallService};
pub use plugin::{DefaultPluginService, PluginService};
pub use plugin_parser::PluginParser;

#[cfg(test)]
pub use extract::MockDefaultExtractService;
#[cfg(test)]
pub use file::MockDefaultFileService;
#[cfg(test)]
pub use install::MockDefaultInstallService;
