# 007 — tokenwatch-aware install / uninstall

**Status:** ✅ completed (2026-05-14) · Codex-AGREE (verify r3, post-impl review r1) · **Owner:** jaewon_lee · **Opened:** 2026-05-14 · **Completed:** 2026-05-14

## Completion

- All Phase A–E checklist items shipped. Implementation in `src/error.rs` (4 new typed variants — `WrapConflict`, `NoInstallTraces`, `StaleWrapPointer`, `InvalidPrev`) and `src/cli/install.rs` (boundary-scan basename detection, `InstallMode` / `UninstallMode` dispatch, `install_wrap`, positive-evidence `resolve_uninstall_mode`).
- One design deviation from the plan: **`tw_prev_path` derives from `settings_path.parent()` instead of `home_dir()`**. Forced by the existing `install_then_uninstall_roundtrip_preserves_unknown_keys` integration test, which doesn't pin `HOME` and was reading the developer's real `~/.claude/.tw-statusline-prev.json`. Sibling-to-settings was the plan's stated semantic anyway (line 45: "Sibling to settings.json"); the default `--settings` resolver still returns `~/.claude/settings.json` so the default prev path is unchanged at `~/.claude/.tw-statusline-prev.json`.
- 8 new unit tests in `src/cli/install.rs::tests` and 12 new integration tests in `tests/install_uninstall_wrap.rs`. Total **192 / 192** tests green; `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean.
- JSON contract bump on `UninstallReport.restored_from` (`PathBuf` → `Option<PathBuf>`) documented in `CHANGELOG.md` under v0.1.3.
- Codex post-impl review (read-only) confirmed: error typing correct (`#[source]` on `InvalidPrev`), mode-resolution ordering load-bearing and correct, `tw_prev_path` change preserves real-world default layout, idempotent flattened `extra` preservation is the safer choice, residual TOCTOU between re-read and `remove_file` is acknowledged as unique to this design without OS-level locking — worst case is deleting neo-mem's fresh prev pointer, identical risk to a manual `rm`. AGREE.

## Versioning

- Cargo.toml `0.1.2 → 0.1.3`.
- `CHANGELOG.md` v0.1.3 entry with explicit "Changed (JSON contract)" callout for the `restored_from` field type bump.
- `INSTALL.md` "Coexistence with neo-mem tokenwatch" section added; troubleshooting note about the statusLine-overwrite race rewritten to reference automatic wrap-mode detection.
- `README.md` integration diagram now leads with "install이 자동 감지해서 wrap 모드로 라우팅합니다" so the user never needs to know about wrap mode unless they're debugging.
- `docs/STATUS.md` + `docs/INDEX.md` updated with the 007 row and implementation-history entry.

## Goal

`install` and `uninstall` must coexist with neo-mem's `tokenwatch-statusline.mjs` (an in-house plugin). tokenwatch is a man-in-the-middle that:

1. Captures `rate_limits` from the Claude Code statusLine stdin payload and writes them to `~/.claude/.tw-statusline-cache.json`. neo-mem worker reads that cache before every push.
2. If `~/.claude/.tw-statusline-prev.json` exists with `{ "command": "..." }`, spawns that inner command with the same stdin so the user-visible statusLine still renders.

Today our `install` overwrites `settings.json` `statusLine` directly. On any developer who already has tokenwatch active, that **silently breaks neo-mem's rate_limits collection** — a regression we cannot ship inside the company.

Acceptance: `install` detects tokenwatch and routes through the wrap-mode `.tw-statusline-prev.json` instead of clobbering `statusLine`. `uninstall` reverses the right artifact. JSON output exposes which mode was used.

## Non-goals

- **No tokenwatch lifecycle management.** We don't install or update tokenwatch; we only coexist with it.
- **No multi-wrap chaining.** If `.tw-statusline-prev.json` already references a non-ccstatusline-rs command, we reject (loud failure) rather than overwriting silently. User decides what wins.
- **No magic re-detection on every renderer call.** install/uninstall do detection once; the renderer itself stays unchanged.

## Mode definitions

