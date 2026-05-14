//! Widget registry. Each `WidgetSpec` is a fn pointer over an immutable
//! `Context`; widgets stay pure so the renderer can be snapshot-tested without
//! touching real time / git / disk.

mod block_timer;
mod context_bar;
mod cwd;
mod model;
mod session_cost;
mod session_tokens;
mod weekly_timer;

use crate::context::Context;
use crate::render::Segment;

pub struct WidgetSpec {
    pub kind: &'static str,
    pub render: fn(&Context) -> Option<Segment>,
}

pub const REGISTRY: &[WidgetSpec] = &[
    WidgetSpec {
        kind: "model",
        render: model::render,
    },
    WidgetSpec {
        kind: "cwd",
        render: cwd::render,
    },
    WidgetSpec {
        kind: "context_bar",
        render: context_bar::render,
    },
    WidgetSpec {
        kind: "session_tokens",
        render: session_tokens::render,
    },
    WidgetSpec {
        kind: "session_cost",
        render: session_cost::render,
    },
    WidgetSpec {
        kind: "block_timer",
        render: block_timer::render,
    },
    WidgetSpec {
        kind: "weekly_timer",
        render: weekly_timer::render,
    },
];

pub fn find(kind: &str) -> Option<&'static WidgetSpec> {
    REGISTRY.iter().find(|w| w.kind == kind)
}

/// Two-row layout matching `docs/design-docs/default-theme.md`.
pub fn default_layout() -> &'static [&'static [&'static str]] {
    &[
        &[
            "model",
            "cwd",
            "context_bar",
            "session_tokens",
            "session_cost",
        ],
        &["block_timer", "weekly_timer"],
    ]
}
