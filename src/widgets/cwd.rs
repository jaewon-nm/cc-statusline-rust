//! Current working directory. No truncation in the default theme — that
//! policy belongs to a future config knob.

use crate::context::Context;
use crate::render::Segment;

pub fn render(ctx: &Context) -> Option<Segment> {
    let cwd = ctx.cwd.as_deref()?;
    Some(Segment::plain(format!("📂 {cwd}")))
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

    #[test]
    fn renders_full_path() {
        let s = render(&ctx_with(Some(r"F:\Works\naya\cc-statusline-rust"))).unwrap();
        assert_eq!(s.text, r"📂 F:\Works\naya\cc-statusline-rust");
    }

    #[test]
    fn missing_cwd_yields_none() {
        assert!(render(&ctx_with(None)).is_none());
    }
}
