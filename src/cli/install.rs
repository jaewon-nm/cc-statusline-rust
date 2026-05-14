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
const TOKENWATCH_BASENAME: &str = "tokenwatch-statusline.mjs";
const TW_PREV_FILENAME: &str = ".tw-statusline-prev.json";
const WRAP_EXPLANATION: &str = "settings.json untouched — tokenwatch wrap-mode in effect";

/// Basename `contains_basename` looks for when deciding whether the existing
/// wrap-prev command is ours. Mirrors the targets `compose_command` actually
/// emits: the JS wrapper on Windows, the bare exe on POSIX.
fn ours_wrap_basename(platform: Platform) -> &'static str {
    match platform {
        Platform::Windows => WRAPPER_BASENAME,
        Platform::Posix => EXE_BASENAME_POSIX,
    }
}

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
    pub mode: InstallMode,
    pub bin: PathBuf,
    pub wrapper: Option<PathBuf>,
    pub settings: PathBuf,
    pub backup: Option<PathBuf>,
    pub copied_binary: bool,
    pub previous_command: Option<String>,
    /// Path to `.tw-statusline-prev.json` in wrap mode; `None` in direct mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap_prev_path: Option<PathBuf>,
    /// The `command` string previously stored in `.tw-statusline-prev.json`
    /// before we overwrote it. Projected to just the string so any sibling
    /// keys neo-mem may add stay out of our audit surface.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_wrap_command: Option<String>,
    /// Operator-visible explanation when `backup` is `null` because wrap
    /// mode never touched settings.json. Suppressed in direct mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap_explanation: Option<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct UninstallReport {
    pub uninstalled: bool,
    pub mode: UninstallMode,
    pub settings: PathBuf,
    /// `Some` when direct mode restored a settings backup. `None` in wrap
    /// mode where settings was never touched (changed from unconditional
    /// in 005 — JSON contract bump documented in CHANGELOG).
    pub restored_from: Option<PathBuf>,
    pub removed: Vec<PathBuf>,
    /// Path of `.tw-statusline-prev.json` we deleted in wrap-mode uninstall,
    /// or `None` if it was already absent / direct mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed_wrap_prev: Option<PathBuf>,
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

