# Installing `ccstatusline-rs`

A single static binary. Three install flows below — pick the recommended one, or drop down to manual if you'd rather not let the binary touch `~/.claude/settings.json` itself.

## Required environment

- An OS shell that can pipe stdout to a child process (Windows PowerShell / cmd, bash, zsh — anything Claude Code already spawns).
- For git widgets (`git_branch`, `git_status`, `git_changes`): `git` on `PATH`. Optional: `gh` / `glab` (not used by current widgets, but reserved).
- No runtime libraries beyond libc (glibc on Linux, msvcrt on Windows, libSystem on macOS). The binary is `--strip`'d at release time.

## Binary size

| Triple | Stripped size | Pre-built archive on Releases? |
|---|---|---|
| `x86_64-pc-windows-msvc` | ~1.9 MB | ✅ |
| `x86_64-unknown-linux-gnu` | ~1.7 MB (estimated) | ✅ |
| `aarch64-apple-darwin` (Apple Silicon) | ~1.7 MB (estimated) | ✅ |
| `x86_64-apple-darwin` (Intel macOS) | ~1.7 MB (estimated) | ❌ build from source |

Intel macOS is not in the CI release matrix — GitHub-hosted `macos-13` Intel runners are being deprecated and queue times are unpredictable. Intel mac users have a clean source-build path: `cargo install --git https://github.com/jaewon-nm/cc-statusline-rust ccstatusline-rs`. Same end binary, just compiled locally.

Final numbers are reported in `docs/profiling/` once `v0.1.0` ships from CI. Locally measured Windows MSVC build: **1,910,272 bytes**.

---

## Recommended — `ccstatusline-rs install`

