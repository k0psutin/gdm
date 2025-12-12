use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

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

pub struct OperationManager {
    multi_progress: MultiProgress,
    main_progress: ProgressBar,
    operation: Operation,
}

impl OperationManager {
    pub fn new(operation: Operation) -> Result<Self> {
        let multi_progress = MultiProgress::new();
        let main_progress = multi_progress.add(ProgressBar::no_length());

        main_progress.set_style(
            ProgressStyle::with_template("{msg}")
                .map_err(|e| anyhow::anyhow!("Failed to create main progress style: {}", e))?,
        );
        main_progress.set_message(Self::get_main_message_by_operation(&operation));

        Ok(Self {
            multi_progress,
            main_progress,
            operation,
        })
    }

    fn get_main_message_by_operation(operation: &Operation) -> String {
        match operation {
            Operation::Install => "Installing plugins".to_string(),
            Operation::Finished => "Installation complete".to_string(),
        }
    }

    pub fn finish(&self) {
        match self.operation {
            Operation::Finished => self.main_progress.finish(),
            _ => self.main_progress.finish_and_clear(),
        }
    }

    pub fn add_progress_bar(
        &self,
        index: usize,
        total: usize,
        title: &str,
        version: &str,
    ) -> Result<ProgressBar> {
        self.operation
            .create_progress_bar(&self.multi_progress, index, total, title, version)
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

    #[test]
    fn test_new_operation_manager_install() {
        let result = OperationManager::new(Operation::Install);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_operation_manager_finished() {
        let result = OperationManager::new(Operation::Finished);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_main_message_by_operation_install() {
        let message = OperationManager::get_main_message_by_operation(&Operation::Install);
        assert_eq!(message, "Installing plugins");
    }

    #[test]
    fn test_get_main_message_by_operation_finished() {
        let message = OperationManager::get_main_message_by_operation(&Operation::Finished);
        assert_eq!(message, "Installation complete");
    }

    #[test]
    fn test_add_progress_bar_install() {
        let manager = OperationManager::new(Operation::Install).unwrap();
        let result = manager.add_progress_bar(1, 5, "Test Plugin", "1.0.0");
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_progress_bar_finished() {
        let manager = OperationManager::new(Operation::Finished).unwrap();
        let result = manager.add_progress_bar(1, 1, "Finished Plugin", "3.0.0");
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_multiple_progress_bars() {
        let manager = OperationManager::new(Operation::Install).unwrap();
        let result1 = manager.add_progress_bar(1, 2, "Plugin 1", "1.0.0");
        let result2 = manager.add_progress_bar(2, 2, "Plugin 2", "2.0.0");
        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_finish() {
        let manager = OperationManager::new(Operation::Install).unwrap();
        manager.finish();
    }

    #[test]
    fn test_operation_manager_workflow() {
        let manager = OperationManager::new(Operation::Install).unwrap();
        let pb = manager
            .add_progress_bar(1, 1, "Workflow Plugin", "1.0.0")
            .unwrap();
        pb.finish();
        manager.finish();
    }
}