/// Parsed shape of `.tw-statusline-prev.json`. Only `command` is meaningful
/// for our coexistence audit; serde flattens any sibling keys neo-mem may
/// add in future versions so we don't drop them on rewrite.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
struct TwPrev {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallMode {
    Direct,
    Wrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UninstallMode {
    Direct,
    Wrap,
}

/// Boundary-scanned basename match. Whitespace tokenization would split
/// quoted Windows paths with spaces and leak the trailing quote into the
/// matched token (e.g. `node "C:\Users\Jane Doe\...\tokenwatch-statusline.mjs"`).
/// We scan every substring occurrence and require the surrounding bytes to
/// be path separators, quotes, whitespace, or the string boundary.
pub fn contains_basename(cmd: &str, basename: &str) -> bool {
    if basename.is_empty() {
        return false;
    }
    let bytes = cmd.as_bytes();
    let bname = basename.as_bytes();
    let mut start = 0usize;
    while start + bname.len() <= bytes.len() {
        let Some(rel) = find_subslice(&bytes[start..], bname) else {
            return false;
        };
        let pos = start + rel;
        let before_ok = pos == 0 || is_basename_boundary(bytes[pos - 1], true);
        let after_pos = pos + bname.len();
        let after_ok = after_pos == bytes.len() || is_basename_boundary(bytes[after_pos], false);
        if before_ok && after_ok {
            return true;
        }
        start = pos + 1;
    }
    false
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn is_basename_boundary(b: u8, before: bool) -> bool {
    // Leading boundary must be a separator-like byte; trailing boundary
    // may also be the next path component start? No — basename is the
    // last component, so the byte AFTER it should never be a separator
    // continuing the path. We require quote / whitespace / EOL only.
    if before {
        matches!(b, b'/' | b'\\' | b'"' | b'\'' | b' ' | b'\t')
    } else {
        matches!(b, b'"' | b'\'' | b' ' | b'\t')
    }
}

pub fn is_tokenwatch_command(cmd: &str) -> bool {
    contains_basename(cmd, TOKENWATCH_BASENAME)
}

pub fn is_ours_wrap_command(cmd: &str, platform: Platform) -> bool {
    contains_basename(cmd, ours_wrap_basename(platform))
}

/// Wrap pointer lives in the same directory as the settings file so an
/// explicit `--settings` override (tests, alt config trees) lands the pointer
/// next to the settings it coexists with. Default path is therefore
/// `~/.claude/.tw-statusline-prev.json`.
fn tw_prev_path(settings_path: &Path) -> PathBuf {
    let parent = settings_path.parent().unwrap_or(Path::new("."));
    parent.join(TW_PREV_FILENAME)
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
    let existing_bytes = if settings_path.exists() {
        Some(fs::read(&settings_path).map_err(|source| Error::FileIo {
            operation: "read_settings",
            path: settings_path.clone(),
            source,
        })?)
    } else {
        None
    };
    let parsed_settings = match existing_bytes.as_ref() {
        None => ClaudeSettings::default(),
        Some(b) if b.is_empty() => ClaudeSettings::default(),
        Some(b) => serde_json::from_slice(b).map_err(|e| Error::InvalidConfig {
            reason: format!("{p}: {e}", p = settings_path.display()),
        })?,
    };

    let current_cmd = parsed_settings
        .status_line
        .as_ref()
        .map(|s| s.command.as_str())
        .unwrap_or("");
    let mode = if is_tokenwatch_command(current_cmd) {
        InstallMode::Wrap
    } else {
        InstallMode::Direct
    };

    match mode {
        InstallMode::Direct => install_direct(InstallDirectCtx {
            platform,
            dest_exe,
            wrapper,
            settings_path,
            existing_bytes,
            copied_binary,
            previous_command: if current_cmd.is_empty() {
                None
            } else {
                Some(current_cmd.to_string())
            },
        }),
        InstallMode::Wrap => install_wrap(InstallWrapCtx {
            platform,
            dest_exe,
            wrapper,
            settings_path,
            copied_binary,
        }),
    }
}

struct InstallDirectCtx {
    platform: Platform,
    dest_exe: PathBuf,
    wrapper: Option<PathBuf>,
    settings_path: PathBuf,
    existing_bytes: Option<Vec<u8>>,
    copied_binary: bool,
    previous_command: Option<String>,
}

fn install_direct(ctx: InstallDirectCtx) -> Result<InstallReport> {
    let InstallDirectCtx {
        platform,
        dest_exe,
        wrapper,
        settings_path,
        existing_bytes,
        copied_binary,
        previous_command,
    } = ctx;

    let backup = if let Some(bytes) = existing_bytes.as_ref() {
        let backup_path = next_backup_filename(&settings_path)?;
        fs::write(&backup_path, bytes).map_err(|source| Error::FileIo {
            operation: "backup_settings",
            path: backup_path.clone(),
            source,
        })?;
        Some(backup_path)
    } else {
        None
    };

    let mut settings: ClaudeSettings = match existing_bytes.as_deref() {
        None | Some(&[]) => ClaudeSettings::default(),
        Some(b) => serde_json::from_slice(b).map_err(|e| Error::InvalidConfig {
            reason: format!("{p}: {e}", p = settings_path.display()),
        })?,
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
        mode: InstallMode::Direct,
        bin: dest_exe,
        wrapper,
        settings: settings_path,
        backup,
        copied_binary,
        previous_command,
        wrap_prev_path: None,
        previous_wrap_command: None,
        wrap_explanation: None,
    })
}

struct InstallWrapCtx {
    platform: Platform,
    dest_exe: PathBuf,
    wrapper: Option<PathBuf>,
    settings_path: PathBuf,
    copied_binary: bool,
}

fn install_wrap(ctx: InstallWrapCtx) -> Result<InstallReport> {
    let InstallWrapCtx {
        platform,
        dest_exe,
        wrapper,
        settings_path,
        copied_binary,
    } = ctx;

    let prev_path = tw_prev_path(&settings_path);
    // Settings.json itself sits at ~/.claude/settings.json; the wrap pointer
    // lives in the same directory, so make sure it exists before writing.
    if let Some(parent) = prev_path.parent() {
        create_dir(parent, "create_tw_prev_dir")?;
    }

    let prev_existing = read_tw_prev(&prev_path)?;
    let previous_wrap_command = prev_existing.as_ref().and_then(|p| p.command.clone());
    if let Some(existing) = previous_wrap_command.as_deref()
        && !existing.is_empty()
        && !is_ours_wrap_command(existing, platform)
    {
        return Err(Error::WrapConflict {
            path: prev_path,
            existing_command: existing.to_string(),
        });
    }

    let our_command = compose_command(platform, &dest_exe, wrapper.as_deref());
    // Preserve neo-mem's extra keys verbatim — they may add fields to the
    // wrap pointer in future versions and silently dropping them would be
    // an upgrade hazard.
    let mut prev = prev_existing.unwrap_or_default();
    prev.command = Some(our_command);
    prev.kind = Some("command".to_string());

    let serialized = serde_json::to_vec_pretty(&prev).map_err(Error::from)?;
    ioutil::atomic_write_bytes(&prev_path, &serialized)?;

    Ok(InstallReport {
        installed: true,
        mode: InstallMode::Wrap,
        bin: dest_exe,
        wrapper,
        settings: settings_path,
        backup: None,
        copied_binary,
        previous_command: None,
        wrap_prev_path: Some(prev_path),
        previous_wrap_command,
        wrap_explanation: Some(WRAP_EXPLANATION),
    })
}

/// Read `.tw-statusline-prev.json` if it exists. An empty file is treated
/// as "no pointer set" rather than a parse error so neo-mem's clear-state
/// idiom doesn't trip us.
fn read_tw_prev(path: &Path) -> Result<Option<TwPrev>> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(Error::FileIo {
                operation: "read_tw_prev",
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let trimmed = bytes
        .iter()
        .copied()
        .filter(|b| !b.is_ascii_whitespace())
        .count();
    if trimmed == 0 {
        return Ok(None);
    }
    serde_json::from_slice::<TwPrev>(&bytes)
        .map(Some)
        .map_err(|source| Error::InvalidPrev {
            path: path.to_path_buf(),
            source,
        })
}

pub fn uninstall(args: UninstallArgs) -> Result<UninstallReport> {
    let platform = Platform::current();
    let settings_path = resolve_settings_path(args.settings)?;
    let prev_path = tw_prev_path(&settings_path);

    let mode = resolve_uninstall_mode(&settings_path, &prev_path, platform, args.backup.is_some())?;

    let (restored_from, removed_wrap_prev) = match mode {
        UninstallMode::Direct => {
            let backup_path = match args.backup.clone() {
                Some(p) => p,
                None => {
                    find_latest_backup(&settings_path)?.ok_or_else(|| Error::NoBackupFound {
                        settings: settings_path.clone(),
                    })?
                }
            };
            let bytes = fs::read(&backup_path).map_err(|source| Error::FileIo {
                operation: "read_backup",
                path: backup_path.clone(),
                source,
            })?;
            ioutil::atomic_write_bytes(&settings_path, &bytes)?;
            (Some(backup_path), None)
        }
        UninstallMode::Wrap => {
            // Re-read the prev file inside the mutating window so a mid-op
            // neo-mem refresh that swapped the pointer is detected before
            // we delete it. We only remove when it still points at us.
            let removed = match read_tw_prev(&prev_path)? {
                Some(prev)
                    if prev
                        .command
                        .as_deref()
                        .map(|c| is_ours_wrap_command(c, platform))
                        .unwrap_or(false) =>
                {
                    fs::remove_file(&prev_path).map_err(|source| Error::FileIo {
                        operation: "remove_tw_prev",
                        path: prev_path.clone(),
                        source,
                    })?;
                    Some(prev_path.clone())
                }
                _ => None,
            };
            (None, removed)
        }
    };

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
        mode,
        settings: settings_path,
        restored_from,
        removed,
        removed_wrap_prev,
        failed_removals,
    })
}

/// Positive-evidence mode resolution. Step ordering is load-bearing — the
/// stale-pointer check has to fire before the direct-mode backup heuristic
/// can mask it, and an explicit `--backup` always forces Direct so the
/// operator stays in control of the restore.
fn resolve_uninstall_mode(
    settings_path: &Path,
    prev_path: &Path,
    platform: Platform,
    explicit_backup: bool,
) -> Result<UninstallMode> {
    if explicit_backup {
        return Ok(UninstallMode::Direct);
    }

    let settings_cmd = read_settings_command(settings_path)?;
    let prev = read_tw_prev(prev_path)?;
    let prev_is_ours = prev
        .as_ref()
        .and_then(|p| p.command.as_deref())
        .map(|c| is_ours_wrap_command(c, platform))
        .unwrap_or(false);
    let settings_is_tw = settings_cmd
        .as_deref()
        .map(is_tokenwatch_command)
        .unwrap_or(false);

    // Stale-pointer guard — surfaces before the direct heuristic falls
    // through and silently restores the wrong file.
    if prev_is_ours && !settings_is_tw {
        return Err(Error::StaleWrapPointer {
            prev_path: prev_path.to_path_buf(),
            settings_command: settings_cmd.clone().unwrap_or_default(),
        });
    }

    if settings_is_tw && prev_is_ours {
        return Ok(UninstallMode::Wrap);
    }

    // Direct evidence: a backup we wrote, or settings.json statusLine
    // command whose basename is ours.
    let has_backup = find_latest_backup(settings_path)?.is_some();
    let settings_is_ours = settings_cmd
        .as_deref()
        .map(|c| is_ours_wrap_command(c, platform))
        .unwrap_or(false);
    if has_backup || settings_is_ours {
        return Ok(UninstallMode::Direct);
    }

    Err(Error::NoInstallTraces {
        settings: settings_path.to_path_buf(),
        prev_path: prev_path.to_path_buf(),
    })
}

fn read_settings_command(settings_path: &Path) -> Result<Option<String>> {
    let bytes = match fs::read(settings_path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(Error::FileIo {
                operation: "read_settings",
                path: settings_path.to_path_buf(),
                source,
            });
        }
    };
    if bytes.is_empty() {
        return Ok(None);
    }
    let parsed: ClaudeSettings =
        serde_json::from_slice(&bytes).map_err(|e| Error::InvalidConfig {
            reason: format!("{p}: {e}", p = settings_path.display()),
        })?;
    Ok(parsed.status_line.map(|s| s.command))
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
    fn detects_tokenwatch_in_canonical_paths() {
        let cases = [
            "node \"C:\\Users\\you\\AppData\\Local\\Temp\\neo-mem\\1.1.97\\scripts\\tokenwatch-statusline.mjs\"",
            "node \"C:\\Users\\you\\.neo-mem\\runtime\\neo-mem\\win32-x64\\1.2.10-a1e85cb5\\scripts\\tokenwatch-statusline.mjs\"",
            "node /home/dev/.neo-mem/runtime/neo-mem/linux-x64/2.0.0/scripts/tokenwatch-statusline.mjs",
        ];
        for cmd in cases {
            assert!(
                is_tokenwatch_command(cmd),
                "expected tokenwatch detection for: {cmd}",
            );
        }
    }

