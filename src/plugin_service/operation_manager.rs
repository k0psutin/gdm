use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use super::operation::Operation;

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
            Operation::Extract => "Extracting plugins".to_string(),
            Operation::Install => "Installing plugins".to_string(),
            Operation::Update => "Updating plugins".to_string(),
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
    fn test_new_operation_manager_install() {
        let result = OperationManager::new(Operation::Install);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_operation_manager_update() {
        let result = OperationManager::new(Operation::Update);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_operation_manager_extract() {
        let result = OperationManager::new(Operation::Extract);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_operation_manager_finished() {
        let result = OperationManager::new(Operation::Finished);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_main_message_by_operation_extract() {
        let message = OperationManager::get_main_message_by_operation(&Operation::Extract);
        assert_eq!(message, "Extracting plugins");
    }

    #[test]
    fn test_get_main_message_by_operation_install() {
        let message = OperationManager::get_main_message_by_operation(&Operation::Install);
        assert_eq!(message, "Installing plugins");
    }

    #[test]
    fn test_get_main_message_by_operation_update() {
        let message = OperationManager::get_main_message_by_operation(&Operation::Update);
        assert_eq!(message, "Updating plugins");
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
    fn test_add_progress_bar_update() {
        let manager = OperationManager::new(Operation::Update).unwrap();
        let result = manager.add_progress_bar(2, 10, "Update Plugin", "2.0.0");
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_progress_bar_extract() {
        let manager = OperationManager::new(Operation::Extract).unwrap();
        let result = manager.add_progress_bar(3, 7, "Extract Plugin", "1.5.0");
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
