# 18.G — Close the dev loop

**Status (2026-09-26): implemented and verified, one real bug found and logged (not fixed — out of
scope).** Every task below is done. `scripts/e2e/common.py` gained `export_diagnostics()`/`finish()`;
all 19 flows now call `finish(page, browser, "<area>")` in place of a bare `browser.close()`.
`scripts/e2e/run.py` archives each flow's diagnostics export, times it against
`docs/diagnostics/perf-baseline.json` (created and seeded this pass), and turns failures/regressions
into ledger issues through `scripts/diagnostics/run.ts --ingest` — the same upsert mechanism (by
fingerprint, reopens `fixed`/`wontfix`, leaves `verified` alone) B7's fingerprint auto-stubbing
already used, extended with an `IngestFinding`/`upsertIssue` API rather than duplicated.
`scripts/verify/run.ts` calls the same `--ingest` path for `verify:mocks` failures (`ACC-`, `--no-ledger`
to skip). Bundle size (`dist/` total bytes) is tracked the same way at a 15% threshold (documented next
to the 25% perf threshold in `scripts/e2e/run.py` — bundle size is far less noisy than wall-clock
timing, so a tighter bound still avoids routine-growth noise). `scripts/memory/analyze/diagnostics.ts`
(new) + a `openDiagnostics` render section give `AGENT_MEMORY.md` an "Open diagnostics" section (by
kind, by area, debug namespaces, full table), right after "Where to find X" in the read order.
`CLAUDE.md`'s Definition of done gained `bun run diag:check`, plus a "18.G" note under Diagnostics
describing the ingest mechanism and the perf baseline.

**Verified end to end** (not just unit-tested): (1) `bun run scripts/diagnostics/run.ts --ingest
<file>` tested standalone — creates, refreshes (bumps `occurrences`), and correctly leaves a
`verified` issue alone. (2) `verify:mocks` tested with one invariant temporarily forced to fail —
produced an `ACC-000x` issue with the failing message and seed, then reverted; a clean rerun is
98 ok / 0 todo / 0 failed with no ledger writes. (3) `scripts/e2e/run.py --only phone-input` run
several times, including with a deliberately broken assertion — produced the expected `BUG-`/`PERF-`
issues via the real dev server, then reverted; a real run of the full suite with `--update-baseline`
found a **genuine pre-existing bug**: `onboarding.py` step 2 hits a Playwright strict-mode selector
ambiguity (`get_by_label("الدولة")` now matches both the country `<select>` and an unrelated switch
whose long accessible name contains the same substring) that used to crash the whole `run.py` process
with a raw traceback. `run.py` now catches any flow exception, logs a `BUG-` issue for the crash via
the same ingest path, marks that one flow failed, and continues — confirmed by two full runs. The
underlying selector ambiguity itself is a pre-existing wizard/e2e issue outside this phase's scope
(dev-loop tooling, not wizard UI) and is logged in `TODO.md` and `docs/diagnostics/issues/BUG-0001-*`
for whoever owns that step next, per the "report, don't fix" rule for out-of-scope issues found
incidentally. `bun run build` and `bun run check` both pass; `docs/diagnostics/perf-baseline.json` is
seeded from the flows that did complete.

- [x] `scripts/e2e/run.py`: after each flow, pull `window.__equal.diag.export()` into `.diagnostics/runs/<ts>/<flow>.jsonl`. Console errors and failures create or refresh ledger issues (by fingerprint).
- [x] Per-flow perf numbers are compared to `docs/diagnostics/perf-baseline.json`. More than 25% slower → a `PERF-` issue. `--update-baseline` flag.
- [x] `verify:mocks` failures → `ACC-` issues with the failing invariant and its case.
- [x] Bundle size (vite build output) tracked the same way.
- [x] `scripts/memory`: a new **"Open diagnostics"** section in `AGENT_MEMORY.md` (open issues by kind and area, with debug namespaces), so agents see known failures before starting work (config in `scripts/memory/config.ts`).
- [x] Definition of done gains `bun run diag:check`.

