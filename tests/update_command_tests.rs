mod common;

use assert_cmd::Command;
use predicates::prelude::*;

fn get_bin() -> Command {
    common::get_bin()
}

#[test]
fn test_update_command_help() {
    let mut cmd = get_bin();
    cmd.arg("update")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("update"));
}

#[test]
fn test_update_without_gdm_json() {
    let temp_dir = common::setup_test_env();
    let mut cmd = get_bin();

    cmd.current_dir(temp_dir.path())
        .arg("update")
        .assert()
        .success()
        .stdout(predicate::str::contains("No plugins installed."));
}

#[test]
fn test_update_with_empty_gdm_json() {
    let temp_dir = common::setup_test_env();
    common::create_project_godot(&temp_dir, common::MINIMAL_PROJECT_GODOT);
    common::create_gdm_json(&temp_dir, common::EMPTY_GDM_JSON);

    let mut cmd = get_bin();
    cmd.current_dir(temp_dir.path())
        .arg("update")
        .assert()
        .success()
        .stdout(predicate::str::contains("No plugins installed."));
}

#[test]
fn test_update_no_arguments_accepted() {
    let mut cmd = get_bin();
    cmd.arg("update")
        .arg("extra-arg")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}
