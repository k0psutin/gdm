#![allow(dead_code)]

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

/// Helper function to create a test environment with a temporary directory
pub fn setup_test_env() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

/// Helper function to get the path to the binary under test
/// Sets up required environment variables for the application
pub fn get_bin() -> Command {
    let mut cmd = Command::cargo_bin("gdm").expect("Failed to find binary");

    // Set required environment variables
    cmd.env("API_BASE_URL", "https://godotengine.org/asset-library/api")
        .env("CONFIG_FILE_PATH", "gdm.json")
        .env("CACHE_FOLDER_PATH", ".gdm")
        .env("GODOT_PROJECT_FILE_PATH", "project.godot")
        .env("ADDON_FOLDER_PATH", "addons");

    cmd
}

/// Helper function to create a minimal project.godot file
pub fn create_project_godot(dir: &TempDir, content: &str) {
    let project_path = dir.path().join("project.godot");
    fs::write(project_path, content).expect("Failed to write project.godot");
}

/// Helper function to create a gdm.json file
pub fn create_gdm_json(dir: &TempDir, content: &str) {
    let gdm_path = dir.path().join("gdm.json");
    fs::write(gdm_path, content).expect("Failed to write gdm.json");
}

/// Standard minimal project.godot content
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
