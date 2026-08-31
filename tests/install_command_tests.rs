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
    fn test_install_without_gdm_toml_should_fail() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("install")
            .assert()
            .failure()
            .stderr(predicate::str::contains("No dependencies installed.\n"));
    }

    #[test]
    fn test_install_with_empty_gdm_toml_should_fail() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        setup::create_gdm_toml(&_temp_dir, setup::EMPTY_GDM_TOML);
        cmd.arg("install")
            .assert()
            .failure()
            .stderr(predicate::str::contains("No dependencies installed."));
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
