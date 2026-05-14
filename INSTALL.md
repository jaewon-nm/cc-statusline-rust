# Installing `ccstatusline-rs`

A single static binary. Pick one of three paths below — all reach the same place.

## Required environment

- An OS shell that can pipe stdout to a child process (Windows PowerShell / cmd, bash, zsh — anything Claude Code already spawns).
- For git widgets (`git_branch`, `git_status`, `git_changes`): `git` on `PATH`. Optional: `gh` / `glab` (not used by current widgets, but reserved).
- No runtime libraries beyond libc (glibc on Linux, msvcrt on Windows, libSystem on macOS). The binary is `--strip`'d at release time.

## Binary size

| Triple | Stripped size |
|---|---|
| `x86_64-pc-windows-msvc` | ~1.9 MB |
| `x86_64-unknown-linux-gnu` | ~1.7 MB (estimated) |
| `aarch64-apple-darwin` | ~1.7 MB (estimated) |
| `x86_64-apple-darwin` | ~1.7 MB (estimated) |

Final numbers are reported in `docs/profiling/` once `v0.1.0` ships from CI. Locally measured Windows MSVC build: **1,910,272 bytes**.

## Path 1 — pre-built binary (recommended once a tag is cut)

GitHub Releases: `https://github.com/jaewon-nm/cc-statusline-rust/releases/latest`

Each release ships archives per triple plus a `.sha256` companion. Verify the hash, drop the binary somewhere on `PATH`:

```bash
# Linux / macOS
tar xzf ccstatusline-rs-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
install -m 0755 ccstatusline-rs-v0.1.0-x86_64-unknown-linux-gnu/ccstatusline-rs ~/.local/bin/
```

```powershell
# Windows
Expand-Archive .\ccstatusline-rs-v0.1.0-x86_64-pc-windows-msvc.zip -DestinationPath .
Move-Item .\ccstatusline-rs-v0.1.0-x86_64-pc-windows-msvc\ccstatusline-rs.exe $env:USERPROFILE\bin\
```

## Path 2 — `cargo install`

```bash
cargo install --git https://github.com/jaewon-nm/cc-statusline-rust ccstatusline-rs
```

Once a tagged release is on crates.io (M5+1), the `--git` flag goes away:

```bash
cargo install ccstatusline-rs
```

## Path 3 — build from source

```bash
git clone https://github.com/jaewon-nm/cc-statusline-rust
cd cc-statusline-rust
cargo build --release
# target/release/ccstatusline-rs (or .exe on Windows)
```

Requires Rust **1.94** (pinned by `rust-toolchain.toml`). `rustup show` will install it on first build.

## Wire it into Claude Code

Edit your Claude Code statusline setting (the upstream docs live at [`code.claude.com/docs/en/statusline`](https://code.claude.com/docs/en/statusline)). The line points at the binary; Claude Code pipes the status payload to stdin every refresh.

```jsonc
// ~/.config/claude-code/config.json (Linux / macOS) — adjust path as needed
{
  "statusLine": {
    "command": "ccstatusline-rs",
    "refreshInterval": 1000
  }
}
```

```jsonc
// Windows — explicit path is safer if you didn't add the binary to PATH
{
  "statusLine": {
    "command": "C:\\Users\\<you>\\bin\\ccstatusline-rs.exe",
    "refreshInterval": 1000
  }
}
```

## Turning color on

`ccstatusline-rs` ships **plain text by default**. Color is opt-in for two reasons: (1) the default golden snapshot stays byte-stable; (2) Claude Code pipes stdout, which would defeat any auto-detection anyway.

Step 1 — pick a styled config:

```bash
ccstatusline-rs config color model --fg cyan --bold
ccstatusline-rs config color session_cost --fg yellow
ccstatusline-rs config color block_timer --fg bright_blue
```

Step 2 — tell Claude Code to surface color. Add an env var to the statusline command:

```jsonc
{
  "statusLine": {
    "command": "ccstatusline-rs",
    "refreshInterval": 1000,
    "env": { "FORCE_COLOR": "1" }
  }
}
```

`NO_COLOR=1` always wins. `FORCE_COLOR=1` and `CLICOLOR_FORCE=1` are equivalent.

## Configuration shape (cheat sheet)

```bash
# Inspect
ccstatusline-rs schema                # JSON Schema of the config
ccstatusline-rs widgets               # available widget kinds
ccstatusline-rs config show --pretty  # current on-disk config (or default)

# Edit
ccstatusline-rs config add git_branch              # append to last line
ccstatusline-rs config add cwd --line 0 --position 1  # insert
ccstatusline-rs config remove --line 1 --position 0
ccstatusline-rs config color session_tokens --fg bright_green
ccstatusline-rs config apply --file my-layout.json

# Dry-run a candidate against a payload
ccstatusline-rs preview --payload sample.json --config candidate.json --diff
```

## Troubleshooting

- **Empty status line.** Most likely the payload lacks `cwd` / `model` / `context_window`. Run the binary by hand: `cat payload.json | ccstatusline-rs` and check what each widget returns by toggling them via `config remove`.
- **`git` widgets blank.** The binary skips git when `cwd` is outside a repo or when probing exceeds an 800 ms wall clock. Confirm with `cd <cwd> && git status` — if that works in <800 ms, file an issue with a profile.
- **Stale numbers.** The JSONL probe and the git probe both cache to disk. The git cache TTL is 2 s; JSONL cache invalidates on `(mtime, size)` change. Delete `~/.cache/ccstatusline-rs/` (Linux/macOS) or `%LOCALAPPDATA%\dev\naya\ccstatusline-rs\cache\` (Windows) to clear.
- **Locale shows commas in numbers.** It shouldn't — formatters are locale-independent. If you see commas, file an issue with your OS / Windows locale.
