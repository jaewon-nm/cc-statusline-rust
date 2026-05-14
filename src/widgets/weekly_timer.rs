//! Weekly (7-day) reset timer. Same composition as `block_timer` but with
//! `M/d HH:mm` reset format and `7d` window label.

use crate::context::Context;
use crate::render::Segment;
use crate::render::format::format_weekly_reset;
use crate::widgets::block_timer;

const WINDOW_LABEL: &str = "7d";

pub fn render(ctx: &Context) -> Option<Vec<Segment>> {
    let t = ctx.weekly.as_ref()?;
    Some(block_timer::timer_segments(
        t.used_percent,
        format_weekly_reset(t.resets_at, &ctx.tz),
        "📅",
        WINDOW_LABEL,
    ))
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
            git: None,
            tz: TimeZone::get("Asia/Seoul").unwrap(),
        }
    }

    fn joined(segs: &[Segment]) -> String {
        segs.iter().map(|s| s.text.as_str()).collect()
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
        let segs = render(&ctx_with(20.0, epoch)).unwrap();
        assert_eq!(joined(&segs), "📅 7d [##........](20%) ↻ 5/19 06:00");
    }

    #[test]
    fn missing_timer_yields_none() {
        let mut ctx = ctx_with(0.0, 0);
        ctx.weekly = None;
        assert!(render(&ctx).is_none());
    }
}
