# Default Theme — Golden Output

**Status:** locked (changes require explicit governance update) · **Owner:** jaewon_lee · **Last touched:** 2026-05-14 (006 — default theme color)

The renderer with the **default config and the canonical payload** produces two layered guarantees:

1. **Underlying text** — the literal character bytes, locked here for visual / log-friendliness.
2. **Colored output** — ANSI escapes that wrap the underlying text per the theme rules below. Locked by the `insta` snapshot at `tests/snapshots/render_default_theme__default_theme_snapshot.snap`.

Both layers must be updated together in any commit that changes the visible appearance.

## Underlying text

```
✦ [Opus 4.7 (1M context)] | 📂 F:\Works\naya\cc-statusline-rust | 🔋 [..........] 80.0K/1.0M(8%) | 📊 85.3K | 💰 $2.55
⏱ 5h [##........](21%) ↻ 12:00 | 📅 7d [##........](20%) ↻ 5/19 06:00
```

## Layout

- **Two lines.** Line 1 = identity + usage. Line 2 = timers.
- **Inter-widget separator:** ` | ` (space · pipe · space).
- **No trailing newline padding** beyond the line break between line 1 and line 2.

## Line 1 widgets (left → right)

| Widget | Icon | Format | Example |
|---|---|---|---|
| Model | `✦ ` | `[<model name (with context suffix)>]` | `✦ [Opus 4.7 (1M context)]` |
| Cwd | `📂 ` | absolute path, untruncated | `📂 F:\Works\naya\cc-statusline-rust` |
| Context bar | `🔋 ` | `[bar] used/total(pct%)` | `🔋 [..........] 80.0K/1.0M(8%)` |
| Session tokens | `📊 ` | abbreviated token count | `📊 85.3K` |
| Session cost | `💰 ` | `$<dollars>.<cents>` USD | `💰 $2.55` |

## Line 2 widgets (left → right)

| Widget | Icon | Format | Example |
|---|---|---|---|
| Block timer | `⏱ ` | `<window>h [bar](pct%) ↻ HH:mm` | `⏱ 5h [##........](21%) ↻ 12:00` |
| Weekly timer | `📅 ` | `<window>d [bar](pct%) ↻ M/d HH:mm` | `📅 7d [##........](20%) ↻ 5/19 06:00` |

## Formatting rules

### Progress bar

- **Width:** 10 cells.
- **Filled char:** `#`. **Empty char:** `.`.
- **Wrapper:** `[…]`.
- **Fill count:** `floor(percent / 10)` cells. `8%` → 0 filled. `21%` → 2 filled. `100%` → 10 filled.

### Numbers

- **Tokens:** abbreviate to `K` (thousands) or `M` (millions) with **one decimal**. Always show the decimal (`80.0K`, not `80K`; `1.0M`, not `1M`). No `B`/`G`.
- **Cost:** `$D.CC` with **two decimals**. Negative or zero is allowed (`$0.00`).
- **Percent:** integer, trailing `%`, no leading space. `(8%)`, `(21%)`.

### Timer windows

- The leading number is the **window length**, not the elapsed time. `5h` and `7d` are constants for the default Claude Code block/weekly schedule.
- `pct%` is `elapsed / window`.

### Reset timestamps

- **Block reset (`↻ HH:mm`):** 24-hour, **KST (Asia/Seoul) by default**, **no seconds**.
- **Weekly reset (`↻ M/d HH:mm`):** no year, **no zero-padding** on month and day (`5/19`, not `05/19`). 24-hour, **KST by default**.
- **Timezone override:** config field `tz` accepts an IANA name (e.g. `"America/Los_Angeles"`) or the literal `"system"` to follow the host clock. `null` / absent / empty resolves to KST.

### Icons

- All icons listed above. No Nerd Font icons in the default theme — base Unicode + emoji only.
- A single space follows each icon before the widget value.

## Color

The default theme **ships with color** as of milestone 006. The underlying text above is the diff-friendly form that you get with `NO_COLOR=1` (or the test seam `ColorMode::Never`). The full theme adds:

### Per-widget foreground

| Widget | Foreground | Notes |
|---|---|---|
| `model` | cyan + bold | The whole `✦ [Opus 4.7 (1M context)]` block. |
| `cwd` | blue | Full `📂 …` segment. |
| `context_bar` | bracket/empty/digits default; filled cells tier-colored | See "Progress bar tiers" below. |
| `session_tokens` | magenta | Whole `📊 85.3K` segment. |
| `session_cost` | yellow | Whole `💰 $2.55` segment. |
| `block_timer` | icon + `5h` label dim; bar tier-colored; `(pct%)` dim; reset clock default | Multi-segment. |
| `weekly_timer` | same as `block_timer` | Multi-segment. |
| `git_branch` | green | Whole `🌿 main` segment. |
| `git_status` | yellow | Whole `⛓ …` segment (per-letter coloring deferred). |
| `git_changes` | `+ins` green, `-dels` red | Multi-segment. |

User overrides via `ccstatusline-rs config color <kind> --fg …` **replace** the theme color end-to-end for the chosen kind (including the threshold-driven bar cells). `config color … --clear` removes the override and falls back to the theme default. The override is replace, not merge — `--fg red` produces a red non-bold widget even if the theme default was cyan-bold.

### Progress bar tiers

`bar_tier_color(percent)` decides the color of the filled cells:

- `0 ≤ percent < 50` → **green** (ok).
- `50 ≤ percent < 80` → **yellow** (warn).
- `percent ≥ 80` → **red** (critical).
- `NaN` collapses to green.

Constants live in `src/render/color.rs` as `BAR_WARN_PERCENT = 50.0` and `BAR_CRIT_PERCENT = 80.0`.

### Color env precedence

`ColorMode::Auto` (the production default) emits color unless overridden:

- `NO_COLOR=<anything>` → off (always wins).
- `CLICOLOR_FORCE=<anything>` → on.
- `FORCE_COLOR=<non-empty, non-"0">` → on.
- Otherwise → on (Claude Code's statusline reliably renders ANSI we feed it).

Tests bypass env via `ColorMode::Always` / `ColorMode::Never` so the suite is deterministic regardless of the developer's shell.

## Change history

- **2026-05-14 — 006 default theme color (this commit).** Color added to the default theme; underlying text unchanged; new `insta` snapshot captures the ANSI bytes. Previous default (plain text, no escapes) is recoverable by running `ColorMode::Never` or piping `NO_COLOR=1` through the binary.
- **2026-05-14 — M0 bootstrap.** First locked default theme; plain text only, no ANSI.

## Test fixture

The canonical payload that produces this output lives at `tests/fixtures/default-payload.json` (to be created in the bootstrap exec-plan). Both the payload and the snapshot must change in the same commit.

## Change policy

This document is **locked**. Edits require:

1. A new design-doc note (or amendment here) explaining the why.
2. The `insta` snapshot updated in the same commit.
3. A line in `docs/STATUS.md` calling out the visible behavior change.
