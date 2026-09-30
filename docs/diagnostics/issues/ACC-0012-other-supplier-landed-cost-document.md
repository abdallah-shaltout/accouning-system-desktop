---
id: ACC-0012
kind: accounting
status: fixed
area: purchases
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0012-other-supplier-landed-cost-document.json
---

## تكلفة إضافية يفوترها مورد آخر (شركة الشحن) تُرحَّل على حسابه دون مستند مفتوح

A landed-cost line billed by a different supplier (a shipper, a customs broker — docs/v2/09 §1,
review E4) posted its own `Cr payable` line tagged with that supplier, but nothing else existed for
it: no purchase order, no bill. So the shipper's `supplierBalance()` showed the amount while the sum
of its open documents was 0 — invariant `supplier-allocation` broke — and the amount could never be
settled through a payment allocation (`getOpenDocuments('supplier', shipper)` was empty). Mock and
Rust (`receive_purchase` step 10) behaved the same.

**Rule now:** each landed-cost line whose `supplierId` is not the order's own supplier becomes that
supplier's own payable document — a `RECEIVED` purchase order with no stock lines, `subTotal =
grandTotal = amount`, `taxAmount = 0`, dated the receipt date, same branch/cost center, note
`تكلفة إضافية "<label>" على أمر الشراء <number>`. Its number is drawn from the purchase-order
counter right after the parent's (before the backorder's and the journal's). The GL is unchanged: the
cost stays inside the receipt's inventory posting and the shipper's `Cr payable` line stays in the
receipt's one entry, now described as `تكلفة إضافية "<label>" — <bill number>` so the supplier
statement names the document. Receiving refuses (`مورد التكلفة الإضافية غير موجود`) when that
supplier does not exist.

Design choice: a purchase order is the existing supplier-billed document every AP reader already
understands — open documents, payment allocation (`targetKind: 'purchaseOrder'`), `paidAmount`/
`paymentStatus`, `purchaseOutstanding`, the supplier statement and both `supplier-allocation`
implementations — so no new table, DTO, IPC command or allocation kind was needed. An expense on
account (`paidFrom: supplier`) was rejected: it is not an open document either.

### خطوات إعادة الإنتاج

1. A supplier, a shipping company (another supplier) and a stock product.
2. Save a purchase order with `confirm: true`, a landed cost for the order's own supplier and a
   landed cost of 77.77 billed by the shipping company.
3. Old code: the shipper's balance is 77.77 with no open document → `supplier-allocation` fails. New
   code: the shipper has one open `RECEIVED` document of 77.77, the invariant holds.

`bun run verify:replay` replays this as
`scripts/verify/cases/ACC-0012-other-supplier-landed-cost-document.json` (one step:
`purchases.savePurchaseOrder`); on the old code it breaks at that step. Parity case
`purchases/p-p4-other-supplier-landed` covers balance, open documents and statement.

### ما جُرِّب ولم ينجح

- A separate journal entry per shipper bill (`Dr inventory / Cr payable`, sourced to the bill): it would
  split one receipt's inventory debit across two entries and break "one posting per receipt", so the
  shipper line stays in the receipt's entry.
- A link field on `LandedCostLine` pointing at the bill: rejected — a DTO/bindings change, and a
  receipt's own landed costs are not stored when the order already has some (07-purchases Q-U4), so
  the link would be unreliable. The bill's note and the journal line name each other instead.

### الملفات ذات الصلة

- `src/mocks/backend/purchases.ts` — `allocateLandedCosts`, `planReceipt`, `newOtherSupplierBill`, `applyReceipt`.
- `src-tauri/src/domains/purchases/service/receive.rs` — `allocate_landed_costs`, `receive_purchase` steps 6b, 10, 14b.
- `src-tauri/tests/domain_purchases.rs` — `acc_0012_other_supplier_landed_cost_becomes_an_open_payable_document`,
  `acc_0012_other_supplier_landed_cost_refused_when_that_supplier_does_not_exist`,
  `landed_costs_own_and_other_supplier_split_by_value`.
