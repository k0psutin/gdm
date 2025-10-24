mod setup;

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
fn test_update_without_gdm_json() {
    let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();

    cmd.arg("update")
        .assert()
        .success()
        .stdout(predicate::str::contains("No plugins installed."));
}

#[test]
fn test_update_with_empty_gdm_json() {
    let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();

    cmd.arg("update")
        .assert()
        .success()
        .stdout(predicate::str::contains("No plugins installed."));
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
