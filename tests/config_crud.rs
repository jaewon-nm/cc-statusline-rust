//! End-to-end coverage for `config add / remove / apply / validate / show`.
//! Each test pins `CCSTATUSLINE_RS_CONFIG` at a tempdir so we never touch the
//! developer's real on-disk config and tests stay parallel-safe.

use std::fs;

use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::TempDir;

fn bin(env_cfg: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("ccstatusline-rs").unwrap();
    cmd.env("CCSTATUSLINE_RS_CONFIG", env_cfg);
    cmd
}

fn parse_stdout(out: &[u8]) -> Value {
    serde_json::from_slice(out).expect("subcommand stdout is JSON")
}

fn cfg_file() -> (TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.json");
    (tmp, path)
}

#[test]
fn show_returns_default_when_file_absent() {
    let (_tmp, path) = cfg_file();
    let out = bin(&path)
        .args(["config", "show"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&out);
    assert_eq!(v["version"], 1);
    assert_eq!(v["lines"].as_array().unwrap().len(), 2);
}

#[test]
fn add_then_show_reflects_persisted_change() {
    let (_tmp, path) = cfg_file();
    let added = bin(&path)
        .args(["config", "add", "git_branch"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let added_v = parse_stdout(&added);
    let last_line = added_v["lines"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(last_line.last().unwrap(), "git_branch");

    let shown = bin(&path)
        .args(["config", "show"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(parse_stdout(&shown), added_v);
}

#[test]
fn add_with_explicit_position_inserts_in_place() {
    let (_tmp, path) = cfg_file();
    let out = bin(&path)
        .args([
            "config",
            "add",
            "git_status",
            "--line",
            "0",
            "--position",
            "0",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&out);
    let first_line = v["lines"][0].as_array().unwrap();
    assert_eq!(first_line[0], "git_status");
    assert_eq!(first_line[1], "model"); // the original first widget
}

#[test]
fn add_with_line_equal_to_len_creates_new_line() {
    let (_tmp, path) = cfg_file();
    let out = bin(&path)
        .args(["config", "add", "git_branch", "--line", "2"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&out);
    let lines = v["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[2].as_array().unwrap()[0], "git_branch");
}

#[test]
fn remove_strips_widget_and_persists() {
    let (_tmp, path) = cfg_file();
    bin(&path)
        .args(["config", "add", "git_branch"])
        .assert()
        .success();
    let out = bin(&path)
        .args(["config", "remove", "--line", "1", "--position", "2"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&out);
    let line_1 = v["lines"][1].as_array().unwrap();
    assert!(!line_1.iter().any(|kind| kind == "git_branch"));
}

#[test]
fn apply_replaces_full_config_atomically() {
    let (_tmp, path) = cfg_file();
    let replacement = path.with_file_name("replacement.json");
    let new_cfg = json!({
        "version": 1,
        "lines": [["model", "cwd", "git_branch"]]
    });
    fs::write(&replacement, serde_json::to_vec(&new_cfg).unwrap()).unwrap();

    bin(&path)
        .args(["config", "apply", "--file", replacement.to_str().unwrap()])
        .assert()
        .success();

    let shown = bin(&path)
        .args(["config", "show"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&shown);
    let lines = v["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].as_array().unwrap()[2], "git_branch");
}

#[test]
fn validate_rejects_unknown_widget_kind() {
    let (_tmp, path) = cfg_file();
    let bad = path.with_file_name("bad.json");
    let bad_cfg = json!({
        "version": 1,
        "lines": [["nonsense_widget"]]
    });
    fs::write(&bad, serde_json::to_vec(&bad_cfg).unwrap()).unwrap();

    let out = bin(&path)
        .args(["config", "validate", "--file", bad.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&out);
    assert_eq!(v["ok"], false);
    let errs = v["errors"].as_array().unwrap();
    assert!(errs[0].as_str().unwrap().contains("nonsense_widget"));
}

#[test]
fn add_unknown_kind_fails_loudly() {
    let (_tmp, path) = cfg_file();
    bin(&path)
        .args(["config", "add", "made_up_kind"])
        .assert()
        .failure();
}

#[test]
fn apply_rejects_unsupported_version() {
    let (_tmp, path) = cfg_file();
    let future = path.with_file_name("future.json");
    let cfg = json!({
        "version": 999,
        "lines": [["model"]]
    });
    fs::write(&future, serde_json::to_vec(&cfg).unwrap()).unwrap();

    bin(&path)
        .args(["config", "apply", "--file", future.to_str().unwrap()])
        .assert()
        .failure();
}
