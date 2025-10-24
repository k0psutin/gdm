mod setup;

use predicates::prelude::*;

#[test]
fn test_outdated_command_help() {
    let mut cmd = setup::get_bin();
    cmd.arg("outdated")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Show outdated plugins"));
}

#[test]
fn test_outdated_without_gdm_json() {
    let temp_dir = setup::setup_test_dir();
    let mut cmd = setup::get_bin();

    cmd.current_dir(temp_dir.path())
        .arg("outdated")
        .assert()
        .success()
        .stdout(predicate::str::contains("No plugins installed."));
}

#[test]
fn test_outdated_with_empty_gdm_json() {
    let temp_dir = setup::setup_test_dir();
    setup::create_project_godot(&temp_dir, setup::MINIMAL_PROJECT_GODOT);
    setup::create_gdm_json(&temp_dir, setup::EMPTY_GDM_JSON);

    let mut cmd = setup::get_bin();
    cmd.current_dir(temp_dir.path())
        .arg("outdated")
        .assert()
        .success()
        .stdout(predicate::str::contains("No plugins installed."));
}

#[test]
fn test_outdated_no_arguments_accepted() {
    let temp_dir = setup::setup_test_dir();
    let mut cmd = setup::get_bin();
    cmd.current_dir(temp_dir.path())
        .arg("outdated")
        .arg("extra-arg")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}
