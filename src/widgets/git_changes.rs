//! Line-level diff summary. Multi-segment: the icon stays default, `+<ins>`
//! is green, `-<dels>` is red, mirroring how IDE diff hunks read.

use anstyle::{AnsiColor, Style};

use crate::context::Context;
use crate::render::Segment;

pub fn render(ctx: &Context) -> Option<Vec<Segment>> {
    let state = ctx.git.as_ref()?;
    let d = &state.diff;
    if d.insertions == 0 && d.deletions == 0 {
        return None;
    }
    let green = Style::new().fg_color(Some(AnsiColor::Green.into()));
    let red = Style::new().fg_color(Some(AnsiColor::Red.into()));
    Some(vec![
        Segment::plain("📝 "),
        Segment::styled(format!("+{}", d.insertions), green),
        Segment::plain(" "),
        Segment::styled(format!("-{}", d.deletions), red),
    ])
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

    fn joined(segs: &[Segment]) -> String {
        segs.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn renders_insertions_and_deletions() {
        let segs = render(&ctx_with(DiffShortstat {
            insertions: 120,
            deletions: 20,
            files_changed: 5,
        }))
        .unwrap();
        assert_eq!(joined(&segs), "📝 +120 -20");
    }

    #[test]
    fn insertions_are_green_deletions_red() {
        let segs = render(&ctx_with(DiffShortstat {
            insertions: 1,
            deletions: 1,
            files_changed: 1,
        }))
        .unwrap();
        let ins = segs.iter().find(|s| s.text.starts_with('+')).unwrap();
        let del = segs.iter().find(|s| s.text.starts_with('-')).unwrap();
        assert_eq!(ins.style.get_fg_color(), Some(AnsiColor::Green.into()));
        assert_eq!(del.style.get_fg_color(), Some(AnsiColor::Red.into()));
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
