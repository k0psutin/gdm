mod common;

use assert_cmd::Command;
use predicates::prelude::*;

fn get_bin() -> Command {
    common::get_bin()
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
    let temp_dir = common::setup_test_env();
    let mut cmd = get_bin();

    cmd.current_dir(temp_dir.path())
        .arg("install")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Error: Godot project file not found: project.godot",
        ));
}

#[test]
fn test_install_without_gdm_json() {
    let temp_dir = common::setup_test_env();
    common::create_project_godot(&temp_dir, common::MINIMAL_PROJECT_GODOT);
    let mut cmd = get_bin();

    cmd.current_dir(temp_dir.path())
        .arg("install")
        .assert()
        .success()
        .stdout(predicate::str::contains("No plugins installed."));
}

#[test]
fn test_install_with_empty_gdm_json() {
    let temp_dir = common::setup_test_env();
    common::create_project_godot(&temp_dir, common::MINIMAL_PROJECT_GODOT);
    common::create_gdm_json(&temp_dir, common::EMPTY_GDM_JSON);
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
