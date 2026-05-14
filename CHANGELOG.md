# Changelog

All notable changes to `ccstatusline-rs`. Date format: ISO `YYYY-MM-DD`. Versioning follows SemVer on the **public surface** (CLI subcommand contract, config schema, default-theme output bytes).

## Unreleased

_No changes yet._

## v0.1.3 — 2026-05-14

Tokenwatch-aware install / uninstall. Coexists with neo-mem's `tokenwatch-statusline.mjs` instead of clobbering its `statusLine` hook.

### Changed (JSON contract — agents that pattern-match this field, update)

- **`UninstallReport.restored_from` is now `Option<PathBuf>`** (was unconditional `PathBuf` in 0.1.0–0.1.2). Direct mode still emits a path string; new wrap mode emits `null` because settings.json was never touched. Same field name, additive value — only consumers that asserted `restored_from` is always a string need a tweak.
- **`UninstallReport` gained `mode: "direct" | "wrap"`** plus a `removed_wrap_prev` field. **`InstallReport` gained `mode`** plus `wrap_prev_path`, `previous_wrap_command`, and `wrap_explanation`. All are additive and absent-when-`null` in serialized output thanks to `skip_serializing_if`.

### Added

- **Tokenwatch wrap-mode detection (007).** `install` now inspects `settings.json statusLine.command` before touching it. If the command's basename is `tokenwatch-statusline.mjs` (neo-mem's man-in-the-middle), install switches to **wrap mode**: copies the binary + Windows wrapper as usual, then writes our command into `~/.claude/.tw-statusline-prev.json` (the pointer tokenwatch reads to delegate downstream). settings.json is left byte-identical. Operators see `mode: "wrap"`, `backup: null`, and a `wrap_explanation` line in the JSON report.
- **Basename detection (`contains_basename`).** Boundary-scanned substring match — surrounding bytes must be path separators, quotes, or whitespace. Survives Windows quoted paths with spaces (e.g. `node "C:\Users\Jane Doe\.neo-mem\…\tokenwatch-statusline.mjs"`) and rejects `my-tokenwatch-statusline.mjs` lookalikes.
- **Positive-evidence uninstall.** Wrap mode requires both `settings.statusLine.command` to be tokenwatch AND `.tw-statusline-prev.json` to point at us. Direct mode falls back to "a backup we wrote exists" OR "settings command is ours". When neither holds, uninstall aborts with `NoInstallTraces` rather than restoring the wrong artifact. An explicit `--backup <path>` always forces Direct mode so operators stay in control.
- **Stale-pointer guard.** If `.tw-statusline-prev.json` is ours but `settings.json` is no longer tokenwatch, uninstall fails loudly with `StaleWrapPointer { prev_path, settings_command }` instead of silently restoring a backup that no longer matches reality.
- **`WrapConflict` rejection.** Pre-existing `.tw-statusline-prev.json` referencing a non-ours command (some other operator-installed wrap) causes install to refuse with the existing command verbatim in the error message. No multi-wrap chaining; the operator decides what wins.
- New typed errors: `Error::WrapConflict`, `Error::NoInstallTraces`, `Error::StaleWrapPointer`, `Error::InvalidPrev { source: serde_json::Error }` (via `#[source]`, never stringified).
- **`InstallMode` / `UninstallMode`** enums on the report structs, JSON-serialized as `"direct" | "wrap"`.

### Notes

- The renderer path is unchanged — wrap mode is purely an install-time routing decision.
- `--bin-dir` relocation (install A → install B) is correctly recognized as "still ours" via basename match, so the prev pointer is updated in place, never flagged as `WrapConflict`.
- 12 new wrap-mode end-to-end tests in `tests/install_uninstall_wrap.rs`, 8 new unit tests in `src/cli/install.rs::tests`. All 192 / 192 tests pass.

## v0.1.2 — 2026-05-14

CI hotfix release. No library / CLI behavior changes.

### Fixed

- **Intel macOS (x86_64-apple-darwin) dropped from the release matrix.** GitHub-hosted `macos-13` Intel runners are being deprecated and queue times grew to 15+ minutes, blocking the publish job (`needs: build`) on otherwise-green runs. Intel macOS users have a clean source-build path: `cargo install --git https://github.com/jaewon-nm/cc-statusline-rust ccstatusline-rs`. The four pre-built archives now shipped on each tag: `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`. Apple Silicon is the only macOS build hosted — adequate given Apple's full transition to ARM. v0.1.1 release run was cancelled mid-queue.

### CI

