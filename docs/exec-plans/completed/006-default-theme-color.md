# 006 — Default theme color + threshold-aware progress bars

**Status:** active · Codex-AGREE (round 3) · **Owner:** jaewon_lee · **Opened:** 2026-05-14 · **Last revision:** 2026-05-14

## Goal

Two coupled changes the renderer needs in one pass:

1. **The default theme itself ships with color.** Each of the seven widgets gets a baked-in foreground style; the golden snapshot rolls forward to the new ANSI bytes. No more `config color …` required for the user to see something pretty.
2. **Progress bars (context_bar, block_timer, weekly_timer) become threshold-aware.** The filled portion changes color at usage tiers: green ≤49% → yellow 50–79% → red ≥80%. The empty portion and brackets stay neutral.

Both pieces require the Segment data model to carry more than one styled run per widget invocation. That's a Phase A refactor; the rest stacks on top.

## Non-goals

- **No per-character coloring within a numeric value.** `80.0K/1.0M(8%)` stays one color; the threshold logic applies to the bar cells only. Per-character styling is a non-starter for the data/styling separation.
- **No Powerline.** Background-color separators are still deferred.
- **No theme presets ecosystem.** "rich" / "monochrome" / "cyberpunk" presets would be a future feature; this plan only changes the default and keeps `config color` / `config apply` as the per-user override.
- **No new env knobs.** `NO_COLOR` / `FORCE_COLOR` / `CLICOLOR_FORCE` semantics stay as M4 set them; only the *default* (color-on vs color-off) flips.

## Phase A — widget signature: `Option<Segment>` → `Option<Vec<Segment>>`

The single-segment-per-widget contract has to go so progress bars can emit `bracket-default · cells-tier-colored · bracket-default · rest-default`.

### A.1 Refactor mechanics

- `widgets::WidgetSpec::render` becomes `fn(&Context) -> Option<Vec<Segment>>`.
- Each widget returns either `None` (suppressed) or `Some(vec![ … ])` with one or more segments. Trivial widgets (`model`, `cwd`, `session_tokens`, `session_cost`, `git_branch`, `git_status`) emit one or two segments with **theme-default styles baked in** (e.g. model returns a single cyan-bold segment; the cyan-bold is part of widget code, not renderer policy).
- Bar widgets (`context_bar`, `block_timer`, `weekly_timer`) and `git_changes` emit multiple styled runs per call — up to ~8 for the timer widgets (icon · `5h` label · `[` · filled cells (tier-colored) · empty cells · `]` · `(pct%)` · ` ↻ ` · `HH:mm`).
- `render::build_line` flattens widget output into `Line.segments`. Inter-widget separator (` | `) rules unchanged. **The only mutation `build_line` performs on widget output is applying a user color override (Phase C below).**

### A.2 Test fanout

- All ten widget unit-test helpers stop comparing to a single `Segment::text` and start asserting on concatenated text + per-segment style. Pattern: `let segs = render(&ctx).unwrap(); assert_eq!(joined(&segs), "✦ [Opus …]"); assert_eq!(segs[0].style.get_fg_color(), Some(AnsiColor::Cyan.into()));`.
- The existing renderer e2e tests assert on the joined text + ANSI envelope, not the exact wire bytes (which the insta snapshot pins instead).

## Phase B — threshold helper

`render/color.rs` gains:

```rust
pub fn bar_tier_color(percent: f64) -> anstyle::Color {
    if percent.is_nan() || percent < 50.0 { AnsiColor::Green.into() }
    else if percent < 80.0 { AnsiColor::Yellow.into() }
    else { AnsiColor::Red.into() }
}
```

Tier boundaries are project-wide constants (`TIER_WARN = 50.0`, `TIER_CRIT = 80.0`). The percent-driven color is **the theme default** for the filled-cell segment of bar widgets. It is *not* layered on top of a user override — see Phase C below for the override-wins rule.

The `format::format_bar_wrapped` helper currently produces a single string `[##........]`. We split it into three strings — `"["`, `"##........"`, `"]"` — and let the widget assemble three segments around them. Inside the cell run, the *empty* portion (`.`) and the *filled* portion (`#`) are also split when they coexist, so the bar widget actually emits up to four cell-related segments: `"["`, `"##"` (tier-colored), `"........"` (neutral), `"]"`.

For widgets at 0% or 100%, one of the cell runs is empty and is omitted.

## Phase C — theme defaults inside widgets, override at renderer

`Config::default_layout()` keeps `colors` empty. Theme default colors live **inside each widget's `render` function** — the widget hand-picks the style for every segment it emits. The renderer's only responsibility is the user-override layer:

```rust
fn build_line(...) {
    for kind in row {
        let widget_segments = (spec.render)(ctx)?;
        let segments = match cfg.colors.get(kind) {
            Some(user_style) => widget_segments.into_iter()
                .map(|seg| Segment { text: seg.text, style: user_style.to_style()? })
                .collect(),
            None => widget_segments,
        };
        // …push into Line…
    }
}
```

