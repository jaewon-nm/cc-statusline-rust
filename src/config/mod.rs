//! Config schema. Bootstrap ships the default layout only; full edit surface
//! (add/remove/set/apply) lands in M2. The schemars-derived schema is exposed
//! via the `schema` subcommand so agents can self-discover the shape.

use std::fs;
use std::path::PathBuf;

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};

pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Config {
    /// Schema version. Bumped only on breaking changes; refuse unknown values.
    pub version: u32,
    /// IANA timezone name, `"system"`, or `null` for the project default (KST).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tz: Option<String>,
    /// Outer vector = lines (rendered top-to-bottom). Inner vector = widget
    /// kinds in left-to-right order. Empty inner vector renders nothing for
    /// that line (and the line is dropped from output).
    pub lines: Vec<Vec<String>>,
}

impl Config {
    pub fn default_layout() -> Self {
        Self {
            version: CONFIG_VERSION,
            tz: None,
            lines: vec![
                vec![
                    "model".into(),
                    "cwd".into(),
                    "context_bar".into(),
                    "session_tokens".into(),
                    "session_cost".into(),
                ],
                vec!["block_timer".into(), "weekly_timer".into()],
            ],
        }
    }

    pub fn schema() -> Value {
        serde_json::to_value(schema_for!(Self)).expect("schemars output is JSON-serializable")
    }

    /// True when any line lists a widget kind beginning with `git_`. Used by
    /// the renderer to decide whether the git probe is worth its cost.
    pub fn needs_git(&self) -> bool {
        self.lines
            .iter()
            .flat_map(|l| l.iter())
            .any(|k| k.starts_with("git_"))
    }

    /// Reject configs whose schema version we don't recognize or that
    /// reference widget kinds outside the registry. Layout edits should
    /// always pass through this gate before persisting.
    pub fn validate(&self, known_kinds: &[&str]) -> Result<()> {
        if self.version != CONFIG_VERSION {
            return Err(Error::InvalidConfig {
                reason: format!(
                    "unsupported version {} (expected {CONFIG_VERSION})",
                    self.version,
                ),
            });
        }
        for (li, line) in self.lines.iter().enumerate() {
            for (pi, kind) in line.iter().enumerate() {
                if !known_kinds.contains(&kind.as_str()) {
                    return Err(Error::InvalidConfig {
                        reason: format!("unknown widget kind '{kind}' at lines[{li}][{pi}]",),
                    });
                }
            }
        }
        Ok(())
    }

    /// Atomic save: serialize to a sibling `.tmp`, then rename onto `path`.
    /// Caller is responsible for picking the destination (typically
    /// [`config_path`]).
    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(Error::from)?;
        }
        let serialized = serde_json::to_vec_pretty(self).map_err(Error::from)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, &serialized).map_err(Error::from)?;
        fs::rename(&tmp, path).map_err(Error::from)?;
        Ok(())
    }
}

/// Resolve the on-disk config path. `CCSTATUSLINE_RS_CONFIG` env var wins so
/// tests can pin to a tempfile; otherwise the OS-specific config dir applies.
pub fn config_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CCSTATUSLINE_RS_CONFIG")
        && !p.is_empty()
    {
        return Some(PathBuf::from(p));
    }
    let dirs = directories::ProjectDirs::from("dev", "naya", "ccstatusline-rs")?;
    Some(dirs.config_dir().join("config.json"))
}

/// Read the config file when present; return the built-in default otherwise.
/// A corrupt file surfaces as `Error::InvalidConfig` so the agent sees the
/// problem — the renderer never silently re-falls to default with broken JSON.
pub fn load_or_default() -> Result<Config> {
    let Some(path) = config_path() else {
        return Ok(Config::default_layout());
    };
    load_from_or_default(&path)
}

/// Read a specific path, returning the default layout when the file is absent.
/// Used by `config show` to mirror what the renderer would see.
pub fn load_from_or_default(path: &std::path::Path) -> Result<Config> {
    let raw = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Config::default_layout());
        }
        Err(err) => return Err(Error::from(err)),
    };
    let cfg: Config = serde_json::from_str(&raw).map_err(|e| Error::InvalidConfig {
        reason: format!("{p}: {e}", p = path.display()),
    })?;
    if cfg.version != CONFIG_VERSION {
        return Err(Error::InvalidConfig {
            reason: format!("unsupported version {}", cfg.version),
        });
    }
    Ok(cfg)
}

/// Read a specific path, erroring instead of falling back. Used by
/// `config apply --file` and `config validate --file`.
pub fn load_from(path: &std::path::Path) -> Result<Config> {
    let raw = fs::read_to_string(path).map_err(Error::from)?;
    let cfg: Config = serde_json::from_str(&raw).map_err(|e| Error::InvalidConfig {
        reason: format!("{p}: {e}", p = path.display()),
    })?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_round_trips() {
        let cfg = Config::default_layout();
        let s = serde_json::to_string(&cfg).unwrap();
        let cfg2: Config = serde_json::from_str(&s).unwrap();
        assert_eq!(cfg.version, cfg2.version);
        assert_eq!(cfg.lines, cfg2.lines);
    }

    #[test]
    fn schema_is_valid_json_object() {
        let schema = Config::schema();
        assert!(schema.is_object());
    }

    #[test]
    fn needs_git_false_for_default_layout() {
        assert!(!Config::default_layout().needs_git());
    }

    #[test]
    fn needs_git_true_when_git_widget_present() {
        let cfg = Config {
            version: CONFIG_VERSION,
            tz: None,
            lines: vec![vec!["model".into(), "git_branch".into()]],
        };
        assert!(cfg.needs_git());
    }
}
