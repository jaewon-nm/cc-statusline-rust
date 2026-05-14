//! Compact porcelain summary. Each character class is shown only when non-zero,
//! so a clean repo collapses to `✓` and a noisy repo emits e.g. `S2 M3 ?1 !1`.
//!
//! - `S` staged
//! - `M` unstaged (modified but not staged)
//! - `?` untracked
//! - `!` conflicts

use crate::context::Context;
use crate::render::Segment;

pub fn render(ctx: &Context) -> Option<Segment> {
    let state = ctx.git.as_ref()?;
    let p = &state.porcelain;
    let mut parts: Vec<String> = Vec::new();
    if p.staged > 0 {
        parts.push(format!("S{}", p.staged));
    }
    if p.unstaged > 0 {
        parts.push(format!("M{}", p.unstaged));
    }
    if p.untracked > 0 {
        parts.push(format!("?{}", p.untracked));
    }
    if p.conflicts > 0 {
        parts.push(format!("!{}", p.conflicts));
    }
    let body = if parts.is_empty() {
        "✓".to_owned()
    } else {
        parts.join(" ")
    };
    Some(Segment::plain(format!("⛓ {body}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::git::{DiffShortstat, GitState, PorcelainCounts};
    use jiff::tz::TimeZone;
    use std::path::PathBuf;

    fn ctx_with(counts: PorcelainCounts) -> Context {
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
                porcelain: counts,
                diff: DiffShortstat::default(),
                captured_at: 0,
            }),
            tz: TimeZone::get("Asia/Seoul").unwrap(),
        }
    }

    #[test]
    fn clean_repo_renders_check_mark() {
        let s = render(&ctx_with(PorcelainCounts::default())).unwrap();
        assert_eq!(s.text, "⛓ ✓");
    }

    #[test]
    fn full_breakdown() {
        let s = render(&ctx_with(PorcelainCounts {
            staged: 2,
            unstaged: 3,
            untracked: 1,
            conflicts: 1,
        }))
        .unwrap();
        assert_eq!(s.text, "⛓ S2 M3 ?1 !1");
    }

    #[test]
    fn untracked_only() {
        let s = render(&ctx_with(PorcelainCounts {
            untracked: 5,
            ..PorcelainCounts::default()
        }))
        .unwrap();
        assert_eq!(s.text, "⛓ ?5");
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
