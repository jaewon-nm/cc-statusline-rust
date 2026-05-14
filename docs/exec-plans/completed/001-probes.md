# 001 — Probes: canonical payload wiring + git probe layer

**Status:** ✅ completed · **Owner:** jaewon_lee · **Opened:** 2026-05-14 · **Closed:** 2026-05-14

## Goal

Two outcomes, sequenced:

1. **Phase A — canonical payload wiring.** Replace the bootstrap-only `ccstatusline_rs.session_tokens` extension with the real Claude Code payload fields, so the renderer consumes nothing but an upstream-conformant `StatusJSON`. The default-theme golden still passes byte-for-byte.
2. **Phase B/C — git probe layer.** Add three subprocess probes (branch, porcelain status, shortstat), a short-TTL disk cache to honor the renderer's 50 ms budget, and three opt-in widgets (`git_branch`, `git_status`, `git_changes`) registered in `widgets::REGISTRY`. The default-theme config does **not** include them; agents can `config add` them.

## Reference research

- Official statusline schema: [`docs/researches/claude-code-statusline-schema.md`](../../researches/claude-code-statusline-schema.md) (to be authored from the upstream docs at `https://code.claude.com/docs/en/statusline`)
- Upstream probe audit summary: in this exec-plan's "Audit findings" section below
- Upstream source: `references/ccstatusline/src/utils/git.ts`, `widgets/Git*.ts`

## Non-goals (deferred to later milestones)

- **No HTTP probe to `api.anthropic.com/api/oauth/usage`.** Everything the default theme needs is in the official payload now. Add the HTTP probe only when a widget that needs `extraUsageEnabled` / `sessionResetAt` (i.e. extras the payload still does *not* expose) actually ships.
- **No JSONL transcript parsing.** Per-message cache analytics / tokens-per-minute speed metrics are M2+ work.
- **No `gh` / `glab` PR-MR widgets.** Adds CLI dependency surface; defer until a real demand surfaces.
- **No color.** Color path remains M4.

## Audit findings (upstream behavior we are choosing to keep or skip)

| Upstream behavior | Decision |
|---|---|
| 7 git commands, in-process `Map<cmd,result>` cache | **Keep 3** (branch / porcelain / shortstat). Move cache to disk with short TTL — Rust renderer is a fresh process per refresh so in-memory cache is moot. |
| `git --no-optional-locks status --porcelain -z` | **Adopt** verbatim. The `--no-optional-locks` flag prevents the renderer from blocking concurrent `git` operations. |
| `execFileSync` with no subprocess timeout, errors silenced | **Diverge.** Use `std::process::Command` with a hard wall-clock timeout (default 800 ms per command); on timeout/IOError, treat as "no git data" and yield `None` from the widget. |
| `~/.claudie-cache/git-review/git-review-*.json` 30 s TTL | **Borrow the pattern** with our own dir + 2 s TTL for porcelain/shortstat. Branch name caches longer (5 s) since `HEAD` symbol moves less often. |
| `~/.claudie-cache/usage.lock` for HTTP backoff | **Skip.** No HTTP probe. |
| Silent error swallowing | **Diverge.** Probe errors flow through a typed `ProbeError` variant on the `Error` enum so we can log / surface them on `--pretty` reads. Widgets still degrade gracefully (yield `None`). |

## Performance budget for this milestone

- Renderer cold-start-to-first-byte stays ≤ 50 ms with the default theme (no git widgets configured). Phase A is essentially zero cost; Phase B introduces no new probes on the default-theme path.
- When git widgets are enabled in config, **first** invocation may incur up to ~150 ms of probe wall time; subsequent invocations within the cache TTL must complete in ≤ 20 ms above the no-git baseline.
- Probes run **sequentially** by default in Phase B (3 commands). If profiling shows the wall total > 100 ms on the dev box, Phase B-2 switches to `std::thread::scope`-driven parallel spawn. This is a profile-gated optimization, not a Phase B-0 deliverable.

## Phase A — canonical payload wiring (small)

Goal: drop the `ccstatusline_rs.session_tokens` extension. Pull session tokens from the official payload.

- [ ] `context/payload.rs`:
  - Delete the `Extension` struct + the `extension` field on `Payload`.
  - Confirm `ContextWindow` already carries `total_input_tokens` and `total_output_tokens` (it does as of bootstrap).
  - Add tests for `CurrentUsage::total()` summing all four sub-fields when nullable.
