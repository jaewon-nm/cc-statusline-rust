//! CLI surface. The default invocation runs the renderer; everything else
//! is a named subcommand whose stdout is machine-parseable JSON unless
//! `--pretty` is passed.

mod config_cmds;
mod inspect;
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
    /// Emit the config JSON Schema.
    Schema {
        #[arg(long)]
        pretty: bool,
    },
    /// List available widget kinds.
    Widgets {
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
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Print the current (or default) config.
    Show {
        #[arg(long)]
        pretty: bool,
    },
    /// Validate the on-disk config against the schema.
    Validate,
}

pub fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        None => render::run_from_stdin(),
        Some(Command::Schema { pretty }) => inspect::schema(pretty),
        Some(Command::Widgets { pretty }) => inspect::widgets(pretty),
        Some(Command::Config { action }) => config_cmds::run(action),
        Some(Command::Preview { payload }) => preview::run(payload.as_deref()),
    }
}