- First-party GitHub Actions (`actions/checkout`, `actions/upload-artifact`, `actions/download-artifact`) bumped to their Node 24 successors (v5 / v6 / v5) so the deprecation warnings on `actions/checkout@v4` etc. disappear ahead of the 2026-09-16 Node 20 removal.

## v0.1.1 — 2026-05-14

CI hotfix release. No library / CLI behavior changes — same binary surface as v0.1.0.

### Fixed

- **macOS CI runner toolchain install.** `dtolnay/rust-toolchain@master` was failing to install Rust 1.94 on `macos-latest` (arm64) and `macos-13` (x86_64) silently, sending `cargo build` through `rustup-init` in bootstrap mode and crashing with `unexpected argument 'build'`. Switched both `ci.yml` and `release.yml` to `actions-rust-lang/setup-rust-toolchain@v1` and added an explicit `rustup show active-toolchain` verification step before any cargo invocation. The v0.1.0 release run was cancelled mid-queue; v0.1.1 is the first tag that exercises the fixed workflow.

## v0.1.0 — 2026-05-14

First tagged release. Aggregates milestones M0 through 006 (default theme color). 172 / 172 tests; Codex-reviewed at every plan boundary. Pre-built archives per target triple on the GitHub Release page.

### Changed (BREAKING — default theme bytes)

- **Default theme now ships with color (006).** Underlying text is unchanged so existing log scrapes / PR-friendly diffs still work, but `cargo run -- < payload.json` now emits ANSI escapes by default. The `insta` snapshot rolled forward to the new colored bytes; the prior plain-text bytes are recoverable via `NO_COLOR=1` or the new `ColorMode::Never` library seam. Theme palette and progress-bar tier rules locked in [`docs/design-docs/default-theme.md`](docs/design-docs/default-theme.md). Codex 4-round verify-plan AGREE + post-impl review.

### Added

- **Threshold-aware progress bars.** `context_bar`, `block_timer`, `weekly_timer` paint the filled portion green / yellow / red at 0–49 / 50–79 / 80+ percent. User `config color` overrides replace tier colors end-to-end (see design-doc).
- **`WidgetSpec::render: fn(&Context) -> Option<Vec<Segment>>`** — widgets emit multiple styled segments. Bar widgets compose `[bracket-default · filled-tier · empty-default · bracket-default · …]`; `git_changes` splits into `+green / -red`. Renderer applies user-override (whole-widget replace) on top.
- **`ColorMode::{Auto, Always, Never}`** library seam (`render::color`, re-exported at lib root). Tests pin `Always` / `Never` so the suite stays deterministic regardless of the developer's `NO_COLOR` / `FORCE_COLOR` shell env. Production callers use `Auto`, which now defaults to on (Claude Code reliably renders our ANSI).
- **`bar_filled_count(percent, width)`** helper in `render::format` so multi-segment widgets can split a bar into independently styled runs.
- **`Segment::styled(text, style)`** constructor.
- **`install` / `uninstall` subcommands (005).** One-shot wiring into Claude Code: copies the binary to `~/bin` (Windows) or `~/.local/bin` (POSIX), writes a `.mjs` wrapper on Windows (works around the Claude Code Windows-native `statusLine` regression [#31670](https://github.com/anthropics/claude-code/issues/31670)), backs up `~/.claude/settings.json`, and rewrites only the `statusLine` block. Unknown top-level settings keys survive verbatim through a `#[serde(flatten)] extra` round-trip. `uninstall` reverts the most-recent install via atomic temp+rename, with `--purge-binary` to also remove the binary and wrapper. Codex 4-round verify-plan AGREE before implementation.
- `crate::ioutil::atomic_write_bytes` — shared atomic-write helper. Temp filename includes pid + monotonic counter so concurrent installers don't collide. `Config::save` now routes through it.
- New typed errors: `Error::FileIo { operation, path, source }` and `Error::NoBackupFound { settings }`.
- **Distribution scaffolding (M5).** GitHub Actions CI (test + clippy + fmt on Linux/macOS/Windows) and Release workflow (4 target triples: `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`). Tagged pushes (`v*.*.*`) produce per-target archives with SHA-256 companions, attached to a generated GitHub Release.
- `INSTALL.md` restructured into three sections (Recommended / Manual / Uninstall).
- `CHANGELOG.md` (this file).

### Changed

- Release profile already set in M0 (`lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`). Local Windows MSVC build measures **1,910,272 bytes**.

### Milestone history (M0–M4)

The bullets below replay the milestone-level deliverables now rolled into v0.1.0. Detailed exec-plans live under `docs/exec-plans/completed/`.

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