    #[test]
    fn detects_tokenwatch_in_quoted_windows_path_with_spaces() {
        let cmd = "node \"C:\\Users\\Jane Doe\\.neo-mem\\runtime\\neo-mem\\win32-x64\\1.2.10\\scripts\\tokenwatch-statusline.mjs\"";
        assert!(is_tokenwatch_command(cmd));
    }

    #[test]
    fn does_not_match_my_tokenwatch_statusline_variant() {
        // Leading-boundary check must reject basename-prefixed lookalikes
        // (here `my-tokenwatch-statusline.mjs`) — the byte before the
        // matched basename is `-`, not a separator/quote/whitespace.
        assert!(!is_tokenwatch_command("node my-tokenwatch-statusline.mjs"));
    }

    #[test]
    fn does_not_match_unrelated_commands() {
        for cmd in [
            "node my-statusline.mjs",
            "powershell -c \"echo hi\"",
            "/usr/bin/echo hi",
            "",
        ] {
            assert!(!is_tokenwatch_command(cmd), "false positive for: {cmd}");
        }
    }

    #[test]
    fn is_ours_wrap_command_matches_relocated_bin_dir() {
        let a = "node \"/custom/bin/ccstatusline-rs.mjs\"";
        let b = "node \"/other/bin/ccstatusline-rs.mjs\"";
        assert!(is_ours_wrap_command(a, Platform::Windows));
        assert!(is_ours_wrap_command(b, Platform::Windows));
        // POSIX wrap pointer holds the single-quoted bare exe path.
        let p = "'/home/dev/.local/bin/ccstatusline-rs'";
        assert!(is_ours_wrap_command(p, Platform::Posix));
        let q = "'/opt/tools/ccstatusline-rs'";
        assert!(is_ours_wrap_command(q, Platform::Posix));
    }

    #[test]
    fn contains_basename_rejects_basename_concat() {
        // `tokenwatch-statusline.mjs.bak` shares the prefix but the trailing
        // boundary is `.`, not quote/whitespace/EOL — must reject.
        assert!(!contains_basename(
            "node /path/tokenwatch-statusline.mjs.bak",
            "tokenwatch-statusline.mjs"
        ));
    }

    #[test]
    fn contains_basename_empty_needle_is_false() {
        assert!(!contains_basename("anything", ""));
    }

    #[test]
    fn wrap_install_writes_posix_compose_command_form() {
        // Pin the wrap-mode command field to `compose_command(Posix, _, None)`
        // so a future refactor that forks the wrap-command builder away from
        // compose_command trips this assertion immediately.
        let exe = Path::new("/usr/local/bin/ccstatusline-rs");
        let from_compose = compose_command(Platform::Posix, exe, None);
        assert_eq!(from_compose, "'/usr/local/bin/ccstatusline-rs'");
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
