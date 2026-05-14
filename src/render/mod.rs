//! Segment composition → ANSI line(s). Segments are data; styling enters here
//! so widgets stay pure and snapshot-test boundaries are clean.

pub mod format;

use anstyle::{Reset, Style};

use crate::config::Config;
use crate::context::Context;
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
    render(ctx, &Config::default_layout().lines)
}

/// Render an arbitrary layout. Empty lines (no widget produced output) are
/// dropped so the final string never contains a blank separator row.
pub fn render(ctx: &Context, lines: &[Vec<String>]) -> String {
    let assembled = lines
        .iter()
        .map(|row| build_line(ctx, row))
        .collect::<Vec<_>>();
    emit(&assembled)
}

fn build_line<S: AsRef<str>>(ctx: &Context, widget_kinds: &[S]) -> Line {
    let mut line = Line::default();
    let mut first = true;
    for kind in widget_kinds {
        let Some(spec) = widgets::find(kind.as_ref()) else {
            continue;
        };
        let Some(segment) = (spec.render)(ctx) else {
            continue;
        };
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
