//! `install` / `uninstall` — wire the renderer into Claude Code in one step,
//! and undo the most recent wiring on demand.
//!
//! Windows requires the statusLine command to follow the `<interpreter>
//! <script>` shape (bare `.exe` and `cmd /c` wrappers are silently dropped),
//! so on that platform we drop both the binary and a tiny `.mjs` wrapper into
//! the chosen bin dir. POSIX uses the bare binary path, single-quoted so
//! paths with spaces survive Claude Code's shell parse.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use jiff::Zoned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::ioutil;

const EXE_BASENAME_WINDOWS: &str = "ccstatusline-rs.exe";
const EXE_BASENAME_POSIX: &str = "ccstatusline-rs";
const WRAPPER_BASENAME: &str = "ccstatusline-rs.mjs";
const BACKUP_INFIX: &str = ".ccstatusline-rs-bak-";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Posix,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::Posix
        }
    }

    fn exe_basename(self) -> &'static str {
        match self {
            Self::Windows => EXE_BASENAME_WINDOWS,
            Self::Posix => EXE_BASENAME_POSIX,
        }
    }
}

pub struct InstallArgs {
    pub bin_dir: Option<PathBuf>,
    pub settings: Option<PathBuf>,
    pub force: bool,
}

pub struct UninstallArgs {
    pub settings: Option<PathBuf>,
    pub backup: Option<PathBuf>,
    pub bin_dir: Option<PathBuf>,
    pub purge_binary: bool,
}

