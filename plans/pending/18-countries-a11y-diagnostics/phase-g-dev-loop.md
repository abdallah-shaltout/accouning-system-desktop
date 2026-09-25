# 18.G — Close the dev loop

- [ ] `scripts/e2e/run.py`: after each flow, pull `window.__equal.diag.export()` into `.diagnostics/runs/<ts>/<flow>.jsonl`. Console errors and failures create or refresh ledger issues (by fingerprint).
- [ ] Per-flow perf numbers are compared to `docs/diagnostics/perf-baseline.json`. More than 25% slower → a `PERF-` issue. `--update-baseline` flag.
- [ ] `verify:mocks` failures → `ACC-` issues with the failing invariant and its case.
- [ ] Bundle size (vite build output) tracked the same way.
- [ ] `scripts/memory`: a new **"Open diagnostics"** section in `AGENT_MEMORY.md` (open issues by kind and area, with debug namespaces), so agents see known failures before starting work (config in `scripts/memory/config.ts`).
- [ ] Definition of done gains `bun run diag:check`.

