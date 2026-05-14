# 003 — `preview --diff` + per-widget color

**Status:** ✅ completed · **Owner:** jaewon_lee · **Opened:** 2026-05-14 · **Closed:** 2026-05-14

## Goal

Two agent-facing capabilities, scoped together because they share the same wiring (renderer now consumes `&Config` directly):

1. **`preview --diff`.** Compare the rendered output of the current on-disk config against a candidate file, against any payload. JSON envelope so an agent can `apply` only after observing the diff.
2. **Per-widget color.** Add a `colors: { widget_kind → { fg, bg, bold } }` map to the config schema; respect `NO_COLOR` / `FORCE_COLOR` / `CLICOLOR_FORCE` at render time. Ship a `config color` subcommand so agents don't need to hand-edit JSON.

## What landed

### M3 — `preview --diff`

`cli/preview.rs`:

- New flags `--config <file>` (candidate config) and `--diff` (JSON envelope).
- Without `--diff`: renders against `--config` if supplied, else the current on-disk config. Payload comes from `--payload <file>` or stdin.
- With `--diff`: requires `--config`; emits `{ "current": "...", "pending": "...", "identical": bool }`. Renderer is called twice over the same payload — once per config.
- Bad usage (`--diff` without `--config`) returns `Error::InvalidConfig` and fails fast with non-zero exit.

### M4 — Color

`render/color.rs` (new module):

- `ColorStyle { fg, bg, bold }` — serde-friendly string encoding (`"red"`, `"bright_blue"`, `"#1abc9c"`). All standard ANSI named colors + the eight `bright_*` variants + 24-bit hex.
- `parse_color(s)` validates and returns an `anstyle::Color`; unknown names / bad hex error out so we can reject at `config validate` / `config color` time, never at render time.
- `ColorStyle::to_style()` assembles an `anstyle::Style`.
- `color_enabled(default)` resolves the three env knobs:
  - `NO_COLOR` (anstyle-query) — disables.
  - `CLICOLOR_FORCE` (anstyle-query) — forces.
  - `FORCE_COLOR` (read directly because anstyle-query doesn't expose it) — forces. We honor both `FORCE_COLOR` and `CLICOLOR_FORCE` because both are widely used.

`config/mod.rs`:

- `Config.colors: BTreeMap<String, ColorStyle>` (sorted output, deterministic JSON).
- `Config::validate` now also checks every color entry: kind must be registered, fg/bg strings must `parse_color`.
- `default_layout()` initializes `colors` empty so the default theme stays plain (snapshot byte-stable).

`cli/config_cmds.rs`:

- New `config color <kind> [--fg <c>] [--bg <c>] [--bold | --no-bold] [--clear]` subcommand.
- Clap-level conflicts: `--bold` vs `--no-bold`, `--clear` vs everything else.
- Validates kind + parses colors before persisting; rejects empty arg sets.
- Emits the new config JSON on stdout so the agent immediately sees the diff.

`render/mod.rs`:

- Signature change: `render(ctx, cfg: &Config)` (was `render(ctx, lines)`). Build path looks up `cfg.colors[widget_kind]` for each segment, applies the style only when `color_enabled(false)` returns true. Color stays opt-in: no config entry = no ANSI, regardless of env.

### Tests added

`tests/preview_and_color.rs` (10 cases, parallel-safe via per-test `CCSTATUSLINE_RS_CONFIG`):

- `preview_diff_reports_identical_when_candidate_equals_current`
- `preview_diff_distinguishes_when_layouts_differ`
- `preview_diff_without_config_fails`
- `config_color_persists_style`
- `config_color_clear_removes_entry`
- `config_color_rejects_invalid_color_string`
- `config_color_rejects_unknown_widget_kind`
- `renderer_emits_ansi_when_force_color_is_set`
- `renderer_strips_color_when_no_color_is_set`
- `config_apply_rejects_invalid_color_in_file`

Plus 5 new unit tests in `render::color::tests` and 2 in `config::tests` (validator covers both color-on-unknown-kind and unparseable color strings).

## Acceptance

- `cargo nextest run --test-threads=1` → **123 / 123 passed**.
- `cargo clippy --all-targets -- -D warnings` clean.
- `cargo fmt --check` clean.
- Default-theme snapshot byte-identical (default `Config` has empty `colors`, so the build path is unchanged when no styling is configured).
- `NO_COLOR` strips ANSI even when `FORCE_COLOR` is set (correct precedence per [NO_COLOR informational standard](https://no-color.org/)).

## Out-of-scope (deferred)

- **Powerline separators / styled separators** — needs its own design pass; out of scope here.
- **Theme presets** (`config apply --theme cyberpunk`) — easy follow-up once a couple of curated themes exist.
- **Per-line / per-position color overrides** — current scope is per-widget-kind. The Segment data model supports the richer case if we ever ship it.
- **Sub-string color highlights** inside a widget — e.g. coloring just the percent number red when above 80%. Belongs to a later "widget options" milestone.

## Notes for downstream

- The `Config::colors` map is **additive**: bumping `CONFIG_VERSION` is not required to introduce it. Older configs without a `colors` key still load cleanly (serde default).
- M5 (distribution) should remember to document `FORCE_COLOR=1` as the canonical opt-in inside the Claude Code installation guide — terminals pipe stdout to the binary, so detection-based color won't fire automatically.