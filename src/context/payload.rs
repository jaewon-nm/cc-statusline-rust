//! Claude Code stdin payload — kept loose so unknown fields survive a refresh
//! schema change. Upstream parses with Zod `.looseObject`; the analog here is
//! `#[serde(flatten)] extra: Map<String, Value>` plus permissive numeric
//! coercion that mirrors upstream `CoercedNumberSchema`.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Payload {
    #[serde(default)]
    pub hook_event_name: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub transcript_path: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub model: Option<ModelField>,
    #[serde(default)]
    pub workspace: Option<Workspace>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub cost: Option<Cost>,
    #[serde(default)]
    pub context_window: Option<ContextWindow>,
    #[serde(default)]
    pub rate_limits: Option<RateLimits>,

    /// Namespaced extension carrying values that upstream computes from the
    /// transcript (e.g. `session_tokens`). Lives under its own key so it never
    /// collides with future upstream fields.
    #[serde(default, rename = "ccstatusline_rs")]
    pub extension: Option<Extension>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ModelField {
    Plain(String),
    Structured {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        display_name: Option<String>,
    },
}

impl ModelField {
    /// `display_name → id → plain`. Empty strings collapse to `None`.
    pub fn display_string(&self) -> Option<String> {
        match self {
            ModelField::Plain(s) => non_empty(s),
            ModelField::Structured { id, display_name } => display_name
                .as_deref()
                .and_then(non_empty)
                .or_else(|| id.as_deref().and_then(non_empty)),
        }
    }
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_owned())
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Workspace {
    #[serde(default)]
    pub current_dir: Option<String>,
    #[serde(default)]
    pub project_dir: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Cost {
    #[serde(default, deserialize_with = "deser_coerced_opt")]
    pub total_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ContextWindow {
    #[serde(default, deserialize_with = "deser_coerced_opt")]
    pub context_window_size: Option<f64>,
    #[serde(default, deserialize_with = "deser_coerced_opt")]
    pub total_input_tokens: Option<f64>,
    #[serde(default, deserialize_with = "deser_coerced_opt")]
    pub total_output_tokens: Option<f64>,
    #[serde(default)]
    pub current_usage: Option<CurrentUsage>,
    #[serde(default, deserialize_with = "deser_coerced_opt")]
    pub used_percentage: Option<f64>,
    #[serde(default, deserialize_with = "deser_coerced_opt")]
    pub remaining_percentage: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CurrentUsage {
    Scalar(#[serde(deserialize_with = "deser_coerced")] f64),
    Structured {
        #[serde(default, deserialize_with = "deser_coerced_opt")]
        input_tokens: Option<f64>,
        #[serde(default, deserialize_with = "deser_coerced_opt")]
        output_tokens: Option<f64>,
        #[serde(default, deserialize_with = "deser_coerced_opt")]
        cache_creation_input_tokens: Option<f64>,
        #[serde(default, deserialize_with = "deser_coerced_opt")]
        cache_read_input_tokens: Option<f64>,
    },
}

impl CurrentUsage {
    pub fn total(&self) -> f64 {
        match self {
            CurrentUsage::Scalar(v) => *v,
            CurrentUsage::Structured {
                input_tokens,
                output_tokens,
                cache_creation_input_tokens,
                cache_read_input_tokens,
            } => {
                input_tokens.unwrap_or(0.0)
                    + output_tokens.unwrap_or(0.0)
                    + cache_creation_input_tokens.unwrap_or(0.0)
                    + cache_read_input_tokens.unwrap_or(0.0)
            }
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RateLimits {
    #[serde(default)]
    pub five_hour: Option<RateLimitPeriod>,
    #[serde(default)]
    pub seven_day: Option<RateLimitPeriod>,
    #[serde(default)]
    pub seven_day_sonnet: Option<RateLimitPeriod>,
    #[serde(default)]
    pub seven_day_opus: Option<RateLimitPeriod>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RateLimitPeriod {
    #[serde(default, deserialize_with = "deser_coerced_opt")]
    pub used_percentage: Option<f64>,
    /// Unix epoch seconds.
    #[serde(default, deserialize_with = "deser_coerced_opt")]
    pub resets_at: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Extension {
    #[serde(default, deserialize_with = "deser_coerced_u64_opt")]
    pub session_tokens: Option<u64>,
}

fn deser_coerced<'de, D>(deser: D) -> std::result::Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    deser_coerced_opt(deser)?.ok_or_else(|| serde::de::Error::custom("expected number"))
}

fn deser_coerced_opt<'de, D>(deser: D) -> std::result::Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<Value>::deserialize(deser)?;
    Ok(raw.and_then(value_to_f64))
}

fn deser_coerced_u64_opt<'de, D>(deser: D) -> std::result::Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<Value>::deserialize(deser)?;
    Ok(raw.and_then(value_to_f64).and_then(|v| {
        if v.is_finite() && v >= 0.0 {
            Some(v as u64)
        } else {
            None
        }
    }))
}

fn value_to_f64(v: Value) -> Option<f64> {
    match v {
        Value::Null => None,
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                t.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

impl fmt::Display for ModelField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.display_string() {
            Some(s) => f.write_str(&s),
            None => f.write_str(""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_plain_round_trip() {
        let p: Payload = serde_json::from_str(r#"{ "model": "Opus 4.7 (1M context)" }"#).unwrap();
        assert_eq!(
            p.model.as_ref().unwrap().display_string().as_deref(),
            Some("Opus 4.7 (1M context)"),
        );
    }

    #[test]
    fn model_structured_prefers_display_name() {
        let p: Payload = serde_json::from_str(
            r#"{ "model": { "id": "claude-opus", "display_name": "Opus 4.7 (1M context)" } }"#,
        )
        .unwrap();
        assert_eq!(
            p.model.as_ref().unwrap().display_string().as_deref(),
            Some("Opus 4.7 (1M context)"),
        );
    }

    #[test]
    fn model_structured_falls_back_to_id() {
        let p: Payload = serde_json::from_str(r#"{ "model": { "id": "claude-opus" } }"#).unwrap();
        assert_eq!(
            p.model.as_ref().unwrap().display_string().as_deref(),
            Some("claude-opus"),
        );
    }

    #[test]
    fn coerced_number_accepts_string() {
        let p: Payload =
            serde_json::from_str(r#"{ "cost": { "total_cost_usd": "2.55" } }"#).unwrap();
        assert_eq!(p.cost.unwrap().total_cost_usd, Some(2.55));
    }

    #[test]
    fn coerced_number_rejects_garbage() {
        let p: Payload =
            serde_json::from_str(r#"{ "cost": { "total_cost_usd": "n/a" } }"#).unwrap();
        assert!(p.cost.unwrap().total_cost_usd.is_none());
    }

    #[test]
    fn current_usage_accepts_scalar() {
        let p: Payload =
            serde_json::from_str(r#"{ "context_window": { "current_usage": 80000 } }"#).unwrap();
        let usage = p.context_window.unwrap().current_usage.unwrap();
        assert_eq!(usage.total(), 80000.0);
    }

    #[test]
    fn current_usage_accepts_structured_sum() {
        let p: Payload = serde_json::from_str(
            r#"{ "context_window": { "current_usage": {
                "input_tokens": 50000,
                "output_tokens": 30000,
                "cache_read_input_tokens": 0
            } } }"#,
        )
        .unwrap();
        let usage = p.context_window.unwrap().current_usage.unwrap();
        assert_eq!(usage.total(), 80000.0);
    }

    #[test]
    fn extension_namespaced_field_parses() {
        let p: Payload =
            serde_json::from_str(r#"{ "ccstatusline_rs": { "session_tokens": 85300 } }"#).unwrap();
        assert_eq!(p.extension.unwrap().session_tokens, Some(85300));
    }

    #[test]
    fn unknown_top_level_field_survives_in_extra() {
        let p: Payload = serde_json::from_str(r#"{ "future_field": { "a": 1 } }"#).unwrap();
        assert!(p.extra.contains_key("future_field"));
    }

    #[test]
    fn round_trip_preserves_payload() {
        let json = r#"{
            "cwd": "F:\\Works\\naya",
            "model": "Opus 4.7 (1M context)",
            "rate_limits": { "five_hour": { "used_percentage": 21, "resets_at": 1747188000 } }
        }"#;
        let p: Payload = serde_json::from_str(json).unwrap();
        let again = serde_json::to_string(&p).unwrap();
        let p2: Payload = serde_json::from_str(&again).unwrap();
        assert_eq!(p.cwd, p2.cwd);
        assert_eq!(
            p.rate_limits
                .as_ref()
                .unwrap()
                .five_hour
                .as_ref()
                .unwrap()
                .used_percentage,
            p2.rate_limits
                .as_ref()
                .unwrap()
                .five_hour
                .as_ref()
                .unwrap()
                .used_percentage,
        );
    }
}
