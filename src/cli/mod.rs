//! CLI surface. The default invocation runs the renderer; everything else
//! is a named subcommand whose stdout is machine-parseable JSON unless
//! `--pretty` is passed.

mod config_cmds;
mod inspect;
pub mod install;
mod preview;
pub mod render;

use clap::{Parser, Subcommand};

use crate::error::Result;

#[derive(Debug, Parser)]
#[command(
    name = "ccstatusline-rs",
    version,
    about = "Agent-friendly status line formatter for Claude Code (Rust port of ccstatusline)",
    long_about = None,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Emit the config JSON Schema (machine-readable shape of the config file).
    Schema {
        /// Indent JSON for human reads. Default is compact one-line output.
        #[arg(long)]
        pretty: bool,
    },
    /// List available widget kinds the renderer understands.
    Widgets {
        /// Indent JSON for human reads.
        #[arg(long)]
        pretty: bool,
    },
    /// Inspect / edit config.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Render against a payload fixture and print the result.
    Preview {
        /// Path to a payload JSON file. Defaults to stdin if not set.
        #[arg(long)]
        payload: Option<std::path::PathBuf>,
        /// Render against a candidate config file instead of the on-disk one.
        #[arg(long)]
        config: Option<std::path::PathBuf>,
        /// Emit JSON with both current + candidate renderings side by side.
        /// Requires `--config` so the candidate has something to diff against.
        #[arg(long)]
        diff: bool,
    },
    /// Drop the binary (and Windows wrapper) into a bin dir and wire the
    /// `statusLine` block of Claude Code's settings file. Atomic and
    /// idempotent; the previous settings are backed up next to the file.
    Install {
        /// Where to drop the binary. Default: ~/bin (Windows), ~/.local/bin (POSIX).
        #[arg(long, value_name = "DIR")]
        bin_dir: Option<std::path::PathBuf>,
        /// Claude Code settings file to patch. Default: ~/.claude/settings.json.
        #[arg(long, value_name = "FILE")]
        settings: Option<std::path::PathBuf>,
        /// Re-copy the binary even when destination bytes already match.
        #[arg(long)]
        force: bool,
        /// Indent the install report JSON for human reads.
        #[arg(long)]
        pretty: bool,
    },
    /// Revert the most recent `install` by restoring the latest backup. With
    /// `--purge-binary` also removes the binary and Windows wrapper from the
    /// chosen bin dir.
    Uninstall {
        /// Settings file to restore. Default: ~/.claude/settings.json.
        #[arg(long, value_name = "FILE")]
        settings: Option<std::path::PathBuf>,
        /// Restore from this specific backup. Default: pick the latest
        /// `*.ccstatusline-rs-bak-*` next to the settings file.
        #[arg(long, value_name = "FILE")]
        backup: Option<std::path::PathBuf>,
        /// Bin dir to scrub when --purge-binary is set. Default matches
        /// `install`'s default.
        #[arg(long, value_name = "DIR")]
        bin_dir: Option<std::path::PathBuf>,
        /// Also delete the binary and Windows wrapper from --bin-dir.
        /// Files we didn't install are left alone.
        #[arg(long)]
        purge_binary: bool,
        /// Indent the uninstall report JSON for human reads.
        #[arg(long)]
        pretty: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Print the current (or default) config as JSON.
    Show {
        /// Indent JSON for human reads.
        #[arg(long)]
        pretty: bool,
    },
    /// Append a widget kind to a line and persist.
    Add {
        /// Widget kind (must match a `widgets` registry entry).
        kind: String,
        /// Target line index (default: last). Use a value equal to the
        /// current line count to start a new line.
        #[arg(long)]
        line: Option<usize>,
        /// Target position within the line (default: append).
        #[arg(long)]
        position: Option<usize>,
    },
    /// Remove a widget by `(line, position)` indices.
    Remove {
        /// Zero-based line index.
        #[arg(long)]
        line: usize,
        /// Zero-based position within the line.
        #[arg(long)]
        position: usize,
    },
    /// Replace the entire on-disk config with the JSON in `--file`.
    Apply {
        /// Path to the candidate config JSON.
        #[arg(long)]
        file: std::path::PathBuf,
    },
    /// Validate the on-disk config or an explicit `--file`. Reports
    /// `{"ok":true}` on pass, `{"ok":false,"errors":[…]}` on failure.
    Validate {
        /// Path to the config JSON to check. Defaults to the on-disk current.
        #[arg(long)]
        file: Option<std::path::PathBuf>,
    },
    /// Set / clear per-widget colors.
    Color {
        /// Widget kind whose styling should change.
        kind: String,
        /// Foreground color. Named (`red`, `bright_blue`) or `#rrggbb`.
        #[arg(long)]
        fg: Option<String>,
        /// Background color, same encoding as `--fg`.
        #[arg(long)]
        bg: Option<String>,
        /// Enable / disable bold. Conflicts with `--no-bold`.
        #[arg(long, conflicts_with = "no_bold")]
        bold: bool,
        /// Explicitly unset bold (different from "absent").
        #[arg(long)]
        no_bold: bool,
        /// Remove the entire colors entry for `<kind>`.
        #[arg(long, conflicts_with_all = ["fg", "bg", "bold", "no_bold"])]
        clear: bool,
    },
}

pub fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        None => render::run_from_stdin(),
        Some(Command::Schema { pretty }) => inspect::schema(pretty),
        Some(Command::Widgets { pretty }) => inspect::widgets(pretty),
        Some(Command::Config { action }) => config_cmds::run(action),
        Some(Command::Preview {
            payload,
            config,
            diff,
        }) => preview::run(payload.as_deref(), config.as_deref(), diff),
        Some(Command::Install {
            bin_dir,
            settings,
            force,
            pretty,
        }) => {
            let report = install::install(install::InstallArgs {
                bin_dir,
                settings,
                force,
            })?;
            install::emit_install_report(&report, pretty)
        }
        Some(Command::Uninstall {
            settings,
            backup,
            bin_dir,
            purge_binary,
            pretty,
        }) => {
            let report = install::uninstall(install::UninstallArgs {
                settings,
                backup,
                bin_dir,
                purge_binary,
            })?;
            install::emit_uninstall_report(&report, pretty)
        }
    }
}
