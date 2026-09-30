---
id: ACC-0023
kind: accounting
status: fixed
area: setup
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0023-opening-equity-mid-wizard.json
---

## `opening-balance-equity` يُفحص أثناء معالج الإعداد قبل إقفال 3900

A false alarm, reported by parity lane L4. §4.9 says "3900 = 0 **once onboarding is complete**", but
the check ran at every step. While the setup wizard is running, opening entries and opening stock
credit 3900 until the wizard re-closes it (`recloseOpeningBalanceEquity`). A before-go-live party
opening balance (docs/v2/05 §4) does the same. So the check failed mid-wizard.

**Rule now:** the check is not enforced while `settings.onboarding` exists without `finishedAt`
(`finished_at` in Rust), that is, while the wizard is still in progress. It then passes with the new
message `onboarding in progress — openingBalanceEquity (3900) not enforced yet` (identical in both
copies). A company with no onboarding record never ran the wizard (the demo seed, an import) and is
treated as complete, so `bun run verify:mocks` still enforces 3900 = 0 on the seed. Once
`finishOnboarding` stamps `finishedAt`, 3900 must be zero again.

### خطوات إعادة الإنتاج

1. ACC-0009 base with `settings.onboarding = { goLiveDate: 2026-09-23, completedStep: 7 }` (in
   progress).
2. Post opening stock (5 × 40), which credits 3900. The old check fails.
3. `recloseOpeningBalanceEquity` into capital, then `finishOnboarding`. 3900 is 0, and the check is
   enforced and passes.

### الملفات ذات الصلة

- `src/mocks/backend/invariants.ts` — `checkOpeningBalanceEquity`.
- `src-tauri/src/shared/invariants/documents.rs` — `check_opening_balance_equity`.
- `src/modules/setup/services/setupService.ts` (`finishOnboarding`), `src/mocks/backend/opening.ts`.
- `src-tauri/tests/shared_invariants.rs` — `opening_equity_waits_for_onboarding_to_finish`; `build_scenario` now closes its opening stock out of 3900.
