//! `preview` — render a payload against the current config or a candidate.
//!
//! With `--diff --config <file>`, emit JSON `{ current, pending, identical }`
//! so an agent can compare a proposed config against the live one before
//! committing it through `config apply`.

use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

use serde_json::json;

use crate::cli::render::render_with;
use crate::config;
use crate::error::{Error, Result};

pub fn run(payload_path: Option<&Path>, config_path: Option<&Path>, diff: bool) -> Result<()> {
    if diff && config_path.is_none() {
        return Err(Error::InvalidConfig {
            reason: "--diff requires --config <file>".into(),
        });
    }

    let raw = load_payload(payload_path)?;
    let current_cfg = config::load_or_default()?;
    let candidate_cfg = match config_path {
        Some(path) => Some(config::load_from(path)?),
        None => None,
    };

    let mut stdout = io::stdout().lock();
    if diff {
        let current = render_with(&raw, &current_cfg)?;
        let pending = render_with(&raw, candidate_cfg.as_ref().expect("guarded above"))?;
        let body = json!({
            "current": current,
            "pending": pending,
            "identical": current == pending,
        });
        serde_json::to_writer(&mut stdout, &body)?;
        stdout.write_all(b"\n")?;
        return Ok(());
    }

    // Non-diff: just render against the chosen config (candidate if provided,
    // else current).
    let cfg = candidate_cfg.unwrap_or(current_cfg);
    let out = render_with(&raw, &cfg)?;
    stdout.write_all(out.as_bytes())?;
    stdout.write_all(b"\n")?;
    Ok(())
}

fn load_payload(path: Option<&Path>) -> Result<String> {
    match path {
        Some(p) => Ok(fs::read_to_string(p)?),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
    }
}
