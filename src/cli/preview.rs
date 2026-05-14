//! `preview` — render against a payload file or stdin without going through
//! the renderer's default invocation path. Useful when an agent wants to
//! inspect a candidate config against fixture data.

use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

use crate::cli::render::render_string;
use crate::error::Result;

pub fn run(payload_path: Option<&Path>) -> Result<()> {
    let raw = match payload_path {
        Some(path) => fs::read_to_string(path)?,
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };
    let out = render_string(&raw)?;
    let mut stdout = io::stdout().lock();
    stdout.write_all(out.as_bytes())?;
    stdout.write_all(b"\n")?;
    Ok(())
}
