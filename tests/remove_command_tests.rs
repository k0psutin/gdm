mod common;

use assert_cmd::Command;
use predicates::prelude::*;

fn get_bin() -> Command {
    common::get_bin()
}

#[test]
fn test_remove_command_help() {
    let mut cmd = get_bin();
    cmd.arg("remove")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("remove"));
}

#[test]
fn test_remove_command_requires_name() {
    let mut cmd = get_bin();
    cmd.arg("remove")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ));
}

#[test]
fn test_remove_without_project_godot() {
    let temp_dir = common::setup_test_env();
    let mut cmd = get_bin();

    cmd.current_dir(temp_dir.path())
        .arg("remove")
        .arg("gut")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Godot project file not found: project.godot",
        ));
}

#[test]
fn test_remove_with_empty_gdm_json() {
    let temp_dir = common::setup_test_env();
    common::create_project_godot(&temp_dir, common::MINIMAL_PROJECT_GODOT);
    common::create_gdm_json(&temp_dir, common::EMPTY_GDM_JSON);

    let mut cmd = get_bin();
    cmd.current_dir(temp_dir.path())
        .arg("remove")
        .arg("gut")
        .assert()
        .success()
        .stdout(predicate::str::contains("Plugin gut is not installed."));
}

#[test]
fn test_remove_should_remove_from_plugin_config() {
    let temp_dir = common::setup_test_env();
    common::create_project_godot(&temp_dir, common::MINIMAL_PROJECT_GODOT);
    common::create_gdm_json(&temp_dir, common::GDM_JSON_WITH_ONE_PLUGIN);

    let mut cmd = get_bin();
    cmd.current_dir(temp_dir.path())
        .arg("remove")
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
    let temp_dir = common::setup_test_env();
    common::create_project_godot(&temp_dir, common::MINIMAL_PROJECT_GODOT);
    common::create_gdm_json(&temp_dir, common::GDM_JSON_WITH_ONE_PLUGIN);

    std::fs::create_dir_all(temp_dir.path().join("addons").join("gut")).unwrap();

    let mut cmd = get_bin();
    cmd.current_dir(temp_dir.path())
        .arg("remove")
        .arg("gut")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Removing plugin folder: addons/gut",
        ))
        .stdout(predicate::str::contains("Plugin gut removed successfully."));
}
