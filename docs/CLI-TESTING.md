# CLI Testing Guide

> **Purpose:** a single reference an agent (Claude or otherwise) can look at before invoking the binary, so a session does not waste turns rediscovering paths, fixture names, or output contracts.

## TL;DR (the four lines agents forget)

1. **Binary name is `ccstatusline-rs`** (note the `-rs` suffix to avoid shadowing the upstream Node binary). Found at `target/debug/ccstatusline-rs(.exe)` or `target/release/...` after build.
2. **Default invocation = renderer.** Pipe a payload JSON to stdin; ANSI status line comes out on stdout. No subcommand.
3. **Every other surface is a subcommand and emits JSON on stdout.** `--pretty` opts into human formatting. Errors → stderr (one line) + JSON envelope on stdout, non-zero exit.
4. **Snapshot tests are the source of truth for rendered output.** Don't compare against hand-typed strings in tests — use `insta` snapshots reading from `tests/fixtures/`.

## Binaries and paths

| What | Path |
|---|---|
| Debug binary | `target/debug/ccstatusline-rs(.exe)` |
| Release binary | `target/release/ccstatusline-rs(.exe)` |
| Sample payload (canonical) | `tests/fixtures/default-payload.json` (created in M0 bootstrap) |
| Golden snapshot | `tests/snapshots/render__default_theme.snap` (insta) |
| Config file (resolved at runtime) | `directories::ProjectDirs("dev", "naya", "ccstatusline-rs").config_dir() / config.json` |

## Running the renderer locally

PowerShell:
```powershell
Get-Content tests/fixtures/default-payload.json | cargo run --bin ccstatusline-rs
```

Bash:
```bash
cat tests/fixtures/default-payload.json | cargo run --bin ccstatusline-rs
```

For raw byte verification (no shell munging):
```bash
cargo run --bin ccstatusline-rs < tests/fixtures/default-payload.json | xxd | head
```

## Running config / inspection subcommands

```powershell
cargo run --bin ccstatusline-rs -- schema | jq .
cargo run --bin ccstatusline-rs -- widgets | jq .
cargo run --bin ccstatusline-rs -- config show
cargo run --bin ccstatusline-rs -- config validate
cargo run --bin ccstatusline-rs -- preview
```

## Test execution

```powershell
# Full suite (unit + integration + CLI snapshot)
cargo nextest run

# Single module
cargo nextest run --package ccstatusline-rs --lib widgets::model

# Snapshot review (after intentional changes)
cargo insta review

# Coverage
cargo llvm-cov --lcov --output-path lcov.info
```

`cargo test` works too but `cargo nextest run` is the project standard (parallelism + flake detection).

## Output-contract assertions

For CLI subcommand tests, **always** assert via `serde_json` round-trip, never on raw string text:

```rust
let out = Command::cargo_bin("ccstatusline-rs")?
    .args(["config", "show"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();

let parsed: serde_json::Value = serde_json::from_slice(&out)?;
assert_eq!(parsed["lines"].as_array().unwrap().len(), 2);
```

Reason: stdout text formatting (whitespace, key order) can change without changing the contract. The contract is the JSON value tree.

## Renderer snapshot pattern

```rust
#[test]
fn default_theme_matches_golden() {
    let payload = include_str!("../fixtures/default-payload.json");
    let rendered = ccstatusline_rs::render_with_default_config(payload).unwrap();
    insta::assert_snapshot!(rendered);
}
```

After an intentional default-theme change, run `cargo insta review`, accept, and update [`design-docs/default-theme.md`](design-docs/default-theme.md) + `STATUS.md` in the same commit.

## Common pitfalls

| Symptom | Cause | Fix |
|---|---|---|
| Binary "command not found" | Forgot to `cargo build` or you're hitting `ccstatusline` (upstream Node) instead of `ccstatusline-rs` | Use the full target path: `target/debug/ccstatusline-rs.exe` |
| Test output flickers between runs | Renderer reads system time / git status, which is nondeterministic | Inject the time/git output via test scaffolding — never read system clock in widget unit tests |
| `insta` snapshot keeps changing | Trailing whitespace, CRLF vs LF, or system-dependent path separator | The default theme spec says `F:\Works\naya\...` literally — payload fixture controls the cwd, not the OS |
| Config file lock / race in tests | Two tests writing the real config dir | Use `tempfile::tempdir()` + `directories` shim via env override (M2 will add `CCSTATUSLINE_RS_CONFIG=…` for tests) |
| Color in test output | A widget styled a segment unconditionally | Default theme is plain text. Style must come from config, not widget code. |

## Diagnosing a hang or crash

The binary has no daemon / no background work, so any hang is either:

1. **Stdin blocked** — renderer waits for EOF. Make sure the test feeds the payload and closes stdin.
2. **Infinite loop in a widget** — usually a malformed `progress` or `truncate` calc. Run with `RUST_LOG=trace cargo run -- --pretty config validate` to see traces.
3. **`git` subprocess hanging** — `gh`/`glab` with an interactive auth prompt. Always run probes with `--no-prompt` style flags and a short timeout (handled in `context/git.rs`).

For panics, set `RUST_BACKTRACE=1`.
