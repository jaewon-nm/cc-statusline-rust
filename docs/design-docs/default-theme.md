# Default Theme — Golden Output

**Status:** locked (changes require explicit governance update) · **Owner:** jaewon_lee · **Last touched:** 2026-05-14

The renderer with the **default config and the canonical payload** must produce exactly this two-line output. This is the `insta` snapshot baseline; any change to the bytes below is a behavior change.

## Golden output

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

- **Default theme is plain text.** No ANSI styling.
- Reason: diff-friendly, captures cleanly in log files and PR descriptions, makes the `insta` snapshot byte-stable across terminals.
- Color is opt-in via config. The renderer must respect `NO_COLOR` and `FORCE_COLOR` once color is enabled (handled in `render/`, not per widget).

## Test fixture

The canonical payload that produces this output lives at `tests/fixtures/default-payload.json` (to be created in the bootstrap exec-plan). Both the payload and the snapshot must change in the same commit.

## Change policy

This document is **locked**. Edits require:

1. A new design-doc note (or amendment here) explaining the why.
2. The `insta` snapshot updated in the same commit.
3. A line in `docs/STATUS.md` calling out the visible behavior change.
