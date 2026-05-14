//! Git state probe. Three subprocess calls (toplevel, porcelain --branch,
//! shortstats), each bounded by a wall-clock timeout. Results are cached to
//! disk per repo for a short TTL so consecutive renderer invocations within
//! the same edit burst don't pay the subprocess cost twice.
//!
//! Failures degrade silently to `None`; widgets render nothing rather than
//! emit a confusing error string in the status bar.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use wait_timeout::ChildExt;
use xxhash_rust::xxh3::xxh3_64;

use crate::error::{Error, Result};

const PROBE_TIMEOUT: Duration = Duration::from_millis(800);
const CACHE_TTL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitState {
    pub repo_root: PathBuf,
    pub branch: Option<String>,
    pub porcelain: PorcelainCounts,
    pub diff: DiffShortstat,
    /// Unix epoch seconds when the snapshot was captured.
    pub captured_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PorcelainCounts {
    pub staged: u32,
    pub unstaged: u32,
    pub untracked: u32,
    pub conflicts: u32,
}

impl PorcelainCounts {
    pub fn is_clean(&self) -> bool {
        self.staged == 0 && self.unstaged == 0 && self.untracked == 0 && self.conflicts == 0
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffShortstat {
    pub insertions: u32,
    pub deletions: u32,
    pub files_changed: u32,
}

/// Probe the repo containing `cwd`. Returns `Ok(None)` for any non-repo
/// directory or when probing fails for benign reasons (timeout, git missing,
/// directory unreadable). The renderer must never bubble these to the user.
pub fn probe(cwd: &Path) -> Result<Option<GitState>> {
    let Some(repo_root) = locate_repo_root(cwd)? else {
        return Ok(None);
    };

    if let Some(cached) = read_cache(&repo_root) {
        return Ok(Some(cached));
    }

    let porcelain_bytes = match run_git_bytes(
        &repo_root,
        &[
            "--no-optional-locks",
            "status",
            "--porcelain=v1",
            "--branch",
            "-z",
        ],
        "porcelain",
    ) {
        Ok(b) => b,
        Err(_) => return Ok(None),
    };
    let (branch_from_porcelain, porcelain) = parse_porcelain(&porcelain_bytes);

    // Fall back to symbolic-ref only when the porcelain header didn't carry a
    // usable name (detached HEAD, or first-commit pre-history).
    let branch = match branch_from_porcelain {
        Some(b) => Some(b),
        None => run_git_string(&repo_root, &["rev-parse", "--abbrev-ref", "HEAD"], "branch")
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty() && s != "HEAD"),
    };

    let unstaged_stat = run_git_string(&repo_root, &["diff", "--shortstat"], "diff_unstaged")
        .map(|s| parse_shortstat(&s))
        .unwrap_or_default();
    let staged_stat = run_git_string(
        &repo_root,
        &["diff", "--cached", "--shortstat"],
        "diff_staged",
    )
    .map(|s| parse_shortstat(&s))
    .unwrap_or_default();
    let diff = DiffShortstat {
        insertions: unstaged_stat.insertions + staged_stat.insertions,
        deletions: unstaged_stat.deletions + staged_stat.deletions,
        files_changed: unstaged_stat.files_changed + staged_stat.files_changed,
    };

    let state = GitState {
        repo_root,
        branch,
        porcelain,
        diff,
        captured_at: now_epoch_seconds(),
    };

    write_cache(&state);
    Ok(Some(state))
}

fn locate_repo_root(cwd: &Path) -> Result<Option<PathBuf>> {
    let output = match run_git_string(cwd, &["rev-parse", "--show-toplevel"], "repo_root") {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };
    let trimmed = output.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(trimmed)))
    }
}

