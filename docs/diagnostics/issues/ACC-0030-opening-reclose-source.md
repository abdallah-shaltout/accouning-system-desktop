---
id: ACC-0030
kind: accounting
status: fixed
area: setup
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0030-opening-reclose-source.json
---

## إعادة إقفال حساب الأرصدة الافتتاحية (3900) تعيد استخدام مصدر الإقفال الأول

The onboarding wizard closes 3900 (`openingBalanceEquity`) when it posts the opening entry, then
posts opening stock (which credits 3900 again) and re-closes it (`recloseOpeningBalanceEquity`). Both
closing entries were filed under the same source, `opening:onboarding-close`, and neither is reversed,
so `one-active-entry` saw two active entries for one document (parity lane L4, setup cases). Both
backends did the same (`closeOpeningBalanceEquity` / `close_opening_balance_equity`).

**Decision:** a `sourceRef` names ONE document with one active entry (docs/v2/02 §4.7); the re-close
is a separate posting (it closes a new 3900 balance), not a replacement of the first close. So the
posting is fixed, not the invariant: the first close keeps the fixed `onboarding-close` id
(`ONBOARDING_CLOSE_SOURCE_ID` in Rust — what imports map `'onboarding-close'` to), and each re-close
gets its own id (`uid('opening-close')` in the mock, a fresh `Id` in Rust). `one-active-entry` is
unchanged for this.

### خطوات إعادة الإنتاج

1. Opening entry (cash 20,000) posted and 3900 closed to capital (source `onboarding-close`).
2. Opening stock 10 × 40 posted (3900 −400).
3. Re-close → old code: a second `opening:onboarding-close` entry, `one-active-entry` fails.
   New code: the re-close has its own source; a second re-close is a no-op.

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0030-opening-reclose-source.json`.

### ما جُرِّب ولم ينجح

- Teaching `one-active-entry` to allow several `opening` CLOSING entries: it would also hide a real
  double posting of the same close.

### الملفات ذات الصلة

- `src/mocks/backend/opening.ts` — `closeSourceId`, `closeOpeningBalanceEquity`.
- `src-tauri/src/domains/setup/service/opening.rs` — `close_source_id`, `close_opening_balance_equity`.
- `src-tauri/tests/domain_setup.rs` — `reclose_after_3900_moves_again_uses_its_own_source`.
- Parity: `scripts/parity/cases/setup/setup-wizard-eg.ts` (reclose step).
