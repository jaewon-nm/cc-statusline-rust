# Documentation Governance

> Governance rules that apply to all implementation work (code, schema, UI, IPC, behavior changes).
> Triggered by the "Documentation Governance (Mandatory)" section in CLAUDE.md.

---

## Scope

These rules apply to **all implementation work involving code, DB schema, UI, IPC channels, or behavior changes**.
Pure refactoring (no behavior change) or documentation-only changes are excluded.

---

## Documentation Workflow

### 1. Before Implementation

1. Read `docs/INDEX.md` to identify relevant documents
2. Reference the relevant `design-docs/` or `product-specs/` to understand design intent
3. Select the corresponding `exec-plans/active/*.md`
   - If none exists, create a new plan file under `exec-plans/active/`
   - File naming format: `{short-description}.md`
4. **Include a test plan section in the plan**
   - Unit test scope per feature (happy path, edge cases, error handling)
   - If cross-module interactions exist, specify integration test targets
   - If user interaction flows exist, specify E2E test scenarios

### 2. During Implementation

- **Immediately** update the exec-plan checklist (check off completed items)
- If implementation deviates from the design, add a **Deviation** section to the plan with explanation

### 3. After Implementation

1. Verify test results (per CLAUDE.md Testing Policy)
2. Write the **Completion section** in the exec-plan (see checklist below)
3. `git mv docs/exec-plans/active/{plan}.md docs/exec-plans/completed/{plan}.md`
4. Update `docs/STATUS.md`

---

## Definition of Done (Including Documentation)

An implementation task is considered "done" only when all of the following are met:

- [ ] Code implementation complete
- [ ] Tests passing (unit + E2E)
- [ ] Exec-plan checklist 100% complete + completion section written
- [ ] Exec-plan moved from `active/` to `completed/`
- [ ] `STATUS.md` updated

---

## Exec-Plan Completion Checklist

Verify the following before moving a plan to `completed/`:

### Pre-checks

- [ ] Plan objectives match actual implementation scope
- [ ] No unchecked items (`- [ ]`) remain
  - Intentionally skipped items should be ~~struck through~~ with a reason noted

### Completion Section (5 items)

Add the following completion section at the bottom of the plan file:

```markdown
---

## Completion

- **Date**: YYYY-MM-DD
- **Summary**: 1-3 line summary of key changes
- **Changed files**: List of major changed files (paths)
- **Verification**: Test commands executed and result summary
- **Follow-up**: Additional work derived from this plan ("None" if none)
```

### Execution

1. `git mv docs/exec-plans/active/{plan}.md docs/exec-plans/completed/{plan}.md`
2. Verify internal relative links are not broken (note the path goes one level deeper)
3. Update `STATUS.md` (see procedure below)

---

## STATUS.md Update Procedure

STATUS.md uses **delta updates** by default. Do not rewrite the entire file — only modify the relevant parts.

### Fields to Update

1. **Header date**: Change the date at the top of the document to today's date
2. **Row status classification**:
   - `A` — Implemented as planned
   - `B` — Feature exists but design/structure changed
   - `C` — Not implemented (needed)
   - `D` — Dropped / unnecessary
3. **New features**: Add a row in the appropriate section
4. Fix any `active/` to `completed/` path references if present

---

## Operating Principles

1. **STATUS uses delta updates** — Do not touch rows unrelated to the current work
2. **No doc update = incomplete work** — Committing code without updating documentation does not meet Definition of Done
3. **Plans are living documents** — Update them continuously during implementation. Do not write everything at the end
4. **Deviations are not bad** — It is fine to implement differently from the design, but it must be documented