fn run_git_bytes(cwd: &Path, args: &[&str], name: &'static str) -> Result<Vec<u8>> {
    let mut child = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| Error::ProbeFailed {
            name,
            reason: format!("spawn: {e}"),
        })?;

    let status = match child.wait_timeout(PROBE_TIMEOUT) {
        Ok(Some(status)) => status,
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::ProbeTimeout {
                name,
                ms: PROBE_TIMEOUT.as_millis() as u64,
            });
        }
        Err(e) => {
            return Err(Error::ProbeFailed {
                name,
                reason: format!("wait: {e}"),
            });
        }
    };

    if !status.success() {
        return Err(Error::ProbeFailed {
            name,
            reason: format!("exit {:?}", status.code()),
        });
    }

    let mut buf = Vec::new();
    if let Some(mut stdout) = child.stdout.take() {
        stdout
            .read_to_end(&mut buf)
            .map_err(|e| Error::ProbeFailed {
                name,
                reason: format!("read: {e}"),
            })?;
    }
    Ok(buf)
}

fn run_git_string(cwd: &Path, args: &[&str], name: &'static str) -> Result<String> {
    let bytes = run_git_bytes(cwd, args, name)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn parse_porcelain(raw: &[u8]) -> (Option<String>, PorcelainCounts) {
    let mut counts = PorcelainCounts::default();
    let mut branch = None;
    let mut iter = raw.split(|&b| b == 0).peekable();

    if let Some(first) = iter.peek()
        && first.starts_with(b"## ")
    {
        let header = String::from_utf8_lossy(first);
        branch = parse_branch_header(&header);
        iter.next();
    }

    while let Some(entry) = iter.next() {
        if entry.is_empty() {
            continue;
        }
        if entry.len() < 2 {
            continue;
        }
        let x = entry[0];
        let y = entry[1];

        // `!!` = ignored, `??` = untracked.
        if x == b'!' && y == b'!' {
            continue;
        }
        if x == b'?' && y == b'?' {
            counts.untracked += 1;
            continue;
        }

        // Conflict markers: any U, or matching AA/DD.
        let is_conflict =
            x == b'U' || y == b'U' || (x == b'A' && y == b'A') || (x == b'D' && y == b'D');
        if is_conflict {
            counts.conflicts += 1;
        } else {
            if x != b' ' {
                counts.staged += 1;
            }
            if y != b' ' {
                counts.unstaged += 1;
            }
        }

        // Rename / copy carries the old-name in the following NUL chunk.
        if x == b'R' || x == b'C' {
            iter.next();
        }
    }

    (branch, counts)
}

fn parse_branch_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("## ")?.trim();
    if rest.starts_with("HEAD (no branch") {
        return None;
    }
    let name_end = rest.find("...").unwrap_or(rest.len());
    let name = rest[..name_end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_owned())
    }
}

fn parse_shortstat(raw: &str) -> DiffShortstat {
    let mut s = DiffShortstat::default();
    for token in raw.split(',').map(str::trim) {
        if token.ends_with("files changed") || token.ends_with("file changed") {
            s.files_changed = token
                .split_whitespace()
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
        } else if token.contains("insertion") {
            s.insertions = token
                .split_whitespace()
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
        } else if token.contains("deletion") {
            s.deletions = token
                .split_whitespace()
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
        }
    }
    s
}

fn cache_path(repo_root: &Path) -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("dev", "naya", "ccstatusline-rs")?;
    let cache_root = dirs.cache_dir().join("git");
    fs::create_dir_all(&cache_root).ok()?;
    let key = format!(
        "{:016x}.json",
        xxh3_64(repo_root.to_string_lossy().as_bytes())
    );
    Some(cache_root.join(key))
}

fn read_cache(repo_root: &Path) -> Option<GitState> {
    let path = cache_path(repo_root)?;
    let metadata = fs::metadata(&path).ok()?;
    let modified = metadata.modified().ok()?;
    if SystemTime::now().duration_since(modified).ok()? > CACHE_TTL {
        return None;
    }
    let raw = fs::read(&path).ok()?;
    let state: GitState = serde_json::from_slice(&raw).ok()?;
    Some(state)
}

