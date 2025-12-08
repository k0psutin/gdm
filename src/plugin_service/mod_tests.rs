#[cfg(test)]
mod tests {
    use anyhow::Ok;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    use crate::api::MockDefaultAssetStoreAPI;
    use crate::api::asset::Asset;
    use crate::api::asset_list_response::{AssetListItem, AssetListResponse};
    use crate::api::asset_response::AssetResponse;
    use crate::app_config::DefaultAppConfig;
    use crate::extract_service::MockDefaultExtractService;
    use crate::file_service::MockDefaultFileService;
    use crate::git_service::MockDefaultGitService;
    use crate::godot_config_repository::MockDefaultGodotConfigRepository;
    use crate::plugin_config_repository::MockDefaultPluginConfigRepository;
    use crate::plugin_config_repository::plugin::Plugin;
    use crate::plugin_config_repository::plugin_config::DefaultPluginConfig;
    use crate::plugin_service::asset_library_installer::AssetLibraryInstaller;
    use crate::plugin_service::git_installer::GitInstaller;
    use crate::plugin_service::{DefaultPluginService, PluginService};

    use mockall::predicate::*;

    // Helper to setup the service with specific versioning scenarios
    fn setup_plugin_service_with_versions(
        asset_id: &str,
        plugin_name: &str,
        installed_version: Option<&str>,
        return_version: &str,
        search_name: Option<&str>,
    ) -> DefaultPluginService {
        let mut godot_config_repository = MockDefaultGodotConfigRepository::default();
        let mut asset_store_api = MockDefaultAssetStoreAPI::default();
        let mut plugin_config_repository = MockDefaultPluginConfigRepository::default();
        let mut extract_service = MockDefaultExtractService::default();
        let file_service = Arc::new(MockDefaultFileService::default());
        let git_service_mock = MockDefaultGitService::default();

        // Setup godot config repository
        godot_config_repository.expect_save().returning(|_| Ok(()));

        godot_config_repository
            .expect_get_godot_version_from_project()
            .returning(|| Ok("4.5".to_string()));

        // Setup plugin config repository
        let asset_id_clone = asset_id.to_string();
        let installed_version_clone = installed_version.map(|v| v.to_string());
        let plugin_name_clone = plugin_name.to_string();

        plugin_config_repository
            .expect_get_plugin_by_asset_id()
            .returning(move |_| {
                Ok(installed_version_clone.as_ref().map(|version| {
                    Plugin::new_asset_store_plugin(
                        asset_id_clone.clone(),
                        Some(format!("addons/{}/plugin.cfg", plugin_name_clone).into()),
                        plugin_name_clone.clone(),
                        version.clone(),
                        String::from("MIT"),
                        vec![],
                    )
                }))
            });

        plugin_config_repository
            .expect_add_plugins()
            .returning(|_| Ok(DefaultPluginConfig::default()));

        // Setup asset store API
        let asset_id_for_api = asset_id.to_string();
        let plugin_name_for_api = plugin_name.to_string();

        // Add get_assets mock if search_name is provided
        if search_name.is_none() {
            asset_store_api
                .expect_get_assets()
                .returning(|_| Ok(AssetListResponse::new(vec![])));
        }

        if let Some(_name) = search_name {
            let asset_id_for_search = asset_id.to_string();
            let plugin_name_for_search = plugin_name.to_string();

            asset_store_api.expect_get_assets().returning(move |_| {
                let asset = AssetListItem::new(
                    asset_id_for_search.clone(),
                    plugin_name_for_search.clone(),
                    "Author".to_string(),
                    "Scripts".to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "official".to_string(),
                    "11".to_string(),
                    "9.1.0".to_string(),
                    "2023-10-01".to_string(),
                );
                Ok(AssetListResponse::new(vec![asset]))
            });

            // Add get_asset_by_id mock for the name search flow
            let asset_id_for_get_by_id = asset_id.to_string();
            let plugin_name_for_get_by_id = plugin_name.to_string();

            asset_store_api
                .expect_get_asset_by_id()
                .returning(move |_| {
                    Ok(AssetResponse::new(
                        asset_id_for_get_by_id.clone(),
                        plugin_name_for_get_by_id.clone(),
                        "11".to_string(),
                        "latest".to_string(),
                        "4.5".to_string(),
                        "5".to_string(),
                        "MIT".to_string(),
                        "Some description".to_string(),
                        "GitHub".to_string(),
                        "commit_hash".to_string(),
                        "2023-10-01".to_string(),
                        format!("https://example.com/{}.zip", asset_id_for_get_by_id),
                    ))
                });
        }

        asset_store_api
            .expect_get_asset_by_id_and_version()
            .returning(move |_, version| {
                Ok(AssetResponse::new(
                    asset_id_for_api.clone(),
                    plugin_name_for_api.clone(),
                    "11".to_string(),
                    version.to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    format!("https://example.com/{}.zip", asset_id_for_api),
                ))
            });

        asset_store_api
            .expect_download_asset()
            .returning(|asset_response, _pb| {
                Ok(Asset::new(
                    PathBuf::from("test_plugin"),
                    asset_response.clone(),
                ))
            });

        let asset_id_owned = asset_id.to_string();
        let plugin_name_owned = plugin_name.to_string();
        let return_version_owned = return_version.to_string();

        asset_store_api
            .expect_find_asset_by_asset_name_and_version_and_godot_version()
            .returning(move |_, _, _| {
                // 2. The closure now owns `asset_id_owned`, which is a String, not a &str
                Ok(AssetResponse::new(
                    asset_id_owned.clone(),
                    plugin_name_owned.clone(),
                    "11".to_string(),
                    return_version_owned.clone(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    format!("https://example.com/{}.zip", asset_id_owned),
                ))
            });

        // Setup extract service
        let extract_asset_version = return_version.to_string();
        let extract_asset_id = asset_id.to_string();
        extract_service
            .expect_extract_asset()
            .returning(move |_file_path, _pb| {
                Ok((
                    String::from("test_plugin"),
                    Plugin::new_asset_store_plugin(
                        extract_asset_id.clone(),
                        Some("addons/test_plugin/plugin.cfg".into()),
                        "Test Plugin".to_string(), // title from AssetResponse in test
                        extract_asset_version.clone(), // version from AssetResponse in test
                        "MIT".to_string(),         // license from AssetResponse in test
                        vec![],
                    ),
                ))
            });

        let app_config = DefaultAppConfig::default();
        let asset_store_api_arc = Arc::new(asset_store_api);
        let extract_service_arc = Arc::new(extract_service);
        let git_service_arc = Arc::new(git_service_mock);

        // --- NEW: Initialize Installers ---
        let asset_installer =
            AssetLibraryInstaller::new(asset_store_api_arc.clone(), extract_service_arc.clone());
        let git_installer = GitInstaller::new(git_service_arc.clone());

        DefaultPluginService::new(
            Box::new(godot_config_repository),
            Box::new(plugin_config_repository),
            app_config,
            file_service,
            asset_store_api_arc,
            vec![Box::new(asset_installer), Box::new(git_installer)],
        )
    }

    // Helper to setup standard mocks
    fn setup_plugin_service_mocks() -> DefaultPluginService {
        let mut godot_config_repository = MockDefaultGodotConfigRepository::default();

        godot_config_repository
            .expect_save()
            .returning(|_path| Ok(()));

        godot_config_repository
            .expect_validate_project_file()
            .returning(|| Ok(()));

        godot_config_repository
            .expect_get_godot_version_from_project()
            .returning(|| Ok("4.5".to_string()));

        let mut asset_store_api = MockDefaultAssetStoreAPI::default();

        let mut plugin_config_repository = MockDefaultPluginConfigRepository::default();
        plugin_config_repository
            .expect_add_plugins()
            .returning(|_plugins| Ok(DefaultPluginConfig::new(_plugins.clone())));

        plugin_config_repository
            .expect_remove_plugins()
            .returning(|_plugin_names| Ok(DefaultPluginConfig::default()));

        plugin_config_repository
            .expect_get_plugin_by_asset_id()
            .returning(|_asset_id| Ok(None));

        plugin_config_repository
            .expect_has_installed_plugins()
            .returning(|| Ok(true));

        let app_config = DefaultAppConfig::default();
        let mut extract_service = MockDefaultExtractService::default();

        let file_service = Arc::new(MockDefaultFileService::default());

        extract_service
            .expect_extract_asset()
            .returning(|_file_path, _pb| {
                Ok((
                    String::from("test_plugin"),
                    Plugin::new_asset_store_plugin(
                        "1234".to_string(),
                        Some("addons/test_plugin/plugin.cfg".into()),
                        "Test Plugin".to_string(),
                        "1.1.1".to_string(),
                        "MIT".to_string(),
                        vec![],
                    ),
                ))
            });
        plugin_config_repository.expect_get_plugins().returning(|| {
            Ok(BTreeMap::from([(
                String::from("test_plugin"),
                Plugin::new_asset_store_plugin(
                    String::from("1234"),
                    Some("addons/test_plugin/plugin.cfg".into()),
                    String::from("Test Plugin"),
                    String::from("1.1.1"),
                    String::from("MIT"),
                    vec![],
                ),
            )]))
        });

        plugin_config_repository
            .expect_get_plugin_by_asset_id()
            .returning(|_asset_id| {
                Ok(Some(Plugin::new_asset_store_plugin(
                    "1234".to_string(),
                    Some("addons/test_plugin/plugin.cfg".into()),
                    "Test Plugin".to_string(),
                    "1.1.1".to_string(),
                    "MIT".to_string(),
                    vec![],
                )))
            });

        plugin_config_repository
            .expect_get_plugin_by_asset_id()
            .returning(|_asset_id| {
                Ok(Some(Plugin::new_asset_store_plugin(
                    "1234".to_string(),
                    Some("addons/test_plugin/plugin.cfg".into()),
                    "Test Plugin".to_string(),
                    "1.1.1".to_string(),
                    "MIT".to_string(),
                    vec![],
                )))
            });

        asset_store_api
            .expect_find_asset_by_id_or_name_and_version()
            .returning(|_, _, _| {
                Ok(AssetResponse::new(
                    "1234".to_string(),
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    "1.1.1".to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    "https://example.com/test_plugin.zip".to_string(),
                ))
            });

        asset_store_api
            .expect_get_asset_by_id_and_version()
            .with(eq("1234"), eq("1.0.0"))
            .returning(|asset_id, version| {
                Err(anyhow::anyhow!(
                    "Asset with ID {} and version {} not found",
                    asset_id,
                    version
                ))
            });
        asset_store_api
            .expect_get_asset_by_id_and_version()
            .with(eq("1234"), eq("1.1.1"))
            .returning(|asset_id, version| {
                Ok(AssetResponse::new(
                    asset_id.to_string(),
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    version.to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    "https://example.com/test_plugin.zip".to_string(),
                ))
            });
        asset_store_api
            .expect_get_asset_by_id()
            .with(eq("1234".to_string()))
            .returning(|asset_id| {
                Ok(AssetResponse::new(
                    asset_id.to_string(),
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    "1.1.1".to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    "https://example.com/test_plugin.zip".to_string(),
                ))
            });
        asset_store_api.expect_download_asset().returning(|_, _pb| {
            Ok(Asset::new(
                PathBuf::from("test_plugin"),
                AssetResponse::new(
                    "1234".to_string(),
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    "1.1.1".to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    "https://example.com/test_plugin.zip".to_string(),
                ),
            ))
        });
        asset_store_api.expect_get_assets().returning(|_params| {
            Ok(AssetListResponse::new(vec![AssetListItem::new(
                "1234".to_string(),
                "Test Plugin".to_string(),
                "Test Maker".to_string(),
                "Tools".to_string(),
                "4.5".to_string(),
                "5".to_string(),
                "MIT".to_string(),
                "??".to_string(),
                "11".to_string(),
                "1.1.1".to_string(),
                "2023-10-01".to_string(),
            )]))
        });

        let git_service_mock = MockDefaultGitService::default();

        let asset_store_api_arc = Arc::new(asset_store_api);
        let extract_service_arc = Arc::new(extract_service);
        let git_service_arc = Arc::new(git_service_mock);

        // --- NEW: Initialize Installers ---
        let asset_installer =
            AssetLibraryInstaller::new(asset_store_api_arc.clone(), extract_service_arc.clone());
        let git_installer = GitInstaller::new(git_service_arc.clone());

        DefaultPluginService::new(
            Box::new(godot_config_repository),
            Box::new(plugin_config_repository),
            app_config,
            file_service,
            asset_store_api_arc,
            vec![Box::new(asset_installer), Box::new(git_installer)],
        )
    }

    // get_asset_list_response_by_name_or_version

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_with_no_results_should_return_ok() {
        let plugin_service = setup_plugin_service_with_versions(
            "1234",
            "some_non_existent_plugin_name",
            Some("1.0.0"),
            "1.0.0",
            None,
        );
        let name = "some_non_existent_plugin_name";
        let version = "4.5";
        let result_list = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result_list.is_ok());
        let result = result_list.unwrap();
        assert!(result.result.is_empty());
    }

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_with_exact_name_should_return_one_result()
     {
        let plugin_service = setup_plugin_service_mocks();
        let name = "Test Plugin";
        let version = "4.5";
        let result = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result.is_ok());
        let assets = result.unwrap();
        assert!(assets.result.len() == 1);
        let asset = assets.result.first().unwrap();
        assert_eq!(asset.title, "Test Plugin");
        assert_eq!(asset.asset_id, "1234");
    }

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_without_name_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        let name = "";
        let version = "4.5";
        let result = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result.is_err());
    }

    // install_all_plugins

    #[tokio::test]
    async fn test_install_plugins_should_install_all_plugins_in_config() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service.install_all_plugins().await;
        assert!(result.is_ok());
        let installed_plugins = result.unwrap();

        let expected_plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new_asset_store_plugin(
                String::from("1234"),
                Some("addons/test_plugin/plugin.cfg".into()),
                String::from("Test Plugin"),
                String::from("1.1.1"),
                String::from("MIT"),
                vec![],
            ),
        )]);

        assert_eq!(installed_plugins, expected_plugins);
    }

    // add_plugin tests (Replaces old install_plugin tests)

    #[tokio::test]
    async fn test_add_plugin_with_asset_id_and_no_version_should_install_asset() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service
            .add_plugin(Some("1234".to_string()), None, None, None, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_plugin_with_only_version_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        // Providing only version
        let result = plugin_service
            .add_plugin(None, None, Some("1.1.1".to_string()), None, None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_plugin_with_asset_id_and_version_should_install_plugin() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service
            .add_plugin(
                Some("1234".to_string()),
                None,
                Some("1.1.1".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_plugin_with_name_should_install_plugin() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service
            .add_plugin(None, Some("Test Plugin".to_string()), None, None, None)
            .await;
        assert!(result.is_ok());
    }

    // Error cases for add_plugin

    #[tokio::test]
    async fn test_add_plugin_with_invalid_asset_id_should_return_err() {
        // We need mocks even for error cases if it reaches the API
        let mut godot_config_repository = MockDefaultGodotConfigRepository::default();
        godot_config_repository
            .expect_get_godot_version_from_project()
            .returning(|| Ok("4.5".to_string()));
        let mut asset_store_api = MockDefaultAssetStoreAPI::default();
        asset_store_api
            .expect_find_asset_by_id_or_name_and_version()
            .returning(|_, _, _| Err(anyhow::anyhow!("Not found")));

        let plugin_config_repository = MockDefaultPluginConfigRepository::default();
        let app_config = DefaultAppConfig::default();
        let file_service = Arc::new(MockDefaultFileService::default());
        let extract_service = Arc::new(MockDefaultExtractService::default());
        let _git_service = Arc::new(MockDefaultGitService::default());

        let asset_store_api_arc = Arc::new(asset_store_api);
        let installer =
            AssetLibraryInstaller::new(asset_store_api_arc.clone(), extract_service.clone());

        let plugin_service = DefaultPluginService::new(
            Box::new(godot_config_repository),
            Box::new(plugin_config_repository),
            app_config,
            file_service,
            asset_store_api_arc,
            vec![Box::new(installer)],
        );

        let result = plugin_service
            .add_plugin(Some("99999".to_string()), None, None, None, None)
            .await;
        assert!(result.is_err());
    }

    // Version comparison tests

    #[tokio::test]
    async fn test_add_plugin_when_newer_version_already_installed_should_downgrade() {
        let plugin_service = setup_plugin_service_with_versions(
            "1234",
            "Test Plugin",
            Some("2.0.0"), // Already installed version (newer)
            "1.5.0",
            None,
        );

        let result = plugin_service
            .add_plugin(
                Some("1234".to_string()),
                None,
                Some("1.5.0".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_plugin_with_name_and_version_should_install_correct_version() {
        let plugin_service = setup_plugin_service_with_versions(
            "1709",
            "GUT - Godot Unit Testing (Godot 4)",
            None, // Not installed
            "9.1.0",
            Some("Godot Unit Testing"), // Enable name search
        );

        let result = plugin_service
            .add_plugin(
                None,
                Some("Godot Unit Testing".to_string()),
                Some("9.1.0".to_string()),
                None,
                None,
            )
            .await;

        assert!(result.is_ok());
    }

    // update_plugins

    fn setup_update_plugin_mocks(
        current_plugin_version: &str,
        update_plugin_version: &str,
    ) -> DefaultPluginService {
        let mut godot_config_repository = MockDefaultGodotConfigRepository::default();

        godot_config_repository
            .expect_save()
            .returning(|_path| Ok(()));
        godot_config_repository
            .expect_get_godot_version_from_project()
            .returning(|| Ok("4.5".to_string()));

        let mut asset_store_api = MockDefaultAssetStoreAPI::default();

        let mut plugin_config_repository = MockDefaultPluginConfigRepository::default();
        plugin_config_repository
            .expect_add_plugins()
            .returning(|_plugins| Ok(DefaultPluginConfig::new(_plugins.clone())));

        plugin_config_repository
            .expect_remove_plugins()
            .returning(|_plugin_names| Ok(DefaultPluginConfig::default()));

        plugin_config_repository
            .expect_has_installed_plugins()
            .returning(|| Ok(true));

        let current_plugin_version_owned = current_plugin_version.to_string();

        plugin_config_repository
            .expect_get_plugin_by_asset_id()
            .returning(move |_asset_id| {
                Ok(Some(Plugin::new_asset_store_plugin(
                    "1234".to_string(),
                    Some("addons/test_plugin/plugin.cfg".into()),
                    "Test Plugin".to_string(),
                    current_plugin_version_owned.clone(),
                    "MIT".to_string(),
                    vec![],
                )))
            });

        let app_config = DefaultAppConfig::default();
        let mut extract_service = MockDefaultExtractService::default();
        let file_service = Arc::new(MockDefaultFileService::default());

        let extract_asset_version = update_plugin_version.to_string();
        extract_service
            .expect_extract_asset()
            .returning(move |_file_path, _pb| {
                Ok((
                    String::from("test_plugin"),
                    Plugin::new_asset_store_plugin(
                        "1234".to_string(),
                        Some("addons/test_plugin/plugin.cfg".into()),
                        "test_plugin".to_string(),
                        extract_asset_version.clone(),
                        "MIT".to_string(),
                        vec![],
                    ),
                ))
            });

        plugin_config_repository.expect_get_plugins().returning({
            let current_plugin_version = current_plugin_version.to_string();
            move || {
                Ok(BTreeMap::from([(
                    String::from("test_plugin"),
                    Plugin::new_asset_store_plugin(
                        String::from("1234"),
                        Some("addons/test_plugin/plugin.cfg".into()),
                        String::from("Test Plugin"),
                        current_plugin_version.clone(),
                        String::from("MIT"),
                        vec![],
                    ),
                )]))
            }
        });

        // Mocks for getting latest assets
        let _get_asset_by_id_version = current_plugin_version.to_string();
        asset_store_api
            .expect_get_asset_by_id_and_version()
            .returning(move |asset_id, version| {
                Ok(AssetResponse::new(
                    asset_id.to_string(),
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    version.to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    "https://example.com/test_plugin.zip".to_string(),
                ))
            });

        // This mock is crucial for `fetch_latest_assets` inside update_plugins
        let asset_store_plugin_version = update_plugin_version.to_string();
        asset_store_api
            .expect_find_asset_by_id_or_name_and_version()
            .returning(move |asset_id, _, _| {
                Ok(AssetResponse::new(
                    asset_id.to_string(),
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    asset_store_plugin_version.to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    "https://example.com/test_plugin.zip".to_string(),
                ))
            });

        // Needed for find_asset_metadata if add_plugin is called
        asset_store_api
            .expect_get_asset_by_id()
            .returning(move |asset_id| {
                Ok(AssetResponse::new(
                    asset_id.to_string(),
                    "Test Plugin".to_string(),
                    "11".to_string(),
                    "1.0.0".to_string(),
                    "4.5".to_string(),
                    "5".to_string(),
                    "MIT".to_string(),
                    "Some description".to_string(),
                    "GitHub".to_string(),
                    "commit_hash".to_string(),
                    "2023-10-01".to_string(),
                    "https://example.com/test_plugin.zip".to_string(),
                ))
            });

        asset_store_api
            .expect_download_asset()
            .returning(|asset_response, _pb| {
                Ok(Asset::new(
                    PathBuf::from("test_plugin"),
                    asset_response.clone(),
                ))
            });

        let git_service_mock = MockDefaultGitService::default();
        let asset_store_api_arc = Arc::new(asset_store_api);
        let extract_service_arc = Arc::new(extract_service);

        let asset_installer =
            AssetLibraryInstaller::new(asset_store_api_arc.clone(), extract_service_arc.clone());
        let git_installer = GitInstaller::new(Arc::new(git_service_mock));

        DefaultPluginService::new(
            Box::new(godot_config_repository),
            Box::new(plugin_config_repository),
            app_config,
            file_service,
            asset_store_api_arc,
            vec![Box::new(asset_installer), Box::new(git_installer)],
        )
    }

    #[tokio::test]
    async fn test_update_plugins_should_return_correct_plugins_if_there_is_an_update_1() {
        let plugin_service = setup_update_plugin_mocks("1.1.1", "1.2.0");
        let result = plugin_service.update_plugins().await;
        assert!(result.is_ok());

        let updated_plugins = result.unwrap();
        let expected_updated_plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new_asset_store_plugin(
                String::from("1234"),
                Some("addons/test_plugin/plugin.cfg".into()),
                String::from("Test Plugin"),
                String::from("1.2.0"),
                String::from("MIT"),
                vec![],
            ),
        )]);
        assert_eq!(updated_plugins, expected_updated_plugins);
    }

    #[tokio::test]
    async fn test_update_plugins_should_return_correct_plugins_if_there_is_no_update() {
        let plugin_service = setup_update_plugin_mocks("1.1.1", "1.1.1");
        let result = plugin_service.update_plugins().await;
        assert!(result.is_ok());

        let updated_plugins = result.unwrap();
        let expected_updated_plugins = BTreeMap::from([]);
        assert_eq!(updated_plugins, expected_updated_plugins);
    }

    // remove_plugin_by_name

    #[tokio::test]
    async fn test_remove_plugin_by_name_should_remove_plugin() {
        let mut godot_config_repository = MockDefaultGodotConfigRepository::default();
        godot_config_repository
            .expect_save()
            .returning(|_path| Ok(()));
        godot_config_repository
            .expect_validate_project_file()
            .returning(|| Ok(()));

        let mut plugin_config_repository = MockDefaultPluginConfigRepository::default();
        plugin_config_repository
            .expect_get_plugin_by_name()
            .with(eq("test_plugin"))
            .returning(|_name| Some(("test_plugin".to_string(), Plugin::create_mock_plugin_1())));
        plugin_config_repository
            .expect_remove_plugins()
            .returning(|_names| Ok(DefaultPluginConfig::default()));
        plugin_config_repository
            .expect_has_installed_plugins()
            .returning(|| Ok(true));

        let mut file_service = MockDefaultFileService::default();
        file_service
            .expect_file_exists()
            .returning(|_path| Ok(true));
        file_service
            .expect_directory_exists()
            .returning(|_path| true);
        file_service
            .expect_remove_dir_all()
            .returning(|_path| Ok(()));

        let git_service_mock = MockDefaultGitService::default();
        let extract_service = Arc::new(MockDefaultExtractService::default());
        let asset_store = Arc::new(MockDefaultAssetStoreAPI::default());

        // Setup Installers even if not used by this method directly,
        // because constructor requires them
        let asset_installer = AssetLibraryInstaller::new(asset_store.clone(), extract_service);
        let git_installer = GitInstaller::new(Arc::new(git_service_mock));

        let plugin_service = DefaultPluginService::new(
            Box::new(godot_config_repository),
            Box::new(plugin_config_repository),
            DefaultAppConfig::default(),
            Arc::new(file_service),
            asset_store,
            vec![Box::new(asset_installer), Box::new(git_installer)],
        );

        let result = plugin_service.remove_plugin_by_name("test_plugin").await;
        assert!(result.is_ok());
    }

    // finish_plugins_operation

    #[test]
    fn test_finish_plugins_operation_should_complete_successfully() {
        // Setup minimal mocks just to satisfy constructor
        let godot_config = MockDefaultGodotConfigRepository::default();
        let plugin_config = MockDefaultPluginConfigRepository::default();
        let app_config = DefaultAppConfig::default();
        let file_service = Arc::new(MockDefaultFileService::default());
        let asset_store = Arc::new(MockDefaultAssetStoreAPI::default());
        let extract = Arc::new(MockDefaultExtractService::default());
        let git_service = MockDefaultGitService::default();

        let asset_installer = AssetLibraryInstaller::new(asset_store.clone(), extract);
        let git_installer = GitInstaller::new(Arc::new(git_service));

        let plugin_service = DefaultPluginService::new(
            Box::new(godot_config),
            Box::new(plugin_config),
            app_config,
            file_service,
            asset_store,
            vec![Box::new(asset_installer), Box::new(git_installer)],
        );

        // Updated test data: Use Vec instead of BTreeMap
        let plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new_asset_store_plugin(
                String::from("1234"),
                Some("addons/test_plugin/plugin.cfg".into()),
                String::from("Test Plugin"),
                String::from("1.1.1"),
                String::from("MIT"),
                vec![],
            ),
        )]);

        let result = plugin_service.finish_plugins_operation(&plugins);
        assert!(result.is_ok());
    }
}
