mod common;

use assert_cmd::Command;
use predicates::prelude::*;

fn get_bin() -> Command {
    common::get_bin()
}

#[test]
fn test_search_command_help() {
    let mut cmd = get_bin();
    cmd.arg("search")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Search for plugins by name"))
        .stdout(predicate::str::contains("NAME"));
}

#[test]
fn test_search_command_requires_name() {
    let mut cmd = get_bin();
    cmd.arg("search")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ));
}

#[test]
fn test_search_with_exact_plugin_name_single_result() {
    let mut cmd = get_bin();
    cmd.arg("search")
        .arg("Godot Unit Testing")
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 1 asset matching"))
        .stdout(predicate::str::contains(
            "gdm add \"GUT - Godot Unit Testing",
        ));

    // When searching for an exact plugin name that returns 1 result,
    // it should show the add command with the full plugin title
}

#[test]
fn test_search_with_partial_name_multiple_results() {
    let mut cmd = get_bin();
    cmd.arg("search")
        .arg("godot")
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("Found").and(predicate::str::contains("assets matching")))
        .stdout(predicate::str::contains("gdm add --asset-id"));

    // When searching with a partial name that returns multiple results,
    // it should suggest using asset ID or narrowing the search
}

#[test]
fn test_search_with_nonexistent_plugin() {
    let mut cmd = get_bin();
    cmd.arg("search")
        .arg("ThisPluginDefinitelyDoesNotExist12345XYZ")
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("No assets found matching"));

    // Should return success but show "No assets found"
}

#[test]
fn test_search_with_godot_version_flag() {
    let mut cmd = get_bin();
    cmd.arg("search")
        .arg("Godot Unit Testing")
        .arg("--godot-version")
        .arg("4.3")
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();

    // This verifies the --godot-version flag is accepted
}

#[test]
fn test_search_missing_godot_version_value() {
    let mut cmd = get_bin();
    cmd.arg("search")
        .arg("Godot Unit Testing")
        .arg("--godot-version")
        .assert()
        .failure()
        .stderr(predicate::str::contains("a value is required"));
}

#[test]
fn test_search_with_empty_string_fails() {
    let mut cmd = get_bin();
    cmd.arg("search")
        .arg("")
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .failure();

    // Empty search string should fail
}

#[test]
fn test_search_output_shows_asset_info() {
    let mut cmd = get_bin();
    cmd.arg("search")
        .arg("Godot Unit Testing")
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("Godot Unit Testing"));

    // Should show the plugin name in the results
}
