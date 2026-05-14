//! Model widget. Wraps the display name in `[…]` and prefixes the project
//! icon. The parenthetical context suffix (e.g. `(1M context)`) is intentionally
//! preserved — upstream's `Context Window` widget is folded in here.
//!
//! Theme default: cyan + bold for the whole widget. User `config color model`
//! overrides this end-to-end at the renderer layer.

use anstyle::{AnsiColor, Style};

use crate::context::Context;
use crate::render::Segment;

pub fn render(ctx: &Context) -> Option<Vec<Segment>> {
    let name = ctx.model_display.as_deref()?;
    let style = Style::new().fg_color(Some(AnsiColor::Cyan.into())).bold();
    Some(vec![Segment::styled(format!("✦ [{name}]"), style)])
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
            git: None,
            tz: TimeZone::get("Asia/Seoul").unwrap(),
        }
    }

    fn joined(segs: &[Segment]) -> String {
        segs.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn renders_with_brackets_and_icon() {
        let segs = render(&ctx_with(Some("Opus 4.7 (1M context)"))).unwrap();
        assert_eq!(joined(&segs), "✦ [Opus 4.7 (1M context)]");
    }

    #[test]
    fn renders_theme_default_cyan_bold() {
        let segs = render(&ctx_with(Some("Opus"))).unwrap();
        assert_eq!(segs[0].style.get_fg_color(), Some(AnsiColor::Cyan.into()));
        assert!(segs[0].style.get_effects().contains(anstyle::Effects::BOLD));
    }

    #[test]
    fn missing_model_yields_none() {
        assert!(render(&ctx_with(None)).is_none());
    }
}
