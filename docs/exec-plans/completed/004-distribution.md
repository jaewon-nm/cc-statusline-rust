# 004 — Distribution (M5)

**Status:** ✅ completed · **Owner:** jaewon_lee · **Opened:** 2026-05-14 · **Closed:** 2026-05-14

## Goal

Ship the infrastructure for tagged GitHub Releases plus the docs an agent or human needs to install the binary into Claude Code, without actually cutting a release yet. Tagging is the user's go-button.

## What landed

### Cargo package polish

- `Cargo.toml`:
  - `repository` and new `homepage` pointed at `https://github.com/jaewon-nm/cc-statusline-rust` (was upstream's URL by mistake).
  - `exclude` list strips `/references/`, `/docs/`, `/.github/`, `/.mcp.json`, `/CLAUDE.md` from `cargo package`, leaving only the runtime artifacts (`src/`, `tests/`, `Cargo.{toml,lock}`, `rust-toolchain.toml`, `README.md`, `INSTALL.md`, `CHANGELOG.md`).
  - Existing `[profile.release]` from M0 unchanged (`lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`).
- `cargo package --list` audited: 40 entries, no governance/research material leaked into the publish set.

### GitHub Actions

`.github/workflows/ci.yml`:

- Runs on every push / PR to `master`.
- Matrix: `ubuntu-latest`, `windows-latest`, `macos-latest`.
- Steps: install Rust 1.94 (pinned), `Swatinem/rust-cache@v2`, `taiki-e/install-action@nextest`, `cargo build --all-targets --locked`, `cargo nextest run --all-targets --locked --no-fail-fast`.
- Separate `lint` job: `cargo fmt --check` + `cargo clippy --all-targets --locked -- -D warnings`.

`.github/workflows/release.yml`:

- Triggers on tag push `v*.*.*`.
- Matrix of four targets:
  - `x86_64-pc-windows-msvc` → zip + sha256
  - `x86_64-unknown-linux-gnu` → tar.gz + sha256
  - `aarch64-apple-darwin` → tar.gz + sha256
  - `x86_64-apple-darwin` → tar.gz + sha256 (runs on `macos-13` to keep an Intel runner explicit)
- Each job builds `cargo build --release --target <triple>`, stages `README.md`/`INSTALL.md`/`LICENSE`, archives + computes SHA-256, uploads as a workflow artifact.
- Final `publish` job downloads all artifacts and creates the GitHub Release via `softprops/action-gh-release@v2` with `generate_release_notes: true`. Tags containing a `-` (e.g. `v0.1.0-rc.1`) flip `prerelease: true`.

### INSTALL.md

Three install paths documented: pre-built archive from GitHub Releases (with sha256 verify), `cargo install --git`, build-from-source. Plus:

- Claude Code wiring example (`statusLine.command` + Windows / POSIX variants).
- Color opt-in instructions — `config color` subcommand + `FORCE_COLOR=1` in the statusline `env` map.
- Configuration cheat sheet: `schema`, `widgets`, `config show / add / remove / color / apply`, `preview --diff`.
- Troubleshooting: empty line, blank git widgets (timeout), stale cache (paths to nuke), locale formatting.
- Binary size table — local Windows MSVC release: **1,910,272 bytes**; other triples to be back-filled from the first CI run.

### CHANGELOG.md

- `Keep a Changelog`-style format.
- `Unreleased` section captures the M5 scaffolding.
- A `0.1.0 — first taggable release (pending)` block summarizes M0–M4 with pointers into `docs/exec-plans/completed/`.
- Documents the SemVer contract: **public surface = CLI subcommand contract + config schema + default-theme output bytes**.

## Acceptance

- `cargo build --release` succeeds; binary `target/release/ccstatusline-rs.exe` measured at 1.9 MB on Windows MSVC.
- `cargo package --list --allow-dirty` produces a clean inventory: only `src/`, `tests/`, manifest, `rust-toolchain.toml`, and the four user-facing docs (`README.md`, `INSTALL.md`, `CHANGELOG.md`). No `docs/`, `.github/`, or governance leaks.
- `cargo nextest run --test-threads=1` still **123 / 123 passed**.
- `cargo clippy --all-targets -- -D warnings` clean.
- `cargo fmt --check` clean.
- Default-theme snapshot byte-identical (no code path changed).

## Out-of-scope (explicit)

- **No tag pushed.** The user controls when `v0.1.0` ships. The release workflow waits on `git push origin v0.1.0`.
- **No crates.io publish.** Once a tag is cut and the GitHub Release is live, `cargo publish` is a single command — defer until the first user beyond `jaewon-nm` exists.
- **No Homebrew tap.** Reasonable follow-up but not in scope here.
- **No code-signing for the Windows binary.** Requires a code-signing cert; track separately if Windows SmartScreen warnings show up in practice.

## Cutting `v0.1.0` (runbook for future-you)

```bash
# 1. Move the Unreleased entries into a 0.1.0 - YYYY-MM-DD heading in CHANGELOG.md
# 2. Commit the changelog tidy
# 3. Tag and push
git tag -a v0.1.0 -m "ccstatusline-rs 0.1.0"
git push origin v0.1.0
```

The release workflow then runs the matrix builds (~3–5 min) and publishes the GitHub Release automatically. Artifact filenames look like `ccstatusline-rs-v0.1.0-x86_64-pc-windows-msvc.zip`.

If a build job fails, the publish step is skipped — investigate locally with `cargo build --release --target <triple>` and re-tag once green (`git tag -d v0.1.0 && git push origin :v0.1.0 && git tag ... && git push ...`).
