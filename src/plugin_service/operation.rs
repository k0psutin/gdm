use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::plugin_config_repository::plugin::Plugin;

/// Enum representing different plugin service operations with their progress bar styles
/// and messages. These enums are also used to customize function behaviors based on operation type.
#[derive(Debug, Clone)]
pub enum Operation {
    Extract,
    Install,
    Finished,
    Update,
}

impl Operation {
    pub fn progress_bar_style(&self) -> Result<ProgressStyle> {
        let template = match self {
            Operation::Install | Operation::Update => {
                "{spinner:.green} {prefix} {msg} [{elapsed_precise}] {bytes} ({bytes_per_sec}) [{eta}]"
            }
            Operation::Extract => {
                "{spinner:.green} {prefix} {msg} [{elapsed_precise}] [{bar:.cyan/blue}] {pos:>7}/{len:7} ({eta})"
            }
            Operation::Finished => "{prefix} {msg}",
        };

        ProgressStyle::with_template(template)
            .context("Failed to create progress bar style")
            .map(|style| style.progress_chars(self.progress_chars()))
    }

    pub fn action_verb(&self) -> &'static str {
        match self {
            Operation::Extract => "Extracting",
            Operation::Install | Operation::Update => "Downloading",
            Operation::Finished => "Installed",
        }
    }

    pub fn default_progress_bar_length(&self) -> u64 {
        match self {
            Operation::Finished => 1,
            _ => 500,
        }
    }

    pub fn progress_chars(&self) -> &'static str {
        "#>-"
    }

    pub fn create_progress_bar(
        &self,
        m: &MultiProgress,
        index: usize,
        total: usize,
        plugin: &Plugin,
    ) -> Result<ProgressBar> {
        let pb = m.add(ProgressBar::new(self.default_progress_bar_length()));
        pb.set_style(self.progress_bar_style()?);
        pb.set_prefix(format!("[{}/{}]", index, total));
        pb.set_message(format!(
            "{}: {} ({})",
            self.action_verb(),
            plugin.title,
            plugin.get_version()
        ));
        Ok(pb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_style_install() {
        let operation = Operation::Install;
        let style = operation.progress_bar_style();
        assert!(style.is_ok());
    }

    #[test]
    fn test_progress_bar_style_update() {
        let operation = Operation::Update;
        let style = operation.progress_bar_style();
        assert!(style.is_ok());
    }

    #[test]
    fn test_progress_bar_style_extract() {
        let operation = Operation::Extract;
        let style = operation.progress_bar_style();
        assert!(style.is_ok());
    }

    #[test]
    fn test_progress_bar_style_finished() {
        let operation = Operation::Finished;
        let style = operation.progress_bar_style();
        assert!(style.is_ok());
    }

    #[test]
    fn test_action_verb_extract() {
        let operation = Operation::Extract;
        assert_eq!(operation.action_verb(), "Extracting");
    }

    #[test]
    fn test_action_verb_install() {
        let operation = Operation::Install;
        assert_eq!(operation.action_verb(), "Downloading");
    }

    #[test]
    fn test_action_verb_update() {
        let operation = Operation::Update;
        assert_eq!(operation.action_verb(), "Downloading");
    }

    #[test]
    fn test_action_verb_finished() {
        let operation = Operation::Finished;
        assert_eq!(operation.action_verb(), "Installed");
    }

    #[test]
    fn test_default_progress_bar_length_finished() {
        let operation = Operation::Finished;
        assert_eq!(operation.default_progress_bar_length(), 1);
    }

    #[test]
    fn test_default_progress_bar_length_install() {
        let operation = Operation::Install;
        assert_eq!(operation.default_progress_bar_length(), 500);
    }

    #[test]
    fn test_default_progress_bar_length_update() {
        let operation = Operation::Update;
        assert_eq!(operation.default_progress_bar_length(), 500);
    }

    #[test]
    fn test_default_progress_bar_length_extract() {
        let operation = Operation::Extract;
        assert_eq!(operation.default_progress_bar_length(), 500);
    }

    #[test]
    fn test_progress_chars() {
        let operation = Operation::Install;
        assert_eq!(operation.progress_chars(), "#>-");
    }

    #[test]
    fn test_create_progress_bar_install() {
        let operation = Operation::Install;
        let m = MultiProgress::new();
        let plugin = Plugin::new(
            "test-plugin".to_string(),
            "Test Plugin".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
        );
        let result = operation.create_progress_bar(&m, 1, 5, &plugin);
        assert!(result.is_ok());
        let pb = result.unwrap();
        assert_eq!(pb.length().unwrap(), 500);
    }

    #[test]
    fn test_create_progress_bar_update() {
        let operation = Operation::Update;
        let m = MultiProgress::new();
        let plugin = Plugin::new(
            "test-plugin".to_string(),
            "Test Plugin".to_string(),
            "2.0.0".to_string(),
            "MIT".to_string(),
        );
        let result = operation.create_progress_bar(&m, 2, 10, &plugin);
        assert!(result.is_ok());
        let pb = result.unwrap();
        assert_eq!(pb.length().unwrap(), 500);
    }

    #[test]
    fn test_create_progress_bar_extract() {
        let operation = Operation::Extract;
        let m = MultiProgress::new();
        let plugin = Plugin::new(
            "extract-plugin".to_string(),
            "Extract Plugin".to_string(),
            "1.5.0".to_string(),
            "Apache-2.0".to_string(),
        );
        let result = operation.create_progress_bar(&m, 3, 7, &plugin);
        assert!(result.is_ok());
        let pb = result.unwrap();
        assert_eq!(pb.length().unwrap(), 500);
    }

    #[test]
    fn test_create_progress_bar_finished() {
        let operation = Operation::Finished;
        let m = MultiProgress::new();
        let plugin = Plugin::new(
            "finished-plugin".to_string(),
            "Finished Plugin".to_string(),
            "3.0.0".to_string(),
            "BSD-3-Clause".to_string(),
        );
        let result = operation.create_progress_bar(&m, 1, 1, &plugin);
        assert!(result.is_ok());
        let pb = result.unwrap();
        assert_eq!(pb.length().unwrap(), 1);
    }

    #[test]
    fn test_operation_clone() {
        let operation = Operation::Install;
        let cloned = operation.clone();
        assert_eq!(operation.action_verb(), cloned.action_verb());
    }

    #[test]
    fn test_operation_debug() {
        let operation = Operation::Extract;
        let debug_str = format!("{:?}", operation);
        assert_eq!(debug_str, "Extract");
    }
}
