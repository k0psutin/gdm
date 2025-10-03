mod setup;

mod search_command_tests {
    use crate::setup;

    use predicates::prelude::*;

    #[test]
    fn test_search_command_help() {
        let (mut cmd, _temp_dir) = setup::get_bin();
        cmd.arg("search")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Search for plugins by name"))
            .stdout(predicate::str::contains("NAME"));
    }

    #[test]
    fn test_search_command_requires_name() {
        let (mut cmd, _temp_dir) = setup::get_bin();
        cmd.arg("search")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "required arguments were not provided",
            ));
    }

    #[test]
    fn test_search_with_exact_plugin_name_single_result() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();

        cmd.arg("search")
            .arg("Godot Unit Testing")
            .arg("-vvvv")
            .timeout(std::time::Duration::from_secs(30))
            .assert()
            .success()
            .stdout(predicate::str::contains("Found 1 asset matching"))
            .stdout(predicate::str::contains(
                "gdm add \"GUT - Godot Unit Testing",
            ));
    }

    #[test]
    fn test_search_with_partial_name_multiple_results() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();

        cmd.arg("search")
            .arg("godot")
            .timeout(std::time::Duration::from_secs(30))
            .assert()
            .success()
            .stdout(
                predicate::str::contains("Found").and(predicate::str::contains("assets matching")),
            )
            .stdout(predicate::str::contains("gdm add --asset-id"));
    }

    #[test]
    fn test_search_with_nonexistent_plugin() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("search")
            .arg("ThisPluginDefinitelyDoesNotExist12345XYZ")
            .timeout(std::time::Duration::from_secs(30))
            .assert()
            .success()
            .stdout(predicate::str::contains("No assets found matching"));
    }

    #[test]
    fn test_search_with_godot_version_flag() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("search")
            .arg("Godot Unit Testing")
            .arg("--godot-version")
            .arg("4.3")
            .timeout(std::time::Duration::from_secs(30))
            .assert()
            .success();
    }

    #[test]
    fn test_search_missing_godot_version_value() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("search")
            .arg("Godot Unit Testing")
            .arg("--godot-version")
            .assert()
            .failure()
            .stderr(predicate::str::contains("a value is required"));
    }

    #[test]
    fn test_search_with_empty_string_fails() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("search")
            .arg("")
            .timeout(std::time::Duration::from_secs(30))
            .assert()
            .failure();
    }

    #[test]
    fn test_search_output_shows_asset_info() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("search")
            .arg("Godot Unit Testing")
            .timeout(std::time::Duration::from_secs(30))
            .assert()
            .success()
            .stdout(predicate::str::contains("Godot Unit Testing"));
    }
}