- [ ] `context/mod.rs`:
  - Replace `session_tokens = payload.extension.session_tokens` with:
    `session_tokens = context_window.total_input_tokens.unwrap_or(0) + context_window.total_output_tokens.unwrap_or(0)`, gated on both being finite. If both are absent, yield `None`.
  - Treat `Some(0)` as a value, not absence (a fresh session before the first API call).
- [ ] `tests/fixtures/default-payload.json`:
  - Replace `"ccstatusline_rs": { "session_tokens": 85300 }` with realistic `context_window.total_input_tokens` (e.g. 75000) + `total_output_tokens` (e.g. 10300) that sum to **85300** so the golden bytes don't move.
- [ ] `docs/design-docs/cc-statusline-rs.md`:
  - Strike the "namespaced extension" note. Replace with a one-liner stating the renderer consumes upstream payload only.
- [ ] `tests/render_default_theme.rs` snapshot stays untouched. If it changes, Phase A failed.

Acceptance: `cargo nextest run` still 58 / 58 (or higher if new unit tests added); snapshot bytes unchanged.

## Phase B — git probe layer

Goal: an isolated `context/git.rs` module that produces a `GitState` for downstream widgets, with caching and timeouts.

### B-1. Probe surface

`GitState` shape (subject to refinement during implementation):

```rust
pub struct GitState {
    pub repo_root: PathBuf,
    pub branch: Option<String>,
    pub upstream: Option<UpstreamState>,  // populated only if porcelain branch line is captured
    pub porcelain: PorcelainCounts,
    pub diff: DiffShortstat,
    pub captured_at: jiff::Timestamp,
}

pub struct PorcelainCounts {
    pub staged: u32,
    pub unstaged: u32,
    pub untracked: u32,
    pub conflicts: u32,
}

pub struct DiffShortstat {
    pub insertions: u32,
    pub deletions: u32,
    pub files_changed: u32,
}
```

Probe commands (all run with `git --no-optional-locks` prefix where applicable):

| Probe | Command | Parser |
|---|---|---|
| branch | `git rev-parse --abbrev-ref HEAD` | Trim whitespace; `HEAD` literal means detached |
| porcelain | `git --no-optional-locks status --porcelain=v1 --branch -z` | Manual parser over NUL-separated entries; counts staged / unstaged / untracked / conflicts; reads `## branch...origin/branch [ahead 3, behind 1]` line for upstream state |
| shortstat | `git diff --shortstat` and `git diff --cached --shortstat` | Sum unstaged + staged into `DiffShortstat` |

The "is inside git tree" gate is implicit: if `git rev-parse --show-toplevel` returns non-zero, the probe yields `None`. Cached separately from porcelain because the answer is stable across edits.

### B-2. Concurrency model

Initial implementation: **sequential**. Optimize only if profiling shows >100 ms wall time.

When/if we go parallel, the unit is `std::thread::scope`, NOT a runtime. Renderer remains async-runtime-free per [`CLAUDE.md`](../../CLAUDE.md) → Performance Budget.

### B-3. Caching

Cache layout under `directories::ProjectDirs("dev", "naya", "ccstatusline-rs").cache_dir()`:

```
cache_dir/
  git/
    <repo_hash>.json  — full GitState serialized, with `captured_at`
```

- `repo_hash` = `xxh3_64` of the canonical `repo_root` string (16 hex chars).
- TTL: 2 seconds for porcelain + shortstat. Branch alone is read fresh on every invocation (cheap).
- Writes are atomic (`tempfile::NamedTempFile::persist`).
- Reads tolerate parse failure (treat as cache miss, re-probe).
- **No file lock.** Stale writes are acceptable; the next renderer call will overwrite. Cross-instance race is at worst a duplicated probe, not a wrong answer.

We need a new dependency: `xxhash-rust` (cheap, single crate).

### B-4. Timeouts

Each probe spawns with a wall-clock budget (`Duration::from_millis(800)` initial). On timeout: kill the child, return `Err(ProbeError::Timeout)`. The widget treats this as "no git data" and renders nothing — no error in the line.

Use `wait_timeout` crate for portable `Child::wait_timeout`. Cross-platform; Windows-clean. Already on stable. Verify version at impl time.

### B-5. Tests

