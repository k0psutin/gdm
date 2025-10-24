mod setup;

use assert_cmd::Command;
use predicates::prelude::*;

fn get_bin() -> Command {
    setup::get_bin()
}

#[test]
fn test_install_command_help() {
    let mut cmd = get_bin();
    cmd.arg("install")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("install"));
}

#[test]
fn test_install_without_godot_project() {
    let temp_dir = setup::setup_test_dir();
    let mut cmd = get_bin();

    cmd.current_dir(temp_dir.path())
        .arg("install")
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "Godot project file not found at root",
        ));
}

#[test]
fn test_install_without_gdm_json() {
    let temp_dir = setup::setup_test_dir();
    setup::create_project_godot(&temp_dir, setup::MINIMAL_PROJECT_GODOT);
    let mut cmd = get_bin();

    cmd.current_dir(temp_dir.path())
        .arg("install")
        .assert()
        .success()
        .stdout(predicate::str::contains("No plugins installed."));
}

#[test]
fn test_install_with_empty_gdm_json() {
    let temp_dir = setup::setup_test_dir();
    setup::create_project_godot(&temp_dir, setup::MINIMAL_PROJECT_GODOT);
    setup::create_gdm_json(&temp_dir, setup::EMPTY_GDM_JSON);
    let mut cmd = get_bin();

    cmd.current_dir(temp_dir.path())
        .arg("install")
        .assert()
        .success()
        .stdout(predicate::str::contains("No plugins installed."));
}

#[test]
fn test_install_no_arguments_accepted() {
    let mut cmd = get_bin();
    cmd.arg("install")
        .arg("extra-arg")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}
