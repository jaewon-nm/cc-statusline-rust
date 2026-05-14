//! `schema` and `widgets` — agent-discovery surfaces. Both emit JSON on
//! stdout; `--pretty` opts into indented output for human reads.

use std::io::{self, Write};

use serde_json::{Value, json};

use crate::config::Config;
use crate::error::Result;
use crate::widgets;

pub fn schema(pretty: bool) -> Result<()> {
    emit(Config::schema(), pretty)
}

pub fn widgets(pretty: bool) -> Result<()> {
    let entries: Vec<Value> = widgets::REGISTRY
        .iter()
        .map(|w| json!({ "kind": w.kind }))
        .collect();
    emit(Value::Array(entries), pretty)
}

fn emit(value: Value, pretty: bool) -> Result<()> {
    let mut stdout = io::stdout().lock();
    if pretty {
        serde_json::to_writer_pretty(&mut stdout, &value)?;
    } else {
        serde_json::to_writer(&mut stdout, &value)?;
    }
    stdout.write_all(b"\n")?;
    Ok(())
}
