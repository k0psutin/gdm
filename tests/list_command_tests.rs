mod setup;

mod list_command_tests {
    use crate::setup;
    use predicates::prelude::*;

    const GDM_TOML_WITH_SORTED_DEPENDENCIES: &str = r#"[dependencies."z-key"]
title = "Zulu Dependency"
version = "v1.35.3"
sub_assets = ["netfox.internals"]

[dependencies."z-key".source]
publisher_slug = "foxssake"
asset_slug = "netfox"

[dependencies."a-key"]
title = "Alpha Dependency"
version = "1.11.2"
sub_assets = []

[dependencies."a-key".source]
publisher_slug = "kenyoni"
asset_slug = "license-manager"
"#;

    const GDM_TOML_WITH_GIT_DEPENDENCIES: &str = r#"[dependencies."https-plugin"]
title = "HTTPS Plugin"
version = ""
sub_assets = []

[dependencies."https-plugin".source]
url = "https://github.com/foo/bar.git"
reference = "main"

[dependencies."http-plugin"]
title = "HTTP Plugin"
version = ""
sub_assets = []

[dependencies."http-plugin".source]
url = "http://gitlab.com/foo/bar/"
reference = "v2.0.0"

[dependencies."ssh-plugin"]
title = "SSH Plugin"
version = ""
sub_assets = []

[dependencies."ssh-plugin".source]
url = "git@github.com:foo/bar.git"
reference = "a83f10c"
"#;

    const GDM_TOML_WITH_EMPTY_TITLE: &str = r#"[dependencies.netfox]
title = ""
version = "v1.35.3"
sub_assets = []

[dependencies.netfox.source]
publisher_slug = "foxssake"
asset_slug = "netfox"
"#;

    #[test]
    fn test_list_command_help() {
        let (mut cmd, _temp_dir) = setup::get_bin();
        cmd.arg("list")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("List dependencies"));
    }

    #[test]
    fn test_list_command_rejects_arguments() {
        let (mut cmd, _temp_dir) = setup::get_bin();
        cmd.arg("list")
            .arg("extra-arg")
            .assert()
            .failure()
            .stderr(predicate::str::contains("unexpected argument"));
    }

    #[test]
    fn test_list_without_project_godot_lists_dependencies() {
        let (mut cmd, temp_dir) = setup::get_bin();
        setup::create_gdm_toml(&temp_dir, setup::GDM_TOML_WITH_ONE_DEPENDENCY);

        cmd.arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("Dependency"))
            .stdout(predicate::str::contains("licenses"))
            .stdout(predicate::str::contains("License Manager").not())
            .stdout(predicate::str::contains("kenyoni/license-manager"))
            .stdout(predicate::str::contains(
                "To remove a dependency, use: gdm remove <dependency>",
            ))
            .stdout(predicate::str::contains("Key").not())
            .stderr(predicate::str::contains("project.godot").not());
    }

    #[test]
    fn test_list_without_manifest_is_an_empty_success() {
        let (mut cmd, _temp_dir) = setup::get_bin();

        cmd.arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("No dependencies found."));
    }

    #[test]
    fn test_list_ignores_legacy_json_manifest() {
        let (mut cmd, temp_dir) = setup::get_bin();
        std::fs::write(
            temp_dir.child("gdm.json"),
            r#"{"plugins":{"legacy":{"title":"Legacy"}}}"#,
        )
        .unwrap();

        cmd.arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("No dependencies found."))
            .stdout(predicate::str::contains("Legacy").not());
    }

    #[test]
    fn test_list_with_empty_manifest_is_an_empty_success() {
        let (mut cmd, temp_dir) = setup::get_bin();
        setup::create_gdm_toml(&temp_dir, setup::EMPTY_GDM_TOML);

        cmd.arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("No dependencies found."));
    }

    #[test]
    fn test_list_with_empty_manifest_file_is_an_empty_success() {
        let (mut cmd, temp_dir) = setup::get_bin();
        setup::create_gdm_toml(&temp_dir, "");

        cmd.arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("No dependencies found."));
    }

    #[test]
    fn test_list_sorts_by_manifest_key_and_does_not_expand_sub_assets() {
        let (mut cmd, temp_dir) = setup::get_bin();
        setup::create_gdm_toml(&temp_dir, GDM_TOML_WITH_SORTED_DEPENDENCIES);

        let output = cmd.arg("list").output().unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();

        assert_eq!(stdout.lines().next(), Some("Dependency  Version  Source"));
        assert!(stdout.find("a-key").unwrap() < stdout.find("z-key").unwrap());
        assert!(!stdout.contains("Alpha Dependency"));
        assert!(!stdout.contains("Zulu Dependency"));
        assert!(!stdout.contains("netfox.internals"));
        assert!(stdout.contains("To remove a dependency, use: gdm remove <dependency>"));
    }

    #[test]
    fn test_list_renders_git_reference_and_normalized_sources() {
        let (mut cmd, temp_dir) = setup::get_bin();
        setup::create_gdm_toml(&temp_dir, GDM_TOML_WITH_GIT_DEPENDENCIES);

        cmd.arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("main"))
            .stdout(predicate::str::contains("github.com/foo/bar"))
            .stdout(predicate::str::contains("v2.0.0"))
            .stdout(predicate::str::contains("gitlab.com/foo/bar"))
            .stdout(predicate::str::contains("a83f10c"))
            .stdout(predicate::str::contains("git@github.com:foo/bar"));
    }

    #[test]
    fn test_list_uses_manifest_key_instead_of_title() {
        let (mut cmd, temp_dir) = setup::get_bin();
        setup::create_gdm_toml(&temp_dir, GDM_TOML_WITH_EMPTY_TITLE);

        cmd.arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("netfox"))
            .stdout(predicate::str::contains("foxssake/netfox"));
    }

    #[test]
    fn test_list_with_malformed_manifest_fails() {
        let (mut cmd, temp_dir) = setup::get_bin();
        setup::create_gdm_toml(&temp_dir, "[dependencies\n");

        cmd.arg("list")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "Failed to parse dependency manifest",
            ));
    }
}
