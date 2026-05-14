//! Abbreviated session token count. Sourced from the namespaced extension
//! `ccstatusline_rs.session_tokens`; a future JSONL probe will replace the
//! injection without changing this widget.

use crate::context::Context;
use crate::render::Segment;
use crate::render::format::abbreviate_tokens;

pub fn render(ctx: &Context) -> Option<Segment> {
    let n = ctx.session_tokens?;
    Some(Segment::plain(format!("📊 {}", abbreviate_tokens(n))))
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

    #[test]
    fn renders_default_value() {
        assert_eq!(render(&ctx_with(Some(85_300))).unwrap().text, "📊 85.3K");
    }

    #[test]
    fn missing_value_yields_none() {
        assert!(render(&ctx_with(None)).is_none());
    }
}
