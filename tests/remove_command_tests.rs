mod setup;

mod remove_command_tests {
    use std::path::Path;

    use crate::setup;

    use predicates::prelude::*;

    #[test]
    fn test_remove_command_help() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("remove")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("remove"));
    }

    #[test]
    fn test_remove_command_requires_name() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("remove")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "required arguments were not provided",
            ));
    }

    #[test]
    fn test_remove_without_project_godot() {
        let (mut cmd, _temp_dir) = setup::get_bin();
        cmd.arg("remove")
            .arg("gut")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "No project.godot file found in the current directory",
            ));
    }

    #[test]
    fn test_remove_without_gdm_json_should_fail() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("remove")
            .arg("gut")
            .assert()
            .failure()
            .stderr(predicate::str::contains("No plugins installed."));
    }

    #[test]
    fn test_remove_with_empty_gdm_json_should_fail() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        setup::create_gdm_json(&_temp_dir, setup::EMPTY_GDM_JSON);
        cmd.arg("remove")
            .arg("gut")
            .assert()
            .failure()
            .stderr(predicate::str::contains("No plugins installed."));
    }

    #[test]
    fn test_remove_should_remove_from_plugin_config() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        setup::create_gdm_json(&_temp_dir, setup::GDM_JSON_WITH_ONE_PLUGIN);

        cmd.arg("remove")
            .arg("gut")
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Plugin folder does not exist, trying to remove from gdm config",
            ))
            .stdout(predicate::str::contains("Plugin gut removed successfully."));
    }

    #[test]
    fn test_remove_should_remove_folder() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        setup::create_gdm_json(&_temp_dir, setup::GDM_JSON_WITH_ONE_PLUGIN);
        let addons_path = _temp_dir.child("addons");
        let gut_path = addons_path.join("gut");
        std::fs::create_dir(_temp_dir.child("addons")).unwrap();
        std::fs::create_dir(gut_path.clone()).unwrap();

        let expected_directory = Path::new("addons").join("gut");

        cmd.arg("remove")
            .arg("gut")
            .assert()
            .success()
            .stdout(predicate::str::contains(format!(
                "Removing plugin folder: {}",
                expected_directory.display()
            )))
            .stdout(predicate::str::contains("Plugin gut removed successfully."));
    }

    #[test]
    fn test_remove_should_remove_all_sub_asset_folders() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("add")
            .arg("Godot Mod Loader")
            .arg("--version")
            .arg("7.0.1")
            .assert()
            .success();

        let addons_path = &_temp_dir.child("addons");
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

        setup::get_cmd(&_temp_dir)
            .arg("remove")
            .arg("mod_loader")
            .assert()
            .success();

        assert!(
            !mod_loader_path.try_exists().unwrap(),
            "Plugin folder should be removed"
        );
        assert!(
            !sub_asset_path.try_exists().unwrap(),
            "Sub-asset folder should be removed"
        );
    }
}
