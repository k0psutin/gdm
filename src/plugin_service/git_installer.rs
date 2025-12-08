use crate::git_service::GitService;
use crate::operation_manager::OperationManager;
use crate::plugin_config_repository::plugin::{Plugin, PluginSource};
use crate::plugin_service::PluginInstaller;

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tracing::error;

pub struct GitInstaller {
    git_service: Arc<dyn GitService + Send + Sync>,
}

impl GitInstaller {
    pub fn new(git_service: Arc<dyn GitService + Send + Sync>) -> Self {
        Self { git_service }
    }
}

#[async_trait]
impl PluginInstaller for GitInstaller {
    fn can_handle(&self, source: &PluginSource) -> bool {
        matches!(source, PluginSource::Git { .. })
    }

    async fn install(
        &self,
        plugins: Vec<Plugin>,
        operation_manager: Arc<OperationManager>,
        start_index: usize,
        total_count: usize,
    ) -> Result<BTreeMap<String, Plugin>> {
        let mut join_set = JoinSet::new();

        for (i, plugin_def) in plugins.iter().enumerate() {
            if let Some(PluginSource::Git { url, reference }) = &plugin_def.source {
                let git_service = self.git_service.clone();
                let url = url.clone();
                let reference = if reference.is_empty() {
                    "main".to_string()
                } else {
                    reference.clone()
                };

                let global_index = start_index + i;

                let pb = operation_manager.add_progress_bar(
                    global_index,
                    total_count,
                    &url,
                    &reference,
                )?;

                pb.enable_steady_tick(Duration::from_millis(100));

                join_set.spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        pb.set_message("Cloning...");

                        let (src, file_count) = git_service
                            .shallow_fetch_repository(&url, Some(reference.clone()))
                            .with_context(|| format!("Failed to fetch {}", url))?;

                        pb.set_message("Extracting files...");
                        pb.set_length(file_count as u64);
                        pb.set_position(0);

                        let main_plugin_name =
                            git_service.extract_main_plugin_name_from_src(&src)?;

                        let addon_folders = git_service.move_downloaded_addons(&src, pb.clone())?;

                        let plugin_source = PluginSource::Git {
                            url: url.clone(),
                            reference: reference.clone(),
                        };

                        let plugins_found = git_service
                            .create_plugins_from_addons_paths(&plugin_source, &addon_folders)?;

                        let main_plugin = git_service
                            .determine_main_plugin_from_main_plugin_name_and_plugins(
                                &plugins_found,
                                &main_plugin_name,
                            )?;

                        let plugin_with_sub_assets = git_service.add_sub_assets_to_plugin(
                            &main_plugin,
                            &plugins_found,
                            &addon_folders,
                        )?;

                        pb.finish_and_clear();

                        Ok::<BTreeMap<String, Plugin>, anyhow::Error>(BTreeMap::from([(
                            main_plugin_name,
                            plugin_with_sub_assets,
                        )]))
                    })
                    .await;

                    match result {
                        Ok(inner_result) => inner_result,
                        Err(join_err) => Err(anyhow::anyhow!("Task panicked: {}", join_err)),
                    }
                });
            }
        }

        let mut installed_results = BTreeMap::new();

        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(Ok(map)) => {
                    installed_results.extend(map);
                }
                Ok(Err(e)) => {
                    error!("Error installing git plugin: {:#}", e);
                }
                Err(join_err) => {
                    error!("Git installation task failed to join: {}", join_err);
                }
            }
        }

        Ok(installed_results)
    }
}
