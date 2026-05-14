//! Real-subprocess smoke for the git probe. Sets up a deterministic temp
//! repo and asserts the probe captures branch + porcelain + diff state. This
//! is the only test that depends on `git` being on PATH; per the project
//! prerequisites it must be.

use std::fs;
use std::path::Path;
use std::process::Command;

use ccstatusline_rs::context::git::probe;
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
    let tmp = tempfile::tempdir().expect("tempdir");
    let p = tmp.path();
    git(p, &["init", "--quiet", "--initial-branch=main"]);
    git(p, &["config", "user.email", "test@example.com"]);
    git(p, &["config", "user.name", "Test"]);
    fs::write(p.join("base.txt"), "v1\n").unwrap();
    git(p, &["add", "base.txt"]);
    git(p, &["commit", "--quiet", "-m", "init"]);
    tmp
}

#[test]
fn probe_yields_branch_and_porcelain() {
    let tmp = build_repo();
    let p = tmp.path();

    // Staged change.
    fs::write(p.join("base.txt"), "v2\n").unwrap();
    git(p, &["add", "base.txt"]);
    // Unstaged change in a different file.
    fs::write(p.join("other.txt"), "u1\n").unwrap();
    git(p, &["add", "other.txt"]);
    git(p, &["commit", "--quiet", "-m", "second"]);
    fs::write(p.join("other.txt"), "u2\n").unwrap();
    // Untracked file.
    fs::write(p.join("scratch.txt"), "tmp\n").unwrap();

    let state = probe(p).expect("probe returns Ok").expect("inside a repo");
    assert_eq!(state.branch.as_deref(), Some("main"));
    assert!(state.porcelain.unstaged >= 1, "{:?}", state.porcelain);
    assert!(state.porcelain.untracked >= 1, "{:?}", state.porcelain);
}

#[test]
fn probe_outside_repo_yields_none() {
    let tmp = tempfile::tempdir().unwrap();
    let state = probe(tmp.path()).expect("probe returns Ok");
    assert!(state.is_none());
}
