mod setup;

mod add_command_tests {
    use crate::setup;
    use predicates::prelude::*;

    #[test]
    fn test_add_command_help() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("add")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Add a dependency"))
            .stdout(predicate::str::contains("NAME"))
            .stdout(predicate::str::contains("asset slug"))
            .stdout(predicate::str::contains("asset ID").not());
    }

    #[test]
    fn test_add_command_should_return_err_requires_name() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("add")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "You must specify either a dependency --name, a --git URL, or both --publisher-slug and --asset-slug",
            ));
    }

    #[test]
    fn test_add_command_should_return_err_if_no_project_godot_file() {
        let (mut cmd, _temp_dir) = setup::get_bin();
        cmd.arg("add")
            .arg("License Manager")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "No project.godot file found in the current directory",
            ));
    }

    #[test]
    fn test_add_with_both_name_and_publisher_slug() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("add")
            .arg("License Manager")
            .arg("--publisher-slug")
            .arg("kenyoni")
            .assert()
            .failure()
            .stderr(predicates::str::contains(
                "Cannot use --git, --publisher-slug, or --asset-slug together with a name argument",
            ));
    }

    #[test]
    fn test_add_missing_version_value() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("add")
            .arg("plugin-name")
            .arg("--version")
            .assert()
            .failure()
            .stderr(predicate::str::contains("a value is required"));
    }

    #[test]
    fn test_add_with_nonexistent_plugin_name_fails() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("add")
            .arg("This Plugin Definitely Does Not Exist 12345")
            .assert()
            .failure()
            .stderr(predicates::str::contains(
                "No dependencies found with name: This Plugin Definitely Does Not Exist 12345",
            ));
    }

    #[test]
    fn test_add_with_invalid_version_fails() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("add")
            .arg("License Manager")
            .arg("--version")
            .arg("999.999.999")
            .assert()
            .failure()
            .stderr(predicates::str::contains(
                "No dependencies found with name: License Manager",
            ));
    }

    #[test]
    fn test_add_and_remove_plugin_by_publisher_and_asset_slug_with_pinned_version() {
        let (mut cmd, temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("add")
            .arg("--publisher-slug")
            .arg("foxssake")
            .arg("--asset-slug")
            .arg("netfox")
            .arg("--version")
            .arg("v1.35.3")
            .arg("--godot-version")
            .arg("4.5")
            .timeout(std::time::Duration::from_secs(120))
            .assert()
            .success();

        let config: toml::Value =
            toml::from_str(&std::fs::read_to_string(temp_dir.child("gdm.toml")).unwrap()).unwrap();
        let netfox = &config["dependencies"]["netfox"];
        assert_eq!(
            netfox["source"]["publisher_slug"].as_str(),
            Some("foxssake")
        );
        assert_eq!(netfox["source"]["asset_slug"].as_str(), Some("netfox"));
        assert_eq!(netfox["version"].as_str(), Some("v1.35.3"));
        assert_eq!(
            netfox["sub_assets"].as_array().unwrap(),
            &[toml::Value::String("netfox.internals".to_string())]
        );
        assert!(temp_dir.child("addons/netfox").is_dir());
        assert!(temp_dir.child("addons/netfox.internals").is_dir());

        let mut remove_cmd = setup::get_cmd(&temp_dir);
        remove_cmd.arg("remove").arg("netfox").assert().success();

        assert!(temp_dir.child("addons").is_dir());
        assert!(!temp_dir.child("addons/netfox").exists());
        assert!(!temp_dir.child("addons/netfox.internals").exists());

        let config: toml::Value =
            toml::from_str(&std::fs::read_to_string(temp_dir.child("gdm.toml")).unwrap()).unwrap();
        assert!(config["dependencies"].as_table().unwrap().is_empty());

        let project = std::fs::read_to_string(temp_dir.child("project.godot")).unwrap();
        assert!(!project.contains("[editor_plugins]"));
        assert!(!project.contains("res://addons/netfox"));
    }

    // Git tests

    #[test]
    fn test_add_plugin_with_git_flag_and_url() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("add")
            .arg("--git")
            .arg("https://github.com/bitwes/Gut")
            .timeout(std::time::Duration::from_secs(60))
            .assert()
            .success();

        let gdm_toml_path = _temp_dir.path().join("gdm.toml");
        assert!(gdm_toml_path.exists(), "gdm.toml should be created");

        let gdm_content = std::fs::read_to_string(&gdm_toml_path).expect("Failed to read gdm.toml");
        assert!(
            gdm_content.contains("Gut"),
            "gdm.toml should contain the dependency title"
        );
        assert!(
            gdm_content.contains("bitwes"),
            "gdm.toml should contain the publisher slug"
        );

        let addons_path = _temp_dir.child("addons");
        let gut_path = addons_path.join("gut");
        assert!(
            gut_path.try_exists().unwrap(),
            "Plugin folder should exists"
        );
    }
}
