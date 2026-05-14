//! `config` subtree. Read / mutate / persist the on-disk layout. Every
//! mutation funnels through `Config::validate` so unknown widget kinds and
//! version mismatches never reach disk.

use std::io::{self, Write};

use serde_json::{Value, json};

use crate::cli::ConfigAction;
use crate::config::{self, Config};
use crate::error::{Error, Result};
use crate::widgets;

pub fn run(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show { pretty } => show(pretty),
        ConfigAction::Add {
            kind,
            line,
            position,
        } => add(kind, line, position),
        ConfigAction::Remove { line, position } => remove(line, position),
        ConfigAction::Apply { file } => apply(&file),
        ConfigAction::Validate { file } => validate(file.as_deref()),
        ConfigAction::Color {
            kind,
            fg,
            bg,
            bold,
            no_bold,
            clear,
        } => color(kind, fg, bg, bold, no_bold, clear),
    }
}

fn show(pretty: bool) -> Result<()> {
    let cfg = config::load_or_default()?;
    emit_value(&serde_json::to_value(&cfg)?, pretty)
}

fn add(kind: String, line: Option<usize>, position: Option<usize>) -> Result<()> {
    let known: Vec<&str> = widgets::REGISTRY.iter().map(|w| w.kind).collect();
    if !known.contains(&kind.as_str()) {
        return Err(Error::InvalidConfig {
            reason: format!("unknown widget kind '{kind}'"),
        });
    }
    let mut cfg = config::load_or_default()?;

    // Resolve target line. `None` → last existing line; equal to len → new
    // empty line; beyond → error so the agent can correct.
    let line_idx = match line {
        None => {
            if cfg.lines.is_empty() {
                cfg.lines.push(Vec::new());
            }
            cfg.lines.len() - 1
        }
        Some(idx) if idx == cfg.lines.len() => {
            cfg.lines.push(Vec::new());
            idx
        }
        Some(idx) if idx < cfg.lines.len() => idx,
        Some(idx) => {
            return Err(Error::InvalidConfig {
                reason: format!(
                    "line {idx} out of range (have {n} lines, use {n} to create a new line)",
                    n = cfg.lines.len(),
                ),
            });
        }
    };

    let target = &mut cfg.lines[line_idx];
    match position {
        None => target.push(kind),
        Some(pos) if pos <= target.len() => target.insert(pos, kind),
        Some(pos) => {
            return Err(Error::InvalidConfig {
                reason: format!(
                    "position {pos} out of range on line {line_idx} ({} widgets)",
                    target.len(),
                ),
            });
        }
    }

    persist(&cfg)?;
    emit_value(&serde_json::to_value(&cfg)?, false)
}

fn remove(line: usize, position: usize) -> Result<()> {
    let mut cfg = config::load_or_default()?;
    if line >= cfg.lines.len() {
        return Err(Error::InvalidConfig {
            reason: format!("line {line} out of range ({} lines)", cfg.lines.len()),
        });
    }
    let row_len = cfg.lines[line].len();
    if position >= row_len {
        return Err(Error::InvalidConfig {
            reason: format!("position {position} out of range on line {line} ({row_len} widgets)"),
        });
    }
    cfg.lines[line].remove(position);
    persist(&cfg)?;
    emit_value(&serde_json::to_value(&cfg)?, false)
}

fn apply(file: &std::path::Path) -> Result<()> {
    let cfg = config::load_from(file)?;
    let known: Vec<&str> = widgets::REGISTRY.iter().map(|w| w.kind).collect();
    cfg.validate(&known)?;
    persist(&cfg)?;
    emit_value(&serde_json::to_value(&cfg)?, false)
}

fn validate(file: Option<&std::path::Path>) -> Result<()> {
    let cfg = match file {
        Some(path) => config::load_from(path)?,
        None => config::load_or_default()?,
    };
    let known: Vec<&str> = widgets::REGISTRY.iter().map(|w| w.kind).collect();
    match cfg.validate(&known) {
        Ok(()) => emit_value(&json!({ "ok": true }), false),
        Err(Error::InvalidConfig { reason }) => {
            emit_value(&json!({ "ok": false, "errors": [reason] }), false)
        }
        Err(err) => Err(err),
    }
}

fn color(
    kind: String,
    fg: Option<String>,
    bg: Option<String>,
    bold: bool,
    no_bold: bool,
    clear: bool,
) -> Result<()> {
    let known: Vec<&str> = widgets::REGISTRY.iter().map(|w| w.kind).collect();
    if !known.contains(&kind.as_str()) {
        return Err(Error::InvalidConfig {
            reason: format!("unknown widget kind '{kind}'"),
        });
    }
    let mut cfg = config::load_or_default()?;

    if clear {
        cfg.colors.remove(&kind);
    } else {
        if fg.is_none() && bg.is_none() && !bold && !no_bold {
            return Err(Error::InvalidConfig {
                reason: "specify at least one of --fg / --bg / --bold / --no-bold / --clear".into(),
            });
        }
        // Validate color strings before touching the on-disk config.
        for (field, value) in [("fg", fg.as_deref()), ("bg", bg.as_deref())] {
            if let Some(v) = value
                && let Err(err) = crate::render::color::parse_color(v)
            {
                return Err(Error::InvalidConfig {
                    reason: format!("{field}: {err}"),
                });
            }
        }
        let entry = cfg
            .colors
            .entry(kind.clone())
            .or_insert_with(Default::default);
        if let Some(v) = fg {
            entry.fg = Some(v);
        }
        if let Some(v) = bg {
            entry.bg = Some(v);
        }
        if bold {
            entry.bold = Some(true);
        } else if no_bold {
            entry.bold = Some(false);
        }
        if entry.is_empty() {
            cfg.colors.remove(&kind);
        }
    }

    persist(&cfg)?;
    emit_value(&serde_json::to_value(&cfg)?, false)
}

fn persist(cfg: &Config) -> Result<()> {
    let Some(path) = config::config_path() else {
        return Err(Error::InvalidConfig {
            reason: "no resolvable config path on this platform".into(),
        });
    };
    cfg.save(&path)
}

fn emit_value(value: &Value, pretty: bool) -> Result<()> {
    let mut stdout = io::stdout().lock();
    if pretty {
        serde_json::to_writer_pretty(&mut stdout, value)?;
    } else {
        serde_json::to_writer(&mut stdout, value)?;
    }
    stdout.write_all(b"\n")?;
    Ok(())
}
