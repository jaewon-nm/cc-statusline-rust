//! End-to-end coverage for the tokenwatch-aware wrap mode. Each test pins
//! `HOME` (POSIX) / `USERPROFILE` (Windows) to a per-test tempdir so the
//! resolved `~/.claude/.tw-statusline-prev.json` lands inside the fixture
//! and tests stay parallel-safe.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::TempDir;

fn bin(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("ccstatusline-rs").unwrap();
    if cfg!(windows) {
        cmd.env("USERPROFILE", home);
    } else {
        cmd.env("HOME", home);
    }
    cmd
}

fn parse_stdout(out: &[u8]) -> Value {
    serde_json::from_slice(out).expect("subcommand stdout is JSON")
}

struct Fixture {
    _root: TempDir,
    home: PathBuf,
    bin_dir: PathBuf,
    settings: PathBuf,
    tw_prev: PathBuf,
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().to_path_buf();
    let bin_dir = home.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let claude_dir = home.join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    let settings = claude_dir.join("settings.json");
    let tw_prev = claude_dir.join(".tw-statusline-prev.json");
    Fixture {
        _root: root,
        home,
        bin_dir,
        settings,
        tw_prev,
    }
}

fn tokenwatch_command_literal() -> &'static str {
    // Plausible Windows neo-mem path; the wrap-mode detector is path-agnostic
    // and just looks at the basename, so the same literal exercises POSIX too.
    if cfg!(windows) {
        "node \"C:\\\\fake\\\\neo-mem\\\\1.2.10\\\\scripts\\\\tokenwatch-statusline.mjs\""
    } else {
        "node /fake/neo-mem/1.2.10/scripts/tokenwatch-statusline.mjs"
    }
}

fn dest_exe_name() -> &'static str {
    if cfg!(windows) {
        "ccstatusline-rs.exe"
    } else {
        "ccstatusline-rs"
    }
}

fn ours_wrap_basename_literal() -> &'static str {
    if cfg!(windows) {
        "ccstatusline-rs.mjs"
    } else {
        "ccstatusline-rs"
    }
}

