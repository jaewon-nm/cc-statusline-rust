//! Default invocation: read payload JSON from stdin, render, write stdout.

use std::io::{self, Read, Write};
use std::path::Path;

use crate::config::{self, Config};
use crate::context::{Context, payload::Payload};
use crate::error::Result;
use crate::render;
use crate::render::color::ColorMode;

pub fn run_from_stdin() -> Result<()> {
    let mut raw = String::new();
    io::stdin().read_to_string(&mut raw)?;
    let cfg = config::load_or_default()?;
    let out = render_with(&raw, &cfg)?;
    io::stdout().write_all(out.as_bytes())?;
    io::stdout().write_all(b"\n")?;
    Ok(())
}

/// Test / library entry point that bypasses on-disk config loading. Always
/// renders against the built-in default theme so snapshot tests stay stable
/// regardless of the developer's local config.
pub fn render_string(raw: &str) -> Result<String> {
    render_with(raw, &Config::default_layout())
}

/// Render against an explicit config — same path the real CLI takes after
/// loading from disk. Defers to [`render_with_mode`] under `ColorMode::Auto`.
pub fn render_with(raw: &str, cfg: &Config) -> Result<String> {
    render_with_mode(raw, cfg, ColorMode::Auto)
}

/// Test seam: render an explicit config under an explicit color mode.
/// Snapshot tests pin `Always`; plain-text invariant tests pin `Never` so
/// the output is deterministic regardless of the developer's local
/// `NO_COLOR` / `FORCE_COLOR` environment.
pub fn render_with_mode(raw: &str, cfg: &Config, mode: ColorMode) -> Result<String> {
    let payload: Payload = if raw.trim().is_empty() {
        Payload::default()
    } else {
        serde_json::from_str(raw)?
    };
    let tz = Context::resolve_tz(cfg.tz.as_deref())?;
    let mut ctx = Context::from_payload(&payload, tz)?;
    if cfg.needs_git()
        && let Some(cwd) = ctx.cwd.clone()
    {
        ctx = ctx.with_git(Path::new(&cwd));
    }
    Ok(render::render_with_mode(&ctx, cfg, mode))
}
