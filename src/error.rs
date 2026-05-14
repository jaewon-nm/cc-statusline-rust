//! Single typed error surface for the library.
//!
//! `anyhow` lives only in `main.rs`. Foreign errors enter through `#[from]`
//! conversions — never stringified — so callers can match on the variant.

use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("payload JSON parse failed: {0}")]
    PayloadParse(#[source] serde_json::Error),

    #[error("stdin read failed")]
    Stdin(#[from] io::Error),

    #[error("invalid timezone: {name}")]
    InvalidTimezone { name: String },

    #[error("config invalid: {reason}")]
    InvalidConfig { reason: String },

    #[error("layout invariant violated: {message}")]
    LayoutInvariant { message: &'static str },

    #[error("probe '{name}' timed out after {ms}ms")]
    ProbeTimeout { name: &'static str, ms: u64 },

    #[error("probe '{name}' failed: {reason}")]
    ProbeFailed { name: &'static str, reason: String },

    #[error("file operation '{operation}' failed for {path:?}: {source}")]
    FileIo {
        operation: &'static str,
        path: std::path::PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("backup not found alongside {settings:?}")]
    NoBackupFound { settings: std::path::PathBuf },

    #[error(
        "tokenwatch wrap pointer at {path:?} already references a non-ccstatusline-rs command: {existing_command}"
    )]
    WrapConflict {
        path: std::path::PathBuf,
        existing_command: String,
    },

    #[error(
        "no install traces found — neither a settings backup nor a tokenwatch wrap pointer at {prev_path:?} points at us (settings: {settings:?})"
    )]
    NoInstallTraces {
        settings: std::path::PathBuf,
        prev_path: std::path::PathBuf,
    },

    #[error(
        "stale wrap pointer: {prev_path:?} references our binary but settings statusLine.command is no longer tokenwatch ({settings_command})"
    )]
    StaleWrapPointer {
        prev_path: std::path::PathBuf,
        settings_command: String,
    },

    #[error("tokenwatch wrap pointer at {path:?} is not valid JSON")]
    InvalidPrev {
        path: std::path::PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::PayloadParse(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
