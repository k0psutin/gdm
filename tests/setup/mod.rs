#![allow(dead_code)]

use assert_cmd::pkg_name;
use assert_cmd::{Command, cargo};
use std::fs;
use temp_dir::TempDir;

pub fn setup_test_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

pub fn get_cmd(temp_dir: &TempDir) -> Command {
    let mut cmd = cargo::cargo_bin_cmd!(pkg_name!());
    cmd.current_dir(temp_dir);
    cmd
}

pub fn get_bin() -> (Command, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut cmd = cargo::cargo_bin_cmd!(pkg_name!());

    cmd.current_dir(&temp_dir);

    (cmd, temp_dir)
}

pub fn get_bin_with_project_godot() -> (Command, TempDir) {
    let (cmd, temp_dir) = get_bin();
    create_project_godot(&temp_dir, MINIMAL_PROJECT_GODOT);
    (cmd, temp_dir)
}

pub fn create_project_godot(dir: &TempDir, content: &str) {
    let project_path = dir.child("project.godot");
    fs::write(project_path, content).expect("Failed to write project.godot");
}

pub fn create_gdm_toml(dir: &TempDir, content: &str) {
    let gdm_path = dir.child("gdm.toml");
    fs::write(gdm_path, content).expect("Failed to write gdm.toml");
}

pub const MINIMAL_PROJECT_GODOT: &str = r#"
; Engine configuration file.
; It's best edited using the editor UI and not directly,
; since the parameters that go here are not all obvious.
;
; Format:
;   [section] ; section goes between []
;   param=value ; assign values to parameters

config_version=5

[application]

config/name="Test Project"
config/features=PackedStringArray("4.6")
"#;

pub const EMPTY_GDM_TOML: &str = "[dependencies]\n";

pub const GDM_TOML_WITH_ONE_DEPENDENCY: &str = r#"[dependencies.licenses]
plugin_cfg_path = "addons/licenses/plugin.cfg"
title = "License Manager"
version = "1.11.2"
license = "MIT"
sub_assets = []

[dependencies.licenses.source]
publisher_slug = "kenyoni"
asset_slug = "license-manager"
"#;

pub const GDM_TOML_WITH_LICENSE_MANAGER_OLD: &str = r#"[dependencies.licenses]
plugin_cfg_path = "addons/licenses/plugin.cfg"
title = "License Manager"
version = "1.9.2"
license = "MIT"
sub_assets = []

[dependencies.licenses.source]
publisher_slug = "kenyoni"
asset_slug = "license-manager"
"#;

pub const GDM_TOML_WITH_NETFOX_OVERLAP: &str = r#"[dependencies.netfox]
plugin_cfg_path = "addons/netfox/plugin.cfg"
title = "netfox"
version = "v1.35.3"
sub_assets = ["netfox.internals"]
license = "MIT"

[dependencies.netfox.source]
publisher_slug = "foxssake"
asset_slug = "netfox"

[dependencies."netfox.extras"]
plugin_cfg_path = "addons/netfox.extras/plugin.cfg"
title = "netfox.extras"
version = "v1.35.3"
sub_assets = ["netfox.internals", "netfox"]
license = "MIT"

[dependencies."netfox.extras".source]
publisher_slug = "foxssake"
asset_slug = "netfox-extras"
"#;

pub const PROJECT_GODOT_WITH_NETFOX_PLUGINS: &str = r#"
config_version=5

[application]

config/name="Test Project"

[editor_plugins]

enabled=PackedStringArray("res://addons/netfox/plugin.cfg", "res://addons/netfox.extras/plugin.cfg")
"#;
