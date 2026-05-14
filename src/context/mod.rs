//! Semantic, presentation-free view over the raw payload.
//!
//! Widgets read here, never directly from `Payload`. Derived values stay as
//! domain types (numbers, durations, timezones) — string formatting happens
//! at `render::format`.

pub mod git;
pub mod jsonl;
pub mod payload;

use std::path::Path;

use jiff::{Timestamp, tz::TimeZone};

use self::payload::{CurrentUsage, Payload};
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct Context {
    pub model_display: Option<String>,
    pub cwd: Option<String>,
    pub context_metrics: Option<ContextWindowMetrics>,
    pub session_tokens: Option<u64>,
    pub session_cost_usd: Option<f64>,
    pub block: Option<TimerMetrics>,
    pub weekly: Option<TimerMetrics>,
    /// Populated only when `Context::with_git` is called. Default-theme paths
    /// keep this `None` so the renderer stays cost-free.
    pub git: Option<git::GitState>,
    /// Renderer-injected timezone. Default is `Asia/Seoul` (KST) so the
    /// golden snapshot is locale-stable; `Context::resolve_tz(Some("system"))`
    /// opts in to the host clock instead.
    pub tz: TimeZone,
}

#[derive(Debug, Clone)]
pub struct ContextWindowMetrics {
    pub used_tokens: u64,
    pub total_tokens: u64,
    pub used_percent: f64,
}

#[derive(Debug, Clone)]
pub struct TimerMetrics {
    pub used_percent: f64,
    pub resets_at: Timestamp,
}

impl Context {
    pub fn from_payload(payload: &Payload, tz: TimeZone) -> Result<Self> {
        let model_display = payload.model.as_ref().and_then(|m| m.display_string());

        let cwd = payload
            .cwd
            .as_deref()
            .and_then(non_empty)
            .or_else(|| {
                payload
                    .workspace
                    .as_ref()
                    .and_then(|w| w.current_dir.as_deref())
                    .and_then(non_empty)
            })
            .or_else(|| {
                payload
                    .workspace
                    .as_ref()
                    .and_then(|w| w.project_dir.as_deref())
                    .and_then(non_empty)
            });

        let context_metrics = payload
            .context_window
            .as_ref()
            .and_then(extract_context_metrics);

        // Priority order:
        //   1. `ccstatusline_rs.session_tokens` — explicit override (tests + agent
        //      overrides bypass disk I/O).
        //   2. JSONL probe over `transcript_path` — the real, cumulative session
        //      sum across all API turns.
        //   3. Neither — yield None so the widget hides.
        let session_tokens = payload
            .extension
            .as_ref()
            .and_then(|e| e.session_tokens)
            .or_else(|| {
                payload
                    .transcript_path
                    .as_deref()
                    .and_then(non_empty_path)
                    .and_then(|p| jsonl::probe_session_tokens_cached(p).ok().flatten())
                    .map(|s| s.total())
            });

        let session_cost_usd = payload
            .cost
            .as_ref()
            .and_then(|c| c.total_cost_usd)
            .filter(|v| v.is_finite());

        let block = payload
            .rate_limits
            .as_ref()
            .and_then(|r| r.five_hour.as_ref())
            .and_then(extract_timer);
        let weekly = payload
            .rate_limits
            .as_ref()
            .and_then(|r| r.seven_day.as_ref())
            .and_then(extract_timer);

        Ok(Self {
            model_display,
            cwd,
            context_metrics,
            session_tokens,
            session_cost_usd,
            block,
            weekly,
            git: None,
            tz,
        })
    }

    /// Run the git probe against `cwd` and stash the result on the context.
    /// Caller decides when to invoke this so the default-theme path stays
    /// free of subprocess overhead.
    pub fn with_git(mut self, cwd: &Path) -> Self {
        self.git = git::probe(cwd).ok().flatten();
        self
    }

    /// `None` and the empty string resolve to KST (Asia/Seoul) — the project
    /// default. Pass `"system"` to opt into the host clock.
    pub fn resolve_tz(name: Option<&str>) -> Result<TimeZone> {
        match name {
            None | Some("") => Ok(default_tz()),
            Some(n) if n.eq_ignore_ascii_case("system") => Ok(TimeZone::system()),
            Some(n) => TimeZone::get(n).map_err(|_| Error::InvalidTimezone { name: n.to_owned() }),
        }
    }
}

/// Project default. Embedded in jiff's tzdb so it works without a system tzdata.
fn default_tz() -> TimeZone {
    TimeZone::get("Asia/Seoul").expect("Asia/Seoul present in jiff-tzdb")
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_owned())
    }
}

fn non_empty_path(s: &str) -> Option<&Path> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(Path::new(t))
    }
}

