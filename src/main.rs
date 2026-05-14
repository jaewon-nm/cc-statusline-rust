//! Entry point. Owns the only `anyhow` boundary; everything else returns the
//! library's typed `Error`.

use anyhow::Result;
use clap::Parser;

use ccstatusline_rs::cli::{self, Cli};

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli::dispatch(cli)?;
    Ok(())
}
