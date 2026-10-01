---
id: ACC-0013
kind: accounting
status: verified
area: purchases
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0013-purchases-atomic-refusal.json
---

## رفض الترحيل في المشتريات يترك تغييرات جزئية (مخزون وصفوف بلا قيد)

The mock purchases write paths mutated state before the step that could still refuse. Receiving
(`receivePurchase`) moved stock, filled `receivedQty`, set the order `RECEIVED` and shrank its totals
before `postJournal`; save-and-confirm (`savePurchase` with `confirm`) also pushed the order (and took
a document number) first; a return (`recordPurchaseReturn`) pushed the return row, raised
`returnedAmount` and moved stock first. When the posting was then refused — a closed fiscal year or a
lock date, an unbalanced entry (ACC-0011), an unresolvable account — the refusal left all of that
behind without its journal, so `inventory-gl`, `vat-input` and `supplier-allocation` broke. A refused
short-delivery backorder (`savePurchase` validation) could also leave a receipt posted without its
backorder. The Rust backend already rolls the whole command back in one transaction.

**Rule now:** every purchases write that posts validates everything that can refuse BEFORE it
mutates anything. `planReceipt` (status/qty guards, missing other-supplier, the posting's period and
balance check through `preflightJournal`, and the backorder's own validation) is pure;
`applyReceipt` only writes. `savePurchase` with `confirm` plans the receipt on the order as it will
be saved before saving it or drawing its number. `recordPurchaseReturn` plans its stock moves against
a running per-product qty/value and calls `preflightJournal` before drawing the return number or
writing anything. `preflightJournal` (`src/mocks/backend/core.ts`) is the one copy of `postJournal`'s
refusals (period, account resolution, balance, ≥ 2 lines); `postJournal` itself now calls it.

### خطوات إعادة الإنتاج

1. A received order (4 @ 25) and a draft order (3 @ 30) for the same stock product.
2. Close the fiscal year that covers the posting dates.
3. Receive the draft, save-and-confirm a new order, and return one unit of the received order — each
   is refused.
4. Old code: every refusal leaves stock/rows changed without a journal (`inventory-gl`, `vat-input`,
   `supplier-allocation` fail). New code: nothing changes, every invariant green.

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0013-purchases-atomic-refusal.json`
(three steps, each a recorded refusal); on the old code it breaks at the first.

### ما جُرِّب ولم ينجح

- Snapshot-and-restore of the whole mock DB around each write: correct, but it hides the order of
  checks instead of making it explicit, and it would also roll back unrelated state (activity feed,
  persistence scheduling). Validate-first keeps the mock's refusal order equal to the Rust command's.

### الملفات ذات الصلة

- `src/mocks/backend/core.ts` — `preflightJournal`, `postJournal`.
- `src/mocks/backend/purchases.ts` — `savePurchase`, `planReceipt`, `applyReceipt`, `receivePurchase`, `recordPurchaseReturn`.
- `src-tauri/tests/domain_purchases.rs` — `acc_0013_purchase_writes_refused_by_a_closed_period_leave_no_trace`.
