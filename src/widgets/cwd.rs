//! Current working directory. No truncation in the default theme — that
//! policy belongs to a future config knob.
//!
//! Theme default: blue.

use anstyle::{AnsiColor, Style};

use crate::context::Context;
use crate::render::Segment;

pub fn render(ctx: &Context) -> Option<Vec<Segment>> {
    let cwd = ctx.cwd.as_deref()?;
    let style = Style::new().fg_color(Some(AnsiColor::Blue.into()));
    Some(vec![Segment::styled(format!("📂 {cwd}"), style)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::tz::TimeZone;

    fn ctx_with(cwd: Option<&str>) -> Context {
        Context {
            model_display: None,
            cwd: cwd.map(str::to_owned),
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
    fn renders_full_path() {
        let segs = render(&ctx_with(Some(r"F:\Works\naya\cc-statusline-rust"))).unwrap();
        assert_eq!(joined(&segs), r"📂 F:\Works\naya\cc-statusline-rust");
    }

    #[test]
    fn renders_theme_default_blue() {
        let segs = render(&ctx_with(Some("/tmp"))).unwrap();
        assert_eq!(segs[0].style.get_fg_color(), Some(AnsiColor::Blue.into()));
    }

    #[test]
    fn missing_cwd_yields_none() {
        assert!(render(&ctx_with(None)).is_none());
    }
}
