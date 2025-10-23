mod common;

use assert_cmd::Command;
use predicates::prelude::*;

fn get_bin() -> Command {
    common::get_bin()
}

#[test]
fn test_cli_no_args_shows_help() {
    let mut cmd = get_bin();
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Usage:"));
}

#[test]
fn test_cli_help_flag() {
    let mut cmd = get_bin();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "A CLI tool to manage Godot addons",
        ))
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_cli_version_flag() {
    let mut cmd = get_bin();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("gdm"));
}

#[test]
fn test_invalid_command() {
    let mut cmd = get_bin();
    cmd.arg("invalid-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_verbosity_flag_short() {
    let mut cmd = get_bin();
    cmd.arg("-v").arg("--help").assert().success();
}

#[test]
fn test_verbosity_flag_long() {
    let mut cmd = get_bin();
    cmd.arg("--verbose").arg("--help").assert().success();
}

#[test]
fn test_quiet_flag_short() {
    let mut cmd = get_bin();
    cmd.arg("-q").arg("--help").assert().success();
}

#[test]
fn test_quiet_flag_long() {
    let mut cmd = get_bin();
    cmd.arg("--quiet").arg("--help").assert().success();
}

#[test]
fn test_all_subcommands_listed_in_help() {
    let mut cmd = get_bin();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("install"))
        .stdout(predicate::str::contains("outdated"))
        .stdout(predicate::str::contains("remove"))
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("update"));
}
