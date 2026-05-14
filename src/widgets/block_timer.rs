//! 5-hour block timer. The label `5h` is a layout constant — it names the
//! window, not the elapsed time. Multi-segment: icon + label dimmed,
//! filled cells tier-colored, percent dimmed, reset clock default.

use anstyle::{AnsiColor, Style};

use crate::context::Context;
use crate::render::Segment;
use crate::render::color::bar_tier_color;
use crate::render::format::{bar_filled_count, format_clock, format_percent_paren};

const WINDOW_LABEL: &str = "5h";
const BAR_WIDTH: usize = 10;
const FILLED_CHAR: char = '#';
const EMPTY_CHAR: char = '.';

pub fn render(ctx: &Context) -> Option<Vec<Segment>> {
    let t = ctx.block.as_ref()?;
    Some(timer_segments(
        t.used_percent,
        format_clock(t.resets_at, &ctx.tz),
        "⏱",
        WINDOW_LABEL,
    ))
}

pub(super) fn timer_segments(
    percent: f64,
    reset_clock: String,
    icon: &str,
    window_label: &str,
) -> Vec<Segment> {
    let dim = Style::new().fg_color(Some(AnsiColor::BrightBlack.into()));
    let tier = Style::new().fg_color(Some(bar_tier_color(percent)));
    let filled = bar_filled_count(percent, BAR_WIDTH);
    let empty = BAR_WIDTH - filled;
    let pct = format_percent_paren(percent);

    let mut out = Vec::with_capacity(8);
    out.push(Segment::styled(format!("{icon} {window_label} "), dim));
    out.push(Segment::plain("["));
    if filled > 0 {
        out.push(Segment::styled(
            FILLED_CHAR.to_string().repeat(filled),
            tier,
        ));
    }
    if empty > 0 {
        out.push(Segment::plain(EMPTY_CHAR.to_string().repeat(empty)));
    }
    out.push(Segment::plain("]"));
    out.push(Segment::styled(pct, dim));
    out.push(Segment::plain(format!(" ↻ {reset_clock}")));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::TimerMetrics;
    use jiff::tz::TimeZone;

    fn ctx_with(pct: f64, epoch_seconds: i64) -> Context {
        Context {
            model_display: None,
            cwd: None,
            context_metrics: None,
            session_tokens: None,
            session_cost_usd: None,
            block: Some(TimerMetrics {
                used_percent: pct,
                resets_at: jiff::Timestamp::from_second(epoch_seconds).unwrap(),
            }),
            weekly: None,
            git: None,
            tz: TimeZone::get("Asia/Seoul").unwrap(),
        }
    }

    fn joined(segs: &[Segment]) -> String {
        segs.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn renders_default_theme() {
        // 2026-05-14 12:00 KST.
        let epoch = jiff::civil::date(2026, 5, 14)
            .at(12, 0, 0, 0)
            .to_zoned(TimeZone::get("Asia/Seoul").unwrap())
            .unwrap()
            .timestamp()
            .as_second();
        let segs = render(&ctx_with(21.0, epoch)).unwrap();
        assert_eq!(joined(&segs), "⏱ 5h [##........](21%) ↻ 12:00");
    }

    #[test]
    fn icon_label_uses_bright_black_dim() {
        let epoch = 0;
        let segs = render(&ctx_with(10.0, epoch)).unwrap();
        // First segment is "⏱ 5h ".
        assert_eq!(
            segs[0].style.get_fg_color(),
            Some(AnsiColor::BrightBlack.into()),
        );
    }

    #[test]
    fn missing_timer_yields_none() {
        let mut ctx = ctx_with(0.0, 0);
        ctx.block = None;
        assert!(render(&ctx).is_none());
    }
}
