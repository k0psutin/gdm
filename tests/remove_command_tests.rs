mod setup;

mod remove_command_tests {
    use std::fs;
    use std::path::Path;

    use crate::setup;
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
            .arg("license-manager")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "No project.godot file found in the current directory",
            ));
    }

    #[test]
    fn test_remove_without_gdm_toml_should_fail() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        cmd.arg("remove")
            .arg("license-manager")
            .assert()
            .failure()
            .stderr(predicate::str::contains("No dependencies installed."));
    }

    #[test]
    fn test_remove_with_empty_gdm_toml_should_fail() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        setup::create_gdm_toml(&_temp_dir, setup::EMPTY_GDM_TOML);
        cmd.arg("remove")
            .arg("license-manager")
            .assert()
            .failure()
            .stderr(predicate::str::contains("No dependencies installed."));
    }

    #[test]
    fn test_remove_should_remove_from_plugin_config() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        setup::create_gdm_toml(&_temp_dir, setup::GDM_TOML_WITH_ONE_DEPENDENCY);

        cmd.arg("remove")
            .arg("licenses")
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Dependency folder does not exist, removing from config only.",
            ))
            .stdout(predicate::str::contains(
                "Dependency licenses removed successfully.",
            ));
    }

    #[test]
    fn test_remove_should_remove_folder() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        let gdm_toml = setup::GDM_TOML_WITH_ONE_DEPENDENCY
            .replace("sub_assets = []", "sub_assets = [\"license-assets\"]");
        setup::create_gdm_toml(&_temp_dir, &gdm_toml);
        let addons_path = _temp_dir.child("addons");
        let plugin_path = addons_path.join("licenses");
        let sub_asset_path = addons_path.join("license-assets");
        std::fs::create_dir(_temp_dir.child("addons")).unwrap();
        std::fs::create_dir(plugin_path.clone()).unwrap();
        std::fs::create_dir(sub_asset_path.clone()).unwrap();

        let expected_directory = Path::new("addons").join("licenses");

        cmd.arg("remove")
            .arg("licenses")
            .assert()
            .success()
            .stdout(predicate::str::contains(format!(
                "Removing dependency folder: {}",
                expected_directory.display()
            )))
            .stdout(predicate::str::contains(
                "Dependency licenses removed successfully.",
            ));

        assert!(
            addons_path.exists(),
            "the addons directory must be preserved"
        );
        assert!(
            !plugin_path.exists(),
            "only the plugin directory should be removed"
        );
        assert!(
            !sub_asset_path.exists(),
            "sub-asset directory should be removed"
        );

        let gdm_content = fs::read_to_string(_temp_dir.child("gdm.toml")).unwrap();
        assert!(!gdm_content.contains("[dependencies.licenses]"));
    }

    #[test]
    fn test_remove_nonexistent_plugin() {
        let (mut cmd, _temp_dir) = setup::get_bin_with_project_godot();
        setup::create_gdm_toml(&_temp_dir, setup::EMPTY_GDM_TOML);

        cmd.arg("remove")
            .arg("this-plugin-does-not-exist")
            .assert()
            .failure()
            .stderr(predicate::str::contains("No dependencies installed."));
    }

    #[test]
    fn test_remove_shared_extra_keeps_shared_folders_and_netfox_config() {
        let (mut cmd, temp_dir) = setup::get_bin();
        setup::create_project_godot(&temp_dir, setup::PROJECT_GODOT_WITH_NETFOX_PLUGINS);
        setup::create_gdm_toml(&temp_dir, setup::GDM_TOML_WITH_NETFOX_OVERLAP);

        let addons_path = temp_dir.child("addons");
        for folder in ["netfox", "netfox.extras", "netfox.internals"] {
            fs::create_dir_all(addons_path.join(folder)).unwrap();
        }

        cmd.arg("remove").arg("netfox.extras").assert().success();

        assert!(!addons_path.join("netfox.extras").exists());
        assert!(addons_path.join("netfox").exists());
        assert!(addons_path.join("netfox.internals").exists());
        assert!(addons_path.exists());

        let gdm_content = fs::read_to_string(temp_dir.child("gdm.toml")).unwrap();
        assert!(gdm_content.contains("[dependencies.netfox]"));
        assert!(!gdm_content.contains("[dependencies.\"netfox.extras\"]"));

        let project_content = fs::read_to_string(temp_dir.child("project.godot")).unwrap();
        assert!(project_content.contains("res://addons/netfox/plugin.cfg"));
        assert!(!project_content.contains("res://addons/netfox.extras/plugin.cfg"));
    }

    #[test]
    fn test_remove_netfox_keeps_shared_folders_and_extra_config() {
        let (mut cmd, temp_dir) = setup::get_bin();
        setup::create_project_godot(&temp_dir, setup::PROJECT_GODOT_WITH_NETFOX_PLUGINS);
        setup::create_gdm_toml(&temp_dir, setup::GDM_TOML_WITH_NETFOX_OVERLAP);

        let addons_path = temp_dir.child("addons");
        for folder in ["netfox", "netfox.extras", "netfox.internals"] {
            fs::create_dir_all(addons_path.join(folder)).unwrap();
        }

        cmd.arg("remove").arg("netfox").assert().success();

        assert!(addons_path.join("netfox.extras").exists());
        assert!(addons_path.join("netfox").exists());
        assert!(addons_path.join("netfox.internals").exists());
        assert!(addons_path.exists());

        let gdm_content = fs::read_to_string(temp_dir.child("gdm.toml")).unwrap();
        assert!(!gdm_content.contains("[dependencies.netfox]"));
        assert!(gdm_content.contains("[dependencies.\"netfox.extras\"]"));

        let project_content = fs::read_to_string(temp_dir.child("project.godot")).unwrap();
        assert!(!project_content.contains("res://addons/netfox/plugin.cfg"));
        assert!(project_content.contains("res://addons/netfox.extras/plugin.cfg"));
    }

    #[test]
    fn test_remove_overlap_plugins_cleans_shared_folders_after_last_owner() {
        let (mut cmd, temp_dir) = setup::get_bin();
        setup::create_project_godot(&temp_dir, setup::PROJECT_GODOT_WITH_NETFOX_PLUGINS);
        setup::create_gdm_toml(&temp_dir, setup::GDM_TOML_WITH_NETFOX_OVERLAP);

        let addons_path = temp_dir.child("addons");
        for folder in ["netfox", "netfox.extras", "netfox.internals"] {
            fs::create_dir_all(addons_path.join(folder)).unwrap();
        }

        cmd.arg("remove").arg("netfox.extras").assert().success();

        let mut cmd = setup::get_cmd(&temp_dir);
        cmd.arg("remove").arg("netfox").assert().success();

        assert!(addons_path.exists());
        assert!(!addons_path.join("netfox").exists());
        assert!(!addons_path.join("netfox.extras").exists());
        assert!(!addons_path.join("netfox.internals").exists());

        let gdm_content = fs::read_to_string(temp_dir.child("gdm.toml")).unwrap();
        assert!(!gdm_content.contains("\"netfox\""));
        assert!(!gdm_content.contains("\"netfox.extras\""));

        let project_content = fs::read_to_string(temp_dir.child("project.godot")).unwrap();
        assert!(!project_content.contains("[editor_plugins]"));
        assert!(!project_content.contains("res://addons/netfox"));
    }

    #[test]
    fn test_remove_rejects_path_traversal_without_deleting_addons() {
        let (mut cmd, temp_dir) = setup::get_bin();
        setup::create_project_godot(&temp_dir, setup::MINIMAL_PROJECT_GODOT);
        setup::create_gdm_toml(
            &temp_dir,
            r#"[dependencies."nested/.."]
plugin_cfg_path = "addons/safe/plugin.cfg"
title = "Safe"
version = "1.0.0"
sub_assets = []
license = "MIT"

[dependencies."nested/..".source]
publisher_slug = "publisher"
asset_slug = "asset"
"#,
        );

        let addons_path = temp_dir.child("addons");
        fs::create_dir_all(addons_path.join("nested")).unwrap();
        fs::create_dir_all(addons_path.join("safe")).unwrap();
        fs::write(addons_path.join("safe").join("sentinel.txt"), "keep").unwrap();

        cmd.arg("remove")
            .arg("nested/..")
            .assert()
            .failure()
            .stderr(predicate::str::contains("Invalid dependency folder name"));

        assert!(addons_path.exists());
        assert!(addons_path.join("safe").join("sentinel.txt").exists());
        let gdm_content = fs::read_to_string(temp_dir.child("gdm.toml")).unwrap();
        assert!(gdm_content.contains("[dependencies.\"nested/..\"]"));
    }

    #[test]
    fn test_remove_rejects_invalid_sub_asset_without_deleting_anything() {
        let (mut cmd, temp_dir) = setup::get_bin();
        setup::create_project_godot(&temp_dir, setup::MINIMAL_PROJECT_GODOT);
        setup::create_gdm_toml(
            &temp_dir,
            r#"[dependencies.safe]
plugin_cfg_path = "addons/safe/plugin.cfg"
title = "Safe"
version = "1.0.0"
sub_assets = ["../outside"]
license = "MIT"

[dependencies.safe.source]
publisher_slug = "publisher"
asset_slug = "asset"
"#,
        );

        let addons_path = temp_dir.child("addons");
        fs::create_dir_all(addons_path.join("safe")).unwrap();
        fs::write(addons_path.join("safe").join("sentinel.txt"), "keep").unwrap();

        cmd.arg("remove")
            .arg("safe")
            .assert()
            .failure()
            .stderr(predicate::str::contains("Invalid dependency folder name"));

        assert!(addons_path.join("safe").join("sentinel.txt").exists());
        let gdm_content = fs::read_to_string(temp_dir.child("gdm.toml")).unwrap();
        assert!(gdm_content.contains("[dependencies.safe]"));
    }
}
