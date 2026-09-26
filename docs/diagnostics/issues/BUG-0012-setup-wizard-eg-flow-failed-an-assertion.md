---
id: BUG-0012
kind: bug
status: open
area: setup-wizard-eg
fingerprint: e2e:setup-wizard-eg:assertion-failure
first_seen: 2026-09-26
last_seen: 2026-09-26
occurrences: 3
---

## setup-wizard-eg flow failed an assertion

المصدر: e2e flow `setup-wizard-eg`

A `check(...)` in `scripts/e2e/flows/setup_wizard_eg.py` failed — see the run's console output for which one. No structured error was exported for this run (see `.diagnostics\runs\20260926T220818/setup-wizard-eg.jsonl` if present).