#[derive(Debug, Serialize)]
pub struct InstallReport {
    pub installed: bool,
    pub bin: PathBuf,
    pub wrapper: Option<PathBuf>,
    pub settings: PathBuf,
    pub backup: Option<PathBuf>,
    pub copied_binary: bool,
    pub previous_command: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UninstallReport {
    pub uninstalled: bool,
    pub settings: PathBuf,
    pub restored_from: PathBuf,
    pub removed: Vec<PathBuf>,
    /// Files we tried to delete during `--purge-binary` that existed but
    /// resisted removal (locked, ACL'd, etc.). Reported, not fatal.
    pub failed_removals: Vec<FailedRemoval>,
}

#[derive(Debug, Serialize)]
pub struct FailedRemoval {
    pub path: PathBuf,
    pub error: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ClaudeSettings {
    #[serde(
        default,
        rename = "statusLine",
        skip_serializing_if = "Option::is_none"
    )]
    status_line: Option<StatusLine>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct StatusLine {
    #[serde(rename = "type")]
    kind: String,
    command: String,
}

pub fn install(args: InstallArgs) -> Result<InstallReport> {
    let platform = Platform::current();
    let bin_dir = resolve_bin_dir(args.bin_dir, platform)?;
    create_dir(&bin_dir, "install_bin_dir")?;

    let src = current_exe()?;
    let dest_exe = bin_dir.join(platform.exe_basename());
    let copied_binary = copy_binary_if_needed(&src, &dest_exe, args.force)?;

    let wrapper = match platform {
        Platform::Windows => {
            let path = bin_dir.join(WRAPPER_BASENAME);
            let body = wrapper_body(&dest_exe);
            ioutil::atomic_write_bytes(&path, body.as_bytes())?;
            Some(path)
        }
        Platform::Posix => None,
    };

    let settings_path = resolve_settings_path(args.settings)?;
    let (previous_command, backup) = if settings_path.exists() {
        let existing_bytes = fs::read(&settings_path).map_err(|source| Error::FileIo {
            operation: "read_settings",
            path: settings_path.clone(),
            source,
        })?;
        let backup_path = next_backup_filename(&settings_path)?;
        fs::write(&backup_path, &existing_bytes).map_err(|source| Error::FileIo {
            operation: "backup_settings",
            path: backup_path.clone(),
            source,
        })?;
        let parsed: ClaudeSettings = if existing_bytes.is_empty() {
            ClaudeSettings::default()
        } else {
            serde_json::from_slice(&existing_bytes).map_err(|e| Error::InvalidConfig {
                reason: format!("{p}: {e}", p = settings_path.display()),
            })?
        };
        let prev = parsed.status_line.as_ref().map(|s| s.command.clone());
        (prev, Some(backup_path))
    } else {
        (None, None)
    };

    let mut settings = if let Some(backup_path) = backup.as_ref() {
        let raw = fs::read(backup_path).map_err(|source| Error::FileIo {
            operation: "reread_settings",
            path: backup_path.clone(),
            source,
        })?;
        if raw.is_empty() {
            ClaudeSettings::default()
        } else {
            serde_json::from_slice(&raw).map_err(|e| Error::InvalidConfig {
                reason: format!("{p}: {e}", p = settings_path.display()),
            })?
        }
    } else {
        ClaudeSettings::default()
    };

    let command = compose_command(platform, &dest_exe, wrapper.as_deref());
    settings.status_line = Some(StatusLine {
        kind: "command".to_owned(),
        command,
    });

    let serialized = serde_json::to_vec_pretty(&settings).map_err(Error::from)?;
    ioutil::atomic_write_bytes(&settings_path, &serialized)?;

    Ok(InstallReport {
        installed: true,
        bin: dest_exe,
        wrapper,
        settings: settings_path,
        backup,
        copied_binary,
        previous_command,
    })
}

pub fn uninstall(args: UninstallArgs) -> Result<UninstallReport> {
    let platform = Platform::current();
    let settings_path = resolve_settings_path(args.settings)?;

    let backup_path = match args.backup {
        Some(p) => p,
        None => find_latest_backup(&settings_path)?.ok_or_else(|| Error::NoBackupFound {
            settings: settings_path.clone(),
        })?,
    };

    let bytes = fs::read(&backup_path).map_err(|source| Error::FileIo {
        operation: "read_backup",
        path: backup_path.clone(),
        source,
    })?;
    ioutil::atomic_write_bytes(&settings_path, &bytes)?;

    let mut removed = Vec::new();
    let mut failed_removals = Vec::new();
    if args.purge_binary {
        let bin_dir = resolve_bin_dir(args.bin_dir, platform)?;
        for candidate in [
            bin_dir.join(platform.exe_basename()),
            bin_dir.join(WRAPPER_BASENAME),
        ] {
            if !candidate.exists() {
                continue;
            }
            match fs::remove_file(&candidate) {
                Ok(()) => removed.push(candidate),
                Err(err) => failed_removals.push(FailedRemoval {
                    path: candidate,
                    error: err.to_string(),
                }),
            }
        }
    }

    Ok(UninstallReport {
        uninstalled: true,
        settings: settings_path,
        restored_from: backup_path,
        removed,
        failed_removals,
    })
}

pub fn emit_install_report(report: &InstallReport, pretty: bool) -> Result<()> {
    emit_json(&serde_json::to_value(report)?, pretty)
}

pub fn emit_uninstall_report(report: &UninstallReport, pretty: bool) -> Result<()> {
    emit_json(&serde_json::to_value(report)?, pretty)
}

fn emit_json(value: &Value, pretty: bool) -> Result<()> {
    let mut stdout = io::stdout().lock();
    if pretty {
        serde_json::to_writer_pretty(&mut stdout, value)?;
    } else {
        serde_json::to_writer(&mut stdout, value)?;
    }
    stdout.write_all(b"\n")?;
    Ok(())
}

// ---------- pure helpers (testable on every OS) ----------

pub fn compose_command(platform: Platform, exe: &Path, wrapper: Option<&Path>) -> String {
    match platform {
        Platform::Windows => {
            // The Windows install path always produces a wrapper; catching the
            // misuse with a debug-only assert keeps prod callers safe without
            // panicking in release builds.
            debug_assert!(
                wrapper.is_some(),
                "compose_command(Windows, _, None) is meaningless — the install path must emit a `.mjs` wrapper",
            );
            let target = wrapper.unwrap_or(exe);
            let target_str = serde_json::to_string(&target.display().to_string())
                .unwrap_or_else(|_| String::from("\"\""));
            format!("node {target_str}")
        }
        Platform::Posix => posix_shell_quote(&exe.display().to_string()),
    }
}

pub fn wrapper_body(exe: &Path) -> String {
    let exe_literal =
        serde_json::to_string(&exe.display().to_string()).unwrap_or_else(|_| String::from("\"\""));
    // Single-line `import`, LF-only line endings: Node on Windows handles LF
    // fine and the test suite stays stable on every checkout.
    let mut out = String::new();
    out.push_str("import { spawn } from 'node:child_process';\n");
    out.push_str(&format!(
        "const child = spawn({exe_literal}, [], {{ stdio: ['inherit', 'inherit', 'inherit'] }});\n"
    ));
    // `code ?? 1` keeps signal-termination visible as a failure exit.
    out.push_str("child.on('exit', code => process.exit(code ?? 1));\n");
    out.push_str("child.on('error', err => { console.error(err); process.exit(1); });\n");
    out
}

pub fn posix_shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            // Single-quote escaping idiom: close, escape literal apostrophe, reopen.
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

// ---------- path resolvers ----------

fn current_exe() -> Result<PathBuf> {
    std::env::current_exe().map_err(|source| Error::FileIo {
        operation: "current_exe",
        path: PathBuf::new(),
        source,
    })
}

fn resolve_bin_dir(explicit: Option<PathBuf>, platform: Platform) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    let home = home_dir()?;
    let candidate = match platform {
        Platform::Windows => home.join("bin"),
        Platform::Posix => home.join(".local").join("bin"),
    };
    Ok(candidate)
}

