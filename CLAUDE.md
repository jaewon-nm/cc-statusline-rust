# cc-statusline-rust - Project Guidelines

**CRITICAL LANGUAGE RULE — NEVER VIOLATE:**
Think and reason internally in English. **Always respond to the user in Korean (한국어).** This applies to all messages, explanations, questions, and status updates. No exceptions.

## Project Overview

A Rust port of [`ccstatusline`](https://github.com/sirmalloc/ccstatusline) — a customizable status line formatter for the Claude Code CLI.

Two surfaces share the same core:

- **Renderer (default invocation):** Reads a single Claude Code status payload from stdin and prints a formatted status line to stdout. Must be fast (~tens of ms) because Claude Code re-runs it on every refresh.
- **Agent-friendly CLI:** Declarative subcommands for AI agents (not humans) to inspect, edit, validate, and preview configuration. **No interactive TUI** — the upstream Ink/React configurator is intentionally omitted. Every user-visible config operation has a single-shot subcommand whose stdout is machine-parseable JSON by default.

Both surfaces are synchronous and single-threaded.

Reference source (TypeScript/Bun, do not modify): [`references/ccstatusline/`](references/ccstatusline/). The reference includes a TUI configurator under `src/tui/` — port the rendering/widget/config logic, **skip** the TUI.

## Architecture

Detailed design — process model, CLI surface, config persistence, source layout — lives in [`docs/design-docs/cc-statusline-rs.md`](docs/design-docs/cc-statusline-rs.md). Read that before changing any of: widget contracts, CLI subcommand list, config file shape, or rendering pipeline.

Hard rules (the rest is in the design doc):

- **Process model:** One short-lived process per invocation. No daemon, no async runtime, no background threads.
- **Agent surface contract:** Non-renderer subcommands emit JSON on stdout. Errors go to stderr + a JSON envelope on stdout with non-zero exit. `--pretty` is the only opt-in to human formatting.
- **Config writes are atomic.** Write-temp + rename. An agent edit may race a renderer read.

## Prerequisites

- Rust toolchain (see Rust Development Rules below for version).
- `git` on PATH for git widgets. Optional: `gh`, `glab` for PR/MR widgets.
- No other native deps. Final artifact is a single static binary per target.

## Rust Development Rules (MANDATORY)

- Use **Rust 1.94** (pinned for cross-project compatibility with neo-codebase-unity and other in-house projects). Do not bump unilaterally; coordinate before upgrading.
- All crates must use **latest versions**. Check crates.io at implementation time.
- Always verify latest API/syntax via **context7 MCP** (`resolve-library-id` → `query-docs`) and **web search** before coding. Do not rely on training data.

### Code Tripwire — Three Quality Wires (trip any wire = incomplete code)

> **Code Tripwire** defines three quality trip wires for this project. Tripping any wire means the code is incomplete and must be fixed before proceeding.

#### Type Wire — strict typing
- Never use `Box<dyn Error>`, `anyhow!("{e}")`, `.unwrap_or(fallback)` to silence type mismatches.
- Use `thiserror` enums with `#[from]` for error conversion.
- `anyhow` is allowed only in `main.rs`; all library code must use concrete error types.
- When a crate returns an incompatible error type, add a proper `#[from]` variant or `.map_err()` with a named variant — never stringify.

#### Test Wire — 100% coverage testing
**Code without tests is not an implementation.**
1. All modules require unit tests. Code-only submissions are incomplete.
2. Failing tests must be fixed until they pass — **fix the CODE, never delete/modify/ignore the test.**
3. Dummy tests (`assert!(true)`) are forbidden. Only tests verifying actual behavior are allowed.
4. Bug fixes must start with a reproduction test.
5. Coverage: happy path + edge cases + error handling.
6. Skipping or deferring tests is not allowed.
7. **If you cannot fix a test failure on your own, discuss with Codex before proceeding.** Never arbitrarily modify, delete, or ignore failing tests.

**Test tools (phased introduction):**
- **Phase 0 (spike):** Built-in `#[test]`, `tempfile`, `assert_cmd` + `predicates`, `insta` for snapshot tests of rendered lines
- **Phase 1+:** `rstest`, `proptest` (widget invariants), `mockall` (only if pure-fn boundaries make it useful)
- **Renderer tests:** golden-file / snapshot tests on rendered ANSI output for representative payloads (`scripts/payload.example.json` in the reference repo is a starting fixture).
- **CLI tests:** `assert_cmd` + `predicates` for subcommand invocation; assert stdout JSON via `serde_json` round-trip, never on raw text.
- **Runner:** `cargo-nextest`
- **Coverage:** `cargo-llvm-cov` (Windows-compatible)
- **Not used:** `test-case` (rstest superset), `quickcheck` (proptest preferred), `cargo-tarpaulin` (no Windows support)

#### Why Wire — WHY-only comments
- Comment only WHY — rationale, constraints, gotchas. No what/how comments.
- Start each file with a 2-4 line `///` or `//!` doc header (purpose + loading context).
- Include reason when suppressing lint: `#[allow(clippy::rule)] // reason`.
- **No temporal task references in code.** Do not write `// Phase 1:`, `// ticket #123`, `// Codex thread 019da...`, or similar work-session tags inside source comments. Phase numbers lose meaning once the codebase evolves and the narrative belongs in commit messages / plan docs, not in code that future readers will see with no context. Keep only the WHY — rationale, constraints, gotchas. Writing new code and touching old stale comments: strip the Phase/ticket prefix and rephrase as pure WHY. Leave existing commit messages and `docs/exec-plans/` narratives alone — those are legitimate work-history artifacts.

## Performance Budget

- **Renderer cold start to first byte:** budget ≤ 50 ms on a warm cache; aim for ≤ 20 ms.
- **No async runtime on the renderer path.** Async is allowed only inside the TUI subcommand if a widget probe genuinely benefits.
- **Avoid heavy crates on the hot path:** `regex` over `fancy-regex`, no `tokio` macros at module level, no `lazy_static`/`once_cell` with heavy init in renderer modules.

## Documentation

- Project docs are organized by category under `docs/`.
- Always read `docs/INDEX.md` first when exploring documentation.
- Key categories: `design-docs/` (design), `exec-plans/` (implementation plans), `researches/` (research)
- Detailed governance rules: `docs/GOVERNANCE.md`

### Documentation Governance (Mandatory)

- Before starting implementation, check `docs/INDEX.md` and reference the relevant `design-docs`.
- During implementation, immediately update the corresponding `docs/exec-plans/active/*.md` checklist.
- On completion, `git mv` the exec-plan from `active/` to `completed/` and fill in the completion section.
- All completed work must update `docs/STATUS.md` in the same commit.

## Distribution

Single static binary per platform.

```bash
# Local dev
cargo run --bin ccstatusline-rs -- < scripts/payload.example.json
cargo run --bin ccstatusline-rs -- configure

# Release
cargo build --release
# target/release/ccstatusline-rs(.exe)
```

Targets to support: `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`. Distribution channels (TBD in exec-plan): `cargo install`, GitHub Releases binaries, optionally Homebrew tap.

## Key Dependencies (target — verify latest at implementation time)

| crate | role |
|-------|------|
| `clap` | CLI subcommand parsing (`derive` feature) |
| `serde` + `serde_json` | stdin payload + config file |
| `directories` | XDG/AppData config path resolution |
| `anstyle` + `anstyle-query` | typed ANSI styling on the renderer path (Segment → ANSI separable for snapshot tests); `anstyle-query` for NO_COLOR / FORCE_COLOR / TTY detection |
| `schemars` | emit JSON Schema for the config (agent-discoverable surface) |
| `thiserror` | typed errors per Type Wire |
| `jiff` | timer/reset-time widgets — IANA tz built-in, calendar-aware durations (chosen over chrono+chrono-tz to avoid embedded tz DB and naive/aware footguns) |
| `unicode-width` | width-aware truncation for narrow terminals |
| `regex` | minimal text munging (model name stripping, etc.) |
| `tempfile` (dev) | snapshot/golden-file scaffolding |
| `assert_cmd` + `predicates` (dev) | end-to-end renderer tests |
| `insta` (dev) | snapshot tests for rendered ANSI |
| `cargo-nextest` (dev runner) | test execution |
| `cargo-llvm-cov` (dev) | coverage |

Crates **not** carried over from neo-codebase-unity governance (intentionally absent): `tokio`, `libsql`, `rmcp`, `petgraph`, `capnp`, `rayon`, `crossbeam`, `moka`, `notify`, `rkyv`, `process-wrap`, `sonic-rs`, `xxhash-rust`, `ratatui`, `crossterm`. Add back only with an exec-plan that justifies the cost on the renderer path.

## Default Theme

The renderer's default output is locked by [`docs/design-docs/default-theme.md`](docs/design-docs/default-theme.md). That file is the canonical `insta` snapshot baseline; any byte change there is a behavior change and follows the change policy in the same doc.
