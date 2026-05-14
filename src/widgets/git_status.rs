//! Compact porcelain summary. Theme default: yellow (one color across the
//! whole widget — per-letter coloring is deferred).

use anstyle::{AnsiColor, Style};

use crate::context::Context;
use crate::render::Segment;

pub fn render(ctx: &Context) -> Option<Vec<Segment>> {
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
    let style = Style::new().fg_color(Some(AnsiColor::Yellow.into()));
    Some(vec![Segment::styled(format!("⛓ {body}"), style)])
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

    fn joined(segs: &[Segment]) -> String {
        segs.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn clean_repo_renders_check_mark() {
        let segs = render(&ctx_with(PorcelainCounts::default())).unwrap();
        assert_eq!(joined(&segs), "⛓ ✓");
    }

    #[test]
    fn full_breakdown() {
        let segs = render(&ctx_with(PorcelainCounts {
            staged: 2,
            unstaged: 3,
            untracked: 1,
            conflicts: 1,
        }))
        .unwrap();
        assert_eq!(joined(&segs), "⛓ S2 M3 ?1 !1");
    }

    #[test]
    fn untracked_only() {
        let segs = render(&ctx_with(PorcelainCounts {
            untracked: 5,
            ..PorcelainCounts::default()
        }))
        .unwrap();
        assert_eq!(joined(&segs), "⛓ ?5");
    }

    #[test]
    fn theme_default_is_yellow() {
        let segs = render(&ctx_with(PorcelainCounts::default())).unwrap();
        assert_eq!(segs[0].style.get_fg_color(), Some(AnsiColor::Yellow.into()));
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
