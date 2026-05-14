//! Line-level diff summary from `git diff --shortstat` (staged + unstaged
//! combined). When the working tree is clean, the widget yields `None` so the
//! separator collapses cleanly.

use crate::context::Context;
use crate::render::Segment;

pub fn render(ctx: &Context) -> Option<Segment> {
    let state = ctx.git.as_ref()?;
    let d = &state.diff;
    if d.insertions == 0 && d.deletions == 0 {
        return None;
    }
    Some(Segment::plain(format!(
        "📝 +{} -{}",
        d.insertions, d.deletions
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::git::{DiffShortstat, GitState, PorcelainCounts};
    use jiff::tz::TimeZone;
    use std::path::PathBuf;

    fn ctx_with(diff: DiffShortstat) -> Context {
        Context {
            model_display: None,
            cwd: None,
            context_metrics: None,
            session_tokens: None,
            session_cost_usd: None,
            block: None,
            weekly: None,
            git: Some(GitState {
                repo_root: PathBuf::from("/tmp/repo"),
                branch: None,
                porcelain: PorcelainCounts::default(),
                diff,
                captured_at: 0,
            }),
            tz: TimeZone::get("Asia/Seoul").unwrap(),
        }
    }

    #[test]
    fn renders_insertions_and_deletions() {
        let s = render(&ctx_with(DiffShortstat {
            insertions: 120,
            deletions: 20,
            files_changed: 5,
        }))
        .unwrap();
        assert_eq!(s.text, "📝 +120 -20");
    }

    #[test]
    fn clean_diff_yields_none() {
        assert!(render(&ctx_with(DiffShortstat::default())).is_none());
    }

    #[test]
    fn yields_none_when_git_absent() {
        let ctx = Context {
            model_display: None,
            cwd: None,
            context_metrics: None,
            session_tokens: None,
            session_cost_usd: None,
            block: None,
            weekly: None,
            git: None,
            tz: TimeZone::get("Asia/Seoul").unwrap(),
        };
        assert!(render(&ctx).is_none());
    }
}
