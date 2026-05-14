# cc-statusline-rust Status

> Last updated: 2026-05-14 (M0 Bootstrap **✅ completed** — renderer + agent CLI surfaces stubbed, default-theme snapshot locked, 58/58 tests pass.)

## Overview

| Phase | Description | Status |
|---|---|---|
| 000-bootstrap | Cargo project init + CLI skeleton + payload schema + default-theme snapshot baseline | ✅ completed (2026-05-14) — see [`exec-plans/completed/000-bootstrap.md`](exec-plans/completed/000-bootstrap.md) |

## Milestones

| Milestone | Scope | Status |
|---|---|---|
| M0 — Bootstrap | Cargo workspace, clap subcommands stubbed, payload struct, fixture, **default theme golden snapshot passing** with Model + Cwd + Context Bar + Session Tokens + Session Cost + Block Timer + Weekly Timer widgets | ✅ |
| M1 — Probes | Real `transcript_path` JSONL probe for `session_tokens`; git/gh/glab probes for git widget family | 🔲 |
| M2 — Config surface | Schema (`schemars`), `config add/set/remove/apply` with atomic on-disk persistence, JSON contract tests | 🔲 |
| M3 — Preview + diff | `preview --diff` between current/pending, payload fixture library | 🔲 |
| M4 — Color + env | Opt-in color via config, `NO_COLOR`/`FORCE_COLOR`, terminal detection, color snapshot tests | 🔲 |
| M5 — Distribution | Release builds for the 4 target triples, GitHub Releases automation, install docs | 🔲 |

## Implementation history

- **2026-05-14 — M0 Bootstrap completed.** Cargo + Rust 1.94 pinned, 7 default-theme widgets, KST-default timezone with `tz` override, namespaced `ccstatusline_rs.session_tokens` extension on payload, `anstyle` Segment model decoupled from rendering, `insta` golden snapshot at `tests/snapshots/render_default_theme__default_theme_snapshot.snap`. 58 tests (lib unit + 5 integration + 2 snapshot/string golden). Codex pre-implementation review on the Phase 1–3 design landed nine concrete changes (fn-pointer over trait objects, namespaced extension key, manual month/day formatting, percent clamp semantics). See [`exec-plans/completed/000-bootstrap.md`](exec-plans/completed/000-bootstrap.md).