One command. Wires everything: copies the binary to `~/bin` (Windows) or `~/.local/bin` (POSIX), writes a `.mjs` wrapper on Windows (required to dodge a Claude Code Windows-native statusLine bug — see [#31670](https://github.com/anthropics/claude-code/issues/31670)), backs up `~/.claude/settings.json`, then rewrites just the `statusLine` block.

```powershell
# Build (or download from a tagged release; once a tag is cut, see "Pre-built archive" below)
cargo build --release

# Install
./target/release/ccstatusline-rs install

# Optional flags
./target/release/ccstatusline-rs install --bin-dir D:\tools --settings D:\claude\settings.json --force --pretty
```

The command prints a single-line JSON report on stdout (use `--pretty` for indented output):

```jsonc
{
  "installed": true,
  "bin": "C:\\Users\\you\\bin\\ccstatusline-rs.exe",
  "wrapper": "C:\\Users\\you\\bin\\ccstatusline-rs.mjs",   // null on POSIX
  "settings": "C:\\Users\\you\\.claude\\settings.json",
  "backup": "C:\\Users\\you\\.claude\\settings.json.ccstatusline-rs-bak-20260514-130000-000",
  "copied_binary": true,
  "previous_command": "node \"...\\old.mjs\""              // whatever statusLine.command pointed at before, for audit
}
```

**Restart Claude Code** for the new `statusLine` to take effect. Same JSON contract on every platform.

### Coexistence with neo-mem tokenwatch

If `tokenwatch-statusline.mjs` (neo-mem's in-house rate-limits collector) is already wired into your `~/.claude/settings.json`, `install` detects it by basename and **routes through wrap mode** instead of overwriting `settings.json`:

- Binary + Windows `.mjs` wrapper are still placed in `--bin-dir` (tokenwatch needs them to exist — it spawns them).
- `~/.claude/.tw-statusline-prev.json` is rewritten to point at our command (or refreshed if it already pointed at us — `--bin-dir A → B` relocation is recognized as "still ours" via basename match).
- `settings.json` is **byte-identical** before and after. No backup is created because nothing changed there.
- If `.tw-statusline-prev.json` already references a non-ours command (some other tool already grabbed the wrap slot), install fails loudly with `WrapConflict` and the existing command verbatim in the error — no silent overwriting. Reconcile manually before re-running.

Wrap-mode JSON output:

```jsonc
{
  "installed": true,
  "mode": "wrap",
  "bin": "C:\\Users\\you\\bin\\ccstatusline-rs.exe",
  "wrapper": "C:\\Users\\you\\bin\\ccstatusline-rs.mjs",
  "settings": "C:\\Users\\you\\.claude\\settings.json",
  "backup": null,
  "copied_binary": true,
  "previous_command": null,
  "wrap_prev_path": "C:\\Users\\you\\.claude\\.tw-statusline-prev.json",
  "previous_wrap_command": null,
  "wrap_explanation": "settings.json untouched — tokenwatch wrap-mode in effect"
}
```

`uninstall` reverses the right artifact based on positive evidence — settings statusLine is tokenwatch AND prev pointer is ours → wrap mode (remove prev, leave settings); a settings backup we wrote → direct mode (restore backup). If nothing matches, uninstall aborts with `NoInstallTraces` rather than restoring the wrong file. If the prev pointer is ours but settings.json is no longer tokenwatch (operator reset tokenwatch externally), uninstall fails with `StaleWrapPointer` so the inconsistency surfaces instead of being silently corrected. An explicit `--backup <path>` always forces direct mode.

### Pre-built archive (once a tag is cut)

`v*.*.*` tag pushes trigger the CI release workflow which uploads per-triple archives + SHA-256 companions to GitHub Releases. Download, verify, extract, then run `ccstatusline-rs install` from the extracted dir:

```bash
# Linux / macOS
tar xzf ccstatusline-rs-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
./ccstatusline-rs-v0.1.0-x86_64-unknown-linux-gnu/ccstatusline-rs install
```

```powershell
# Windows
Expand-Archive .\ccstatusline-rs-v0.1.0-x86_64-pc-windows-msvc.zip -DestinationPath .
.\ccstatusline-rs-v0.1.0-x86_64-pc-windows-msvc\ccstatusline-rs.exe install
```

`cargo install --git https://github.com/jaewon-nm/cc-statusline-rust ccstatusline-rs` also lands the binary; follow with `ccstatusline-rs install` to wire it.

---

## Manual / zero-trust install

For users who don't want a binary to touch their `~/.claude/settings.json` automatically. Same end state, three manual steps.

1. **Place the binary.** Drop `ccstatusline-rs(.exe)` somewhere stable: `~/bin` on Windows, `~/.local/bin` on POSIX, or anywhere on PATH.

2. **Write the Windows `.mjs` wrapper.** Skip this step on Linux/macOS — Claude Code accepts a bare absolute binary path there. On Windows, create `<bin-dir>/ccstatusline-rs.mjs`:

   ```js
   import { spawn } from 'node:child_process';
   const child = spawn("C:\\Users\\you\\bin\\ccstatusline-rs.exe", [], { stdio: ['inherit', 'inherit', 'inherit'] });
   child.on('exit', code => process.exit(code ?? 1));
   child.on('error', err => { console.error(err); process.exit(1); });
   ```

   Replace the path literal with your actual binary location (use `serde_json::to_string` semantics — backslashes doubled inside the JS string).

3. **Edit `~/.claude/settings.json`.** Add or replace the `statusLine` block. Back up first:

   ```jsonc
   {
     // …other keys preserved as-is…
     "statusLine": {
       "type": "command",
       // Windows:
       "command": "node \"C:\\\\Users\\\\you\\\\bin\\\\ccstatusline-rs.mjs\""
       // POSIX (single-quoted to handle paths with spaces):
       // "command": "'/home/you/.local/bin/ccstatusline-rs'"
     }
   }
   ```

   Save a timestamped copy of `settings.json` somewhere before editing — `uninstall` won't find a backup it didn't create.

Restart Claude Code for the change to take effect.

---

## Uninstall / restore

```powershell
./target/release/ccstatusline-rs uninstall
```

Reverts the **most recent** install by restoring the latest `settings.json.ccstatusline-rs-bak-*` backup. Atomic — the rewrite uses the same temp + rename pattern as install, so a concurrent renderer never reads a partial file.

```jsonc
{
  "uninstalled": true,
  "settings": "C:\\Users\\you\\.claude\\settings.json",
  "restored_from": "C:\\Users\\you\\.claude\\settings.json.ccstatusline-rs-bak-20260514-130000-000",
  "removed": []
}
```

Flags:

| Flag | Effect |
|---|---|
| `--backup <path>` | Restore a specific backup (any earlier install snapshot). |
| `--bin-dir <path>` | Override bin dir; only consulted with `--purge-binary`. |
| `--purge-binary` | Also delete `<bin-dir>/ccstatusline-rs(.exe)` and `<bin-dir>/ccstatusline-rs.mjs`. Other files in the dir are untouched. |
| `--pretty` | Indented JSON output. |

If you installed manually, restore your hand-saved backup yourself. `uninstall` only knows about its own backup filename convention.

---

## Wire it into Claude Code manually (after install or manual setup)

`install` writes `statusLine` directly. If you already have a custom config and just want to know what it ends up looking like:

```jsonc
// Windows
{
  "statusLine": {
    "type": "command",
    "command": "node \"C:\\\\Users\\\\you\\\\bin\\\\ccstatusline-rs.mjs\""
  }
}
```

```jsonc
// POSIX
{
  "statusLine": {
    "type": "command",
    "command": "'/home/you/.local/bin/ccstatusline-rs'"
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

Step 2 — tell Claude Code to surface color. Add an env var to the statusline command in `~/.claude/settings.json`:

```jsonc
{
  "statusLine": {
    "type": "command",
    "command": "node \"...\\ccstatusline-rs.mjs\"",
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

- **statusline blank after install on Windows.** Make sure you fully closed every Claude Code window before reopening — only a cold start reloads `settings.json`.
- **Empty status line in renderer.** Most likely the payload lacks `cwd` / `model` / `context_window`. Run the binary by hand: `cat payload.json | ccstatusline-rs` and toggle widgets via `config remove`.
- **`git` widgets blank.** The binary skips git when `cwd` is outside a repo or when probing exceeds an 800 ms wall clock. Confirm with `cd <cwd> && git status` — if that works in <800 ms, file an issue with a profile.
- **Stale numbers.** The JSONL probe and the git probe both cache to disk. The git cache TTL is 2 s; JSONL cache invalidates on `(mtime, size)` change. Delete `~/.cache/ccstatusline-rs/` (Linux/macOS) or `%LOCALAPPDATA%\dev\naya\ccstatusline-rs\cache\` (Windows) to clear.
- **Locale shows commas in numbers.** It shouldn't — formatters are locale-independent. If you see commas, file an issue with your OS / Windows locale.
- **`statusLine` keeps getting overwritten by another tool (neo-mem).** Re-run `ccstatusline-rs install` — starting in v0.1.3 it detects `tokenwatch-statusline.mjs` and routes through wrap mode (`~/.claude/.tw-statusline-prev.json`) instead of overwriting `statusLine`. See "Coexistence with neo-mem tokenwatch" above. Plugins that wrap us through a different basename are not auto-detected — disable that plugin's statusline auto-install setting or use wrap mode manually.
