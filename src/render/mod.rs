//! Segment composition → ANSI line(s). Segments are data; styling enters here
//! so widgets stay pure and snapshot-test boundaries are clean.

pub mod color;
pub mod format;

use anstyle::{Reset, Style};

use crate::config::Config;
use crate::context::Context;
use crate::render::color::color_enabled;
use crate::widgets;

#[derive(Debug, Clone)]
pub struct Segment {
    pub text: String,
    pub style: Style,
}

impl Segment {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: Style::new(),
        }
    }
}

/// Pre-joined widget output for one logical row plus any styling state.
#[derive(Debug, Clone, Default)]
pub struct Line {
    pub segments: Vec<Segment>,
}

impl Line {
    pub fn push(&mut self, segment: Segment) {
        self.segments.push(segment);
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

const INTER_WIDGET_SEPARATOR: &str = " | ";

/// Render the default layout. Two lines, ` | ` separator, no trailing newline.
pub fn render_default(ctx: &Context) -> String {
    render(ctx, &Config::default_layout())
}

/// Render against an explicit config. Empty lines (no widget produced output)
/// are dropped so the final string never contains a blank separator row.
/// Per-widget styling is applied here so widgets stay pure.
pub fn render(ctx: &Context, cfg: &Config) -> String {
    let color_on = color_enabled(false) && !cfg.colors.is_empty();
    let assembled = cfg
        .lines
        .iter()
        .map(|row| build_line(ctx, row, cfg, color_on))
        .collect::<Vec<_>>();
    emit(&assembled)
}

fn build_line<S: AsRef<str>>(
    ctx: &Context,
    widget_kinds: &[S],
    cfg: &Config,
    color_on: bool,
) -> Line {
    let mut line = Line::default();
    let mut first = true;
    for kind in widget_kinds {
        let kind_str = kind.as_ref();
        let Some(spec) = widgets::find(kind_str) else {
            continue;
        };
        let Some(mut segment) = (spec.render)(ctx) else {
            continue;
        };
        if color_on
            && let Some(style_cfg) = cfg.colors.get(kind_str)
            && let Ok(style) = style_cfg.to_style()
        {
            segment.style = style;
        }
        if !first {
            line.push(Segment::plain(INTER_WIDGET_SEPARATOR));
        }
        line.push(segment);
        first = false;
    }
    line
}

fn emit(lines: &[Line]) -> String {
    let mut out = String::new();
    let mut first_line = true;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if !first_line {
            out.push('\n');
        }
        first_line = false;
        for seg in &line.segments {
            if seg.style == Style::new() {
                out.push_str(&seg.text);
            } else {
                out.push_str(&seg.style.render().to_string());
                out.push_str(&seg.text);
                out.push_str(&Reset.render().to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_joins_lines_with_newline_no_trailing() {
        let lines = vec![
            Line {
                segments: vec![Segment::plain("a")],
            },
            Line {
                segments: vec![Segment::plain("b")],
            },
        ];
        let s = emit(&lines);
        assert_eq!(s, "a\nb");
    }

    #[test]
    fn emit_skips_empty_lines() {
        let lines = vec![
            Line {
                segments: vec![Segment::plain("a")],
            },
            Line::default(),
            Line {
                segments: vec![Segment::plain("b")],
            },
        ];
        let s = emit(&lines);
        assert_eq!(s, "a\nb");
    }

    #[test]
    fn styled_segment_wraps_with_reset() {
        let style = Style::new().bold();
        let lines = vec![Line {
            segments: vec![Segment {
                text: "x".into(),
                style,
            }],
        }];
        let s = emit(&lines);
        assert!(s.contains("x"));
        assert!(s.starts_with("\x1b["));
        assert!(s.ends_with("\x1b[0m"));
    }
}
