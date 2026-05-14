//! End-to-end: build a temp git repo, point the renderer's cwd at it through
//! a synthetic payload, run a custom config that exercises the three git
//! widgets, and assert the rendered line carries the expected markers.

use std::fs;
use std::path::Path;
use std::process::Command;

use ccstatusline_rs::config::{CONFIG_VERSION, Config};
use ccstatusline_rs::render_with;
use serde_json::json;
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git on PATH");
    assert!(status.success(), "git {args:?} failed");
}

fn build_repo() -> TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path();
    git(p, &["init", "--quiet", "--initial-branch=main"]);
    git(p, &["config", "user.email", "test@example.com"]);
    git(p, &["config", "user.name", "Test"]);
    fs::write(p.join("base.txt"), "v1\n").unwrap();
    git(p, &["add", "base.txt"]);
    git(p, &["commit", "--quiet", "-m", "init"]);
    tmp
}

fn cfg_with(lines: Vec<Vec<&str>>) -> Config {
    Config {
        version: CONFIG_VERSION,
        tz: None,
        lines: lines
            .into_iter()
            .map(|row| row.into_iter().map(String::from).collect())
            .collect(),
    }
}

#[test]
fn git_widgets_render_against_real_repo() {
    let tmp = build_repo();
    let p = tmp.path();
    fs::write(p.join("scratch.txt"), "tmp\n").unwrap(); // untracked

    let payload = json!({ "cwd": p.to_string_lossy() }).to_string();
    let cfg = cfg_with(vec![vec!["git_branch", "git_status"]]);
    let out = render_with(&payload, &cfg).expect("render succeeds");

    assert!(out.contains("🌿 main"), "branch widget missing in: {out}");
    assert!(
        out.contains("⛓ ?1"),
        "status widget missing untracked count in: {out}"
    );
}

#[test]
fn no_git_probe_when_config_lacks_git_widgets() {
    // Use a payload pointing at a non-existent directory; if a git probe ran
    // it would still safely yield None — the assertion here is that the
    // renderer doesn't fail or alter output for the no-git config case.
    let payload = json!({ "cwd": "/nonexistent/dir/that/does/not/exist" }).to_string();
    let cfg = cfg_with(vec![vec!["model", "cwd"]]);
    let out = render_with(&payload, &cfg).expect("render succeeds");
    assert!(!out.contains("🌿"));
    assert!(!out.contains("⛓"));
}
