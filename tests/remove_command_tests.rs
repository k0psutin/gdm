mod setup;

use predicates::prelude::*;

#[test]
fn test_remove_command_help() {
    let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
    cmd.arg("remove")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("remove"));
}

#[test]
fn test_remove_command_requires_name() {
    let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
    cmd.arg("remove")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ));
}

#[test]
fn test_remove_without_project_godot() {
    let (mut cmd, _temp_dir) = setup::get_bin();

    cmd.arg("remove")
        .arg("gut")
        .assert()
        .failure()
        .stdout(predicate::str::contains("Godot project file not found"));
}

#[test]
fn test_remove_with_empty_gdm_json() {
    let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
    cmd.arg("remove")
        .arg("gut")
        .assert()
        .success()
        .stdout(predicate::str::contains("Plugin gut is not installed."));
}

#[test]
fn test_remove_should_remove_from_plugin_config() {
    let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
    setup::create_gdm_json(&_temp_dir, setup::GDM_JSON_WITH_ONE_PLUGIN);

    cmd.arg("remove")
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
    let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
    setup::create_gdm_json(&_temp_dir, setup::GDM_JSON_WITH_ONE_PLUGIN);
    let addons_path = _temp_dir.child("addons");
    let gut_path = addons_path.join("gut");
    std::fs::create_dir(_temp_dir.child("addons")).unwrap();
    std::fs::create_dir(gut_path.clone()).unwrap();

    let expected = format!(
        "Removing plugin folder: {}",
        gut_path.clone().into_os_string().display()
    );

    cmd.arg("remove")
        .arg("gut")
        .assert()
        .success()
        .stdout(predicate::str::contains(expected))
        .stdout(predicate::str::contains("Plugin gut removed successfully."));
}