fn seed_settings_with_tokenwatch(settings: &Path) {
    let v = json!({
        "statusLine": { "type": "command", "command": tokenwatch_command_literal() }
    });
    fs::write(settings, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
}

#[test]
fn wrap_install_when_tokenwatch_present() {
    let f = fixture();
    seed_settings_with_tokenwatch(&f.settings);
    let pre_settings = fs::read(&f.settings).unwrap();

    let out = bin(&f.home)
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
    assert_eq!(v["mode"], "wrap");
    assert_eq!(v["backup"], Value::Null);
    assert_eq!(v["previous_wrap_command"], Value::Null);
    assert!(v["wrap_prev_path"].is_string());

    // Settings.json byte-identical — wrap mode must not touch it.
    assert_eq!(
        fs::read(&f.settings).unwrap(),
        pre_settings,
        "wrap install must leave settings.json untouched",
    );
    assert!(f.tw_prev.exists());
    let prev: Value = serde_json::from_slice(&fs::read(&f.tw_prev).unwrap()).unwrap();
    assert!(
        prev["command"]
            .as_str()
            .unwrap()
            .contains(ours_wrap_basename_literal()),
        "wrap prev should point at our wrapper basename: {prev}",
    );
}

#[test]
fn wrap_install_idempotent() {
    let f = fixture();
    seed_settings_with_tokenwatch(&f.settings);
    bin(&f.home)
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success();
    // Second invocation must succeed and stay wrap mode (basename match).
    let out = bin(&f.home)
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
    assert_eq!(v["mode"], "wrap");
    // Re-install detects the prior pointer as ours, so previous_wrap_command
    // should be present and basename-ours.
    let prev_cmd = v["previous_wrap_command"].as_str().unwrap();
    assert!(prev_cmd.contains(ours_wrap_basename_literal()));
}

#[test]
fn wrap_install_relocation_overwrites_prev() {
    let f = fixture();
    seed_settings_with_tokenwatch(&f.settings);
    let bin_a = f.home.join("bin-a");
    let bin_b = f.home.join("bin-b");
    fs::create_dir_all(&bin_a).unwrap();
    fs::create_dir_all(&bin_b).unwrap();

    bin(&f.home)
        .args([
            "install",
            "--bin-dir",
            bin_a.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success();
    let out = bin(&f.home)
        .args([
            "install",
            "--bin-dir",
            bin_b.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&out);
    assert_eq!(v["mode"], "wrap");

    let prev: Value = serde_json::from_slice(&fs::read(&f.tw_prev).unwrap()).unwrap();
    let cmd = prev["command"].as_str().unwrap();
    assert!(
        cmd.contains("bin-b"),
        "relocation must rewrite prev.command to point at the new bin dir: {cmd}",
    );
    assert!(!cmd.contains("bin-a"));
}

#[test]
fn wrap_install_rejects_pre_existing_other_wrap() {
    let f = fixture();
    seed_settings_with_tokenwatch(&f.settings);
    let foreign = json!({ "type": "command", "command": "node other-tool.mjs" });
    fs::write(&f.tw_prev, serde_json::to_vec_pretty(&foreign).unwrap()).unwrap();
    let pre_settings = fs::read(&f.settings).unwrap();
    let pre_prev = fs::read(&f.tw_prev).unwrap();

    let assertion = bin(&f.home)
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).to_string();
    assert!(
        stderr.contains("other-tool.mjs"),
        "WrapConflict must surface existing command verbatim, got: {stderr}",
    );

    assert_eq!(fs::read(&f.settings).unwrap(), pre_settings);
    assert_eq!(fs::read(&f.tw_prev).unwrap(), pre_prev);
}

#[test]
fn wrap_install_rejects_invalid_prev_json() {
    let f = fixture();
    seed_settings_with_tokenwatch(&f.settings);
    fs::write(&f.tw_prev, b"{ not json").unwrap();
    let pre_settings = fs::read(&f.settings).unwrap();
    let pre_prev = fs::read(&f.tw_prev).unwrap();

    bin(&f.home)
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .failure();

    assert_eq!(fs::read(&f.settings).unwrap(), pre_settings);
    assert_eq!(fs::read(&f.tw_prev).unwrap(), pre_prev);
}

#[test]
fn wrap_uninstall_removes_prev_only() {
    let f = fixture();
    seed_settings_with_tokenwatch(&f.settings);
    let pre_settings = fs::read(&f.settings).unwrap();

    bin(&f.home)
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(f.tw_prev.exists());

    let out = bin(&f.home)
        .args([
            "uninstall",
            "--settings",
            f.settings.to_str().unwrap(),
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = parse_stdout(&out);
    assert_eq!(v["mode"], "wrap");
    assert_eq!(v["restored_from"], Value::Null);
    assert!(v["removed_wrap_prev"].is_string());

    assert!(!f.tw_prev.exists());
    assert_eq!(
        fs::read(&f.settings).unwrap(),
        pre_settings,
        "wrap uninstall must leave settings.json byte-identical to pre-install snapshot",
    );
}

#[test]
fn wrap_uninstall_requires_positive_evidence() {
    let f = fixture();
    // settings.json carries tokenwatch but no prev file exists → we did
    // not install anything; bail out instead of pretending we own this.
    seed_settings_with_tokenwatch(&f.settings);
    bin(&f.home)
        .args([
            "uninstall",
            "--settings",
            f.settings.to_str().unwrap(),
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
        ])
        .assert()
        .failure();
}

#[test]
fn wrap_uninstall_stale_pointer_fails_loudly() {
    let f = fixture();
    // settings.json is no longer tokenwatch (some other tool overwrote it),
    // but `.tw-statusline-prev.json` still points at us — reconcile manually.
    fs::write(&f.settings, b"{}").unwrap();
    let ours = json!({
        "type": "command",
        "command": format!("node /some/path/{}", ours_wrap_basename_literal())
    });
    fs::write(&f.tw_prev, serde_json::to_vec_pretty(&ours).unwrap()).unwrap();
    let pre_prev = fs::read(&f.tw_prev).unwrap();
    bin(&f.home)
        .args([
            "uninstall",
            "--settings",
            f.settings.to_str().unwrap(),
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
        ])
        .assert()
        .failure();
    // Stale-pointer must NOT delete the prev file behind the operator's back.
    assert_eq!(fs::read(&f.tw_prev).unwrap(), pre_prev);
}

#[test]
fn direct_install_unchanged_behavior() {
    let f = fixture();
    fs::write(&f.settings, b"{}").unwrap();
    let out = bin(&f.home)
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
    assert_eq!(v["mode"], "direct");
    assert!(v["backup"].is_string());
    assert!(!f.tw_prev.exists(), "direct mode must not touch wrap prev");
}

#[test]
fn uninstall_backup_flag_forces_direct() {
    let f = fixture();
    seed_settings_with_tokenwatch(&f.settings);
    // Install wraps, but explicit --backup forces Direct path which then
    // fails because the file we point at is bogus — the failure mode is
    // the assertion: --backup precedence wins over wrap heuristic.
    bin(&f.home)
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success();
    let bogus = f.home.join("no-such-backup.bak");
    bin(&f.home)
        .args([
            "uninstall",
            "--settings",
            f.settings.to_str().unwrap(),
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--backup",
            bogus.to_str().unwrap(),
        ])
        .assert()
        .failure();
    // Prev file still there — direct path was chosen and never touched it.
    assert!(f.tw_prev.exists());
}

#[test]
fn uninstall_fails_when_no_traces() {
    let f = fixture();
    // No tokenwatch in settings, no backup, no prev pointer.
    fs::write(&f.settings, b"{}").unwrap();
    bin(&f.home)
        .args([
            "uninstall",
            "--settings",
            f.settings.to_str().unwrap(),
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
        ])
        .assert()
        .failure();
}

#[test]
fn purge_binary_works_in_both_modes() {
    let f = fixture();
    seed_settings_with_tokenwatch(&f.settings);
    bin(&f.home)
        .args([
            "install",
            "--bin-dir",
            f.bin_dir.to_str().unwrap(),
            "--settings",
            f.settings.to_str().unwrap(),
        ])
        .assert()
        .success();
    let exe = f.bin_dir.join(dest_exe_name());
    assert!(exe.exists());
    let out = bin(&f.home)
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
    assert_eq!(v["mode"], "wrap");
    let removed = v["removed"].as_array().unwrap();
    assert!(
        !removed.is_empty(),
        "purge_binary should report removed files in wrap mode too",
    );
    assert!(!exe.exists());
}
