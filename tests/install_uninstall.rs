//! End-to-end coverage for `install` / `uninstall`. Each test pins both
//! `--bin-dir` and `--settings` to a per-test `tempdir` so we never touch the
//! developer's real Claude Code configuration and tests stay parallel-safe.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::TempDir;

fn bin(env_overrides: &[(&str, &str)]) -> Command {
    let mut cmd = Command::cargo_bin("ccstatusline-rs").unwrap();
    for (k, v) in env_overrides {
        cmd.env(k, v);
    }
    cmd
}

fn parse_stdout(out: &[u8]) -> Value {
    serde_json::from_slice(out).expect("subcommand stdout is JSON")
}

struct Fixture {
    _root: TempDir,
    bin_dir: PathBuf,
    settings: PathBuf,
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let bin_dir = root.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let settings = root.path().join("settings.json");
    Fixture {
        _root: root,
        bin_dir,
        settings,
    }
}

fn dest_exe_path(bin_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        bin_dir.join("ccstatusline-rs.exe")
    } else {
        bin_dir.join("ccstatusline-rs")
    }
}

fn dest_wrapper_path(bin_dir: &Path) -> Option<PathBuf> {
    if cfg!(windows) {
        Some(bin_dir.join("ccstatusline-rs.mjs"))
    } else {
        None
    }
}

#[test]
fn install_fresh_writes_binary_and_settings() {
    let f = fixture();
    let out = bin(&[])
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&out);
    assert_eq!(v["installed"], true);
    assert_eq!(v["backup"], Value::Null);
    assert_eq!(v["copied_binary"], true);
    assert!(dest_exe_path(&f.bin_dir).exists());
    if let Some(w) = dest_wrapper_path(&f.bin_dir) {
        assert!(w.exists(), "wrapper missing at {w:?}");
    }
    // statusLine block written to settings.
    let settings_json: Value = serde_json::from_slice(&fs::read(&f.settings).unwrap()).unwrap();
    assert_eq!(settings_json["statusLine"]["type"], "command");
    assert!(settings_json["statusLine"]["command"].is_string());
}

#[test]
fn install_preserves_unrelated_settings_keys() {
    let f = fixture();
    let prior = json!({
        "env": { "FOO": "1" },
        "hooks": { "SessionStart": [], "PreToolUse": [] },
        "statusLine": { "type": "command", "command": "node old.mjs" },
        "enabledPlugins": { "neo-mem@neo-marketplace": true },
        "theme": "dark-daltonized"
    });
    fs::write(&f.settings, serde_json::to_vec_pretty(&prior).unwrap()).unwrap();

    bin(&[])
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success();

    let after: Value = serde_json::from_slice(&fs::read(&f.settings).unwrap()).unwrap();
    assert_eq!(after["env"], prior["env"]);
    assert_eq!(after["hooks"], prior["hooks"]);
    assert_eq!(after["enabledPlugins"], prior["enabledPlugins"]);
    assert_eq!(after["theme"], prior["theme"]);
    // statusLine was rewritten to our command.
    assert_ne!(
        after["statusLine"]["command"],
        prior["statusLine"]["command"]
    );
}

#[test]
fn install_idempotent_no_force() {
    let f = fixture();
    bin(&[])
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success();
    let second = bin(&[])
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&second);
    assert_eq!(v["copied_binary"], false, "second install must skip copy");
}

#[test]
fn install_force_overwrites() {
    let f = fixture();
    bin(&[])
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success();
    let out = bin(&[])
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
            "--force",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&out);
    assert_eq!(v["copied_binary"], true, "force must re-copy");
}

#[test]
fn install_records_previous_command() {
    let f = fixture();
    let prior = json!({
        "statusLine": { "type": "command", "command": "node tokenwatch.mjs" }
    });
    fs::write(&f.settings, serde_json::to_vec(&prior).unwrap()).unwrap();
    let out = bin(&[])
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&out);
    assert_eq!(v["previous_command"], "node tokenwatch.mjs");
}

