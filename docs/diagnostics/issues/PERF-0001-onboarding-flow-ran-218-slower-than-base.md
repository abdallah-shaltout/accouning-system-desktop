---
id: PERF-0001
kind: perf
status: open
area: onboarding
fingerprint: perf:onboarding:duration
first_seen: 2026-09-26
last_seen: 2026-09-26
occurrences: 8
---

## onboarding flow ran 218% slower than baseline

Baseline: 4777 ms. Actual: 15189 ms. Threshold: 25%.

المصدر: `scripts/e2e/flows/onboarding.py`

لتحديث الأساس بعد تحسين متعمد: `python scripts/e2e/run.py --update-baseline`.
