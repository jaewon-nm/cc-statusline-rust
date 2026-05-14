//! Context usage as `[bar] used/total(pct%)`.

use crate::context::Context;
use crate::render::Segment;
use crate::render::format::{abbreviate_tokens, format_bar_wrapped, format_percent_paren};

pub fn render(ctx: &Context) -> Option<Segment> {
    let m = ctx.context_metrics.as_ref()?;
    let bar = format_bar_wrapped(m.used_percent, 10, '#', '.');
    let used = abbreviate_tokens(m.used_tokens);
    let total = abbreviate_tokens(m.total_tokens);
    let pct = format_percent_paren(m.used_percent);
    Some(Segment::plain(format!("🔋 {bar} {used}/{total}{pct}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextWindowMetrics;
    use jiff::tz::TimeZone;

    fn ctx_with(used: u64, total: u64, pct: f64) -> Context {
        Context {
            model_display: None,
            cwd: None,
            context_metrics: Some(ContextWindowMetrics {
                used_tokens: used,
                total_tokens: total,
                used_percent: pct,
            }),
            session_tokens: None,
            session_cost_usd: None,
            block: None,
            weekly: None,
            git: None,
            tz: TimeZone::get("Asia/Seoul").unwrap(),
        }
    }

    #[test]
    fn renders_eight_percent_default_theme() {
        let s = render(&ctx_with(80_000, 1_000_000, 8.0)).unwrap();
        assert_eq!(s.text, "🔋 [..........] 80.0K/1.0M(8%)");
    }

    #[test]
    fn missing_metrics_yields_none() {
        let mut ctx = ctx_with(0, 0, 0.0);
        ctx.context_metrics = None;
        assert!(render(&ctx).is_none());
    }
}
