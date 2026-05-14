# Changelog

All notable changes to `ccstatusline-rs`. Date format: ISO `YYYY-MM-DD`. Versioning follows SemVer on the **public surface** (CLI subcommand contract, config schema, default-theme output bytes).

## Unreleased

### Added

- **`install` / `uninstall` subcommands (005).** One-shot wiring into Claude Code: copies the binary to `~/bin` (Windows) or `~/.local/bin` (POSIX), writes a `.mjs` wrapper on Windows (works around the Claude Code Windows-native `statusLine` regression [#31670](https://github.com/anthropics/claude-code/issues/31670)), backs up `~/.claude/settings.json`, and rewrites only the `statusLine` block. Unknown top-level settings keys survive verbatim through a `#[serde(flatten)] extra` round-trip. `uninstall` reverts the most-recent install via atomic temp+rename, with `--purge-binary` to also remove the binary and wrapper. Codex 4-round verify-plan AGREE before implementation.
- `crate::ioutil::atomic_write_bytes` — shared atomic-write helper. Temp filename includes pid + monotonic counter so concurrent installers don't collide. `Config::save` now routes through it.
- New typed errors: `Error::FileIo { operation, path, source }` and `Error::NoBackupFound { settings }`.
- **Distribution scaffolding (M5).** GitHub Actions CI (test + clippy + fmt on Linux/macOS/Windows) and Release workflow (4 target triples: `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`). Tagged pushes (`v*.*.*`) produce per-target archives with SHA-256 companions, attached to a generated GitHub Release.
- `INSTALL.md` restructured into three sections (Recommended / Manual / Uninstall).
- `CHANGELOG.md` (this file).

### Changed

- Release profile already set in M0 (`lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`). Local Windows MSVC build measures **1,910,272 bytes**.

## 0.1.0 — first taggable release (pending)

Aggregates M0–M4. See [`docs/STATUS.md`](docs/STATUS.md) and the per-phase exec-plans under `docs/exec-plans/completed/`.

### M0 — Bootstrap (2026-05-14)

- Cargo + Rust 1.94 pinned.
- 7 default-theme widgets (`model`, `cwd`, `context_bar`, `session_tokens`, `session_cost`, `block_timer`, `weekly_timer`).
- `anstyle` Segment data model decoupled from ANSI emission.
- KST default timezone with `tz` override; payload `transcript_path` reserved.
- `insta` golden snapshot for the locked default theme.

### M1 — Probes (2026-05-14)

- `context/jsonl.rs` cumulative session-token probe with streaming dedup.
- `context/git.rs` branch / porcelain / shortstat probes (800 ms timeout, xxh3-keyed disk cache, 2 s TTL).
- `git_branch` / `git_status` / `git_changes` widgets (opt-in via config).
- `config::load_or_default` + `$CCSTATUSLINE_RS_CONFIG` test override + `needs_git` gate.

### M2 — Config persistence + JSONL cache (2026-05-14)

- `config add / remove / apply / validate` subcommands; atomic temp + rename.
- Widget-kind validation against `widgets::REGISTRY`; unknown kinds rejected before disk write.
- JSONL probe gained an `(mtime_ns, size)`-keyed disk cache (`cache_dir/jsonl/<xxh3>.json`).

### M3 — `preview --diff` (2026-05-14)

- `preview --config <file> --diff` → JSON `{ current, pending, identical }` so an agent can compare a candidate against the live render before `config apply`.

### M4 — Per-widget color (2026-05-14)

- `ColorStyle { fg, bg, bold }` on the config, additive (no schema bump).
- `parse_color` validates `"red"` / `"bright_blue"` / `"#rrggbb"` at edit time, never at render time.
- `color_enabled()` honors `NO_COLOR` (wins) → `CLICOLOR_FORCE` → `FORCE_COLOR` → default.
- `config color <kind> [--fg <c>] [--bg <c>] [--bold | --no-bold] [--clear]`.

## Conventions

- The `Unreleased` section accumulates changes between tagged releases.
- When cutting a release, rename the heading to `v<x.y.z> — <YYYY-MM-DD>` and add a new empty `Unreleased`.
- Default-theme byte changes are a public surface change → minor bump + entry under **Changed** with the new golden block linked.
