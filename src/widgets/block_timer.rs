//! 5-hour block timer. The label `5h` is a layout constant — it names the
//! window, not the elapsed time.

use crate::context::Context;
use crate::render::Segment;
use crate::render::format::{format_bar_wrapped, format_clock, format_percent_paren};

const WINDOW_LABEL: &str = "5h";

pub fn render(ctx: &Context) -> Option<Segment> {
    let t = ctx.block.as_ref()?;
    let bar = format_bar_wrapped(t.used_percent, 10, '#', '.');
    let pct = format_percent_paren(t.used_percent);
    let clock = format_clock(t.resets_at, &ctx.tz);
    Some(Segment::plain(format!(
        "⏱ {WINDOW_LABEL} {bar}{pct} ↻ {clock}"
    )))
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
            tz: TimeZone::get("Asia/Seoul").unwrap(),
        }
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
        let s = render(&ctx_with(21.0, epoch)).unwrap();
        assert_eq!(s.text, "⏱ 5h [##........](21%) ↻ 12:00");
    }

    #[test]
    fn missing_timer_yields_none() {
        let mut ctx = ctx_with(0.0, 0);
        ctx.block = None;
        assert!(render(&ctx).is_none());
    }
}
