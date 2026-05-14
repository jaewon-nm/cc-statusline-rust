# Prerequisites

## Required

| Tool | Version | Why |
|---|---|---|
| Rust toolchain | **1.94** (pinned, see `CLAUDE.md` → Rust Development Rules) | Compile / test / build |
| `cargo-nextest` | latest | Test runner |
| `cargo-llvm-cov` | latest | Coverage (Windows-compatible) |

Install once:

```powershell
rustup toolchain install 1.94 --profile minimal
rustup default 1.94
cargo install cargo-nextest cargo-llvm-cov
```

## Optional (only needed at runtime for specific widgets)

| Tool | Used by | Notes |
|---|---|---|
| `git` | Git widgets (branch, dirty status, file counts) | Must be on `PATH`. Renderer shells out — no `git2` linkage. |
| `gh` | GitHub PR/MR widget | Auth via `gh auth login`. Only invoked when widget enabled. |
| `glab` | GitLab PR/MR widget | Same as above. |

## Known gotchas

- **Windows long paths.** Some Claude Code workspaces produce >260 char cwd. Enable Windows long path support if you see truncation:
  ```powershell
  reg add HKLM\SYSTEM\CurrentControlSet\Control\FileSystem /v LongPathsEnabled /t REG_DWORD /d 1
  ```
- **Locale-dependent number formatting.** Renderer must emit `.` as decimal separator regardless of system locale (default theme spec). Don't rely on `format!`'s locale; `jiff`/`std::fmt` are locale-independent already, but verify on Korean Windows.
- **Terminal color detection.** `anstyle-query` reads `NO_COLOR`, `CLICOLOR_FORCE`, `FORCE_COLOR`, and whether stdout is a TTY. Claude Code runs the renderer with stdout piped, so default-no-color is fine; color must be explicitly opted in via config.
