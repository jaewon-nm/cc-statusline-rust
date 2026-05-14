# cc-statusline-rust Documentation Index

## Structure

- `design-docs/` — product design (architecture, surface contracts, theme spec)
- `exec-plans/active/` — in-flight implementation plans
- `exec-plans/completed/` — finished plans (kept as history)
- `researches/` — research notes, comparisons, prior-art analysis
- `reviews/` — code review notes (per phase / per PR)
- `profiling/` — perf measurements (renderer cold start, fixture rendering)
- `GOVERNANCE.md` — doc workflow rules
- `POST-WORK-AUDIT.md` — post-work documentation hygiene checklist
- `BUILD-AND-DIST.md` — build, cross-compile, release / distribution flow
- `CLI-TESTING.md` — testing reference (binary name, fixtures, snapshot tests, common pitfalls)
- `PREREQUISITES.md` — required toolchain / optional tools

## Documents

| Category | Document | Status |
|---|---|---|
| design-docs | [cc-statusline-rs.md](design-docs/cc-statusline-rs.md) | draft |
| design-docs | [default-theme.md](design-docs/default-theme.md) | locked |
| exec-plans | [completed/000-bootstrap.md](exec-plans/completed/000-bootstrap.md) | ✅ completed (2026-05-14) |
| exec-plans | [completed/001-probes.md](exec-plans/completed/001-probes.md) | ✅ completed (2026-05-14) |
| exec-plans | [completed/002-config-persistence-and-jsonl-cache.md](exec-plans/completed/002-config-persistence-and-jsonl-cache.md) | ✅ completed (2026-05-14) |
| exec-plans | [completed/003-preview-diff-and-color.md](exec-plans/completed/003-preview-diff-and-color.md) | ✅ completed (2026-05-14) |
| exec-plans | [completed/004-distribution.md](exec-plans/completed/004-distribution.md) | ✅ completed (2026-05-14) |
| exec-plans | [completed/005-install-uninstall.md](exec-plans/completed/005-install-uninstall.md) | ✅ completed (2026-05-14) |
| exec-plans | [completed/006-default-theme-color.md](exec-plans/completed/006-default-theme-color.md) | ✅ completed (2026-05-14) |
| researches | _(none yet)_ | — |

## Reference source (read-only)

- [`../references/ccstatusline/`](../references/ccstatusline/) — upstream TypeScript/Bun implementation, git-cloned for porting reference. **Do not modify.**
