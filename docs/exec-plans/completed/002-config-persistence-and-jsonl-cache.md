# 002 — Config persistence + JSONL probe cache

**Status:** ✅ completed · **Owner:** jaewon_lee · **Opened:** 2026-05-14 · **Closed:** 2026-05-14

## Goal

Close the two follow-ups from M1:

1. **JSONL probe caching.** Long sessions accumulate megabytes of transcript. Cache the parsed `SessionTokens` keyed on `(transcript_path → mtime_ns, size)` so subsequent renderer invocations within the same edit burst skip the parse.
2. **Config persistence CRUD.** Agents need to actually mutate the on-disk config to enable git widgets, change tz, or rearrange the layout. Ship `config add / remove / apply / validate` (`show` already lands in M0) with atomic writes and widget-kind validation against the registry.

## Phase A — JSONL cache (shipped)

`context/jsonl.rs`:

- New `probe_session_tokens_cached(path)` — production entry; cache root resolved via `directories::ProjectDirs`.
- New `probe_session_tokens_with_cache(path, cache_root)` — test seam taking an explicit cache directory so unit tests pin a `tempdir` and never touch the developer's real cache.
- Cache file layout: one JSON per transcript, named `<xxh3_64(transcript_path)>.json`, body = `{ mtime_ns, size, tokens: { input, output, cached } }`.
- Atomic write: serialize → write to sibling `.json.tmp` → `fs::rename`. A racing renderer either reads the previous file or the new one, never partial.
- Invalidation: `(mtime_ns, size)` mismatch re-probes. Same content + same mtime returns the cached `SessionTokens` without opening the file.
- `Context::from_payload` now calls the cached variant.

Tests added (in `src/context/jsonl.rs#tests`):

- `cache_hit_skips_reparse` — first call populates cache, second after a content + size change re-parses.
- `cache_hit_returns_same_result_without_filesystem_change` — two back-to-back calls yield equal `SessionTokens`.

The existing 7 parser tests still pass against the uncached `probe_session_tokens`.

## Phase B — Config CRUD (shipped)

`config/mod.rs`:

- `Config::validate(known_kinds)` — rejects unknown kinds + non-matching `version`.
- `Config::save(path)` — pretty-prints + atomic temp + rename.
- `load_from(path)` — explicit-path read that errors instead of falling back (for `apply` / `validate --file`).
- `load_from_or_default(path)` — explicit-path read with default fallback (for `show`).
- `load_or_default` now delegates to `load_from_or_default(config_path())`.

`cli/mod.rs` — `ConfigAction` enum extended with:

| Subcommand | Args | Behavior |
|---|---|---|
| `add` | `<kind> [--line N] [--position M]` | Append or insert. `--line = lines.len()` creates a new empty line. Unknown kinds rejected before any disk write. |
| `remove` | `--line N --position M` | Remove the widget at `(N, M)`. Out-of-range = `Error::InvalidConfig`. |
| `apply` | `--file <path>` | Load → validate → atomic save. Single-step replace. |
| `validate` | `[--file <path>]` | Validate the given file (or the on-disk current). Emits `{"ok":true}` on pass, `{"ok":false,"errors":[…]}` on schema/kind failure. |

`cli/config_cmds.rs`:

- All emit JSON on stdout (machine-parseable contract from `docs/design-docs/cc-statusline-rs.md`).
- `add` and `remove` re-emit the **new** config after persisting so an agent doesn't need a second `show` to confirm.
- `apply` and `validate` use the registry as the kind allowlist (`widgets::REGISTRY.iter().map(|w| w.kind)`).
- Persistence routes through `config::config_path()`, which already honors `$CCSTATUSLINE_RS_CONFIG` for tests.

Tests added (`tests/config_crud.rs`, 9 cases, parallel-safe — each pins `CCSTATUSLINE_RS_CONFIG` to a per-test tempfile):

- `show_returns_default_when_file_absent`
- `add_then_show_reflects_persisted_change`
- `add_with_explicit_position_inserts_in_place`
- `add_with_line_equal_to_len_creates_new_line`
- `remove_strips_widget_and_persists`
- `apply_replaces_full_config_atomically`
- `validate_rejects_unknown_widget_kind`
- `add_unknown_kind_fails_loudly`
- `apply_rejects_unsupported_version`

## Acceptance

- `cargo nextest run --test-threads=1` → **105 / 105 passed, 0 skipped**.
- `cargo clippy --all-targets -- -D warnings` clean.
- `cargo fmt --check` clean.
- Default-theme snapshot byte-identical (no renderer behavior change).
- New surfaces all return JSON on stdout; failures still flow through typed `Error::InvalidConfig` (no stringification of foreign errors).

## Out-of-scope (deferred)

- `config set <dotted.path> <value>` — the original plan listed it, but `add` + `remove` + `apply` cover the practical edits agents need. `set` will land if real usage shows demand.
- Error JSON envelope on stdout for command errors (currently errors print to stderr via anyhow display + non-zero exit). The agent-output contract calls for a richer JSON shape; defer to a separate hardening pass.
- Migration logic for older schema versions — current behavior is "reject and report"; we'll add migrations when `CONFIG_VERSION` actually bumps.

## Notes for downstream milestones

- M3 (`preview --diff`) can reuse `config::load_from(path)` to load a candidate file and render alongside the current.
- M4 (color) will extend `Config` with a `colors` map but should preserve the version contract — bump `CONFIG_VERSION` and add migration only at that point.