fn resolve_settings_path(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    Ok(home_dir()?.join(".claude").join("settings.json"))
}

fn home_dir() -> Result<PathBuf> {
    directories::BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .ok_or_else(|| Error::InvalidConfig {
            reason: "no home directory resolvable on this platform".into(),
        })
}

fn copy_binary_if_needed(src: &Path, dest: &Path, force: bool) -> Result<bool> {
    let src_canon = src.canonicalize().map_err(|source| Error::FileIo {
        operation: "canonicalize_src",
        path: src.to_path_buf(),
        source,
    })?;
    let dest_canon = dest.canonicalize().ok();
    if let Some(d) = dest_canon
        && d == src_canon
    {
        return Ok(false);
    }
    if dest.exists() && !force && files_equal(&src_canon, dest)? {
        return Ok(false);
    }
    fs::copy(&src_canon, dest).map_err(|source| Error::FileIo {
        operation: "copy_binary",
        path: dest.to_path_buf(),
        source,
    })?;
    Ok(true)
}

fn files_equal(a: &Path, b: &Path) -> Result<bool> {
    let am = fs::metadata(a).map_err(|source| Error::FileIo {
        operation: "stat_src",
        path: a.to_path_buf(),
        source,
    })?;
    let bm = fs::metadata(b).map_err(|source| Error::FileIo {
        operation: "stat_dest",
        path: b.to_path_buf(),
        source,
    })?;
    if am.len() != bm.len() {
        return Ok(false);
    }
    let abytes = fs::read(a).map_err(|source| Error::FileIo {
        operation: "read_src",
        path: a.to_path_buf(),
        source,
    })?;
    let bbytes = fs::read(b).map_err(|source| Error::FileIo {
        operation: "read_dest",
        path: b.to_path_buf(),
        source,
    })?;
    Ok(abytes == bbytes)
}

fn create_dir(path: &Path, op: &'static str) -> Result<()> {
    fs::create_dir_all(path).map_err(|source| Error::FileIo {
        operation: op,
        path: path.to_path_buf(),
        source,
    })
}

// ---------- backup naming ----------

fn next_backup_filename(settings: &Path) -> Result<PathBuf> {
    let ts = Zoned::now().strftime("%Y%m%d-%H%M%S").to_string();
    next_backup_filename_at(settings, &ts)
}

