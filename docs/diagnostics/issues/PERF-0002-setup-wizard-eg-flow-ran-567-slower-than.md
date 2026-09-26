---
id: PERF-0002
kind: perf
status: open
area: setup-wizard-eg
fingerprint: perf:setup-wizard-eg:duration
first_seen: 2026-09-26
last_seen: 2026-09-26
occurrences: 1
---

## setup-wizard-eg flow ran 567% slower than baseline

Baseline: 4039 ms. Actual: 26929 ms. Threshold: 25%.

المصدر: `scripts/e2e/flows/setup_wizard_eg.py`

لتحديث الأساس بعد تحسين متعمد: `python scripts/e2e/run.py --update-baseline`.
