//! Cumulative session-token probe over the Claude Code transcript JSONL.
//!
//! The transcript path arrives in the payload (`payload.transcript_path`). We
//! sum `message.usage.{input,output,cache_creation,cache_read}_tokens` across
//! every non-streaming entry. Streaming-partial duplicates are filtered by
//! `stop_reason` (intermediate emissions have it null); the very last entry is
//! always counted even when `stop_reason` is null so the live in-progress turn
//! still contributes.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::error::Result;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SessionTokens {
    pub input: u64,
    pub output: u64,
    pub cached: u64,
}

impl SessionTokens {
    pub fn total(self) -> u64 {
        self.input + self.output + self.cached
    }
}

#[derive(Debug, Deserialize)]
struct TranscriptLine {
    #[serde(default)]
    message: Option<MessageNode>,
    /// Marker that the entry is a sub-agent turn (we still count it — upstream
    /// excludes only for the *context length* widget, not the cumulative sum).
    #[serde(default, rename = "isSidechain")]
    _is_sidechain: Option<bool>,
    /// Filter out lines that the API marked as error transcripts.
    #[serde(default, rename = "isApiErrorMessage")]
    is_api_error: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct MessageNode {
    #[serde(default)]
    usage: Option<Usage>,
    /// Empty / null on streaming-partial frames; non-null on the final frame.
    #[serde(default)]
    stop_reason: Option<Value>,
}

#[derive(Debug, Default, Deserialize)]
struct Usage {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
    #[serde(default)]
    cache_creation_input_tokens: Option<u64>,
    #[serde(default)]
    cache_read_input_tokens: Option<u64>,
}

/// Parse the JSONL at `path` and accumulate session token usage.
///
/// Returns `Ok(None)` when:
/// - the file is missing or unreadable (renderer must not crash on a fresh
///   session whose transcript has not been created yet)
/// - the file has no usage-bearing entries
///
/// Returns `Err` only for the renderer's own bugs (I/O sentinel failures we
/// did not anticipate). Per-line parse failures are tolerated — corrupt lines
/// are skipped, not propagated.
pub fn probe_session_tokens(path: &Path) -> Result<Option<SessionTokens>> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    let reader = BufReader::new(file);

    let mut accum = SessionTokens::default();
    let mut counted_any = false;
    let mut saw_stop_reason = false;
    let mut pending_last: Option<Usage> = None;

    for line in reader.lines() {
        let raw = match line {
            Ok(s) => s,
            Err(_) => continue,
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parsed: TranscriptLine = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if parsed.is_api_error.unwrap_or(false) {
            continue;
        }
        let Some(msg) = parsed.message else { continue };
        let Some(usage) = msg.usage else { continue };

        let entry_has_stop = msg.stop_reason.as_ref().is_some_and(|v| !v.is_null());
        if entry_has_stop {
            saw_stop_reason = true;
            counted_any = true;
            add_usage(&mut accum, &usage);
            pending_last = None;
        } else {
            // Defer streaming partials; only the final iteration survives if no
            // terminal entry follows. This matches upstream's dedup heuristic.
            pending_last = Some(usage);
        }
    }

    if !saw_stop_reason && let Some(usage) = pending_last.as_ref() {
        // Legacy / streaming-only transcript: count whatever we kept.
        add_usage(&mut accum, usage);
        counted_any = true;
    } else if let Some(usage) = pending_last.as_ref() {
        // Live final turn — count it on top of the already-accumulated history.
        add_usage(&mut accum, usage);
        counted_any = true;
    }

    if counted_any {
        Ok(Some(accum))
    } else {
        Ok(None)
    }
}

fn add_usage(accum: &mut SessionTokens, u: &Usage) {
    accum.input += u.input_tokens.unwrap_or(0);
    accum.output += u.output_tokens.unwrap_or(0);
    accum.cached +=
        u.cache_creation_input_tokens.unwrap_or(0) + u.cache_read_input_tokens.unwrap_or(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_jsonl(lines: &[&str]) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(f, "{line}").unwrap();
        }
        f.flush().unwrap();
        f
    }

    #[test]
    fn missing_file_yields_none() {
        let result = probe_session_tokens(Path::new("/nonexistent/path.jsonl")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn empty_file_yields_none() {
        let f = write_jsonl(&[]);
        let result = probe_session_tokens(f.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn sums_input_output_and_cached_across_entries() {
        let f = write_jsonl(&[
            r#"{"message":{"usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":2},"stop_reason":"end_turn"}}"#,
            r#"{"message":{"usage":{"input_tokens":7,"output_tokens":3,"cache_creation_input_tokens":4},"stop_reason":"end_turn"}}"#,
        ]);
        let s = probe_session_tokens(f.path()).unwrap().unwrap();
        assert_eq!(s.input, 17);
        assert_eq!(s.output, 8);
        assert_eq!(s.cached, 6);
        assert_eq!(s.total(), 31);
    }

    #[test]
    fn skips_streaming_partial_when_terminal_present() {
        let f = write_jsonl(&[
            // Streaming partial — discard.
            r#"{"message":{"usage":{"input_tokens":1,"output_tokens":1},"stop_reason":null}}"#,
            // Terminal — count.
            r#"{"message":{"usage":{"input_tokens":10,"output_tokens":5},"stop_reason":"end_turn"}}"#,
        ]);
        let s = probe_session_tokens(f.path()).unwrap().unwrap();
        assert_eq!(s.input, 10);
        assert_eq!(s.output, 5);
    }

    #[test]
    fn counts_live_final_turn_with_null_stop_reason() {
        let f = write_jsonl(&[
            r#"{"message":{"usage":{"input_tokens":10,"output_tokens":5},"stop_reason":"end_turn"}}"#,
            // Live in-progress turn — still want it visible.
            r#"{"message":{"usage":{"input_tokens":7,"output_tokens":2},"stop_reason":null}}"#,
        ]);
        let s = probe_session_tokens(f.path()).unwrap().unwrap();
        assert_eq!(s.input, 17);
        assert_eq!(s.output, 7);
    }

    #[test]
    fn legacy_transcript_without_stop_reason_keeps_last_entry() {
        let f = write_jsonl(&[
            r#"{"message":{"usage":{"input_tokens":3,"output_tokens":1}}}"#,
            r#"{"message":{"usage":{"input_tokens":5,"output_tokens":2}}}"#,
        ]);
        let s = probe_session_tokens(f.path()).unwrap().unwrap();
        // Conservative legacy fallback: only the most recent line.
        assert_eq!(s.input, 5);
        assert_eq!(s.output, 2);
    }

    #[test]
    fn skips_api_error_entries() {
        let f = write_jsonl(&[
            r#"{"message":{"usage":{"input_tokens":100,"output_tokens":50},"stop_reason":"end_turn"},"isApiErrorMessage":true}"#,
            r#"{"message":{"usage":{"input_tokens":3,"output_tokens":1},"stop_reason":"end_turn"}}"#,
        ]);
        let s = probe_session_tokens(f.path()).unwrap().unwrap();
        assert_eq!(s.input, 3);
        assert_eq!(s.output, 1);
    }

    #[test]
    fn skips_malformed_lines() {
        let f = write_jsonl(&[
            "not json at all",
            r#"{"message":{"usage":{"input_tokens":5,"output_tokens":2},"stop_reason":"end_turn"}}"#,
            r#"{ this is { invalid"#,
        ]);
        let s = probe_session_tokens(f.path()).unwrap().unwrap();
        assert_eq!(s.input, 5);
        assert_eq!(s.output, 2);
    }
}
