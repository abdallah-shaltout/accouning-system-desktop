---
id: BUG-0007
kind: bug
status: open
area: full-persona-pass
fingerprint: e2e:full-persona-pass:assertion-failure
first_seen: 2026-09-26
last_seen: 2026-09-26
occurrences: 1
---

## full-persona-pass flow failed an assertion

المصدر: e2e flow `full-persona-pass`

A `check(...)` in `scripts/e2e/flows/full_persona_pass.py` failed — see the run's console output for which one. No structured error was exported for this run (see `.diagnostics\runs\20260926T210814/full-persona-pass.jsonl` if present).
