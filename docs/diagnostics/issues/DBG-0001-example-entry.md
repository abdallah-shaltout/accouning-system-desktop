---
id: DBG-0001
kind: debug
status: wontfix
area: diagnostics
first_seen: 2026-09-26
last_seen: 2026-09-26
occurrences: 0
debug_namespace: posting
---

## Example ledger entry

This is the ledger's own worked example (18.B7), kept as `wontfix` so it never shows as an open
issue. It shows the schema every other `docs/diagnostics/issues/*.md` file follows:

- YAML frontmatter with `id` / `kind` / `status` / `area` / `fingerprint?` / `first_seen` /
  `last_seen` / `occurrences` / `debug_namespace?` / `regression_test?` / `fixed_in?`.
- A body with repro steps, what was tried and failed, and the related files.

### خطوات إعادة الإنتاج

N/A — this entry documents the schema, not a real bug.

### ما جُرِّب ولم ينجح

N/A.

### الملفات ذات الصلة

- `scripts/diagnostics/run.ts` — regenerates `docs/diagnostics/ISSUES.md` from every file in this folder.
- `scripts/diagnostics/frontmatter.ts` — the frontmatter schema this file follows.
