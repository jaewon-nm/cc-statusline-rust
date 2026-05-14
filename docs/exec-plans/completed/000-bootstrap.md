# 000 — Bootstrap

**Status:** ✅ completed · **Owner:** jaewon_lee · **Opened:** 2026-05-14 · **Closed:** 2026-05-14

## Goal

Stand up the Cargo project end-to-end so that a single command renders the **default theme** byte-for-byte against the canonical payload fixture, with an `insta` snapshot locking it.

Success = `cargo nextest run` is green and includes a snapshot test whose recorded output equals the golden block in [`../../design-docs/default-theme.md`](../../design-docs/default-theme.md).

## Non-goals (deferred)

- Config file persistence (M2)
- Most CLI subcommands beyond `schema`/`widgets` stubs (M2)
- Color (M4)
- Git / GitHub / GitLab probes — the bootstrap widgets read everything from the payload (M1 will add real probes for any data not in the payload)
- Cross-compile / release CI (M5)

## Phase 0 — Cargo workspace

- [ ] `cargo init --bin --name ccstatusline-rs`
- [ ] Pin toolchain: `rust-toolchain.toml` with `channel = "1.94"`, `profile = "minimal"`
- [ ] `Cargo.toml`:
  - `[package] edition = "2021"` (1.94's latest stable edition)
  - `[[bin]] name = "ccstatusline-rs"`, `path = "src/main.rs"`
  - Dependencies: `clap` (derive), `serde`/`serde_json` (derive), `thiserror`, `anstyle`, `anstyle-query`, `schemars`, `jiff`, `unicode-width`, `directories`, `tempfile` (dev), `assert_cmd` (dev), `predicates` (dev), `insta` (dev)
  - Verify each version on crates.io at implementation time (CLAUDE.md rule)
- [ ] `.gitignore` (Rust defaults + `target/`, `.snap.new`)
- [ ] `cargo build` succeeds with empty `main.rs`

## Phase 1 — Source skeleton

- [ ] Module tree:
  ```
  src/
    main.rs              — clap dispatch + anyhow boundary
    lib.rs               — re-exports for tests
    cli/mod.rs           — subcommand types
    cli/render.rs        — default invocation (renderer)
    cli/inspect.rs       — schema, widgets stubs
    cli/config_cmds.rs   — config show/add/set/remove/apply/validate stubs (return "not implemented" JSON for now)
    cli/preview.rs       — preview stub
    config/mod.rs        — Config struct (serde + schemars), default(), validate()
    context/mod.rs       — Context struct (payload + derived fields)
    context/payload.rs   — Claude Code stdin payload struct
    widgets/mod.rs       — Widget trait + Segment struct + registry
    widgets/model.rs
    widgets/cwd.rs
    widgets/context_bar.rs
    widgets/session_tokens.rs
    widgets/session_cost.rs
    widgets/block_timer.rs
    widgets/weekly_timer.rs
    render/mod.rs        — segment list → ANSI line(s); width fitting
    render/format.rs     — token abbrev, cost, percent, bar, timestamps
    error.rs             — single thiserror enum
  ```
- [ ] File-level `//!` doc headers (Why Wire rule)

## Phase 2 — Payload struct & fixture

- [ ] Inspect upstream payload via `references/ccstatusline/scripts/payload.example.json` (and any callsites in `src/`)
- [ ] Define `Payload` in `context/payload.rs` with `serde(deny_unknown_fields = false)` (Claude Code may add fields)
- [ ] Author `tests/fixtures/default-payload.json` matching the values in the golden output:
  - model: `Opus 4.7 (1M context)`
  - cwd: `F:\Works\naya\cc-statusline-rust`
  - context: 80,000 used / 1,000,000 total → 8%
  - session: 85,300 tokens, $2.55 cost
  - block: 5-hour window, 21% elapsed, reset at 12:00 local
  - weekly: 7-day window, 20% elapsed, reset at 5/19 06:00 local
- [ ] Unit test: payload round-trips through serde (parse + serialize + reparse)

## Phase 3 — Widgets producing default theme

Each widget = pure `fn(&Context) -> Option<Segment>`. Segment carries text + style; style stays default (no color) for M0.

- [ ] Model widget → `✦ [Opus 4.7 (1M context)]`
- [ ] Cwd widget → `📂 F:\Works\naya\cc-statusline-rust`
- [ ] Context bar widget → `🔋 [..........] 80.0K/1.0M(8%)`
- [ ] Session tokens widget → `📊 85.3K`
- [ ] Session cost widget → `💰 $2.55`
- [ ] Block timer widget → `⏱ 5h [##........](21%) ↻ 12:00`
- [ ] Weekly timer widget → `📅 7d [##........](20%) ↻ 5/19 06:00`
- [ ] Each widget: unit test on a minimal context that exercises edge cases (zero, max, fractional)
- [ ] `render/format.rs`: `abbreviate_tokens`, `format_cost`, `format_percent_paren`, `format_bar(width=10, pct, '#', '.')`, `format_block_reset`, `format_weekly_reset` — all locale-independent

## Phase 4 — Renderer

- [ ] `render::render(config, context) -> String` composes two lines, ` | ` separator, `\n` between
- [ ] CLI default branch (`main.rs`): read stdin → parse Payload → build Context → render → write stdout
- [ ] Integration test via `assert_cmd`: pipe `default-payload.json`, compare stdout to fixture string
- [ ] **`insta` snapshot test** locking the exact bytes; this is the canonical golden snapshot

## Phase 5 — Inspection stubs

- [ ] `ccstatusline-rs schema` → emits `schemars`-generated JSON Schema for `Config`
- [ ] `ccstatusline-rs widgets` → JSON array of `{ "kind": "...", "options_schema": {...} }`
- [ ] Each returns valid JSON; assert by `serde_json::from_slice`

## Phase 6 — Wire-up & gates

- [ ] `cargo nextest run` green, no `#[ignore]`, no skipped tests
- [ ] `cargo clippy --all-targets -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] `cargo llvm-cov --summary-only` reports ≥ 80% line coverage on the bootstrap modules (hard target tuned by what the implementation actually needs)
- [ ] `STATUS.md` updated with bootstrap completion + snapshot path
- [ ] `git mv docs/exec-plans/active/000-bootstrap.md docs/exec-plans/completed/`

## Risks & open questions

- **Payload schema drift.** Claude Code may evolve the payload. Mitigation: `serde(deny_unknown_fields = false)` and a `_extra: serde_json::Value` catch-all on `Payload` so unknown fields don't fail parsing.
- **Locale-dependent formatting.** Korean Windows can default to comma decimal. Verify the snapshot is stable on the dev box (`F:\…`).
- **Emoji width on Windows Terminal.** `unicode-width` reports `✦`, `📂`, `🔋`, etc. — confirm the snapshot doesn't rely on terminal-side width handling. The renderer should not pad to a fixed width in the default theme.
- **`jiff` API surface.** Reset timestamps use local timezone. Verify the API for "format with month/day no padding" — may need `strftime`-style or a manual format helper.

## Acceptance

The single command below produces output byte-identical to the golden block in [`default-theme.md`](../../design-docs/default-theme.md):

```powershell
Get-Content tests/fixtures/default-payload.json | cargo run --bin ccstatusline-rs
```

And the snapshot test runs as part of `cargo nextest run` with no `--accept`.

## Completion (2026-05-14)

Met. Single command on a release binary produces byte-identical golden:

```
✦ [Opus 4.7 (1M context)] | 📂 F:\Works\naya\cc-statusline-rust | 🔋 [..........] 80.0K/1.0M(8%) | 📊 85.3K | 💰 $2.55
⏱ 5h [##........](21%) ↻ 12:00 | 📅 7d [##........](20%) ↻ 5/19 06:00
```

### Acceptance

- `cargo nextest run` → **58/58 passed, 0 skipped** (lib unit + integration + snapshot).
- `cargo clippy --all-targets -- -D warnings` clean.
- `cargo fmt --check` clean.
- `insta` snapshot locked at `tests/snapshots/render_default_theme__default_theme_snapshot.snap`.
- End-to-end smoke: `Get-Content tests/fixtures/default-payload.json | ./target/release/ccstatusline-rs.exe` matches the golden line-for-line.

### Design decisions captured (vs. plan as written)

- **Edition.** Crate compiles on Rust 1.94 with `edition = "2024"` (stabilized 1.85) rather than the `2021` shown in the plan.
- **Widget dispatch.** `WidgetSpec { kind: &'static str, render: fn(&Context) -> Option<Segment> }` over a `REGISTRY` array, not a trait object — pure fn over an immutable `Context` does not need trait objects (Codex review).
- **`Context`.** Carries semantic types (`u64`, `f64`, `Timestamp`), never pre-formatted strings; presentation lives in `render/format.rs`.
- **`session_tokens` injection.** Lives under the namespaced extension key `"ccstatusline_rs": { "session_tokens": … }` on the payload, not a top-level `_bootstrap_*` field. A future JSONL probe will populate it without disturbing the renderer.
- **Default timezone.** KST (`Asia/Seoul`) is the project default. `tz: null` / `""` → KST; `"system"` opts into the host clock; explicit IANA names are accepted (per-user request mid-implementation).
- **`#[serde(flatten)] extra: Map<String, Value>`** preserves unknown payload fields without failing parsing — replaces the non-existent `serde(deny_unknown_fields = false)` originally mentioned.
- **Numeric coercion** mirrors upstream Zod (`Number → f64`, `"2.55" → 2.55`, garbage → `None`).
- **Percent clamp.** NaN collapses to 0, then `f64::clamp(0, 100)`. Implemented in two places (`context::mod` and `render::format`) by design — same rule, different layers; refactor to a shared helper is a follow-up.

### Test counts

| Surface | Count |
|---|---|
| `payload::tests` | 9 |
| `context::tests` | 7 |
| `render::tests` | 3 |
| `render::format::tests` | 12 |
| `config::tests` | 2 |
| `widgets/*::tests` | 14 (2 per widget) |
| `tests/render_default_theme.rs` | 2 (string + snapshot) |
| `tests/cli_integration.rs` | 5 |
| **Total** | **58** |

### Out of scope (deferred per plan)

- Coverage gate (`cargo llvm-cov --summary-only ≥ 80%`) — measured manually only; CI wiring + threshold lives with M5 / Distribution.
- Real JSONL probe for `session_tokens` (M1).
- Color (M4).
- Git / GitHub / GitLab widgets (M1+ when probes are added).
- Config persistence + `add/remove/set/apply` (M2).
