//! Weekly (7-day) reset timer. The label `7d` is a layout constant.

use crate::context::Context;
use crate::render::Segment;
use crate::render::format::{format_bar_wrapped, format_percent_paren, format_weekly_reset};

const WINDOW_LABEL: &str = "7d";

pub fn render(ctx: &Context) -> Option<Segment> {
    let t = ctx.weekly.as_ref()?;
    let bar = format_bar_wrapped(t.used_percent, 10, '#', '.');
    let pct = format_percent_paren(t.used_percent);
    let when = format_weekly_reset(t.resets_at, &ctx.tz);
    Some(Segment::plain(format!(
        "📅 {WINDOW_LABEL} {bar}{pct} ↻ {when}"
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
            block: None,
            weekly: Some(TimerMetrics {
                used_percent: pct,
                resets_at: jiff::Timestamp::from_second(epoch_seconds).unwrap(),
            }),
            tz: TimeZone::get("Asia/Seoul").unwrap(),
        }
    }

    #[test]
    fn renders_default_theme() {
        // 2026-05-19 06:00 KST.
        let epoch = jiff::civil::date(2026, 5, 19)
            .at(6, 0, 0, 0)
            .to_zoned(TimeZone::get("Asia/Seoul").unwrap())
            .unwrap()
            .timestamp()
            .as_second();
        let s = render(&ctx_with(20.0, epoch)).unwrap();
        assert_eq!(s.text, "📅 7d [##........](20%) ↻ 5/19 06:00");
    }

    #[test]
    fn missing_timer_yields_none() {
        let mut ctx = ctx_with(0.0, 0);
        ctx.weekly = None;
        assert!(render(&ctx).is_none());
    }
}
