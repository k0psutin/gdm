mod setup;

mod update_command_tests {
    use crate::setup;

    use predicates::prelude::*;

    #[test]
    fn test_update_command_help() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("update")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("update"));
    }

    #[test]
    fn test_update_without_project_godot_should_fail() {
        let (mut cmd, _temp_dir) = setup::get_bin();
        cmd.arg("update")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "No project.godot file found in the current directory",
            ));
    }

    #[test]
    fn test_update_without_gdm_toml_should_fail() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();

        cmd.arg("update")
            .assert()
            .failure()
            .stderr(predicate::str::contains("No dependencies installed."));
    }

    #[test]
    fn test_update_with_empty_gdm_toml_should_fail() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        setup::create_gdm_toml(&_temp_dir, setup::EMPTY_GDM_TOML);

        cmd.arg("update")
            .assert()
            .failure()
            .stderr(predicate::str::contains("No dependencies installed."));
    }

    #[test]
    fn test_update_no_arguments_accepted() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("update")
            .arg("extra-arg")
            .assert()
            .failure()
            .stderr(predicate::str::contains("unexpected argument"));
    }

    #[test]
    fn test_update_with_actual_update() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        setup::create_gdm_toml(&_temp_dir, setup::GDM_TOML_WITH_LICENSE_MANAGER_OLD);

        cmd.arg("update")
            .timeout(std::time::Duration::from_secs(120))
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Dependencies updated successfully.",
            ));

        // Verify gdm.toml was updated to a newer version
        let gdm_content = std::fs::read_to_string(_temp_dir.path().join("gdm.toml"))
            .expect("Failed to read gdm.toml");
        assert!(
            gdm_content.contains("\"1.11"),
            "gdm.toml should contain an updated version (1.11.x)"
        );
    }

    #[test]
    fn test_update_with_all_up_to_date() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        setup::create_gdm_toml(&_temp_dir, setup::GDM_TOML_WITH_ONE_DEPENDENCY);

        cmd.arg("update")
            .timeout(std::time::Duration::from_secs(60))
            .assert()
            .success()
            .stdout(predicate::str::contains("All dependencies are up to date."));
    }
}