- **direct mode** — settings.json `statusLine.command` points at our `.mjs` (Windows) or our binary path (POSIX). `.tw-statusline-prev.json` is absent or empty. This is the v0.1.0–v0.1.2 behavior.
- **wrap mode** — settings.json `statusLine.command` keeps tokenwatch's path. `~/.claude/.tw-statusline-prev.json` holds our command in `{ "command": "...", "type": "command" }`. tokenwatch handles cache + wrap.

## Detection

`contains_basename(cmd: &str, basename: &str) -> bool` — **boundary scan, not whitespace tokenize** (Codex round 2 #1). Whitespace tokenization breaks on Windows paths with spaces inside quoted strings (e.g. `node "C:\Users\Jane Doe\.neo-mem\...\tokenwatch-statusline.mjs"` — the trailing `"` would leak into the matched basename).

Algorithm:

1. Find every occurrence of `basename` as a substring in `cmd`.
2. For each match, check that:
   - The character immediately before is a path separator (`\`, `/`), a quote (`"` `'`), whitespace, OR the start of the string. The basename must not be preceded by a non-separator letter (which would make it a longer name like `my-tokenwatch-statusline.mjs`).
   - The character immediately after is a quote, whitespace, OR end of the string. (Basename rarely has trailing path components in a single token.)
3. Return `true` if any match satisfies both boundaries.

`is_tokenwatch_command(cmd)` = `contains_basename(cmd, "tokenwatch-statusline.mjs")`.

`is_ours_wrap_command(cmd)` = `contains_basename(cmd, ours_basename())` where `ours_basename()` = `"ccstatusline-rs.mjs"` (Windows) / `"ccstatusline-rs"` (POSIX). This lets `install` relocate via `--bin-dir` without registering as a `WrapConflict` against an earlier install's prev file (Codex round 1 #7).

Accepts every neo-mem path variant we've seen — `~/.neo-mem/runtime/.../1.2.10-a1e85cb5/scripts/tokenwatch-statusline.mjs`, legacy Temp dir, future major versions — and the Windows quoted-path-with-space form. Rejects `my-tokenwatch-statusline.mjs` because the boundary-before check requires a separator.

`tw_prev_path()` resolves to `~/.claude/.tw-statusline-prev.json`. (Sibling to settings.json.)

`tw_cache_path()` resolves to `~/.claude/.tw-statusline-cache.json` — install / uninstall touch only `.tw-statusline-prev.json`, but documentation references the cache path for the operator-facing "what's where" table.

## Install algorithm

```
1. Parse existing settings.json.
2. Inspect statusLine.command:
   a. None / empty   → mode = Direct. Existing behavior.
   b. tokenwatch?    → mode = Wrap.
   c. Other command  → mode = Direct (we win; this is the legacy uninstall path
                       since we still record `previous_command` for audit).

3. If mode == Wrap:
   a. **Binary + wrapper still placed** (Codex round 1 #3) — wrap mode means
      tokenwatch delegates to us, so the file `.tw-statusline-prev.json`
      points at must actually exist. Run the same copy-binary + write-wrapper
      flow as Direct.
   b. Read .tw-statusline-prev.json if present.
      - Parse failure → `Error::InvalidPrev { path, source }`. Loud.
      - Command IS ours (basename match, path may differ — relocation OK)
        → update to current absolute path, idempotent overwrite.
      - Command is NOT ours → `Error::WrapConflict { path, existing_command }`.
        Operator decides.
   c. Write .tw-statusline-prev.json atomically via `ioutil::atomic_write_bytes` with the command produced by **`compose_command(platform, &dest_exe, wrapper.as_deref())`** (Codex round 2 #2). On Windows: `node "<absolute-mjs-path>"`. On POSIX: `'<absolute-exe-path>'` (single-quote shell-escaped) — same routing logic the direct install path already uses, no fork.
   d. settings.json is NOT modified. No backup created either — we touched
      nothing there.

4. If mode == Direct:
   a. Existing 005 behavior: backup settings.json, replace statusLine block,
      atomic write.

5. Emit JSON with new field `mode: "wrap" | "direct"`. Direct mode JSON is unchanged from 005.

   Wrap mode JSON also adds:
     - `wrap_prev_path: <path>`
     - `previous_wrap_command: Option<String>` — **projection of just the
       `command` string**, not the raw Value (Codex round 1 #6). Future
       neo-mem versions may add fields to `.tw-statusline-prev.json` and we
       don't want to leak them into our audit output.
     - `wrap_explanation: "settings.json untouched — tokenwatch wrap-mode in effect"`
       so operators see at a glance why no settings backup was created.
     - `backup` field stays `null` in wrap mode (we didn't touch settings).
```

## Uninstall algorithm

Mode resolution requires **positive evidence we installed something** (Codex round 1 #2). Heuristics like "settings.json statusLine is tokenwatch" alone are not enough — that's just "tokenwatch is present", not "we are installed".

```
1. If --backup <path> is set explicitly, force Direct mode regardless.
2. Parse settings.json statusLine.command.
3. **Stale-pointer check first** (Codex round 2 #3): if
   .tw-statusline-prev.json exists AND is_ours_wrap_command(prev.command)
   AND settings.json statusLine.command is NOT tokenwatch (basename) →
   Error::StaleWrapPointer. Surfaces before direct-backup heuristic
   masks it.
4. Try wrap detection (positive evidence):
   a. settings.json statusLine.command is tokenwatch  (basename match)
   b. AND .tw-statusline-prev.json exists
   c. AND its parsed command is ours (is_ours_wrap_command)
   → mode = Wrap.
5. Else try direct detection:
   a. A settings backup file (`*.ccstatusline-rs-bak-*`) exists next to
      settings.json
   OR
   b. settings.json statusLine.command is_ours (basename match)
   → mode = Direct.
6. Else ABORT with Error::NoInstallTraces { settings, prev_path }.

If mode == Wrap:
   - .tw-statusline-prev.json reread for safety, command rechecked — only
     remove the file if it still points at us. Mid-operation neo-mem
     refresh can't fool us into deleting their pointer.
   - settings.json untouched.

If mode == Direct:
   - Existing 005 behavior: restore latest backup atomically (or the
     --backup-specified one).

Stale-artifact case is already handled by step 3 above (Codex round 2 #3) — the explicit early check ensures `StaleWrapPointer` surfaces before the direct-backup heuristic falls through to mode = Direct and silently restores the wrong file.

With --purge-binary: same as today, exe + .mjs removed by exact filename.

Emit JSON with `mode: "wrap" | "direct"`.
   - Direct: `restored_from: <PathBuf>` (was unconditional in 005 — now
     Option<PathBuf> per Codex round 1 #4).
   - Wrap: `restored_from: null`, `removed_wrap_prev: <PathBuf or null>`.
```

## Phase A — pure helpers + new error variants

`src/cli/install.rs`:

- `fn basename_matches(cmd: &str, basename: &str) -> bool` — shared helper. Tokenize on whitespace, strip surrounding quotes from each token, take `Path::file_name`, compare exact.
- `fn is_tokenwatch_command(cmd: &str) -> bool` — `basename_matches(cmd, "tokenwatch-statusline.mjs")`.
- `fn is_ours_wrap_command(cmd: &str) -> bool` — `basename_matches(cmd, "ccstatusline-rs.mjs")` on Windows or `"ccstatusline-rs"` on POSIX.
- `fn tw_prev_path(home: &Path) -> PathBuf` — `~/.claude/.tw-statusline-prev.json`.
- `enum InstallMode { Direct, Wrap }`.
- `enum UninstallMode { Direct, Wrap }`.

`src/error.rs` — three new typed variants (Codex round 1 #7, no `serde_json::Error` stringification):

- `Error::WrapConflict { path: PathBuf, existing_command: String }` — wrap-mode install but `.tw-statusline-prev.json` already references a non-ours command. Operator-facing message includes the existing command verbatim.
- `Error::NoInstallTraces { settings: PathBuf, prev_path: PathBuf }` — uninstall called but neither a settings backup nor a recognized wrap-prev pointing at us is found.
- `Error::StaleWrapPointer { prev_path: PathBuf, settings_command: String }` — `.tw-statusline-prev.json` is ours but `settings.json statusLine.command` is no longer tokenwatch. Surface so operators can reconcile manually.
- `Error::InvalidPrev { path: PathBuf, source: serde_json::Error }` (via `#[source]`) — `.tw-statusline-prev.json` exists but parse failed. Not stringified.

## Phase B — install integration

`install::install` gains a mode-resolution step right after reading settings. Code path:

```rust
let parsed: ClaudeSettings = ...;
let current_cmd = parsed.status_line.as_ref().map(|s| s.command.as_str()).unwrap_or("");
let mode = if is_tokenwatch_command(current_cmd) {
    InstallMode::Wrap
} else {
    InstallMode::Direct
};
match mode {
    InstallMode::Direct => existing_path(...),
    InstallMode::Wrap   => install_wrap(...),
}
```

`install_wrap`:

- Validate prev-file contents (parse + check command).
- Write atomic via `ioutil::atomic_write_bytes`.
- Returns an `InstallReport` whose `mode`, `wrap_prev_path`, `previous_wrap` are populated.

## Phase C — uninstall integration

`install::uninstall` adds the same mode resolution, then dispatches. `--bin-dir` / `--purge-binary` semantics unchanged. `--backup` **forces Direct mode** — operators who explicitly pass a backup path are unambiguously asking for the direct-flow restore. Without `--backup`, mode is auto-resolved per the algorithm above.

`UninstallReport.restored_from` becomes `Option<PathBuf>` (was unconditional in 005) to accommodate wrap-mode `null` (Codex round 1 #4). CHANGELOG must call this out as a JSON contract change for any agent that pattern-matches the field.

## Phase D — tests

`src/cli/install.rs::tests`:

- `detects_tokenwatch_in_canonical_paths` — basename match on three real-world paths (1.1.97 Temp, 1.2.10 .neo-mem, future major).
- `detects_tokenwatch_in_quoted_windows_path_with_spaces` — `node "C:\Users\Jane Doe\.neo-mem\runtime\neo-mem\win32-x64\1.2.10\scripts\tokenwatch-statusline.mjs"` → `true`. Guards the boundary-scan implementation against whitespace-tokenize regression (Codex round 2 #1).
- `does_not_match_my_tokenwatch_statusline_variant` — `node my-tokenwatch-statusline.mjs` → `false`. The leading-boundary check rejects basename-prefixed lookalikes.
- `does_not_match_unrelated_commands` — `node my-statusline.mjs`, `powershell -c ...`, `/usr/bin/echo hi` all `false`.
- `is_ours_wrap_command_matches_relocated_bin_dir` — `node "/custom/bin/ccstatusline-rs.mjs"` AND `node "/other/bin/ccstatusline-rs.mjs"` both `true` (relocation OK).
- `wrap_install_writes_posix_compose_command_form` — On a POSIX target, the persisted `.tw-statusline-prev.json` `command` field equals `'<absolute-exe-path>'` (single-quote shell-escaped, no wrapper). Pinned to detect a future regression that forks the wrap command builder from `compose_command` (Codex round 2 #2).

`tests/install_uninstall_wrap.rs` (new):

| Case | Asserts |
|---|---|
| `wrap_install_when_tokenwatch_present` | settings.json unchanged; `.tw-statusline-prev.json` now references our `.mjs`; JSON output reports `mode: "wrap"`, `backup: null`, `previous_wrap_command: null`. |
| `wrap_install_idempotent` | Re-run is a no-op write (no error). |
| `wrap_install_relocation_overwrites_prev` | Install once with `--bin-dir A`, then again with `--bin-dir B`. Prev points at B; not a `WrapConflict` because basename match identifies us. |
| `wrap_install_rejects_pre_existing_other_wrap` | Pre-seed `.tw-statusline-prev.json` with `{ "command": "node other-tool.mjs" }` → install fails with `WrapConflict`, both settings and prev file untouched. JSON error message contains `other-tool.mjs`. |
| `wrap_install_rejects_invalid_prev_json` | Pre-seed prev file with garbled bytes → `InvalidPrev`, settings and prev file untouched. |
| `wrap_uninstall_removes_prev_only` | install → uninstall: `.tw-statusline-prev.json` gone; settings.json byte-identical to pre-install snapshot. |
| `wrap_uninstall_requires_positive_evidence` | settings.json statusLine is tokenwatch but prev file is missing → `NoInstallTraces`. We don't claim to own anything. |
| `wrap_uninstall_stale_pointer_fails_loudly` | prev file IS ours but settings.json statusLine no longer tokenwatch → `StaleWrapPointer`. |
| `direct_install_unchanged_behavior` | Settings without tokenwatch → existing 005 flow, no `.tw-statusline-prev.json` touched. |
| `uninstall_backup_flag_forces_direct` | Pre-install via wrap mode, then run uninstall with explicit `--backup <bogus path>` → fails as Direct path (file not found), not as wrap path. Confirms backup flag's mode-forcing precedence. |
| `uninstall_fails_when_no_traces` | Settings without tokenwatch + no backup + no prev → `NoInstallTraces`. |
| `purge_binary_works_in_both_modes` | Same exe/wrapper deletion in wrap mode as in direct mode. |

`tests/install_uninstall.rs` existing cases unchanged.

## Phase E — docs + gates

- [ ] `INSTALL.md` adds a "Coexistence with neo-mem tokenwatch" section explaining wrap mode.
- [ ] `README.md` 통합 다이어그램 섹션을 install가 자동으로 처리한다는 한 줄로 보강.
- [ ] `CHANGELOG.md` Unreleased: behavior change (tokenwatch detection in install/uninstall).
- [ ] `docs/STATUS.md` + `docs/INDEX.md` updated.
- [ ] `cargo nextest run --test-threads=1` green.
- [ ] `cargo clippy --all-targets -- -D warnings` clean.
- [ ] `cargo fmt --check` clean.
- [ ] `git mv` plan to completed/.
- [ ] Cargo.toml bump 0.1.2 → 0.1.3, tag v0.1.3, CI builds release.

## Risks & open questions

1. **Basename detection brittleness.** Detection settled as boundary-scanned basename match against the literal `tokenwatch-statusline.mjs`. If neo-mem renames the script in a future release, our detection breaks silently and we revert to direct mode — undesired but recoverable (operator re-runs install after the rename ships through). The trade-off was made deliberately: stricter than substring (rejects `my-tokenwatch-statusline.mjs`) but doesn't pull in a full shell-line parser. If neo-mem ever ships a manifest that publishes the canonical basename, switch to that source-of-truth.

2. **`previous_wrap` JSON content.** The pre-existing `.tw-statusline-prev.json` may legitimately point at another tool the operator set up before us. Plan rejects with `WrapConflict` — by design — but the error message should include the existing command verbatim so the operator sees it without grepping the file.

3. **`uninstall_fails_when_no_backup` regression.** The existing test expects failure when no backup exists. With the new `NoInstallTraces` variant, that test still fails (it just gets a more specific error). Verify the integration test's assertion is on non-zero exit, not on a specific error message.

4. **Operator confusion — wrap mode looks like it did nothing to settings.** Wrap mode JSON's `backup: null` may surprise operators expecting a backup. INSTALL.md needs to make this explicit. Add a line in the JSON output too: `wrap_explanation: "settings.json untouched — tokenwatch wrap-mode in effect"`.

5. **No `--no-wrap` flag in v1** (Codex round 1 #5). Originally proposed as an opt-out for operators who want to override tokenwatch on a specific machine — but the flag exactly undoes the protection this milestone exists to provide. If a real demand surfaces, add it later as a single explicit flag like `--force-direct-tokenwatch` and require a dedicated test case. For now, operators who need direct mode while tokenwatch is present can manually clear `statusLine.command` first.

## Acceptance

- Installing on a machine with active tokenwatch produces wrap mode, leaves settings.json byte-identical, writes `.tw-statusline-prev.json`.
- Uninstalling reverses the right artifact based on detection — never both.
- Pre-existing non-ours `.tw-statusline-prev.json` causes loud rejection, not silent overwrite.
- JSON output's `mode` field lets agents route follow-up actions correctly.
- All existing tests still pass; new wrap-mode test suite green.
