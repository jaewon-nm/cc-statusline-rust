# ccstatusline-rs

Rust port of [`ccstatusline`](https://github.com/sirmalloc/ccstatusline) — an agent-friendly status line formatter for the [Claude Code](https://docs.anthropic.com/en/docs/claude-code) CLI.

The renderer reads Claude Code's stdin payload and prints a formatted status line. Unlike upstream there is **no interactive TUI** — every config edit is a single-shot CLI subcommand that emits JSON, so an AI agent can drive the configuration end-to-end.

> Project status, architecture, and design specs live under [`docs/`](docs/). Start with [`docs/INDEX.md`](docs/INDEX.md).

## Default output

```
✦ [Opus 4.7 (1M context)] | 📂 F:\Works\naya\cc-statusline-rust | 🔋 [..........] 80.0K/1.0M(8%) | 📊 85.3K | 💰 $2.55
⏱ 5h [##........](21%) ↻ 12:00 | 📅 7d [##........](20%) ↻ 5/19 06:00
```

Locked spec: [`docs/design-docs/default-theme.md`](docs/design-docs/default-theme.md).

## Quick start

```powershell
# Build
cargo build --release

# Render against a payload
Get-Content tests/fixtures/default-payload.json | ./target/release/ccstatusline-rs.exe

# Inspect the config / widget surfaces (agent-friendly, JSON output)
./target/release/ccstatusline-rs.exe schema
./target/release/ccstatusline-rs.exe widgets
./target/release/ccstatusline-rs.exe config show --pretty
```

## Layout

```
src/
  cli/         CLI dispatch and subcommand implementations
  config/      serde + schemars config schema
  context/     payload parser + semantic Context type
  render/      Segment → ANSI assembly + locale-independent formatters
  widgets/     One file per widget; pure fn(&Context) -> Option<Segment>
tests/
  fixtures/    Canonical payloads
  snapshots/   insta-locked golden output

docs/
  design-docs/ Product design (architecture, default theme, …)
  exec-plans/  Implementation plans (active / completed)
```

## Upstream reference

For TS/Bun upstream parity work, clone the reference alongside this repo (it is intentionally not vendored — see `.gitignore`):

```bash
git clone https://github.com/sirmalloc/ccstatusline references/ccstatusline
```

## Governance

- Rust 1.94 pinned (`rust-toolchain.toml`).
- Strict typed errors via `thiserror`; `anyhow` only in `main.rs`.
- 100% test coverage on shipped modules; no skipped or `#[ignore]` tests outside benchmarks.
- WHY-only comments; no Phase/ticket tags in code.
- KST default timezone; `tz` config field accepts an IANA name or `"system"`.

Full rules: [`CLAUDE.md`](CLAUDE.md). Doc workflow: [`docs/GOVERNANCE.md`](docs/GOVERNANCE.md).

## License

MIT — same as upstream.
