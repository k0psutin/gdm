#[cfg(test)]
mod tests {
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
    use crate::godot_config_repository::MockDefaultGodotConfigRepository;
    use crate::plugin_config_repository::MockDefaultPluginConfigRepository;
    use crate::plugin_config_repository::plugin::Plugin;
    use crate::plugin_config_repository::plugin_config::DefaultPluginConfig;
    use crate::plugin_service::operation::Operation;
    use crate::plugin_service::{DefaultPluginService, PluginService};

    use mockall::predicate::*;

    fn setup_plugin_service_with_versions(
        asset_id: &str,
        plugin_name: &str,
        installed_version: Option<&str>,
        search_name: Option<&str>,
    ) -> DefaultPluginService {
        let mut godot_config_repository = MockDefaultGodotConfigRepository::default();
        let mut asset_store_api = MockDefaultAssetStoreAPI::default();
        let mut plugin_config_repository = MockDefaultPluginConfigRepository::default();
        let mut extract_service = MockDefaultExtractService::default();
        let file_service = Arc::new(MockDefaultFileService::default());

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
                installed_version_clone.as_ref().map(|version| {
                    Plugin::new(
                        asset_id_clone.clone(),
                        plugin_name_clone.clone(),
                        version.clone(),
                        String::from("MIT"),
                        vec![],
                        true,
                    )
                })
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

        // Setup extract service
        extract_service
            .expect_extract_asset()
            .returning(|_file_path, _pb| {
                Ok((
                    String::from("test_plugin"),
                    Plugin::new(
                        "1234".to_string(),        // asset_id from AssetResponse in test
                        "Test Plugin".to_string(), // title from AssetResponse in test
                        "1.1.1".to_string(),       // version from AssetResponse in test
                        "MIT".to_string(),         // license from AssetResponse in test
                        vec![],
                        true,
                    ),
                ))
            });

        let app_config = DefaultAppConfig::default();
        DefaultPluginService::new(
            Box::new(godot_config_repository),
            Arc::new(asset_store_api),
            Box::new(plugin_config_repository),
            app_config,
            Arc::new(extract_service),
            file_service,
        )
    }

    // find_plugin_by_id_or_name

    #[tokio::test]
    async fn test_find_plugin_by_id_or_name_with_id_with_none_parameters_should_return_error() {
        let plugin_service = setup_plugin_service_mocks();
        let asset_id = "";
        let name = "";
        let result = plugin_service
            .find_plugin_by_id_or_name(asset_id, name)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_plugin_by_id_or_name_with_id_should_return_asset() {
        let plugin_service = setup_plugin_service_mocks();
        let asset_id = "1234";
        let name = "";
        let result = plugin_service
            .find_plugin_by_id_or_name(asset_id, name)
            .await;
        assert!(result.is_ok());
        let asset = result.unwrap();
        assert_eq!(asset.asset_id, "1234");
    }

    #[tokio::test]
    async fn test_find_plugin_by_id_or_name_with_name_should_return_asset() {
        let plugin_service = setup_plugin_service_mocks();
        let asset_id = "";
        let name = "Test Plugin";
        let result = plugin_service
            .find_plugin_by_id_or_name(asset_id, name)
            .await;

        assert!(result.is_ok());
        let asset = result.unwrap();
        assert_eq!(asset.title, "Test Plugin");
        assert_eq!(asset.asset_id, "1234");
    }

    // find_plugin_by_asset_id_and_version

    #[tokio::test]
    async fn test_find_plugin_by_asset_id_and_version_missing_version_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        let asset_id = String::from("1234");
        let version = String::from("");
        let result = plugin_service
            .find_plugin_by_asset_id_and_version(asset_id, version)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_plugin_by_asset_id_and_version_missing_asset_id_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        let asset_id = String::from("");
        let version = String::from("1.0.0");
        let result = plugin_service
            .find_plugin_by_asset_id_and_version(asset_id, version)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_plugin_by_asset_id_and_version_should_return_asset() {
        let plugin_service = setup_plugin_service_mocks();
        let asset_id = String::from("1234");
        let version = String::from("1.1.1");
        let result = plugin_service
            .find_plugin_by_asset_id_and_version(asset_id, version)
            .await;

        assert!(result.is_ok());
        let asset = result.unwrap();
        assert_eq!(asset.title, "Test Plugin");
        assert_eq!(asset.asset_id, "1234");
        assert_eq!(asset.version_string, "1.1.1");
    }

    // find_plugin_by_asset_name_and_version

    #[tokio::test]
    async fn test_find_plugin_by_asset_name_and_version_should_return_asset() {
        let plugin_service = setup_plugin_service_mocks();
        let name = "Godot Unit Testing";
        let version = "1.1.1";
        let result = plugin_service
            .find_plugin_by_asset_name_and_version(name, version)
            .await;

        assert!(result.is_ok());
        let asset = result.unwrap();
        assert_eq!(asset.title, "Test Plugin");
        assert_eq!(asset.asset_id, "1234");
        assert_eq!(asset.version_string, "1.1.1");
    }

    #[tokio::test]
    async fn test_find_plugin_by_asset_name_and_version_missing_name_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        let name = "";
        let version = "1.1.1";
        let result = plugin_service
            .find_plugin_by_asset_name_and_version(name, version)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_plugin_by_asset_name_and_version_missing_version_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        let name = "Test Plugin";
        let version = "";
        let result = plugin_service
            .find_plugin_by_asset_name_and_version(name, version)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_plugin_by_asset_name_and_version_with_invalid_name_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        let name = "NonExistentPluginXYZ123";
        let version = "1.0.0";
        let result = plugin_service
            .find_plugin_by_asset_name_and_version(name, version)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_plugin_by_asset_name_and_version_with_invalid_version_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        let name = "SomethingSomething";
        let version = "1.0.0"; // Non-existent version
        let result = plugin_service
            .find_plugin_by_asset_name_and_version(name, version)
            .await;

        assert!(result.is_err());
    }