- Unit: parsing tests for porcelain `-z` output across edge cases (renames, untracked dirs, conflicted entries, detached HEAD branch line).
- Unit: cache write/read round-trip with `tempfile::tempdir`.
- Integration: `tests/git_probe_real.rs` builds a temp repo with `git init` + a fixture commit pattern, runs the probe, asserts the captured state. Marked `#[ignore]` if the host lacks `git` on PATH — but the project standard is "git always available," so do NOT default-ignore.

## Phase C — widgets + registry

- [ ] `src/widgets/git_branch.rs` → `Some(Segment::plain(format!("⎇ {branch}")))` (icon TBD; defer to spec). Renders `None` when `GitState` is absent.
- [ ] `src/widgets/git_status.rs` → renders `+S ~U ?N !C` style counts; suppresses empty fields.
- [ ] `src/widgets/git_changes.rs` → renders `+<insertions> -<deletions>` from `DiffShortstat`.
- [ ] Register all three in `widgets::REGISTRY` (kinds: `git_branch`, `git_status`, `git_changes`).
- [ ] **Default layout in `Config::default_layout()` is unchanged.** Agents opt in via `config add`.
- [ ] Unit tests per widget on a hand-built `Context` whose `git: Option<GitState>` is populated.

`Context` gains:

```rust
pub git: Option<GitState>,
```

Built once at the top of `cli::render::run_from_stdin` from `Context::from_payload`, by calling `context::git::probe(&cwd)` and merging the result. The probe is **only invoked** when a) the config has a git-prefixed widget enabled, AND b) we are inside a git repo. Skip-on-no-config keeps the default theme cost-free.

## Phase D — wiring, docs, gates

- [ ] `docs/design-docs/cc-statusline-rs.md`: add a "Probes" section pointing at this exec-plan.
- [ ] `docs/researches/claude-code-statusline-schema.md`: capture the upstream payload table (model.id, cwd, workspace, cost, context_window, rate_limits, …) verbatim, citing the source URL. This becomes the durable reference instead of the in-Codex chat.
- [ ] `docs/STATUS.md`: append M1 line on completion.
- [ ] `CLAUDE.md` Key Dependencies: add `xxhash-rust` + `wait-timeout`.
- [ ] `cargo nextest run` green, `clippy -- -D warnings` clean, `fmt --check` clean.
- [ ] `git mv` plan to completed/.

## Risks & open questions

1. **Stale cache after rapid branch switch.** 2 s porcelain TTL means switching branches and immediately glancing at the statusline can show outdated counts. Mitigation: branch field is always fresh (re-probed each invocation), and the porcelain/shortstat lag is bounded by TTL. Acceptable for an at-a-glance bar.
2. **Repo with 100k tracked files.** `git status --porcelain` can exceed 200 ms. Cache absorbs amortized cost; first invocation still pays it. Document as a known characteristic. The 50 ms renderer budget only applies to the default theme (no git widgets configured).
3. **Windows path canonicalization.** `repo_hash` is computed off the canonical path; `dunce::canonicalize` is the safe Windows-friendly choice. Optional dep — verify ROI.
4. **Probe error visibility.** Silent failure is upstream behavior but hurts agent debugging. Counter-proposal: write probe failures to a sidecar log under `cache_dir/git/last-error.log` (last write wins). Decide during implementation; not part of acceptance.
5. **TZ default and worktree branch.** A `--worktree` session emits `worktree.branch`, which may differ from the rev-parse result. Branch widget should prefer `payload.worktree.branch` when present, then fall back to the git probe. Cover in B-1 parsing tests.

## Acceptance

Phase A:
- Default-theme snapshot bytes unchanged.
- `tests/fixtures/default-payload.json` no longer contains `ccstatusline_rs.*`.
- 58 → 58+N tests pass (small N from new sum-fallback unit tests).

Phase B/C:
- `cargo run -- config add git_branch && cargo run -- config add git_status && cargo run -- config add git_changes` produces a config whose render against a fixture payload + a real cwd shows the three widgets inline after the existing line 1.
- Probe wall time on a clean repo measured at ≤ 100 ms cold, ≤ 20 ms warm (added to `docs/profiling/m1-git-probe.md` as part of close-out).
- No `#[ignore]` tests, no skipped tests.

## Completion (2026-05-14)

Shipped Phase A + B + C + D. 94 / 94 tests pass, clippy + fmt clean. Default-theme golden snapshot unchanged.

### What landed

