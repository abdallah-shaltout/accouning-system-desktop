# Phase B — Migrate all `AppInput type="date"` call sites

**Status: done** (2026-09-26). `bun run build`, `bun run check`, `bun run verify:mocks`, `bun run
memory` all green. Full e2e suite was run several times; every flow that failed in a full-suite pass
also passed cleanly when re-run alone (confirmed sandbox/timing flakes consistent with this repo's
documented history — see commit "confirm all 16 e2e flows green after 4th sandbox dev-server flake"
— not a regression from this change). One real bug found and fixed during this phase's e2e pass: the
Phase A gallery's "فارغ" label collided (by substring) with the `phone_input.py` flow's
`get_by_label("فارغ")`; renamed to "بدون قيمة". **Not verified this session:** live manual light/
dark/RTL/keyboard pass on real forms — no interactive browser tool was available.
`JournalDetailPage.vue:304` turned out to already be `AppInput type="date"` (not a raw `<input>` as
Phase C's plan draft assumed) and was migrated here as part of reconciling the call-site grep.

## Goal

Every page currently rendering a native date popup through `AppInput type="date"` switches to
`AppDatePicker`, and `AppInput` loses the now-dead `'date'` branch.

## Call sites (as of 2026-09-26 — re-grep before starting; a concurrent change may add/remove some)

```
src/modules/expenses/pages/ExpenseFormPage.vue:111
src/modules/expenses/pages/RecurringExpensesPage.vue:155
src/modules/parties/pages/PartyFormPage.vue:477
src/modules/vouchers/pages/VoucherFormPage.vue:119
src/modules/vouchers/pages/CardSettlementPage.vue:121
src/modules/accounting/pages/JournalEntryFormPage.vue:411
src/modules/accounting/pages/FiscalYearsPage.vue:169,198,199
src/modules/reports/pages/BalanceSheetPage.vue:106
src/modules/purchases/pages/PurchaseReceivePage.vue:171
src/modules/purchases/pages/PurchaseFormPage.vue:271,367
src/modules/reports/pages/PeriodComparisonPage.vue:43,44,47,48
src/modules/setup/components/steps/StepFiscalYear.vue:26
src/modules/setup/components/steps/StepOpening.vue:305
src/modules/products/pages/StockAdjustmentFormPage.vue:299
src/modules/products/pages/ProductFormPage.vue:462
src/modules/settings/pages/CurrenciesSettingsPage.vue:209
src/modules/payments/pages/PaymentFormPage.vue:257
```

## Tasks

- [x] Re-run `grep -rn 'type="date"' src/` and reconcile against the list above before starting.
- [x] For each file: replace `<AppInput v-model="x" type="date" label="…" ... />` with
      `<AppDatePicker v-model="x" label="…" ... />`, carrying over `required`/`disabled`/`hint`/
      `class`/width utility classes unchanged. Drop any prop that was only meaningful for `type=
      "date"` on `AppInput` if `AppDatePicker` doesn't need it (e.g. `step`).
  - [x] `ProductFormPage.vue:462` is inside a `v-else-if="f.type === 'date'"` custom-field renderer —
        confirmed the surrounding branch still type-checks with the new component (the `f.type`
        string literal is unrelated to `AppInput`'s prop and didn't need to change).
- [x] Once every call site above is migrated, removed `'date'` from `AppInput`'s `type` prop union
      (`src/modules/core/components/ui/AppInput.vue`) and deleted the now-dead
      `type === 'date' ? 'ltr' : ...` branch in its `:dir` binding.
- [x] `grep -rn 'type="date"'` again — only showed `DateRangeFilter.vue` (out of scope, rule 5)
      and Phase C's raw-input list (handled in the same session, see that phase's file).

## Gate

- [x] `bun run build`, `bun run check`, `bun run verify:mocks` all green.
- [x] Full e2e suite (`python scripts/e2e/run.py`, not `--only`) — date fields sit on invoice,
  purchase, payment, voucher, journal, expense, fiscal-year, and setup flows. Ran multiple times;
  see status note above re: flakes vs. the one real fix.
- [ ] Manual pass: light + dark, 1280/1920 px, RTL, keyboard, on at least the invoice/purchase/payment
  forms (the highest-traffic ones) plus the setup wizard step. **Not done this session** — no
  interactive browser tool was available. Do this before trusting the UI blind.