fn write_cache(state: &GitState) {
    let Some(path) = cache_path(&state.repo_root) else {
        return;
    };
    let Ok(serialized) = serde_json::to_vec(state) else {
        return;
    };
    // Atomic-ish write: write to a sibling `.tmp` then rename. A racing
    // renderer either reads the previous file or the new one, never partial.
    let tmp_path = path.with_extension("json.tmp");
    if fs::write(&tmp_path, &serialized).is_err() {
        return;
    }
    let _ = fs::rename(&tmp_path, &path);
}

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_porcelain_empty() {
        let (branch, counts) = parse_porcelain(b"");
        assert!(branch.is_none());
        assert!(counts.is_clean());
    }

    #[test]
    fn parse_porcelain_branch_only() {
        // `## main...origin/main\0` (no entries).
        let raw = b"## main...origin/main\0";
        let (branch, counts) = parse_porcelain(raw);
        assert_eq!(branch.as_deref(), Some("main"));
        assert!(counts.is_clean());
    }

    #[test]
    fn parse_porcelain_counts_staged_unstaged_untracked() {
        // Two NUL-separated entries after the branch header.
        // `M ` = staged-modified, ` M` = unstaged-modified, `??` = untracked.
        let mut raw: Vec<u8> = b"## main\0".to_vec();
        raw.extend_from_slice(b"M  staged.rs\0");
        raw.extend_from_slice(b" M unstaged.rs\0");
        raw.extend_from_slice(b"?? newfile.rs\0");
        let (_, counts) = parse_porcelain(&raw);
        assert_eq!(counts.staged, 1);
        assert_eq!(counts.unstaged, 1);
        assert_eq!(counts.untracked, 1);
        assert_eq!(counts.conflicts, 0);
    }

    #[test]
    fn parse_porcelain_detects_conflicts() {
        let mut raw: Vec<u8> = b"## main\0".to_vec();
        raw.extend_from_slice(b"UU conflict.rs\0");
        let (_, counts) = parse_porcelain(&raw);
        assert_eq!(counts.conflicts, 1);
        assert_eq!(counts.staged, 0);
        assert_eq!(counts.unstaged, 0);
    }

    #[test]
    fn parse_porcelain_rename_consumes_oldpath() {
        let mut raw: Vec<u8> = b"## main\0".to_vec();
        raw.extend_from_slice(b"R  new.rs\0");
        raw.extend_from_slice(b"old.rs\0");
        raw.extend_from_slice(b"?? another.rs\0");
        let (_, counts) = parse_porcelain(&raw);
        // R = renamed staged; the old-path NUL chunk is consumed, not
        // mis-counted as an entry.
        assert_eq!(counts.staged, 1);
        assert_eq!(counts.untracked, 1);
    }

    #[test]
    fn parse_porcelain_detached_head() {
        let raw = b"## HEAD (no branch)\0";
        let (branch, counts) = parse_porcelain(raw);
        assert!(branch.is_none());
        assert!(counts.is_clean());
    }

    #[test]
    fn parse_shortstat_full() {
        let s = parse_shortstat(" 5 files changed, 100 insertions(+), 20 deletions(-)");
        assert_eq!(s.files_changed, 5);
        assert_eq!(s.insertions, 100);
        assert_eq!(s.deletions, 20);
    }

    #[test]
    fn parse_shortstat_only_insertions() {
        let s = parse_shortstat(" 1 file changed, 3 insertions(+)");
        assert_eq!(s.files_changed, 1);
        assert_eq!(s.insertions, 3);
        assert_eq!(s.deletions, 0);
    }

    #[test]
    fn parse_shortstat_empty() {
        let s = parse_shortstat("");
        assert_eq!(s, DiffShortstat::default());
    }

    #[test]
    fn probe_on_non_repo_yields_none() {
        let tmp = tempfile::tempdir().unwrap();
        let result = probe(tmp.path()).unwrap();
        assert!(result.is_none());
    }
}
