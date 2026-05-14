# cc-statusline-rust Status

> Last updated: 2026-05-14 (M2 Config persistence + JSONL cache **✅ completed** — `config add / remove / apply / validate` shipped, JSONL probe now mtime+size-keyed cached, **105/105 tests pass**, default-theme snapshot unchanged.)

## Overview

| Phase | Description | Status |
|---|---|---|
| 000-bootstrap | Cargo project init + CLI skeleton + payload schema + default-theme snapshot baseline | ✅ completed (2026-05-14) — see [`exec-plans/completed/000-bootstrap.md`](exec-plans/completed/000-bootstrap.md) |
| 001-probes | JSONL session-tokens + git probe + git_branch/git_status/git_changes widgets + config disk loading | ✅ completed (2026-05-14) — see [`exec-plans/completed/001-probes.md`](exec-plans/completed/001-probes.md) |
| 002-config-persistence-and-jsonl-cache | Config CRUD (`add/remove/apply/validate`) with atomic disk save + JSONL probe `(mtime,size)` cache | ✅ completed (2026-05-14) — see [`exec-plans/completed/002-config-persistence-and-jsonl-cache.md`](exec-plans/completed/002-config-persistence-and-jsonl-cache.md) |

## Milestones

| Milestone | Scope | Status |
|---|---|---|
| M0 — Bootstrap | Cargo workspace, clap subcommands stubbed, payload struct, fixture, **default theme golden snapshot passing** with Model + Cwd + Context Bar + Session Tokens + Session Cost + Block Timer + Weekly Timer widgets | ✅ |
| M1 — Probes | Real `transcript_path` JSONL probe for `session_tokens`; git probes (branch / porcelain / shortstat) with disk cache + timeout; 3 git widgets registered; config disk loading with `needs_git` gate | ✅ |
| M2 — Config surface | Schema (`schemars`), `config add / remove / apply / validate` with atomic on-disk persistence, JSON contract tests, JSONL cache | ✅ |
| M3 — Preview + diff | `preview --diff` between current/pending, payload fixture library | 🔲 |
| M4 — Color + env | Opt-in color via config, `NO_COLOR`/`FORCE_COLOR`, terminal detection, color snapshot tests | 🔲 |
| M5 — Distribution | Release builds for the 4 target triples, GitHub Releases automation, install docs | 🔲 |

## Implementation history

- **2026-05-14 — M2 Config persistence + JSONL cache completed.** `config add / remove / apply / validate` subcommands added (atomic write-temp + rename, widget-kind validation against `widgets::REGISTRY`, JSON output on stdout for every action). JSONL probe gained an mtime+size-keyed disk cache at `cache_dir/jsonl/<xxh3-transcript-path>.json`; cache miss falls through to the existing parser, hit returns `SessionTokens` without opening the JSONL. `Config::validate` and `Config::save` are public, `load_from` / `load_from_or_default` added. 9 new integration tests in `tests/config_crud.rs` (parallel-safe via per-test `CCSTATUSLINE_RS_CONFIG` env override). 105 / 105 tests; default-theme snapshot byte-identical. See [`exec-plans/completed/002-config-persistence-and-jsonl-cache.md`](exec-plans/completed/002-config-persistence-and-jsonl-cache.md).
- **2026-05-14 — M1 Probes completed.** Added `context/jsonl.rs` (cumulative session-tokens probe with streaming dedup) and `context/git.rs` (branch / porcelain / shortstat probes with 800 ms wall-clock timeout via `wait-timeout`, 2 s xxh3-keyed disk cache). Registered three new widgets (`git_branch`, `git_status`, `git_changes`); default layout intentionally excludes them so the renderer pays nothing extra unless an agent opts in. Generalized `render::render` to take `&[Vec<String>]`; added `config::load_or_default()` with `$CCSTATUSLINE_RS_CONFIG` test override. 94 / 94 tests, clippy + fmt clean, default-theme snapshot byte-identical. See [`exec-plans/completed/001-probes.md`](exec-plans/completed/001-probes.md).
- **2026-05-14 — M0 Bootstrap completed.** Cargo + Rust 1.94 pinned, 7 default-theme widgets, KST-default timezone with `tz` override, namespaced `ccstatusline_rs.session_tokens` extension on payload, `anstyle` Segment model decoupled from rendering, `insta` golden snapshot at `tests/snapshots/render_default_theme__default_theme_snapshot.snap`. 58 tests (lib unit + 5 integration + 2 snapshot/string golden). Codex pre-implementation review on the Phase 1–3 design landed nine concrete changes (fn-pointer over trait objects, namespaced extension key, manual month/day formatting, percent clamp semantics). See [`exec-plans/completed/000-bootstrap.md`](exec-plans/completed/000-bootstrap.md).