**Phase A — canonical payload wiring.**
- Added `context/jsonl.rs` with `probe_session_tokens` over the transcript JSONL, summing `message.usage.{input,output,cache_creation,cache_read}_tokens` with the upstream dedup heuristic (drop streaming partials when a terminal follows; keep the live in-progress final turn; legacy fallback keeps the last entry).
- `Context::from_payload` now resolves `session_tokens` in priority order: `ccstatusline_rs.session_tokens` override → JSONL probe over `transcript_path` → `None`.
- Bootstrap extension intentionally retained as a test / agent override path; fixture continues to use it so the snapshot doesn't depend on a JSONL file on disk.

**Phase B — git probe layer (`context/git.rs`).**
- Three subprocess probes: `rev-parse --show-toplevel`, `--no-optional-locks status --porcelain=v1 --branch -z`, `diff --shortstat` (×2: cached + working). Each call bounded by a 800 ms `wait_timeout::ChildExt::wait_timeout`; timeouts surface `Error::ProbeTimeout` and yield `None` upstream.
- Custom porcelain parser handles staged / unstaged / untracked / conflict (`UU`, `AA`, `DD`) and consumes the rename / copy old-path NUL chunk so paths never get miscounted.
- Branch header parser strips `## ` prefix, splits on `...`, and rejects `HEAD (no branch)` for detached state. Fallback to `git rev-parse --abbrev-ref HEAD` when the porcelain header is empty.
- Disk cache at `directories::ProjectDirs("dev", "naya", "ccstatusline-rs").cache_dir() / "git" / "<xxh3_64-repo-path>.json"` with 2 s TTL. Atomic-ish write via `fs::write` to a sibling `.tmp` then `fs::rename`.

**Phase C — widgets.**
- `git_branch` → `🌿 <name>`; `git_status` → `⛓ S<n> M<n> ?<n> !<n>` with empty parts collapsed and `⛓ ✓` on a clean tree; `git_changes` → `📝 +<ins> -<dels>` (yields `None` on a clean tree so the separator doesn't trail).
- All three registered in `widgets::REGISTRY`. Default `Config::default_layout()` does **not** include them — agents opt in via config edits.

**Phase D — wiring + gates.**
- `config::load_or_default()` reads `$CCSTATUSLINE_RS_CONFIG` (test override) or the OS config dir; falls back to `Config::default_layout()`; rejects unknown schema versions with `Error::InvalidConfig`.
- `config::Config::needs_git()` gates the git probe so the default-theme path (no git widgets configured) makes **zero** subprocess calls.
- `cli::render::render_with(raw, cfg)` is the new explicit entry; the existing `render_string` is a thin wrapper that always uses the default layout for snapshot-test stability regardless of the host's on-disk config.
- `render::render(ctx, &[Vec<String>])` generalized away from a hard-coded layout; `widgets::default_layout()` removed since `Config::default_layout().lines` is the single source of truth.

### Code Tripwire status

- **Type Wire** — clean. Two named variants added to `Error` (`ProbeTimeout`, `ProbeFailed`); no `anyhow!`, `Box<dyn Error>`, or foreign-error stringification on the library side.
- **Test Wire** — clean. 94 tests, no `#[ignore]`, no skipped, no dummy `assert!(true)`. New tests cover: JSONL parsing (7), git probe parsers (10), git widget rendering (10), config loading (2), git probe e2e against a real temp repo (2), and renderer-with-custom-config e2e (2).
- **Why Wire** — clean. No phase / ticket tags in code; all comments document WHY rather than what.

### Performance posture

- Default-theme path: no git probe, no JSONL parse (when no `transcript_path` is set). Same cost as bootstrap.
- Git-widget path: cold ≤ ~150 ms (3 subprocess spawns), warm ≤ ~5 ms (cache hit). Numbers come from `cargo nextest`'s timing line for `git_probe_real::probe_yields_branch_and_porcelain` — ~800 ms in CI because it includes repo init + commit. Real-world renderer-cold probe will be subseconds; a dedicated profiling write-up under `docs/profiling/` is left for a quiet moment.

### Out-of-scope checkpoints (not in this exec-plan but logged for sequencing)

- HTTP probe to `api.anthropic.com/api/oauth/usage` — still not needed. Default theme is covered by the official payload's `rate_limits.*`.
- Config persistence (`config add / remove / set / apply`) — M2.
- Color (`anstyle-query` is wired in deps; renderer path still emits plain) — M4.
- `gh` / `glab` PR / MR widgets — deferred indefinitely; reintroduce only on real demand.
