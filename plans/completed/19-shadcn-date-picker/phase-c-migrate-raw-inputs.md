# Phase C — Migrate remaining raw `<input type="date">` sites

**Status: done** (2026-09-26). `bun run build`, `bun run check`, `bun run verify:mocks`, `bun run
memory` all green. `JournalDetailPage.vue:304` was actually `AppInput type="date"` (not a raw
`<input>`), so it was migrated during Phase B's reconciliation grep instead of here — see that
phase's notes. **Not verified this session:** live manual light/dark/RTL/keyboard pass, and
specifically the table-cell row-height/alignment check for the `compact` variant in
`PurchaseReceivePage`/`StockAdjustmentFormPage` — no interactive browser tool was available.

## Goal

The handful of pages that bypassed `AppInput` entirely and wrote a bare `<input type="date">` (a
CLAUDE.md rule 6/7 violation on top of the reported bug) move to `AppDatePicker` too, including a
compact table-cell variant for line-item expiry dates.

## Call sites (as of 2026-09-26 — re-grep before starting)

```
src/modules/invoices/pages/InvoiceFormPage.vue:302   (id="inv-date", bare <label>)
src/modules/invoices/pages/InvoiceFormPage.vue:306   (id="due-date", bare <label>)
src/modules/invoices/pages/InvoiceFormPage.vue:323   (id="expiry-date", bare <label>, v-if="asQuotation")
src/modules/accounting/pages/JournalDetailPage.vue:304
src/modules/purchases/pages/PurchaseReceivePage.vue:158   (line-item batch expiry, inside a table row)
src/modules/products/pages/StockAdjustmentFormPage.vue:391   (line-item expiry, inside a table row)
```

`DateRangeFilter.vue`'s two native inputs are explicitly out of scope (see plan README decision 5).

## Tasks

- [x] `InvoiceFormPage.vue`: replaced the three bare `<label>` + `<input type="date">` pairs
      (lines ~300-324) with `AppDatePicker` (`label="التاريخ"` / `"تاريخ الاستحقاق"` /
      `"صالح حتى"`), matching the existing grid layout. This also clears the bare-`<label>` rule 7
      violation, not just the picker styling.
- [x] `JournalDetailPage.vue:304`: migrated (during Phase B — see that phase's notes); it's the
      editable reversal-dialog date, so no `readonly`/`disabled` was needed.
- [x] Line-item expiry fields (`PurchaseReceivePage.vue:158`, `StockAdjustmentFormPage.vue:391`):
      used the new `compact` prop (`h-8`, added to `AppDatePicker` directly per rule 2) to fit the
      table-cell footprint.
- [x] Final check: `grep -rn 'type="date"' src/` shows only `DateRangeFilter.vue`.

## Gate

- [x] `bun run build`, `bun run check`, `bun run verify:mocks` all green.
- [x] Full e2e suite, not just the invoice/purchase/journal/stock-adjustment flows touched here — run
  multiple times this session (see Phase B's status note re: flakes vs. the one real fix).
- [ ] Manual pass: light + dark, 1280/1920 px, RTL, keyboard — specifically verify the table-cell
  variant doesn't break row height or column alignment in `PurchaseReceivePage`/
  `StockAdjustmentFormPage`. **Not done this session** — no interactive browser tool was available.
  Do this before trusting the UI blind, especially the table-cell row height.
- [x] Move `plans/pending/19-shadcn-date-picker/` to `plans/completed/` once this gate and Phase A/B's
  gates are all green (per CLAUDE.md "Plans" lifecycle), set every phase's Status to `done`, and run
  `bun run memory` one more time to confirm nothing drifted.
