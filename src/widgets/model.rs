//! Model widget. Wraps the display name in `[…]` and prefixes the project
//! icon. The parenthetical context suffix (e.g. `(1M context)`) is intentionally
//! preserved — upstream's `Context Window` widget is folded in here.

use crate::context::Context;
use crate::render::Segment;

pub fn render(ctx: &Context) -> Option<Segment> {
    let name = ctx.model_display.as_deref()?;
    Some(Segment::plain(format!("✦ [{name}]")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::tz::TimeZone;

    fn ctx_with(model: Option<&str>) -> Context {
        Context {
            model_display: model.map(str::to_owned),
            cwd: None,
            context_metrics: None,
            session_tokens: None,
            session_cost_usd: None,
            block: None,
            weekly: None,
            tz: TimeZone::get("Asia/Seoul").unwrap(),
        }
    }

    #[test]
    fn renders_with_brackets_and_icon() {
        let s = render(&ctx_with(Some("Opus 4.7 (1M context)"))).unwrap();
        assert_eq!(s.text, "✦ [Opus 4.7 (1M context)]");
    }

    #[test]
    fn missing_model_yields_none() {
        assert!(render(&ctx_with(None)).is_none());
    }
}
