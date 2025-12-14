use crate::installers::PluginInstaller;
use crate::models::{Plugin, PluginSource};
use crate::services::{GitService, InstallService};
use crate::ui::OperationManager;

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

pub struct GitInstaller {
    git_service: Arc<dyn GitService + Send + Sync>,
}

impl Default for GitInstaller {
    fn default() -> Self {
        let git_service = Arc::new(crate::services::DefaultGitService::default());
        Self::new(git_service)
    }
}

impl GitInstaller {
    pub fn new(git_service: Arc<dyn GitService + Send + Sync>) -> Self {
        Self { git_service }
    }
}

#[async_trait]
impl PluginInstaller for GitInstaller {
    fn can_handle(&self, source: Option<PluginSource>) -> bool {
        matches!(source, Some(PluginSource::Git { .. }))
    }

    async fn install(
        &self,
        index: usize,
        total: usize,
        install_service: &dyn InstallService,
        plugin: &Plugin,
        operation_manager: Arc<OperationManager>,
    ) -> Result<(String, Plugin)> {
        let plugin_source = match &plugin.source {
            Some(PluginSource::Git { url, reference }) => (url.clone(), reference.clone()),
            _ => {
                anyhow::bail!("Invalid plugin source for GitInstaller");
            }
        };

        let git_service = self.git_service.clone();
        let url = &plugin_source.0;
        let reference = &plugin_source.1;

        let pb = operation_manager.add_progress_bar(index, total, url, reference)?;

        pb.enable_steady_tick(Duration::from_millis(100));

        let (staging_dir, _) = tokio::task::spawn_blocking(move || {
            let url = &plugin_source.0;
            let reference = &plugin_source.1;
            git_service.shallow_fetch_repository(url, Some(reference.clone()))
        })
        .await??;

        pb.finish_and_clear();

        let repo_name = self
            .git_service
            .extract_repo_name_from_src(&staging_dir)
            .unwrap_or_else(|_| "unknown".to_string());

        let source = plugin.source.clone().unwrap();

        let (folder_name, plugin, folders_to_move) =
            install_service.discover_and_analyze_plugins(&source, &staging_dir, &repo_name)?;

        install_service.install_from_cache(&staging_dir, &folders_to_move)?;

        Ok((folder_name, plugin))
    }
}
