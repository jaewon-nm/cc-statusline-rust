//! End-to-end exercise of the binary: payload on stdin, status line on stdout,
//! plus the agent-discovery subcommands. Output is asserted via JSON parse,
//! never raw text match (per `docs/CLI-TESTING.md`).

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

const FIXTURE: &[u8] = include_bytes!("fixtures/default-payload.json");

#[test]
fn renderer_default_invocation_matches_golden_plain() {
    // Drive the underlying-text invariant through the real binary; NO_COLOR
    // is the public opt-out so the bytes drop the ANSI escapes the default
    // theme would otherwise emit.
    let assert = Command::cargo_bin("ccstatusline-rs")
        .unwrap()
        .env("NO_COLOR", "1")
        .env_remove("FORCE_COLOR")
        .env_remove("CLICOLOR_FORCE")
        .write_stdin(FIXTURE)
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let expected = concat!(
        "✦ [Opus 4.7 (1M context)] | 📂 F:\\Works\\naya\\cc-statusline-rust",
        " | 🔋 [..........] 80.0K/1.0M(8%) | 📊 85.3K | 💰 $2.55\n",
        "⏱ 5h [##........](21%) ↻ 12:00 | 📅 7d [##........](20%) ↻ 5/19 06:00\n",
    );
    assert_eq!(stdout, expected);
}

#[test]
fn renderer_default_invocation_emits_ansi_by_default() {
    let assert = Command::cargo_bin("ccstatusline-rs")
        .unwrap()
        .env_remove("NO_COLOR")
        .write_stdin(FIXTURE)
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    // Auto mode without NO_COLOR emits ANSI escapes (theme cyan + threshold
    // greens populate the bar's filled cells).
    assert!(
        stdout.contains("\x1b["),
        "expected ANSI escape in default output"
    );
    assert!(stdout.contains("Opus 4.7"));
}

#[test]
fn schema_subcommand_emits_json_object() {
    let out = Command::cargo_bin("ccstatusline-rs")
        .unwrap()
        .args(["schema"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&out).expect("schema is valid JSON");
    assert!(parsed.is_object(), "schema must be a JSON object");
}

#[test]
fn widgets_subcommand_lists_registered_kinds() {
    let out = Command::cargo_bin("ccstatusline-rs")
        .unwrap()
        .args(["widgets"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    let arr = parsed.as_array().expect("widgets list is an array");
    let kinds: Vec<&str> = arr
        .iter()
        .filter_map(|v| v.get("kind").and_then(Value::as_str))
        .collect();
    assert!(kinds.contains(&"model"));
    assert!(kinds.contains(&"block_timer"));
    assert!(kinds.contains(&"weekly_timer"));
}

#[test]
fn config_show_returns_valid_config_with_layout() {
    let out = Command::cargo_bin("ccstatusline-rs")
        .unwrap()
        .args(["config", "show"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(parsed["version"], 1);
    let lines = parsed["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 2);
}

#[test]
fn config_validate_returns_ok() {
    Command::cargo_bin("ccstatusline-rs")
        .unwrap()
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"));
}
