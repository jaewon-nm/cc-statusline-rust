//! Configurable per-widget styling.
//!
//! Config stores `ColorStyle` as strings (`"red"`, `"bright_blue"`,
//! `"#1abc9c"`) so it round-trips cleanly through JSON. We parse to
//! `anstyle::Color` only at render time, surfacing invalid values through
//! `parse_color` so `config validate` can reject them before the renderer
//! ever sees them.
//!
//! Env knobs:
//! - `NO_COLOR` (any non-empty value) → strip all styling.
//! - `FORCE_COLOR` / `CLICOLOR_FORCE` → emit styling even when stdout looks
//!   like a pipe (Claude Code always pipes, so this is the lever to enable
//!   color in production).

use anstyle::{AnsiColor, Color, RgbColor, Style};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct ColorStyle {
    /// Foreground color. Named (`"red"`, `"bright_blue"`) or `#RRGGBB`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fg: Option<String>,
    /// Background color, same encoding as `fg`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bg: Option<String>,
    /// Bold attribute. `None` = unset; `false` = explicitly off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
}

impl ColorStyle {
    pub fn is_empty(&self) -> bool {
        self.fg.is_none() && self.bg.is_none() && self.bold.is_none()
    }

    /// Build a concrete `anstyle::Style`. Caller is responsible for the
    /// `NO_COLOR` gate — see [`color_enabled`].
    pub fn to_style(&self) -> Result<Style, String> {
        let mut style = Style::new();
        if let Some(fg) = &self.fg {
            style = style.fg_color(Some(parse_color(fg)?));
        }
        if let Some(bg) = &self.bg {
            style = style.bg_color(Some(parse_color(bg)?));
        }
        if self.bold == Some(true) {
            style = style.bold();
        }
        Ok(style)
    }
}

/// `"red"` / `"bright_blue"` / `"#aabbcc"` → `anstyle::Color`. Empty / unknown
/// names return `Err` so callers (`config color`, `config validate`) reject
/// bad input loudly instead of silently dropping styling at render time.
pub fn parse_color(s: &str) -> Result<Color, String> {
    let t = s.trim();
    if let Some(hex) = t.strip_prefix('#') {
        return parse_hex(hex).ok_or_else(|| format!("invalid hex color '{s}'"));
    }
    match t.to_ascii_lowercase().as_str() {
        "black" => Ok(AnsiColor::Black.into()),
        "red" => Ok(AnsiColor::Red.into()),
        "green" => Ok(AnsiColor::Green.into()),
        "yellow" => Ok(AnsiColor::Yellow.into()),
        "blue" => Ok(AnsiColor::Blue.into()),
        "magenta" | "purple" => Ok(AnsiColor::Magenta.into()),
        "cyan" => Ok(AnsiColor::Cyan.into()),
        "white" | "grey" | "gray" => Ok(AnsiColor::White.into()),
        "bright_black" | "bright-black" | "dark_grey" | "dark_gray" => {
            Ok(AnsiColor::BrightBlack.into())
        }
        "bright_red" | "bright-red" => Ok(AnsiColor::BrightRed.into()),
        "bright_green" | "bright-green" => Ok(AnsiColor::BrightGreen.into()),
        "bright_yellow" | "bright-yellow" => Ok(AnsiColor::BrightYellow.into()),
        "bright_blue" | "bright-blue" => Ok(AnsiColor::BrightBlue.into()),
        "bright_magenta" | "bright-magenta" => Ok(AnsiColor::BrightMagenta.into()),
        "bright_cyan" | "bright-cyan" => Ok(AnsiColor::BrightCyan.into()),
        "bright_white" | "bright-white" => Ok(AnsiColor::BrightWhite.into()),
        other => Err(format!("unknown color '{other}'")),
    }
}

fn parse_hex(hex: &str) -> Option<Color> {
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(RgbColor(r, g, b)))
}

/// Decide whether styling should be emitted given the env. Returns `false`
/// when `NO_COLOR` is set; `true` when `FORCE_COLOR` / `CLICOLOR_FORCE` is
/// set; otherwise `default`. `FORCE_COLOR` is checked directly because
/// `anstyle-query` only covers `CLICOLOR_FORCE` — the renderer should honor
/// both since both are widely used.
pub fn color_enabled(default: bool) -> bool {
    if anstyle_query::no_color() {
        return false;
    }
    if anstyle_query::clicolor_force() {
        return true;
    }
    if let Ok(v) = std::env::var("FORCE_COLOR")
        && !v.is_empty()
        && v != "0"
    {
        return true;
    }
    default
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_named() {
        assert_eq!(parse_color("red").unwrap(), AnsiColor::Red.into());
        assert_eq!(
            parse_color("bright_blue").unwrap(),
            AnsiColor::BrightBlue.into(),
        );
        assert_eq!(parse_color("Cyan").unwrap(), AnsiColor::Cyan.into());
    }

    #[test]
    fn parses_hex() {
        assert_eq!(
            parse_color("#ff8800").unwrap(),
            Color::Rgb(RgbColor(0xff, 0x88, 0x00)),
        );
    }

    #[test]
    fn rejects_unknown() {
        assert!(parse_color("burnt sienna").is_err());
        assert!(parse_color("#xyzxyz").is_err());
        assert!(parse_color("#abc").is_err());
    }

    #[test]
    fn empty_color_style_is_empty() {
        assert!(ColorStyle::default().is_empty());
        let cs = ColorStyle {
            fg: Some("red".into()),
            ..ColorStyle::default()
        };
        assert!(!cs.is_empty());
    }

    #[test]
    fn to_style_round_trip() {
        let cs = ColorStyle {
            fg: Some("red".into()),
            bg: Some("#003366".into()),
            bold: Some(true),
        };
        let style = cs.to_style().unwrap();
        assert_eq!(style.get_fg_color(), Some(AnsiColor::Red.into()));
        assert!(style.get_bg_color().is_some());
        assert!(style.get_effects().contains(anstyle::Effects::BOLD));
    }

    #[test]
    fn no_color_env_overrides_default() {
        // SAFETY: Cargo runs tests in-process; mutating env affects siblings
        // running in parallel. The CI test runner is single-threaded for our
        // suite (nextest spawns per test) so this is safe enough; still keep
        // the scope tight.
        // SAFETY: see comment above on env mutation.
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }
        assert!(!color_enabled(true));
        // SAFETY: see comment above on env mutation.
        unsafe {
            std::env::remove_var("NO_COLOR");
        }
    }
}
