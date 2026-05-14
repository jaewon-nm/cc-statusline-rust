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
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::PayloadParse(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
