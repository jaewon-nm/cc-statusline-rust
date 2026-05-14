# cc-statusline-rs — Top-level Design

**Status:** draft · **Owner:** jaewon_lee · **Last touched:** 2026-05-14

A Rust port of [`ccstatusline`](https://github.com/sirmalloc/ccstatusline) (TypeScript/Bun, [`references/ccstatusline/`](../../references/ccstatusline/)) with one deliberate divergence: the interactive TUI is **removed**. All configuration happens through an agent-friendly CLI surface.

## Goals

1. **Drop-in renderer for Claude Code.** Reads the same stdin payload the upstream binary reads; emits a status line indistinguishable from upstream for the default layout.
2. **Agent-driven configuration.** AI agents (the user's primary editor in this project) edit config through declarative subcommands, never an interactive menu.
3. **Single static binary.** No runtime dependency beyond `git`/`gh`/`glab` on PATH (optional for git widgets).
4. **Snapshot-tested rendering.** The default theme is a golden output, locked by an `insta` snapshot.

## Non-goals

- Interactive (curses/ratatui) configurator. Anyone wanting one can use the upstream Node binary.
- Embedding `git2`. Shell-out matches upstream behavior and avoids libgit2's footprint.
- Async runtime on the renderer path. See CLAUDE.md → Performance Budget.

## Architecture

```
                       ┌─ (default, sync)  renderer ───► stdout (ANSI)
stdin JSON ─► CLI ─────┤
  (Claude Code or      ├─ config show / add / set / remove / apply / validate
   agent)              ├─ preview [--payload …] [--diff]
                       ├─ widgets   (list widget kinds + option schemas, JSON)
                       └─ schema    (emit JSON Schema for the config)
                                │
                                ▼
                         config file
                  (XDG / %APPDATA% via `directories`)
```

```
src/
  main.rs                — clap dispatch, owns the only anyhow boundary
  cli/                   — subcommand implementations, JSON I/O contract
  widgets/               — each widget = pure fn(&Context) -> Option<Segment>
  render/                — Segment → ANSI line assembly, width-aware truncation
  config/                — serde-backed schema + schemars JSON Schema + atomic save
  context/               — payload parse + git/usage/voice/timer probes
```

Key shape rules:

- **Widget = pure function.** `fn(&Context) -> Option<Segment>`. No I/O inside the function; I/O lives in `context/` probes that build the `Context` once at startup.
- **Segment is data, not ANSI.** A `Segment { text, style }` is rendered to ANSI in `render/`, never in widgets. This is what lets us snapshot-test layout (Segment list) separately from styling (ANSI bytes).
- **Config is the source of truth.** The renderer reads config + payload → builds segments → emits ANSI. There is no implicit state.

## CLI Surface

| Subcommand | Purpose | stdout |
|---|---|---|
| *(none)* | Render: read payload from stdin, write ANSI to stdout. | ANSI text |
| `config show` | Print current config. | JSON |
| `config add <kind>` | Append a widget of `<kind>` with default options. | JSON (new config) |
| `config remove <id>` | Remove widget by id. | JSON (new config) |
| `config set <path> <value>` | Edit one field by dotted path (e.g. `lines[0].widgets[2].color=red`). | JSON (new config) |
| `config apply --file x.json` | Replace whole config; schema-validate before save. | JSON (new config) |
| `config validate [--file x.json]` | Validate current or given config against schema. | JSON `{"ok":true}` or `{"ok":false,"errors":[…]}` |
| `preview [--payload f.json]` | Render against a payload (default: bundled sample) and print ANSI. | ANSI text |
| `preview --diff` | Show side-by-side current-vs-pending preview (after `--stash` editing). | ANSI text |
| `widgets` | List available widget kinds + per-kind option schema. | JSON |
| `schema` | Emit the full config JSON Schema. | JSON Schema |

### Output contract for agents

- **Default stdout is machine-parseable JSON** for any non-renderer subcommand. `--pretty` opts into human formatting.
- **Errors** go to stderr (one human line) **and** stdout as `{"error":{"code":"…","message":"…","details":{…}}}`. Exit code non-zero.
- **Idempotent writes.** Every `config *` mutation is write-temp + rename. A concurrent renderer either sees the old file or the new file, never a half-written one.
- **No stdin prompts, no TTY detection that changes behavior.** The agent treats this binary as a pure function over (config, payload).

## Config file

- **Location:** resolved via `directories::ProjectDirs` (`~/.config/ccstatusline-rs/config.json` on Linux, `%APPDATA%\ccstatusline-rs\config.json` on Windows, `~/Library/Application Support/ccstatusline-rs/config.json` on macOS).
- **Format:** JSON. Hand-editable, schema-validated, schemars-emitted schema available via `ccstatusline-rs schema`.
- **Versioning:** top-level `"$schema"` and `"version"` fields. Reject unknown versions with a clear error; never silent-upgrade.
- **Compatibility with upstream:** out of scope. Don't try to read upstream's TOML config.

## Default theme

See [`default-theme.md`](default-theme.md) for the golden output spec and formatting rules.

## Open questions

- **Color in default theme?** Currently locked to plain text in [`default-theme.md`](default-theme.md). Revisit once we have the bare renderer working — may want a `color: auto` mode that respects `NO_COLOR`/`FORCE_COLOR` and emits foreground colors for status/cost.
- **Powerline support?** Defer to a separate design doc. The Segment model must not bake in non-Powerline assumptions.
- **Git probe caching?** A sub-100ms budget means `git status` per refresh is borderline. Possible mitigation: cache the result for N ms in a temp file. Mark as a Phase 1+ concern.
