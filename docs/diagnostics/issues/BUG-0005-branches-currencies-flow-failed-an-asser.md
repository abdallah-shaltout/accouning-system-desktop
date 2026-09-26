---
id: BUG-0005
kind: bug
status: open
area: branches-currencies
fingerprint: e2e:branches-currencies:assertion-failure
first_seen: 2026-09-26
last_seen: 2026-09-26
occurrences: 2
---

## branches-currencies flow failed an assertion

المصدر: e2e flow `branches-currencies`

A `check(...)` in `scripts/e2e/flows/branches_currencies.py` failed — see the run's console output for which one. No structured error was exported for this run (see `.diagnostics\runs\20260926T210814/branches-currencies.jsonl` if present).
