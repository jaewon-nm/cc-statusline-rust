//! End-to-end coverage for `preview --diff` and the color subsystem.
//! Pins `CCSTATUSLINE_RS_CONFIG` per test so we never touch real config.

use std::fs;

use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::TempDir;

fn bin(env_cfg: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("ccstatusline-rs").unwrap();
    cmd.env("CCSTATUSLINE_RS_CONFIG", env_cfg);
    // Tests must not be at the mercy of the developer's NO_COLOR / FORCE_COLOR.
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("FORCE_COLOR");
    cmd.env_remove("CLICOLOR_FORCE");
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

const PAYLOAD: &[u8] = include_bytes!("fixtures/default-payload.json");

fn write_payload(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("payload.json");
    fs::write(&path, PAYLOAD).unwrap();
    path
}

#[test]
fn preview_diff_reports_identical_when_candidate_equals_current() {
    let (tmp, cfg_path) = cfg_file();
    let payload = write_payload(tmp.path());
    let candidate = tmp.path().join("same.json");
    fs::write(
        &candidate,
        serde_json::to_vec(&serde_json::json!({
            "version": 1,
            "lines": [
                ["model", "cwd", "context_bar", "session_tokens", "session_cost"],
                ["block_timer", "weekly_timer"],
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let out = bin(&cfg_path)
        .args([
            "preview",
            "--payload",
            payload.to_str().unwrap(),
            "--config",
            candidate.to_str().unwrap(),
            "--diff",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&out);
    assert_eq!(v["identical"], true);
    assert_eq!(v["current"], v["pending"]);
}

#[test]
fn preview_diff_distinguishes_when_layouts_differ() {
    let (tmp, cfg_path) = cfg_file();
    let payload = write_payload(tmp.path());
    let candidate = tmp.path().join("different.json");
    fs::write(
        &candidate,
        serde_json::to_vec(&serde_json::json!({
            "version": 1,
            "lines": [["model"]]
        }))
        .unwrap(),
    )
    .unwrap();

    let out = bin(&cfg_path)
        .args([
            "preview",
            "--payload",
            payload.to_str().unwrap(),
            "--config",
            candidate.to_str().unwrap(),
            "--diff",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&out);
    assert_eq!(v["identical"], false);
    assert_ne!(v["current"], v["pending"]);
    assert!(v["pending"].as_str().unwrap().contains("Opus 4.7"));
    assert!(!v["pending"].as_str().unwrap().contains("📊"));
}

#[test]
fn preview_diff_without_config_fails() {
    let (_tmp, cfg_path) = cfg_file();
    bin(&cfg_path)
        .args(["preview", "--diff"])
        .assert()
        .failure();
}

#[test]
fn config_color_persists_style() {
    let (_tmp, cfg_path) = cfg_file();
    let out = bin(&cfg_path)
        .args(["config", "color", "model", "--fg", "cyan", "--bold"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&out);
    assert_eq!(v["colors"]["model"]["fg"], "cyan");
    assert_eq!(v["colors"]["model"]["bold"], true);
}

#[test]
fn config_color_clear_removes_entry() {
    let (_tmp, cfg_path) = cfg_file();
    bin(&cfg_path)
        .args(["config", "color", "model", "--fg", "red"])
        .assert()
        .success();
    let out = bin(&cfg_path)
        .args(["config", "color", "model", "--clear"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&out);
    assert!(
        v["colors"]
            .as_object()
            .map(|o| o.is_empty())
            .unwrap_or(true)
    );
}

#[test]
fn config_color_rejects_invalid_color_string() {
    let (_tmp, cfg_path) = cfg_file();
    bin(&cfg_path)
        .args(["config", "color", "model", "--fg", "burnt-sienna"])
        .assert()
        .failure();
}

#[test]
fn config_color_rejects_unknown_widget_kind() {
    let (_tmp, cfg_path) = cfg_file();
    bin(&cfg_path)
        .args(["config", "color", "made_up", "--fg", "red"])
        .assert()
        .failure();
}

#[test]
fn renderer_emits_ansi_when_force_color_is_set() {
    let (tmp, cfg_path) = cfg_file();
    let payload = write_payload(tmp.path());

    bin(&cfg_path)
        .args(["config", "color", "model", "--fg", "red", "--bold"])
        .assert()
        .success();

    let out = bin(&cfg_path)
        .env("FORCE_COLOR", "1")
        .args(["preview", "--payload", payload.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    // ANSI SGR sequence with at least one parameter; \x1b[ ... m.
    assert!(text.contains("\x1b["));
    assert!(text.contains("Opus 4.7"));
}

#[test]
fn renderer_strips_color_when_no_color_is_set() {
    let (tmp, cfg_path) = cfg_file();
    let payload = write_payload(tmp.path());

    bin(&cfg_path)
        .args(["config", "color", "model", "--fg", "red"])
        .assert()
        .success();

    let out = bin(&cfg_path)
        .env("NO_COLOR", "1")
        .env("FORCE_COLOR", "1") // even when forced, NO_COLOR wins
        .args(["preview", "--payload", payload.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(!text.contains("\x1b["), "unexpected ANSI in: {text}");
}

#[test]
fn config_apply_rejects_invalid_color_in_file() {
    let (tmp, cfg_path) = cfg_file();
    let bad = tmp.path().join("bad-color.json");
    fs::write(
        &bad,
        serde_json::to_vec(&json!({
            "version": 1,
            "lines": [["model"]],
            "colors": { "model": { "fg": "neon-mauve" } }
        }))
        .unwrap(),
    )
    .unwrap();
    bin(&cfg_path)
        .args(["config", "apply", "--file", bad.to_str().unwrap()])
        .assert()
        .failure();
}
