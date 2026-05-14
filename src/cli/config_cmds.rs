//! `config` subtree. Bootstrap exposes `show` and `validate` against the
//! built-in default — the full edit surface (add/remove/set/apply) lands
//! in M2 when on-disk persistence comes online.

use std::io::{self, Write};

use serde_json::json;

use crate::cli::ConfigAction;
use crate::config::Config;
use crate::error::Result;

pub fn run(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show { pretty } => show(pretty),
        ConfigAction::Validate => validate(),
    }
}

fn show(pretty: bool) -> Result<()> {
    let cfg = Config::default_layout();
    let mut stdout = io::stdout().lock();
    if pretty {
        serde_json::to_writer_pretty(&mut stdout, &cfg)?;
    } else {
        serde_json::to_writer(&mut stdout, &cfg)?;
    }
    stdout.write_all(b"\n")?;
    Ok(())
}

fn validate() -> Result<()> {
    // Bootstrap: the in-memory default is always valid by construction.
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &json!({ "ok": true }))?;
    stdout.write_all(b"\n")?;
    Ok(())
}
