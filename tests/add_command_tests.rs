mod setup;

mod add_command_tests {
    use crate::setup;
    use predicates::prelude::*;
    use serde_json::json;

    #[test]
    fn test_add_command_help() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("add")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Add a plugin"))
            .stdout(predicate::str::contains("NAME"));
    }

    #[test]
    fn test_add_command_should_return_err_requires_name_or_asset_id() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("add")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "Either name, asset_id, version OR git URL/reference must be provided.",
            ));
    }

    #[test]
    fn test_add_command_should_return_err_if_no_project_godot_file() {
        let (mut cmd, _temp_dir) = setup::get_bin();
        cmd.arg("add")
            .arg("Godot Unit Testing")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "No project.godot file found in the current directory",
            ));
    }

    #[test]
    fn test_add_with_plugin_name_without_gdm_json_should_not_fail() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        let output = cmd
            .arg("add")
            .arg("Godot Unit Testing")
            .output()
            .expect("Failed to run command");

        assert!(output.status.success());

        let gdm_json_path = _temp_dir.path().join("gdm.json");
        assert!(gdm_json_path.exists(), "gdm.json should be created");

        let gdm_content = std::fs::read_to_string(&gdm_json_path).expect("Failed to read gdm.json");
        assert!(
            gdm_content.contains("GUT - Godot Unit Testing (Godot 4)"),
            "gdm.json should contain the installed plugin"
        );
        assert!(
            gdm_content.contains("\"asset_id\": \"1709\""),
            "gdm.json should contain the correct asset_id"
        );

        let addons_path = _temp_dir.path().join("addons").join("gut");
        assert!(
            addons_path.exists(),
            "Plugin should be extracted to addons/gut folder"
        );
    }

    #[test]
    fn test_add_with_version_flag_without_gdm_json() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();

        let output = cmd
            .arg("add")
            .arg("Godot Unit Testing")
            .arg("--version")
            .arg("9.1.0")
            .output()
            .expect("Failed to run command");

        assert!(output.status.success());

        let gdm_json_path = _temp_dir.path().join("gdm.json");
        assert!(gdm_json_path.exists(), "gdm.json should be created");

        let gdm_content = std::fs::read_to_string(&gdm_json_path).expect("Failed to read gdm.json");
        assert!(
            gdm_content.contains("GUT - Godot Unit Testing (Godot 4)"),
            "gdm.json should contain the installed plugin"
        );
        assert!(
            gdm_content.contains("\"asset_id\": \"1709\""),
            "gdm.json should contain the correct asset_id"
        );

        assert!(
            gdm_content.contains("\"version\": \"9.1.0\""),
            "gdm.json should contain the correct version"
        );

        let addons_path = _temp_dir.path().join("addons").join("gut");
        assert!(
            addons_path.exists(),
            "Plugin should be extracted to addons/gut folder"
        );
    }

    #[test]
    fn test_add_plugin_with_sub_assets_should_show_correct_gdm_json() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("add")
            .arg("Godot Mod Loader")
            .arg("--version")
            .arg("7.0.1")
            .assert()
            .success();

        let gdm_json_path = _temp_dir.path().join("gdm.json");
        assert!(gdm_json_path.exists(), "gdm.json should be created");

        let expected_gdm_json = json!({
                "plugins": {
        "mod_loader": {
          "source": {
            "asset_id": "4107"
          },
          "title": "Godot Mod Loader",
          "plugin_cfg_path": "addons/mod_loader/_export_plugin/plugin.cfg",
          "version": "7.0.1",
          "sub_assets": [
            "JSON_Schema_Validator"
          ],
          "license": "CC0"
        }
                }
            });

        let gdm_content = std::fs::read_to_string(&gdm_json_path).expect("Failed to read gdm.json");
        let gdm_json = serde_json::from_str::<serde_json::Value>(&gdm_content)
            .expect("Failed to parse gdm.json");
        assert_eq!(gdm_json, expected_gdm_json);

        let addons_path = _temp_dir.child("addons");
        let mod_loader_path = addons_path.join("mod_loader");
        let sub_asset_path = addons_path.join("JSON_Schema_Validator");
        assert!(
            mod_loader_path.try_exists().unwrap(),
            "Plugin folder should exists"
        );
        assert!(
            sub_asset_path.try_exists().unwrap(),
            "Sub-asset folder exists"
        );
    }

    #[test]
    fn test_add_plugin_with_sub_assets_should_show_not_add_files_to_addons_root_directory() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("add")
            .arg("Godot Mod Loader")
            .arg("--version")
            .arg("7.0.1")
            .assert()
            .success();

        let addons_path = _temp_dir.child("addons");
        // get all files in addons directory
        let mut files = vec![];
        let entries = std::fs::read_dir(&addons_path).expect("Failed to read addons directory");
        for entry in entries {
            let entry = entry.expect("Failed to get directory entry");
            if entry.path().is_file() {
                files.push(entry.path());
            }
        }
        assert!(
            files.is_empty(),
            "No files should be added to the addons root directory"
        );
    }

    #[test]
    fn test_add_with_bad_asset_id() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();

        cmd.arg("add")
            .arg("--asset-id")
            .arg("999999999")
            .assert()
            .failure()
            .stderr(predicates::str::contains(
                "No asset found with asset ID \'999999999\'\n",
            ));
    }

    #[test]
    fn test_add_with_asset_id_and_version() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();

        cmd.arg("add")
            .arg("--asset-id")
            .arg("999999999")
            .arg("--version")
            .arg("999.999.999")
            .assert()
            .failure()
            .stderr(predicates::str::contains(
                "Failed to find plugin with asset ID \'999999999\' and version \'999.999.999\'\n",
            ));
    }

    #[test]
    fn test_add_without_project_godot_fails() {
        let (mut cmd, _temp_dir) = setup::get_bin();

        cmd.arg("add").arg("Godot Unit Testing").assert().failure();
    }

    #[test]
    fn test_add_with_nonexistent_plugin_name_fails() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();

        cmd.arg("add")
            .arg("This Plugin Definitely Does Not Exist 12345")
            .assert()
            .failure()
            .stderr(predicates::str::contains(
                "No asset found with name \'This Plugin Definitely Does Not Exist 12345\'\n",
            ));
    }

    #[test]
    fn test_add_with_invalid_asset_id_fails() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();

        cmd.arg("add")
            .arg("--asset-id")
            .arg("999999999")
            .assert()
            .failure()
            .stderr(predicates::str::contains(
                "No asset found with asset ID \'999999999\'\n",
            ));
    }

    #[test]
    fn test_add_with_invalid_version_fails() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();

        cmd.arg("add")
        .arg("Godot Unit Testing")
        .arg("--version")
        .arg("999.999.999")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "Failed to find plugin with name \'Godot Unit Testing\' and version \'999.999.999\'\n",
        ));
    }

    #[test]
    fn test_add_with_both_name_and_asset_id() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();

        cmd.arg("add")
            .arg("Godot Unit Testing")
            .arg("--asset-id")
            .arg("67845")
            .assert()
            .failure()
            .stderr(predicates::str::contains(
                "Cannot specify both name and asset ID",
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
    fn test_add_missing_asset_id_value() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("add")
            .arg("--asset-id")
            .assert()
            .failure()
            .stderr(predicate::str::contains("a value is required"));
    }
}
