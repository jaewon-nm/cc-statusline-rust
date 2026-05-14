//! ccstatusline-rs — agent-friendly status line formatter for Claude Code.
//!
//! Public re-exports are minimal and tracked here; integration tests reach in
//! through these names so the surface stays auditable.

pub mod cli;
pub mod config;
pub mod context;
pub mod error;
pub mod ioutil;
pub mod render;
pub mod widgets;

pub use cli::render::{render_string, render_with};
pub use error::{Error, Result};
