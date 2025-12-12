mod extract;
mod file;
mod git;
mod http;
mod plugin;
mod plugin_parser;
mod staging;

pub use extract::{DefaultExtractService, ExtractService};
pub use file::{DefaultFileService, FileService};
pub use git::{DefaultGitService, GitService};
pub use http::{DefaultHttpService, HttpService};
pub use plugin::{DefaultPluginService, PluginService};
pub use plugin_parser::PluginParser;
pub use staging::StagingService;

#[cfg(test)]
pub use extract::MockDefaultExtractService;
#[cfg(test)]
pub use file::MockDefaultFileService;
#[cfg(test)]
pub use git::MockDefaultGitService;
#[cfg(test)]
pub use http::MockDefaultHttpService;
