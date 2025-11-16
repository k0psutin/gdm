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
    cmd.env("API_BASE_URL", "https://godotengine.org/asset-library/api")
        .env("CONFIG_FILE_PATH", temp_dir.child("gdm.json"))
        .env("CACHE_FOLDER_PATH", temp_dir.child(".gdm"))
        .env("GODOT_PROJECT_FILE_PATH", temp_dir.child("project.godot"))
        .env("ADDON_FOLDER_PATH", temp_dir.child("addons"))
        .current_dir(temp_dir);
    cmd
}

pub fn get_bin() -> (Command, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut cmd = cargo::cargo_bin_cmd!(pkg_name!());

    cmd.env("API_BASE_URL", "https://godotengine.org/asset-library/api")
        .env("CONFIG_FILE_PATH", temp_dir.child("gdm.json"))
        .env("CACHE_FOLDER_PATH", temp_dir.child(".gdm"))
        .env("GODOT_PROJECT_FILE_PATH", temp_dir.child("project.godot"))
        .env("ADDON_FOLDER_PATH", temp_dir.child("addons"))
        .current_dir(&temp_dir);

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

pub fn create_gdm_json(dir: &TempDir, content: &str) {
    let gdm_path = dir.child("gdm.json");
    fs::write(gdm_path, content).expect("Failed to write gdm.json");
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
config/features=PackedStringArray("4.5")
"#;

pub const EMPTY_GDM_JSON: &str = r#"{
  "plugins": {}
}"#;

pub const GDM_JSON_WITH_ONE_PLUGIN: &str = r#"{
  "plugins": {
    "gut": {
      "asset_id": "1709",
      "title": "GUT - Godot Unit Testing (Godot 4)",
      "version": "9.1.0",
      "license": "MIT"
    }
  }
}"#;
