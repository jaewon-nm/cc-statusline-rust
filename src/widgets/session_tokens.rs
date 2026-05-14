//! Cumulative session token count. Sourced from the namespaced extension
//! override or the JSONL transcript probe in `context::mod`. Theme default
//! is magenta.

use anstyle::{AnsiColor, Style};

use crate::context::Context;
use crate::render::Segment;
use crate::render::format::abbreviate_tokens;

pub fn render(ctx: &Context) -> Option<Vec<Segment>> {
    let n = ctx.session_tokens?;
    let style = Style::new().fg_color(Some(AnsiColor::Magenta.into()));
    Some(vec![Segment::styled(
        format!("📊 {}", abbreviate_tokens(n)),
        style,
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::tz::TimeZone;

    fn ctx_with(tokens: Option<u64>) -> Context {
        Context {
            model_display: None,
            cwd: None,
            context_metrics: None,
            session_tokens: tokens,
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
    fn renders_default_value() {
        let segs = render(&ctx_with(Some(85_300))).unwrap();
        assert_eq!(joined(&segs), "📊 85.3K");
    }

    #[test]
    fn renders_theme_default_magenta() {
        let segs = render(&ctx_with(Some(1))).unwrap();
        assert_eq!(
            segs[0].style.get_fg_color(),
            Some(AnsiColor::Magenta.into())
        );
    }

    #[test]
    fn missing_value_yields_none() {
        assert!(render(&ctx_with(None)).is_none());
    }
}
