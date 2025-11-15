mod setup;

mod install_command_tests {
    use crate::setup;
    use predicates::prelude::*;

    #[test]
    fn test_install_command_help() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("install")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("install"));
    }

    #[test]
    fn test_install_without_godot_project() {
        let (mut cmd, _temp_dir) = setup::get_bin();

        cmd.arg("install")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "No project.godot file found in the current directory",
            ));
    }

    #[test]
    fn test_install_without_gdm_json_should_fail() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();

        cmd.arg("install")
            .assert()
            .failure()
            .stderr(predicate::str::contains("No plugins installed.\n"));
    }

    #[test]
    fn test_install_with_empty_gdm_json_should_fail() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();

        let output = cmd.arg("install").output().expect("Failed to run command");

        assert!(!output.status.success());

        let gdm_json_path = _temp_dir.path().join("gdm.json");
        assert!(!gdm_json_path.exists(), "gdm.json should not be created");
    }

    #[test]
    fn test_install_plugins() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        setup::create_gdm_json(&_temp_dir, setup::GDM_JSON_WITH_ONE_PLUGIN);

        let output = cmd.arg("install").output().expect("Failed to run command");

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
    fn test_install_no_arguments_accepted() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("install")
            .arg("extra-arg")
            .assert()
            .failure()
            .stderr(predicate::str::contains("unexpected argument"));
    }
}
