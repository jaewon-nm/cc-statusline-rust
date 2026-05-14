//! Session cost in USD.

use crate::context::Context;
use crate::render::Segment;
use crate::render::format::format_cost_usd;

pub fn render(ctx: &Context) -> Option<Segment> {
    let v = ctx.session_cost_usd?;
    Some(Segment::plain(format!("💰 {}", format_cost_usd(v))))
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

    #[test]
    fn renders_default_cost() {
        assert_eq!(render(&ctx_with(Some(2.55))).unwrap().text, "💰 $2.55");
    }

    #[test]
    fn missing_cost_yields_none() {
        assert!(render(&ctx_with(None)).is_none());
    }
}
