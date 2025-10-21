use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use super::operation::Operation;
use crate::plugin_config_repository::plugin::Plugin;

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
            ProgressStyle::with_template("{spinner:.green} {msg}")
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
        self.main_progress.finish_and_clear();
    }

    pub fn add_progress_bar(
        &self,
        index: usize,
        total: usize,
        plugin: &Plugin,
    ) -> Result<ProgressBar> {
        self.operation
            .create_progress_bar(&self.multi_progress, index, total, plugin)
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
        let plugin = Plugin::new(
            "test-plugin".to_string(),
            "Test Plugin".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
        );
        let result = manager.add_progress_bar(1, 5, &plugin);
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_progress_bar_update() {
        let manager = OperationManager::new(Operation::Update).unwrap();
        let plugin = Plugin::new(
            "update-plugin".to_string(),
            "Update Plugin".to_string(),
            "2.0.0".to_string(),
            "Apache-2.0".to_string(),
        );
        let result = manager.add_progress_bar(2, 10, &plugin);
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_progress_bar_extract() {
        let manager = OperationManager::new(Operation::Extract).unwrap();
        let plugin = Plugin::new(
            "extract-plugin".to_string(),
            "Extract Plugin".to_string(),
            "1.5.0".to_string(),
            "BSD-3-Clause".to_string(),
        );
        let result = manager.add_progress_bar(3, 7, &plugin);
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_progress_bar_finished() {
        let manager = OperationManager::new(Operation::Finished).unwrap();
        let plugin = Plugin::new(
            "finished-plugin".to_string(),
            "Finished Plugin".to_string(),
            "3.0.0".to_string(),
            "GPL-3.0".to_string(),
        );
        let result = manager.add_progress_bar(1, 1, &plugin);
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_multiple_progress_bars() {
        let manager = OperationManager::new(Operation::Install).unwrap();
        let plugin1 = Plugin::new(
            "plugin1".to_string(),
            "Plugin 1".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
        );
        let plugin2 = Plugin::new(
            "plugin2".to_string(),
            "Plugin 2".to_string(),
            "2.0.0".to_string(),
            "MIT".to_string(),
        );
        let result1 = manager.add_progress_bar(1, 2, &plugin1);
        let result2 = manager.add_progress_bar(2, 2, &plugin2);
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
        let plugin = Plugin::new(
            "workflow-plugin".to_string(),
            "Workflow Plugin".to_string(),
            "1.0.0".to_string(),
            "MIT".to_string(),
        );
        let pb = manager.add_progress_bar(1, 1, &plugin).unwrap();
        pb.finish();
        manager.finish();
    }
}