Why this shape (Codex round 1 #4 + round 2 structural fix):

- **Widget owns its styling story end-to-end.** Bar widget knows that its filled cells get tier color, its empty cells stay default, its brackets stay default. Renderer can't accidentally over-paint the empty cells because it never picks defaults.
- **User override wins absolutely** (Codex round 1 #5). When `cfg.colors[kind]` is set, the renderer **replaces every segment style** with the user color. So `config color context_bar --fg red` makes the entire context bar red — filled cells, empty cells, brackets, everything. Threshold and theme are bypassed entirely. This is the "user said red so it stays red" intuition.
- **Override is replace, not merge** (Codex round 3). `config color model --fg red` produces a *red non-bold* model widget — it does not retain the theme's cyan-bold and tint it red. Documented in the schema description and in `config color`'s help text.
- **Invalid `ColorStyle` at render time is defensively handled.** `Config::validate` runs at every mutation path (`config color`, `config apply`, `config add/remove`) and rejects unparseable color strings before they touch disk. Note that `config::load_from_or_default` (the read path used by the renderer and by `config show`) does **not** call `validate(known_kinds)` — it only enforces the schema version. A hand-edited corrupt color string can therefore reach the renderer; `build_line`'s `user_style.to_style().ok()` fallback silently drops the offending override and uses the widget's own theme style. This is intentional — the renderer never panics on a malformed config — and surfaces the issue on the user's next `config validate` call.
- **Existing user configs survive intact.** Configs without a `colors` map render with theme defaults because the widgets bake them in.
- **`config color … --clear`** semantics: it removes the user override, which means the widget's own styles take effect again. Document as "revert to theme default", not "plain".
- **`config show --pretty`** stays compact — no big colors map clutters fresh installs.

Picks (subject to one round of taste-bikeshedding before merge):

| Widget kind | Foreground | Bold |
|---|---|---|
| `model` | `cyan` | yes |
| `cwd` | `blue` | no |
| `context_bar` | (none — the bar handles its own colors; brackets/empty stay default) | – |
| `session_tokens` | `magenta` | no |
| `session_cost` | `yellow` | no |
| `block_timer` | (icon + `5h` label colored `bright_black`; bar internal) | – |
| `weekly_timer` | (same scheme as `block_timer`) | – |
| `git_branch` | `green` | no |
| `git_status` | (per-cell; clean = green ✓, dirty parts colored individually — out of scope for now, use single `yellow` for the whole widget) | – |
| `git_changes` | (insertions `green`, deletions `red` — multi-segment) | – |

Per-widget theme colors target *the icon + literal text outside the bar*. Threshold colors target *the filled bar cells only*. Both live inside the widget code — bar widgets emit a mix of theme-default-styled, threshold-styled, and unstyled segments in one `Vec<Segment>`.

**User-override precedence over everything** (Codex round 1 #5, round 2 structural). The widget produces theme-styled segments regardless of `cfg.colors`. The renderer is the only place that knows about `cfg.colors[kind]`; when present, it overwrites every segment's style with the user color. So `config color context_bar --fg red` turns the bar entirely red — filled cells, empty cells, brackets — bypassing both theme and threshold.

For `git_changes`, since the widget already emits two distinct numeric pieces (`+120 -20`), the multi-segment refactor naturally lets `+` part go green and `-` part go red without bar-specific logic.

## Phase D — `ColorMode` programmatic seam + always-on default

Replace env-only resolution with an explicit mode resolved before rendering (Codex round 1 #6):

```rust
pub enum ColorMode { Auto, Always, Never }

impl ColorMode {
    pub fn resolve(self) -> bool { /* Auto consults NO_COLOR/FORCE_COLOR/CLICOLOR_FORCE */ }
}
```

- `Auto` (production default): NO_COLOR wins (off) → CLICOLOR_FORCE / FORCE_COLOR force on → otherwise **on** (the renderer always feeds Claude Code which renders ANSI, regardless of TTY status of our piped stdout).
- `Always` (tests + future CLI need): emit color unconditionally.
- `Never` (tests + future CLI need): never emit color.

`render::render(ctx, cfg)` calls `ColorMode::Auto.resolve()`. A new `render::render_with_mode(ctx, cfg, mode)` is the test seam — snapshot tests pass `Always`, plain-text invariant tests pass `Never`. Env mutation in tests is forbidden (was unstable on parallel runners). Env-precedence is covered by isolated unit tests for `ColorMode::resolve()` that *do* mutate env but run single-threaded under their own `#[cfg(test)]` mutex.

`color_enabled(default)` becomes `ColorMode::Auto.resolve_with_default(default: bool)` internally; the old test asserts adapt.

## Phase E — golden snapshot rolls forward via `ColorMode`

`docs/design-docs/default-theme.md`:

- Plain-text golden block stays at the top labeled **"underlying text"** for diff-friendly reads.
- A new section documents the **default theme color spec** (the Phase C table) and **threshold tiers** (Phase B constants).
- The `insta` snapshot at `tests/snapshots/render_default_theme__default_theme_snapshot.snap` is **replaced**, not deleted. Old bytes archived in the design doc's "Change history" section so anyone wondering what the M0 default looked like can reconstruct it.

The `default_theme_matches_golden_string` test that asserts on the *plain text* (no ANSI escapes) is retained and calls `render::render_with_mode(ctx, cfg, ColorMode::Never)`. The colored snapshot test calls `render::render_with_mode(ctx, cfg, ColorMode::Always)`. **No env mutation in test setup** (Codex round 1 #6) — both tests are deterministic against any developer's local `NO_COLOR` / `FORCE_COLOR` state. No `strip-ansi-escapes` crate added (Codex round 1 #9) — the test seam makes it unnecessary.

`ColorMode` is not exposed as a CLI flag in this milestone (Codex round 1 #7) — `NO_COLOR=1` remains the only public opt-out. A future plan can promote `ColorMode` to a `--color {auto,always,never}` arg if the demand is real.

## Phase F — docs + gates

- [ ] `default-theme.md` updated per Phase E.
- [ ] `CLAUDE.md` Default Theme section needs a sentence: "Default ships with color; opt-out via `NO_COLOR`."
- [ ] `CHANGELOG.md` Unreleased: behavior change (default theme is now colored, threshold-colored bars).
- [ ] `STATUS.md` row + history line.
- [ ] `cargo nextest run --test-threads=1` green.
- [ ] `cargo clippy --all-targets -- -D warnings` clean.
- [ ] `cargo fmt --check` clean.
- [ ] **New** snapshot captures the colored default bytes; `cargo insta review` accepted in one commit.
- [ ] `git mv` plan to completed/.

## Risks & open questions

1. **Yellow vs orange.** Anstyle's named palette has `yellow` but not `orange`. The Phase B tier uses `yellow` for the warning band. RGB (`#ff8800`) is available but breaks on 8-color terminals. Decision: stick with `yellow` for the AnsiColor name; users who want true orange can run `config color` with a hex override on a future per-tier-color knob (out of scope here).
2. **Renderer cold-start budget.** Splitting bars into segments adds ~5 `String` allocations per progress widget per invocation. Three progress widgets = ~15 allocations. Renderer budget is ≤50 ms cold; this is rounding noise. Verify via `cargo bench` only if profiling shows regression.
3. **`Vec<Segment>` allocation per widget.** Every widget call now allocates a `Vec` even when emitting one segment. Smallvec-style optimization is premature; if profile shows a hotspot, add `tinyvec` later.
4. **git_status multi-color** out of scope per Phase C, so `S2 M3 ?1 !1` stays one yellow chunk. A future refactor can give each letter its own color (e.g., `S` red, `M` yellow, `?` blue, `!` red) once we want it.
5. **User `config color` interaction with default theme colors.** When the user runs `config color model --fg red`, the renderer replaces every segment style emitted by the `model` widget with red — overriding the widget's own cyan-bold. No initialization in `default_layout()` is needed; theme defaults live in widget code, user overrides in `cfg.colors`, renderer applies the override last. The two systems can't fight because they don't share storage.
6. **Threshold customization** deferred. Users who want different bands (e.g., 60/90 instead of 50/80) get no knob in this milestone. Track separately.
7. **Snapshot stability under `NO_COLOR`** — addressed by the `ColorMode` seam (Phase D / E). Snapshot test pins `ColorMode::Always`; plain-text invariant pins `ColorMode::Never`. Tests don't mutate env, so a developer with `NO_COLOR=1` in their shell still sees green tests. Env precedence covered by separate tests targeting `ColorMode::resolve()` directly.

8. **Pre-existing user configs without a `colors` map** (Codex round 1 #4 sharp edge) — the code-level fallback in Phase C means they get the new theme automatically. A test asserts: "config with `lines` set but no `colors` field, parsed from JSON, renders with the new default theme colors." Without this guarantee, users who upgrade after running `config add git_branch` once would silently miss the new theme.

## Acceptance

- `cargo run -- < tests/fixtures/default-payload.json` produces the new colored output with ANSI escapes inline.
- Visual inspection: model = cyan-bold, cwd = blue, context bar's filled cells = green (since 8% < 50), block timer's filled cells = green (21%), weekly timer's filled cells = green (20%). At higher percentages the same fixture (modified) would show yellow / red — covered by unit tests for `bar_tier_color`.
- The new `insta` snapshot is byte-stable across runs (no time leaks).
- Underlying text (ANSI stripped) still matches the M0 golden — proves no characters were lost in the color rollover.
- All existing test cases adapted to the new widget shape; no `#[ignore]`, no skipped.
- New regression test: a user `Config` with explicit `lines` but absent `colors` map (round-tripped from JSON) renders with the new default theme colors. Guards against the M0-user-upgrades-silently-loses-color failure mode.
- New unit tests for `ColorMode::resolve()` covering all four env-precedence rules (NO_COLOR wins; CLICOLOR_FORCE forces; FORCE_COLOR forces; default applies otherwise). These tests do mutate env and are gated single-threaded.
