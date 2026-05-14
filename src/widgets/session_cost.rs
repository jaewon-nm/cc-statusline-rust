//! Session cost in USD. Theme default: yellow.

use anstyle::{AnsiColor, Style};

use crate::context::Context;
use crate::render::Segment;
use crate::render::format::format_cost_usd;

pub fn render(ctx: &Context) -> Option<Vec<Segment>> {
    let v = ctx.session_cost_usd?;
    let style = Style::new().fg_color(Some(AnsiColor::Yellow.into()));
    Some(vec![Segment::styled(
        format!("💰 {}", format_cost_usd(v)),
        style,
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::tz::TimeZone;

    fn ctx_with(cost: Option<f64>) -> Context {
        Context {
            model_display: None,
            cwd: None,
            context_metrics: None,
            session_tokens: None,
            session_cost_usd: cost,
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
    fn renders_default_cost() {
        let segs = render(&ctx_with(Some(2.55))).unwrap();
        assert_eq!(joined(&segs), "💰 $2.55");
    }

    #[test]
    fn renders_theme_default_yellow() {
        let segs = render(&ctx_with(Some(0.0))).unwrap();
        assert_eq!(segs[0].style.get_fg_color(), Some(AnsiColor::Yellow.into()));
    }

    #[test]
    fn missing_cost_yields_none() {
        assert!(render(&ctx_with(None)).is_none());
    }
}
