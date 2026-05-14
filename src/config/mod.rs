//! Config schema. Bootstrap ships the default layout only; full edit surface
//! (add/remove/set/apply) lands in M2. The schemars-derived schema is exposed
//! via the `schema` subcommand so agents can self-discover the shape.

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
}
