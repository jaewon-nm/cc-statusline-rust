# Build & Distribution Guide

> Build, cross-compile, and release ccstatusline-rs.
> Toolchain install in [`PREREQUISITES.md`](PREREQUISITES.md); test recipes in [`CLI-TESTING.md`](CLI-TESTING.md).

## Local dev

```powershell
# Renderer (read fixture payload from stdin)
Get-Content scripts/payload.example.json | cargo run --bin ccstatusline-rs

# Agent CLI surface (always JSON unless --pretty)
cargo run --bin ccstatusline-rs -- schema
cargo run --bin ccstatusline-rs -- widgets
cargo run --bin ccstatusline-rs -- config show
cargo run --bin ccstatusline-rs -- preview
```

## Release build

```powershell
cargo build --release --bin ccstatusline-rs
# target/release/ccstatusline-rs.exe (Windows)
# target/release/ccstatusline-rs     (Linux/macOS)
```

Strip if size matters (Linux/macOS):
```bash
strip target/release/ccstatusline-rs
```

## Cross-compile targets

Final artifact is one static binary per triple. Target list:

| Triple | Notes |
|---|---|
| `x86_64-pc-windows-msvc` | Primary dev platform |
| `x86_64-unknown-linux-gnu` | glibc; consider `-musl` for fully static |
| `aarch64-apple-darwin` | Apple Silicon |
| `x86_64-apple-darwin` | Intel macOS |

Cross-builds are driven from CI (target details land in the exec-plan when M5 — Distribution — starts). For now, local cross-builds via `cargo-zigbuild` are acceptable but not required.

## Distribution channels (TBD)

To be decided in M5:

- `cargo install ccstatusline-rs` from crates.io
- GitHub Releases pre-built binaries per triple
- Optional: Homebrew tap (`brew install <user>/tap/ccstatusline-rs`)

Channel choice and the version-pinning story (the upstream npm package supports pinning) are deferred until the renderer + config surfaces stabilize.

## Versioning

- SemVer on the public surface = CLI subcommand contract + config schema + default-theme output bytes.
- Default-theme bytes are a **public surface**. Changing them requires a minor bump and a `STATUS.md` line — see [`design-docs/default-theme.md`](design-docs/default-theme.md) change policy.
- Crate version in `Cargo.toml` is the source of truth; release tags follow `v<x.y.z>`.

## Claude Code integration

Once published, point Claude Code's `statusLine` config at the binary:

```json
// ~/.config/claude-code/config.json (or per-workspace equivalent)
{
  "statusLine": {
    "command": "ccstatusline-rs",
    "refreshInterval": 1000
  }
}
```

The binary reads the payload from stdin every refresh and writes the formatted line(s) to stdout. No subcommand needed for the renderer path.
