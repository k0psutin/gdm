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
            .stdout(predicate::str::contains("Search for dependencies by name"))
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
    fn test_search_with_nonexistent_plugin() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("search")
            .arg("ThisPluginDefinitelyDoesNotExist12345XYZ")
            .timeout(std::time::Duration::from_secs(30))
            .assert()
            .success()
            .stdout(predicate::str::contains("No dependencies found matching"));
    }

    #[test]
    fn test_search_with_empty_string_fails() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("search")
            .arg("")
            .timeout(std::time::Duration::from_secs(30))
            .assert()
            .failure()
            .stderr(predicate::str::contains("Search name cannot be empty"));
    }
}
