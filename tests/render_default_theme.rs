//! Golden snapshot for the default theme.
//!
//! Two layers of guarantee:
//! 1. **Underlying text invariant** — `default_theme_underlying_text` strips
//!    color via `ColorMode::Never` and pins the literal text bytes. Any
//!    refactor that changes a single visible character has to update this.
//! 2. **Colored snapshot** — `default_theme_snapshot` pins the full ANSI
//!    output under `ColorMode::Always`. Any tier color / theme palette
//!    change rolls the snapshot forward in the same commit.
//!
//! Both tests bypass env (`NO_COLOR` / `FORCE_COLOR`) via the programmatic
//! `ColorMode` seam so developer shells can't break the suite.

use ccstatusline_rs::config::Config;
use ccstatusline_rs::{ColorMode, render_with_mode};

const FIXTURE: &str = include_str!("fixtures/default-payload.json");

#[test]
fn default_theme_underlying_text() {
    let cfg = Config::default_layout();
    let out = render_with_mode(FIXTURE, &cfg, ColorMode::Never).expect("render succeeds");
    let expected = concat!(
        "✦ [Opus 4.7 (1M context)] | 📂 F:\\Works\\naya\\cc-statusline-rust",
        " | 🔋 [..........] 80.0K/1.0M(8%) | 📊 85.3K | 💰 $2.55\n",
        "⏱ 5h [##........](21%) ↻ 12:00 | 📅 7d [##........](20%) ↻ 5/19 06:00",
    );
    assert_eq!(out, expected);
}

#[test]
fn default_theme_snapshot() {
    let cfg = Config::default_layout();
    let out = render_with_mode(FIXTURE, &cfg, ColorMode::Always).expect("render succeeds");
    insta::assert_snapshot!(out);
}

#[test]
fn deserialized_config_without_colors_still_paints_theme() {
    // Round-trip simulates "user has an older config saved before milestone
    // 006 landed" — no `colors` key in the JSON. The widget code carries
    // the theme styles itself, so the rendered output must still contain
    // cyan + bold (the model widget's theme default) without any user
    // override or explicit `colors` map.
    let cfg_json = serde_json::json!({
        "version": 1,
        "lines": [["model"]]
    });
    let cfg: Config = serde_json::from_value(cfg_json).expect("deserialize succeeds");
    assert!(cfg.colors.is_empty(), "colors map must round-trip as empty");
    let out = render_with_mode(FIXTURE, &cfg, ColorMode::Always).expect("render succeeds");
    // Bold escape (`\x1b[1m`) is part of the model widget's theme style.
    assert!(
        out.contains("\x1b[1") || out.contains("\x1b[36"),
        "expected ANSI for cyan/bold in: {out}",
    );
    assert!(out.contains("Opus 4.7"));
}
