mod setup;

mod outdated_command_tests {
    use crate::setup;

    use predicates::prelude::*;

    #[test]
    fn test_outdated_command_help() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("outdated")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Show outdated dependencies"));
    }

    #[test]
    fn test_outdated_without_project_godot_should_fail() {
        let (mut cmd, _temp_dir) = setup::get_bin();
        cmd.arg("outdated")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "No project.godot file found in the current directory",
            ));
    }

    #[test]
    fn test_outdated_without_gdm_toml_should_fail() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();

        cmd.arg("outdated")
            .assert()
            .failure()
            .stderr(predicate::str::contains("No dependencies installed."));
    }

    #[test]
    fn test_outdated_with_empty_gdm_toml_should_fail() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        setup::create_gdm_toml(&_temp_dir, setup::EMPTY_GDM_TOML);

        cmd.arg("outdated")
            .assert()
            .failure()
            .stderr(predicate::str::contains("No dependencies installed."));
    }

    #[test]
    fn test_outdated_no_arguments_accepted() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("outdated")
            .arg("extra-arg")
            .assert()
            .failure()
            .stderr(predicate::str::contains("unexpected argument"));
    }

    #[test]
    fn test_outdated_with_outdated_plugin() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        setup::create_gdm_toml(&_temp_dir, setup::GDM_TOML_WITH_LICENSE_MANAGER_OLD);

        cmd.arg("outdated")
            .timeout(std::time::Duration::from_secs(60))
            .assert()
            .success()
            .stdout(predicate::str::contains("License Manager"))
            .stdout(predicate::str::contains("(update available)"));
    }

    #[test]
    fn test_outdated_with_up_to_date_plugins() {
        let (mut _cmd, _temp_dir) = setup::get_bin_with_project_godot();
        setup::create_gdm_toml(&_temp_dir, setup::GDM_TOML_WITH_LICENSE_MANAGER_OLD);
        // Run outdated first to confirm it shows an update, then...
        // For up-to-date, use the latest version constant
        setup::create_gdm_toml(&_temp_dir, setup::GDM_TOML_WITH_ONE_DEPENDENCY);

        _cmd.arg("outdated")
            .timeout(std::time::Duration::from_secs(60))
            .assert()
            .success()
            .stdout(predicate::str::contains("All dependencies are up to date"));
    }
}
