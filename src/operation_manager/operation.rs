use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

/// Enum representing different plugin service operations with their progress bar styles
/// and messages. These enums are also used to customize function behaviors based on operation type.
#[derive(Debug, Clone)]
pub enum Operation {
    Install,
    Finished,
}

impl Operation {
    pub fn progress_bar_style(&self) -> Result<ProgressStyle> {
        let template = match self {
            Operation::Install => {
                "{spinner:.green} {prefix} {msg} [{elapsed_precise}] {bytes} ({bytes_per_sec}) [{eta}]"
            }
            Operation::Finished => "{prefix} {msg}",
        };

        ProgressStyle::with_template(template)
            .context("Failed to create progress bar style")
            .map(|style| style.progress_chars(self.progress_chars()))
    }

    pub fn action_verb(&self) -> &'static str {
        match self {
            Operation::Install => "Downloading",
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
        title: &str,
        version: &str,
    ) -> Result<ProgressBar> {
        let pb = m.add(ProgressBar::new(self.default_progress_bar_length()));
        pb.set_style(self.progress_bar_style()?);
        pb.set_prefix(format!("[{}/{}]", index + 1, total));
        pb.set_message(format!("{}: {} ({})", self.action_verb(), title, version));
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
    fn test_progress_bar_style_finished() {
        let operation = Operation::Finished;
        let style = operation.progress_bar_style();
        assert!(style.is_ok());
    }

    #[test]
    fn test_action_verb_install() {
        let operation = Operation::Install;
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
    fn test_progress_chars() {
        let operation = Operation::Install;
        assert_eq!(operation.progress_chars(), "#>-");
    }

    #[test]
    fn test_create_progress_bar_install() {
        let operation = Operation::Install;
        let m = MultiProgress::new();
        let result = operation.create_progress_bar(&m, 1, 5, "Test Plugin", "1.0.0");
        assert!(result.is_ok());
        let pb = result.unwrap();
        assert_eq!(pb.length().unwrap(), 500);
    }

    #[test]
    fn test_create_progress_bar_finished() {
        let operation = Operation::Finished;
        let m = MultiProgress::new();
        let result = operation.create_progress_bar(&m, 1, 1, "Finished Plugin", "3.0.0");
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
}
