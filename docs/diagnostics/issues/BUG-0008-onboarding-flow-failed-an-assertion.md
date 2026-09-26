---
id: BUG-0008
kind: bug
status: open
area: onboarding
fingerprint: e2e:onboarding:assertion-failure
first_seen: 2026-09-26
last_seen: 2026-09-26
occurrences: 3
---

## onboarding flow failed an assertion

المصدر: e2e flow `onboarding`

A `check(...)` in `scripts/e2e/flows/onboarding.py` failed — see the run's console output for which one. No structured error was exported for this run (see `.diagnostics\runs\20260926T212017/onboarding.jsonl` if present).
