//! Context usage as `🔋 [bar] used/total(pct%)`. Multi-segment so the
//! filled cells take a threshold-tier color (green / yellow / red) while
//! the icon, brackets, empty cells, and trailing numbers stay default.

use anstyle::Style;

use crate::context::Context;
use crate::render::Segment;
use crate::render::color::bar_tier_color;
use crate::render::format::{abbreviate_tokens, bar_filled_count, format_percent_paren};

const BAR_WIDTH: usize = 10;
const FILLED_CHAR: char = '#';
const EMPTY_CHAR: char = '.';

pub fn render(ctx: &Context) -> Option<Vec<Segment>> {
    let m = ctx.context_metrics.as_ref()?;
    let filled = bar_filled_count(m.used_percent, BAR_WIDTH);
    let empty = BAR_WIDTH - filled;
    let used = abbreviate_tokens(m.used_tokens);
    let total = abbreviate_tokens(m.total_tokens);
    let pct = format_percent_paren(m.used_percent);

    let mut out = Vec::with_capacity(7);
    out.push(Segment::plain("🔋 ["));
    if filled > 0 {
        let tier = Style::new().fg_color(Some(bar_tier_color(m.used_percent)));
        out.push(Segment::styled(
            FILLED_CHAR.to_string().repeat(filled),
            tier,
        ));
    }
    if empty > 0 {
        out.push(Segment::plain(EMPTY_CHAR.to_string().repeat(empty)));
    }
    out.push(Segment::plain(format!("] {used}/{total}{pct}")));
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextWindowMetrics;
    use anstyle::AnsiColor;
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

    fn joined(segs: &[Segment]) -> String {
        segs.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn renders_eight_percent_default_theme() {
        let segs = render(&ctx_with(80_000, 1_000_000, 8.0)).unwrap();
        assert_eq!(joined(&segs), "🔋 [..........] 80.0K/1.0M(8%)");
    }

    #[test]
    fn filled_cells_are_green_below_warn() {
        // 21% → 2 filled cells, tier = green.
        let segs = render(&ctx_with(210_000, 1_000_000, 21.0)).unwrap();
        let filled_seg = segs
            .iter()
            .find(|s| s.text.chars().all(|c| c == FILLED_CHAR) && !s.text.is_empty())
            .expect("filled segment present");
        assert_eq!(filled_seg.text, "##");
        assert_eq!(
            filled_seg.style.get_fg_color(),
            Some(AnsiColor::Green.into())
        );
    }

    #[test]
    fn filled_cells_are_yellow_in_warn_band() {
        let segs = render(&ctx_with(550_000, 1_000_000, 55.0)).unwrap();
        let filled_seg = segs
            .iter()
            .find(|s| s.text.chars().all(|c| c == FILLED_CHAR) && !s.text.is_empty())
            .unwrap();
        assert_eq!(
            filled_seg.style.get_fg_color(),
            Some(AnsiColor::Yellow.into())
        );
    }

    #[test]
    fn filled_cells_are_red_at_or_above_crit() {
        let segs = render(&ctx_with(920_000, 1_000_000, 92.0)).unwrap();
        let filled_seg = segs
            .iter()
            .find(|s| s.text.chars().all(|c| c == FILLED_CHAR) && !s.text.is_empty())
            .unwrap();
        assert_eq!(filled_seg.style.get_fg_color(), Some(AnsiColor::Red.into()));
    }

    #[test]
    fn zero_percent_omits_filled_segment() {
        let segs = render(&ctx_with(0, 1_000_000, 0.0)).unwrap();
        assert!(!segs.iter().any(|s| s.text.contains(FILLED_CHAR)));
        assert_eq!(joined(&segs), "🔋 [..........] 0/1.0M(0%)");
    }

    #[test]
    fn full_percent_omits_empty_segment() {
        let segs = render(&ctx_with(1_000_000, 1_000_000, 100.0)).unwrap();
        // No segment is a *pure* empty-cell run; `.` still appears in `1.0M`.
        assert!(
            !segs
                .iter()
                .any(|s| !s.text.is_empty() && s.text.chars().all(|c| c == EMPTY_CHAR)),
        );
        assert_eq!(joined(&segs), "🔋 [##########] 1.0M/1.0M(100%)");
    }

    #[test]
    fn missing_metrics_yields_none() {
        let mut ctx = ctx_with(0, 0, 0.0);
        ctx.context_metrics = None;
        assert!(render(&ctx).is_none());
    }
}
