//! Branch name from the git probe. Falls back silently when not inside a repo
//! or when the probe returned nothing.

use crate::context::Context;
use crate::render::Segment;

pub fn render(ctx: &Context) -> Option<Segment> {
    let state = ctx.git.as_ref()?;
    let branch = state.branch.as_deref()?;
    Some(Segment::plain(format!("🌿 {branch}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::git::{DiffShortstat, GitState, PorcelainCounts};
    use jiff::tz::TimeZone;
    use std::path::PathBuf;

    fn ctx_with(branch: Option<&str>) -> Context {
        let git = branch.map(|b| GitState {
            repo_root: PathBuf::from("/tmp/repo"),
            branch: Some(b.to_owned()),
            porcelain: PorcelainCounts::default(),
            diff: DiffShortstat::default(),
            captured_at: 0,
        });
        Context {
            model_display: None,
            cwd: None,
            context_metrics: None,
            session_tokens: None,
            session_cost_usd: None,
            block: None,
            weekly: None,
            git,
            tz: TimeZone::get("Asia/Seoul").unwrap(),
        }
    }

    #[test]
    fn renders_branch_name_with_icon() {
        let s = render(&ctx_with(Some("main"))).unwrap();
        assert_eq!(s.text, "🌿 main");
    }

    #[test]
    fn yields_none_when_git_absent() {
        assert!(render(&ctx_with(None)).is_none());
    }

    #[test]
    fn yields_none_on_detached_head() {
        let mut ctx = ctx_with(Some("dummy"));
        if let Some(g) = ctx.git.as_mut() {
            g.branch = None;
        }
        assert!(render(&ctx).is_none());
    }
}