fn extract_context_metrics(cw: &payload::ContextWindow) -> Option<ContextWindowMetrics> {
    let total = cw
        .context_window_size
        .filter(|v| v.is_finite() && *v > 0.0)? as u64;
    let used_tokens = cw
        .total_input_tokens
        .or_else(|| cw.current_usage.as_ref().map(CurrentUsage::total))
        .filter(|v| v.is_finite() && *v >= 0.0)
        .map(|v| v as u64)?;
    let used_percent = cw
        .used_percentage
        .filter(|v| v.is_finite())
        .map(clamp_percent)
        .unwrap_or_else(|| {
            let raw = (used_tokens as f64 / total as f64) * 100.0;
            clamp_percent(raw)
        });
    Some(ContextWindowMetrics {
        used_tokens,
        total_tokens: total,
        used_percent,
    })
}

fn extract_timer(period: &payload::RateLimitPeriod) -> Option<TimerMetrics> {
    let pct = period
        .used_percentage
        .filter(|v| v.is_finite())
        .map(clamp_percent)?;
    let resets_at = period
        .resets_at
        .filter(|v| v.is_finite())
        .and_then(|sec| Timestamp::from_second(sec as i64).ok())?;
    Some(TimerMetrics {
        used_percent: pct,
        resets_at,
    })
}

fn clamp_percent(v: f64) -> f64 {
    // NaN must collapse to 0; `f64::clamp` would propagate it.
    if v.is_nan() { 0.0 } else { v.clamp(0.0, 100.0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Context {
        let p: Payload = serde_json::from_str(json).unwrap();
        let tz = TimeZone::get("Asia/Seoul").unwrap();
        Context::from_payload(&p, tz).unwrap()
    }

    #[test]
    fn cwd_prefers_top_level_field() {
        let ctx = parse(
            r#"{ "cwd": "/top", "workspace": { "current_dir": "/curr", "project_dir": "/proj" } }"#,
        );
        assert_eq!(ctx.cwd.as_deref(), Some("/top"));
    }

    #[test]
    fn cwd_falls_back_through_workspace() {
        let ctx = parse(r#"{ "workspace": { "project_dir": "/proj" } }"#);
        assert_eq!(ctx.cwd.as_deref(), Some("/proj"));
    }

    #[test]
    fn percent_clamps_above_100() {
        let ctx = parse(
            r#"{ "rate_limits": { "five_hour": { "used_percentage": 150, "resets_at": 0 } } }"#,
        );
        assert_eq!(ctx.block.unwrap().used_percent, 100.0);
    }

    #[test]
    fn percent_clamps_below_zero() {
        let ctx = parse(
            r#"{ "rate_limits": { "five_hour": { "used_percentage": -5, "resets_at": 0 } } }"#,
        );
        assert_eq!(ctx.block.unwrap().used_percent, 0.0);
    }

    #[test]
    fn context_metrics_derives_percent_when_absent() {
        let ctx = parse(
            r#"{ "context_window": { "context_window_size": 1000000, "current_usage": 80000 } }"#,
        );
        let m = ctx.context_metrics.unwrap();
        assert_eq!(m.used_tokens, 80000);
        assert_eq!(m.total_tokens, 1000000);
        assert!((m.used_percent - 8.0).abs() < 0.01);
    }

    #[test]
    fn invalid_tz_returns_named_error() {
        let err = Context::resolve_tz(Some("Bogus/Place")).unwrap_err();
        match err {
            Error::InvalidTimezone { name } => assert_eq!(name, "Bogus/Place"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn default_tz_is_kst() {
        let tz = Context::resolve_tz(None).unwrap();
        assert_eq!(tz.iana_name(), Some("Asia/Seoul"));
    }

    #[test]
    fn empty_tz_falls_back_to_kst() {
        let tz = Context::resolve_tz(Some("")).unwrap();
        assert_eq!(tz.iana_name(), Some("Asia/Seoul"));
    }

    #[test]
    fn session_tokens_use_extension_override_when_present() {
        let ctx = parse(r#"{ "ccstatusline_rs": { "session_tokens": 42 } }"#);
        assert_eq!(ctx.session_tokens, Some(42));
    }

    #[test]
    fn session_tokens_fall_back_to_jsonl_probe() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            f,
            r#"{{"message":{{"usage":{{"input_tokens":75000,"output_tokens":10300}},"stop_reason":"end_turn"}}}}"#
        )
        .unwrap();
        f.flush().unwrap();

        let path = f.path().to_str().unwrap().replace('\\', "\\\\");
        let json = format!(r#"{{ "transcript_path": "{path}" }}"#);
        let ctx = parse(&json);
        assert_eq!(ctx.session_tokens, Some(85_300));
    }
}
