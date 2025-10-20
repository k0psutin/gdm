use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

/// Enum representing different plugin service operations with their progress bar styles
/// and messages. These enums are also used to customize function behaviors based on operation type.
#[derive(Debug, Clone)]
pub enum Operation {
    Download,
    Extract,
    Install,
    Finished,
    Update,
}

impl Operation {
    /// Returns the progress bar style for this operation
    pub fn progress_bar_style(&self) -> Result<ProgressStyle> {
        let template = match self {
            Operation::Download => {
                "{spinner:.green} {prefix} {msg} [{elapsed_precise}] {bytes} ({bytes_per_sec})"
            }
            Operation::Extract => {
                "{spinner:.green} {prefix} {msg} [{elapsed_precise}] [{bar:.cyan/blue}] {pos:>7}/{len:7} ({eta})"
            }
            Operation::Install => {
                "{spinner:.green} {prefix} {msg} [{elapsed_precise}] {bytes} ({bytes_per_sec})"
            }
            Operation::Finished => "{prefix} {msg}",
            Operation::Update => {
                "{spinner:.green} {prefix} {msg} [{elapsed_precise}] {bytes} ({bytes_per_sec})"
            }
        };

        ProgressStyle::with_template(template)
            .context("Failed to create progress bar style")
            .map(|style| style.progress_chars(self.progress_chars()))
    }

    /// Returns the action verb for this operation (e.g., "Downloading", "Installing")
    pub fn action_verb(&self) -> &'static str {
        match self {
            Operation::Download => "Downloading",
            Operation::Extract => "Extracting",
            Operation::Install => "Installing",
            Operation::Finished => "Installed",
            Operation::Update => "Updating",
        }
    }

    /// Returns the main message for this operation (e.g., "Downloading plugins")
    pub fn main_message(&self) -> String {
        match self {
            Operation::Download => "Downloading plugins".to_string(),
            Operation::Extract => "Extracting plugins".to_string(),
            Operation::Install => "Installing plugins".to_string(),
            Operation::Update => "Updating plugins".to_string(),
            Operation::Finished => "Installation complete".to_string(),
        }
    }

    /// Returns the default progress bar length for this operation
    pub fn default_progress_bar_length(&self) -> u64 {
        match self {
            Operation::Download => 5_000_000,
            Operation::Extract => 5_000_000,
            Operation::Install => 5_000_000,
            Operation::Update => 5_000_000,
            Operation::Finished => 1,
        }
    }

    /// Returns the progress chars pattern for the progress bar
    pub fn progress_chars(&self) -> &'static str {
        "#>-"
    }

    /// Creates and configures a progress bar for this operation
    pub fn create_progress_bar(
        &self,
        m: &MultiProgress,
        index: usize,
        total: usize,
        title: &str,
        version: &str,
    ) -> Result<ProgressBar> {
        let pb = m.add(ProgressBar::new(self.default_progress_bar_length()));
        pb.set_style(self.progress_bar_style()?);
        pb.set_prefix(format!("[{}/{}]", index, total));
        pb.set_message(format!("{}: {} ({})", self.action_verb(), title, version));
        Ok(pb)
    }
}