/// Test seam: timestamp is injected so a unit test that calls this twice in
/// the same wall-clock second still observes the `-000` → `-001` counter
/// bump deterministically.
fn next_backup_filename_at(settings: &Path, ts: &str) -> Result<PathBuf> {
    let parent = settings.parent().unwrap_or(Path::new("."));
    let stem = settings
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or_else(|| Error::InvalidConfig {
            reason: "settings path has no file name".into(),
        })?;
    for n in 0u32..=999 {
        let candidate = parent.join(format!("{stem}{BACKUP_INFIX}{ts}-{n:03}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(Error::InvalidConfig {
        reason: format!("exhausted backup counter at {ts} (>=1000 in same second)"),
    })
}

fn find_latest_backup(settings: &Path) -> Result<Option<PathBuf>> {
    let parent = settings.parent().unwrap_or(Path::new("."));
    let stem = settings
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or_else(|| Error::InvalidConfig {
            reason: "settings path has no file name".into(),
        })?;
    let prefix = format!("{stem}{BACKUP_INFIX}");
    let read = match fs::read_dir(parent) {
        Ok(r) => r,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(Error::FileIo {
                operation: "read_backup_dir",
                path: parent.to_path_buf(),
                source,
            });
        }
    };
    let mut matches: Vec<PathBuf> = read
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .map(|name| name.to_string_lossy().starts_with(&prefix))
                .unwrap_or(false)
        })
        .collect();
    matches.sort();
    Ok(matches.pop())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_command_windows_uses_node_wrapper() {
        let exe = Path::new("C:\\bin\\ccstatusline-rs.exe");
        let wrapper = PathBuf::from("C:\\bin\\ccstatusline-rs.mjs");
        let cmd = compose_command(Platform::Windows, exe, Some(&wrapper));
        assert_eq!(cmd, "node \"C:\\\\bin\\\\ccstatusline-rs.mjs\"");
    }

    #[test]
    fn compose_command_posix_single_quotes_exe_path() {
        let plain = Path::new("/usr/local/bin/ccstatusline-rs");
        assert_eq!(
            compose_command(Platform::Posix, plain, None),
            "'/usr/local/bin/ccstatusline-rs'",
        );

        let with_space = Path::new("/home/user with space/bin/ccstatusline-rs");
        assert_eq!(
            compose_command(Platform::Posix, with_space, None),
            "'/home/user with space/bin/ccstatusline-rs'",
        );

        let with_quote = Path::new("/home/o'reilly/bin/ccstatusline-rs");
        assert_eq!(
            compose_command(Platform::Posix, with_quote, None),
            "'/home/o'\\''reilly/bin/ccstatusline-rs'",
        );
    }

    #[test]
    fn posix_shell_quote_handles_empty_and_quotes() {
        assert_eq!(posix_shell_quote(""), "''");
        assert_eq!(posix_shell_quote("simple"), "'simple'");
        assert_eq!(posix_shell_quote("it's"), "'it'\\''s'");
        assert_eq!(posix_shell_quote("a'b'c"), "'a'\\''b'\\''c'");
    }

    #[test]
    fn wrapper_body_uses_lf_line_endings() {
        let body = wrapper_body(Path::new("C:\\bin\\ccstatusline-rs.exe"));
        assert!(!body.contains('\r'), "wrapper body must not contain CR");
        assert!(body.ends_with('\n'));
    }

    #[test]
    fn wrapper_body_exits_one_on_signal() {
        let body = wrapper_body(Path::new("/x/y"));
        assert!(
            body.contains("code ?? 1"),
            "wrapper must exit 1 on signal termination, body=\n{body}",
        );
    }

    #[test]
    fn wrapper_body_escapes_unicode_paths() {
        let body = wrapper_body(Path::new(r"C:\Users\홍길동\bin\ccstatusline-rs.exe"));
        // The literal must round-trip back to the original path through any
        // JSON-conformant string encoding (ASCII `\uXXXX` escapes or raw UTF-8
        // are both valid serde_json::to_string outputs depending on version).
        let first_arg_start = body.find("spawn(").unwrap() + "spawn(".len();
        let first_arg_end = first_arg_start + body[first_arg_start..].find(",").unwrap();
        let literal = &body[first_arg_start..first_arg_end];
        let parsed: String = serde_json::from_str(literal).unwrap();
        assert_eq!(parsed, r"C:\Users\홍길동\bin\ccstatusline-rs.exe");
    }

    #[test]
    fn find_latest_backup_picks_highest_filename() {
        let dir = tempfile::tempdir().unwrap();
        let settings = dir.path().join("settings.json");
        std::fs::write(&settings, b"{}").unwrap();
        for ts in [
            "20260514-100000-000",
            "20260514-100001-000",
            "20260514-100000-001",
        ] {
            std::fs::write(
                dir.path().join(format!("settings.json{BACKUP_INFIX}{ts}")),
                b"x",
            )
            .unwrap();
        }
        let latest = find_latest_backup(&settings).unwrap().unwrap();
        assert!(
            latest
                .file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with("20260514-100001-000"),
        );
    }

    #[test]
    fn find_latest_backup_returns_none_when_dir_missing() {
        let settings = std::path::PathBuf::from("/nonexistent/dir/settings.json");
        let result = find_latest_backup(&settings).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn next_backup_filename_skips_existing() {
        // Pin the timestamp so the second call lands in the *same* second
        // regardless of how slow the test runner is — otherwise the counter
        // resets on the next second and `-001` never appears.
        let dir = tempfile::tempdir().unwrap();
        let settings = dir.path().join("settings.json");
        let ts = "20260514-120000";
        let first = next_backup_filename_at(&settings, ts).unwrap();
        std::fs::write(&first, b"x").unwrap();
        let second = next_backup_filename_at(&settings, ts).unwrap();
        assert_ne!(first, second);
        let second_name = second.file_name().unwrap().to_string_lossy().into_owned();
        assert!(second_name.ends_with("-001"));
    }
}
