//! Default invocation: read payload JSON from stdin, render, write stdout.

use std::io::{self, Read, Write};

use crate::context::{Context, payload::Payload};
use crate::error::Result;
use crate::render;

pub fn run_from_stdin() -> Result<()> {
    let mut raw = String::new();
    io::stdin().read_to_string(&mut raw)?;
    let payload: Payload = if raw.trim().is_empty() {
        Payload::default()
    } else {
        serde_json::from_str(&raw)?
    };
    let tz = Context::resolve_tz(None)?;
    let ctx = Context::from_payload(&payload, tz)?;
    let out = render::render_default(&ctx);
    io::stdout().write_all(out.as_bytes())?;
    io::stdout().write_all(b"\n")?;
    Ok(())
}

pub fn render_string(raw: &str) -> Result<String> {
    let payload: Payload = if raw.trim().is_empty() {
        Payload::default()
    } else {
        serde_json::from_str(raw)?
    };
    let tz = Context::resolve_tz(None)?;
    let ctx = Context::from_payload(&payload, tz)?;
    Ok(render::render_default(&ctx))
}