    // get_asset_list_response_by_name_or_version

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_with_no_results_should_return_ok() {
        let plugin_service = setup_plugin_service_with_versions(
            "1234",
            "some_non_existent_plugin_name",
            Some("1.0.0"),
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

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_without_version_should_return_response()
     {
        let plugin_service = setup_plugin_service_mocks();
        let name = "Godot Unit Testing";
        let version = "";
        let result = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_asset_list_response_by_name_or_version_without_name_or_version_should_return_err()
     {
        let plugin_service = setup_plugin_service_mocks();
        let name = "";
        let version = "";
        let result = plugin_service
            .get_asset_list_response_by_name_or_version(name, version)
            .await;
        assert!(result.is_err());
    }

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
            .returning(|_asset_id| None);

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
                    Plugin::new(
                        "1234".to_string(),
                        "Test Plugin".to_string(),
                        "1.1.1".to_string(),
                        "MIT".to_string(),
                        vec![],
                        true,
                    ),
                ))
            });
        plugin_config_repository.expect_get_plugins().returning(|| {
            Ok(BTreeMap::from([(
                String::from("test_plugin"),
                Plugin::new(
                    String::from("1234"),
                    String::from("Test Plugin"),
                    String::from("1.1.1"),
                    String::from("MIT"),
                    vec![],
                    true,
                ),
            )]))
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
        DefaultPluginService::new(
            Box::new(godot_config_repository),
            Arc::new(asset_store_api),
            Box::new(plugin_config_repository),
            app_config,
            Arc::new(extract_service),
            file_service,
        )
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
            Plugin::new(
                String::from("1234"),
                String::from("Test Plugin"),
                String::from("1.1.1"),
                String::from("MIT"),
                vec![],
                true,
            ),
        )]);

        assert_eq!(installed_plugins, expected_plugins);
    }

    // install_plugins

    #[tokio::test]
    async fn test_install_plugin_with_asset_id_and_no_version_should_install_asset() {
        let plugin_service = setup_plugin_service_mocks();
        let name = "";
        let asset_id = "1234";
        let version = "";
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_ok());
        let installed_plugins = result.unwrap();

        let expected_plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new(
                String::from("1234"),
                String::from("Test Plugin"),
                String::from("1.1.1"),
                String::from("MIT"),
                vec![],
                true,
            ),
        )]);

        assert_eq!(installed_plugins, expected_plugins);
    }

    #[tokio::test]
    async fn test_install_plugin_with_only_version_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        let name = "";
        let asset_id = "";
        let version = "1.1.1";
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_install_plugin_with_asset_id_and_version_should_install_plugin() {
        let plugin_service = setup_plugin_service_mocks();
        let name = "";
        let asset_id = "1234";
        let version = "1.1.1";
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_ok());

        let installed_plugins = result.unwrap();
        let expected_plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new(
                String::from("1234"),
                String::from("Test Plugin"),
                String::from("1.1.1"),
                String::from("MIT"),
                vec![],
                true,
            ),
        )]);
        assert_eq!(installed_plugins, expected_plugins);
    }

    #[tokio::test]
    async fn test_install_plugin_with_name_should_install_plugin() {
        let plugin_service = setup_plugin_service_mocks();
        let name = "Test Plugin";
        let asset_id = "";
        let version = "";
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_ok());

        let installed_plugins = result.unwrap();
        let expected_plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new(
                String::from("1234"),
                String::from("Test Plugin"),
                String::from("1.1.1"),
                String::from("MIT"),
                vec![],
                true,
            ),
        )]);
        assert_eq!(installed_plugins, expected_plugins);
    }

    // Error cases for install_plugin

    #[tokio::test]
    async fn test_install_plugin_with_invalid_asset_id_should_return_err() {
        let plugin_service = DefaultPluginService::default();
        let name = "";
        let asset_id = "99999";
        let version = "";
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_install_plugin_with_invalid_name_should_return_err() {
        let plugin_service = DefaultPluginService::default();
        let name = "NonExistentPluginThatDoesNotExist12345";
        let asset_id = "";
        let version = "";
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_install_plugin_with_nonexistent_version_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        let name = "";
        let asset_id = "1234";
        let version = "1.0.0";
        let result = plugin_service.install_plugin(name, asset_id, version).await;
        assert!(result.is_err());
    }

    // Version comparison tests for install_plugin

    #[tokio::test]
    async fn test_install_plugin_when_newer_version_already_installed_should_downgrade() {
        let plugin_service = setup_plugin_service_with_versions(
            "1234",
            "Test Plugin",
            Some("2.0.0"), // Already installed version (newer)
            None,          // No name search needed
        );

        let result = plugin_service.install_plugin("", "1234", "1.5.0").await;
        assert!(result.is_ok());

        let installed_plugins = result.unwrap();
        let plugin = installed_plugins.get("test_plugin").unwrap();
        assert_eq!(plugin.get_version(), "1.5.0");
    }

    #[tokio::test]
    async fn test_install_plugin_when_older_version_already_installed_should_upgrade() {
        let plugin_service = setup_plugin_service_with_versions(
            "1234",
            "Test Plugin",
            Some("1.0.0"), // Already installed version (older)
            None,          // No name search needed
        );

        let result = plugin_service.install_plugin("", "1234", "2.0.0").await;
        assert!(result.is_ok());

        let installed_plugins = result.unwrap();
        let plugin = installed_plugins.get("test_plugin").unwrap();
        assert_eq!(plugin.get_version(), "2.0.0");
    }

    #[tokio::test]
    async fn test_install_plugin_when_same_version_already_installed_should_succeed() {
        let plugin_service = setup_plugin_service_with_versions(
            "1234",
            "Test Plugin",
            Some("1.5.0"), // Already installed version
            None,          // No name search needed
        );

        let result = plugin_service.install_plugin("", "1234", "1.5.0").await;
        assert!(result.is_ok());

        let installed_plugins = result.unwrap();
        let plugin = installed_plugins.get("test_plugin").unwrap();
        assert_eq!(plugin.get_version(), "1.5.0");
    }

    #[tokio::test]
    async fn test_install_plugin_when_not_installed_should_install_requested_version() {
        let plugin_service = setup_plugin_service_with_versions(
            "1234",
            "Test Plugin",
            None, // Not installed
            None, // No name search needed
        );

        let result = plugin_service.install_plugin("", "1234", "1.2.3").await;
        assert!(result.is_ok());

        let installed_plugins = result.unwrap();
        let plugin = installed_plugins.get("test_plugin").unwrap();
        assert_eq!(plugin.get_version(), "1.2.3");
    }

    #[tokio::test]
    async fn test_install_plugin_version_comparison_with_prerelease_versions() {
        let plugin_service = setup_plugin_service_with_versions(
            "1234",
            "Test Plugin",
            Some("1.0.0-beta"), // Already installed prerelease version
            None,               // No name search needed
        );

        let result = plugin_service.install_plugin("", "1234", "1.0.0").await;
        assert!(result.is_ok());

        let installed_plugins = result.unwrap();
        let plugin = installed_plugins.get("test_plugin").unwrap();
        assert_eq!(plugin.get_version(), "1.0.0");
    }

    // install_plugin with name and version

    #[tokio::test]
    async fn test_install_plugin_with_name_and_version_should_install_correct_version() {
        let plugin_service = setup_plugin_service_with_versions(
            "1709",
            "GUT - Godot Unit Testing (Godot 4)",
            None,                       // Not installed
            Some("Godot Unit Testing"), // Enable name search
        );

        let name = "Godot Unit Testing";
        let asset_id = "";
        let version = "9.1.0";
        let result = plugin_service.install_plugin(name, asset_id, version).await;

        assert!(result.is_ok());
        let installed_plugins = result.unwrap();
        assert_eq!(installed_plugins.len(), 1);

        // Verify the correct version was installed
        let plugin = installed_plugins.get("test_plugin").unwrap();
        assert_eq!(plugin.asset_id, "1709");
        assert_eq!(plugin.get_version(), "9.1.0");
    }

    #[tokio::test]
    async fn test_install_plugin_with_name_and_version_when_already_installed_newer_should_downgrade()
     {
        let plugin_service = setup_plugin_service_with_versions(
            "1709",
            "GUT - Godot Unit Testing (Godot 4)",
            Some("9.5.0"),              // Already installed newer version
            Some("Godot Unit Testing"), // Enable name search
        );

        let name = "Godot Unit Testing";
        let asset_id = "";
        let version = "9.1.0";
        let result = plugin_service.install_plugin(name, asset_id, version).await;

        assert!(result.is_ok());
        let installed_plugins = result.unwrap();
        let plugin = installed_plugins.get("test_plugin").unwrap();
        assert_eq!(plugin.get_version(), "9.1.0");
    }

    #[tokio::test]
    async fn test_install_plugin_with_name_and_version_when_already_installed_older_should_upgrade()
    {
        let plugin_service = setup_plugin_service_with_versions(
            "1709",
            "GUT - Godot Unit Testing (Godot 4)",
            Some("9.0.0"),              // Already installed older version
            Some("Godot Unit Testing"), // Enable name search
        );

        let name = "Godot Unit Testing";
        let asset_id = "";
        let version = "9.1.0";
        let result = plugin_service.install_plugin(name, asset_id, version).await;

        assert!(result.is_ok());
        let installed_plugins = result.unwrap();
        let plugin = installed_plugins.get("test_plugin").unwrap();
        assert_eq!(plugin.get_version(), "9.1.0");
    }

    #[tokio::test]
    async fn test_install_plugin_with_name_and_invalid_version_should_return_err() {
        let plugin_service = setup_plugin_service_with_versions(
            "31231",
            "Some Plugin Name",
            None, // Not installed
            None, // Enable name search
        );
        let name = "Some Plugin Name";
        let asset_id = "";
        let version = "0.0.1"; // Non-existent version
        let result = plugin_service.install_plugin(name, asset_id, version).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_install_plugin_with_name_and_version_but_multiple_matches_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        // Using a generic term that might match multiple plugins
        let name = "unit"; // This should match multiple results
        let asset_id = "";
        let version = "1.0.0";
        let result = plugin_service.install_plugin(name, asset_id, version).await;

        // Should fail because find_plugin_by_id_or_name expects exactly one match
        assert!(result.is_err());
    }

    // download_plugins_operation

    #[tokio::test]
    async fn test_download_and_extract_plugins_should_return_correct_plugins() {
        let plugin_service = setup_plugin_service_mocks();
        let plugin = vec![AssetResponse::new(
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
        )];
        let result = plugin_service
            .download_plugins_operation(Operation::Install, &plugin)
            .await;
        assert!(result.is_ok());

        let downloaded_assets = result.unwrap();
        assert_eq!(downloaded_assets.len(), 1);

        let asset = &downloaded_assets[0];
        assert_eq!(asset.file_path, PathBuf::from("test_plugin"));
        assert_eq!(asset.asset_response.asset_id, "1234");
        assert_eq!(asset.asset_response.title, "Test Plugin");
        assert_eq!(asset.asset_response.version_string, "1.1.1");
    }

    // extract_plugins_operation

    #[tokio::test]
    async fn test_extract_plugins_operation_should_return_correct_plugins() {
        let plugin_service = setup_plugin_service_mocks();
        let assets = vec![Asset::new(
            PathBuf::from("test_plugin.zip"),
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
        )];

        let result = plugin_service.extract_plugins_operation(&assets).await;
        assert!(result.is_ok());

        let extracted_plugins = result.unwrap();
        assert_eq!(extracted_plugins.len(), 1);

        let plugin = extracted_plugins.get("test_plugin").unwrap();
        assert_eq!(plugin.asset_id, "1234");
        assert_eq!(plugin.title, "Test Plugin");
        assert_eq!(plugin.get_version(), "1.1.1");
        assert_eq!(plugin.license, "MIT");
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

        let app_config = DefaultAppConfig::default();
        let mut extract_service = MockDefaultExtractService::default();

        let file_service = Arc::new(MockDefaultFileService::default());

        extract_service
            .expect_extract_asset()
            .returning(|_file_path, _pb| {
                Ok((
                    String::from("test_plugin"),
                    Plugin::new(
                        "mock_asset_id".to_string(),
                        "test_plugin".to_string(),
                        "1.0.0".to_string(),
                        "MIT".to_string(),
                        vec![],
                        true,
                    ),
                ))
            });

        plugin_config_repository.expect_get_plugins().returning({
            let current_plugin_version = current_plugin_version.to_string();
            move || {
                Ok(BTreeMap::from([(
                    String::from("test_plugin"),
                    Plugin::new(
                        String::from("1234"),
                        String::from("Test Plugin"),
                        current_plugin_version.clone(),
                        String::from("MIT"),
                        vec![],
                        true,
                    ),
                )]))
            }
        });
        let get_asset_by_id_version = current_plugin_version.to_string();
        asset_store_api
            .expect_get_asset_by_id_and_version()
            .with(eq("1234".to_string()), eq(get_asset_by_id_version.clone()))
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
        let asset_store_plugin_version = update_plugin_version.to_string();
        asset_store_api
            .expect_get_asset_by_id()
            .with(eq("1234".to_string()))
            .returning(move |asset_id| {
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
        asset_store_api
            .expect_download_asset()
            .returning(|asset_response, _pb| {
                Ok(Asset::new(
                    PathBuf::from("test_plugin"),
                    asset_response.clone(),
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
        DefaultPluginService::new(
            Box::new(godot_config_repository),
            Arc::new(asset_store_api),
            Box::new(plugin_config_repository),
            app_config,
            Arc::new(extract_service),
            file_service,
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
            Plugin::new(
                String::from("1234"),
                String::from("Test Plugin"),
                String::from("1.2.0"),
                String::from("MIT"),
                vec![],
                true,
            ),
        )]);
        assert_eq!(updated_plugins, expected_updated_plugins);
    }

    #[tokio::test]
    async fn test_update_plugins_should_return_correct_plugins_if_there_is_an_update_2() {
        let plugin_service = setup_update_plugin_mocks("1.1.1", "1.1.12");
        let result = plugin_service.update_plugins().await;
        assert!(result.is_ok());

        let updated_plugins = result.unwrap();
        let expected_updated_plugins = BTreeMap::from([(
            String::from("test_plugin"),
            Plugin::new(
                String::from("1234"),
                String::from("Test Plugin"),
                String::from("1.1.12"),
                String::from("MIT"),
                vec![],
                true,
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

    // search_assets_by_name_or_version

    #[tokio::test]
    async fn test_search_assets_by_name_or_version_with_name_should_display_results() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service
            .search_assets_by_name_or_version("Godot Unit Testing", "")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_assets_by_name_or_version_with_no_results_should_display_empty() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service
            .search_assets_by_name_or_version("NonExistentPlugin12345", "")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_assets_by_name_or_version_with_empty_params_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service
            .search_assets_by_name_or_version("", "")
            .await;
        assert!(result.is_err());
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
            .expect_get_plugin_key_by_name()
            .with(eq("test_plugin"))
            .returning(|_name| Some("test_plugin".to_string()));
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
            .expect_remove_dir_all()
            .returning(|_path| Ok(()));

        let plugin_service = DefaultPluginService::new(
            Box::new(godot_config_repository),
            Arc::new(MockDefaultAssetStoreAPI::default()),
            Box::new(plugin_config_repository),
            DefaultAppConfig::default(),
            Arc::new(MockDefaultExtractService::default()),
            Arc::new(file_service),
        );

        let result = plugin_service.remove_plugin_by_name("test_plugin").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_plugin_by_name_with_nonexistent_plugin_should_return_ok() {
        let mut godot_config_repository = MockDefaultGodotConfigRepository::default();
        godot_config_repository
            .expect_validate_project_file()
            .returning(|| anyhow::Result::Ok(()));

        let mut plugin_config_repository = MockDefaultPluginConfigRepository::default();
        plugin_config_repository
            .expect_get_plugin_key_by_name()
            .with(eq("nonexistent"))
            .returning(|_name| None);

        plugin_config_repository
            .expect_has_installed_plugins()
            .returning(|| Ok(true));

        let plugin_service = DefaultPluginService::new(
            Box::new(godot_config_repository),
            Arc::new(MockDefaultAssetStoreAPI::default()),
            Box::new(plugin_config_repository),
            DefaultAppConfig::default(),
            Arc::new(MockDefaultExtractService::default()),
            Arc::new(MockDefaultFileService::default()),
        );

        let result = plugin_service.remove_plugin_by_name("nonexistent").await;
        assert!(result.is_ok());
    }

    // add_plugin_by_id_or_name_and_version

    #[tokio::test]
    async fn test_add_plugin_by_id_or_name_and_version_with_asset_id_should_add_plugin() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service
            .add_plugin_by_id_or_name_and_version(Some("1234".to_string()), None, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_plugin_by_id_or_name_and_version_with_no_params_should_return_err() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service
            .add_plugin_by_id_or_name_and_version(None, None, None)
            .await;
        assert!(result.is_err());
    }

    // fetch_installed_assets

    #[tokio::test]
    async fn test_fetch_installed_assets_should_return_assets() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service.fetch_installed_assets().await;
        assert!(result.is_ok());
        let assets = result.unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].asset_id, "1234");
    }

    // fetch_latest_assets

    #[tokio::test]
    async fn test_fetch_latest_assets_should_return_assets() {
        let plugin_service = setup_plugin_service_mocks();
        let result = plugin_service.fetch_latest_assets().await;
        assert!(result.is_ok());
        let assets = result.unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].asset_id, "1234");
    }

    // check_outdated_plugins

    #[tokio::test]
    async fn test_check_outdated_plugins_with_outdated_plugin_should_display_message() {
        let plugin_service = setup_update_plugin_mocks("1.0.0", "1.1.0");
        let result = plugin_service.check_outdated_plugins().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_outdated_plugins_with_no_outdated_plugins_should_display_message() {
        let plugin_service = setup_update_plugin_mocks("1.1.0", "1.1.0");
        let result = plugin_service.check_outdated_plugins().await;
        assert!(result.is_ok());
    }

    // add_plugins

    #[test]
    fn test_add_plugins_should_add_to_config() {
        let mut godot_config_repository = MockDefaultGodotConfigRepository::default();
        godot_config_repository
            .expect_save()
            .returning(|_path| Ok(()));

        let mut plugin_config_repository = MockDefaultPluginConfigRepository::default();
        plugin_config_repository
            .expect_add_plugins()
            .returning(|plugins| Ok(DefaultPluginConfig::new(plugins.clone())));

        let plugin_service = DefaultPluginService::new(
            Box::new(godot_config_repository),
            Arc::new(MockDefaultAssetStoreAPI::default()),
            Box::new(plugin_config_repository),
            DefaultAppConfig::default(),
            Arc::new(MockDefaultExtractService::default()),
            Arc::new(MockDefaultFileService::default()),
        );

        let plugins = BTreeMap::from([(
            "test_plugin".to_string(),
            Plugin::new(
                "1234".to_string(),
                "Test Plugin".to_string(),
                "1.0.0".to_string(),
                "MIT".to_string(),
                vec![],
                true,
            ),
        )]);

        let result = plugin_service.add_plugins(&plugins);
        assert!(result.is_ok());
    }

    // finish_plugins_operation

    #[test]
    fn test_finish_plugins_operation_should_complete_successfully() {
        let mut godot_config_repository = MockDefaultGodotConfigRepository::default();
        godot_config_repository
            .expect_save()
            .returning(|_path| Ok(()));

        let mut plugin_config_repository = MockDefaultPluginConfigRepository::default();
        plugin_config_repository
            .expect_add_plugins()
            .returning(|plugins| Ok(DefaultPluginConfig::new(plugins.clone())));

        let plugin_service = DefaultPluginService::new(
            Box::new(godot_config_repository),
            Arc::new(MockDefaultAssetStoreAPI::default()),
            Box::new(plugin_config_repository),
            DefaultAppConfig::default(),
            Arc::new(MockDefaultExtractService::default()),
            Arc::new(MockDefaultFileService::default()),
        );

        let plugins = BTreeMap::from([(
            "test_plugin".to_string(),
            Plugin::new(
                "1234".to_string(),
                "Test Plugin".to_string(),
                "1.0.0".to_string(),
                "MIT".to_string(),
                vec![],
                true,
            ),
        )]);

        let result = plugin_service.finish_plugins_operation(&plugins);
        assert!(result.is_ok());
    }
}