#[test]
fn install_back_to_back_counter_bumps() {
    let f = fixture();
    // Seed a settings file so the first install produces a backup.
    fs::write(&f.settings, b"{}").unwrap();
    bin(&[])
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success();
    bin(&[])
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success();
    let parent = f.settings.parent().unwrap();
    let backups: Vec<_> = fs::read_dir(parent)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".ccstatusline-rs-bak-"))
        .collect();
    assert!(backups.len() >= 2, "expected ≥2 backups, found {backups:?}",);
    // No two filenames collide.
    let unique: std::collections::HashSet<_> = backups.iter().collect();
    assert_eq!(unique.len(), backups.len());
}

#[test]
fn uninstall_restores_backup_atomically() {
    let f = fixture();
    let original = json!({
        "env": {},
        "statusLine": { "type": "command", "command": "node old.mjs" },
        "theme": "dark"
    });
    fs::write(&f.settings, serde_json::to_vec_pretty(&original).unwrap()).unwrap();

    bin(&[])
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success();

    let after_install: Value = serde_json::from_slice(&fs::read(&f.settings).unwrap()).unwrap();
    assert_ne!(
        after_install["statusLine"]["command"],
        original["statusLine"]["command"]
    );

    let uninstall_out = bin(&[])
        .args(["uninstall", "--settings", f.settings.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&uninstall_out);
    assert_eq!(v["uninstalled"], true);
    assert!(v["restored_from"].is_string());

    let restored: Value = serde_json::from_slice(&fs::read(&f.settings).unwrap()).unwrap();
    assert_eq!(
        restored, original,
        "uninstall must restore the pre-install value tree"
    );
}

#[test]
fn uninstall_fails_when_no_backup() {
    let f = fixture();
    fs::write(&f.settings, b"{}").unwrap();
    bin(&[])
        .args(["uninstall", "--settings", f.settings.to_str().unwrap()])
        .assert()
        .failure();
}

#[test]
fn uninstall_purge_binary_removes_files() {
    let f = fixture();
    // Seed settings so install produces a restorable backup.
    fs::write(&f.settings, b"{}").unwrap();
    bin(&[])
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success();
    let exe = dest_exe_path(&f.bin_dir);
    let wrapper = dest_wrapper_path(&f.bin_dir);
    assert!(exe.exists());
    if let Some(w) = wrapper.as_ref() {
        assert!(w.exists());
    }
    let out = bin(&[])
        .args([
            "uninstall",
            "--settings",
            f.settings.to_str().unwrap(),
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--purge-binary",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&out);
    let removed = v["removed"].as_array().unwrap();
    assert!(
        !removed.is_empty(),
        "purge_binary should report removed files",
    );
    assert!(!exe.exists(), "exe should be deleted");
    if let Some(w) = wrapper {
        assert!(!w.exists(), "wrapper should be deleted");
    }
}

#[test]
fn uninstall_purge_binary_skips_unrelated_files() {
    let f = fixture();
    fs::write(&f.settings, b"{}").unwrap();
    bin(&[])
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success();
    let bystander = f.bin_dir.join("sibling.txt");
    fs::write(&bystander, b"i belong to the user").unwrap();
    bin(&[])
        .args([
            "uninstall",
            "--settings",
            f.settings.to_str().unwrap(),
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--purge-binary",
        ])
        .assert()
        .success();
    assert!(
        bystander.exists(),
        "non-ccstatusline files must survive purge"
    );
}

#[test]
fn install_then_uninstall_roundtrip_preserves_unknown_keys() {
    let f = fixture();
    let original = json!({
        "future_top_level_key": { "anything": 42 },
        "theme": "dark",
        "enabledPlugins": { "p": true }
    });
    fs::write(&f.settings, serde_json::to_vec_pretty(&original).unwrap()).unwrap();
    bin(&[])
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success();
    bin(&[])
        .args(["uninstall", "--settings", f.settings.to_str().unwrap()])
        .assert()
        .success();
    let after: Value = serde_json::from_slice(&fs::read(&f.settings).unwrap()).unwrap();
    assert_eq!(after, original, "roundtrip must preserve all unknown keys");
}
