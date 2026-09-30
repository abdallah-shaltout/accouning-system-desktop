---
id: BUG-0020
kind: bug
status: fixed
area: purchases
first_seen: 2026-09-30
last_seen: 2026-09-30
occurrences: 1
---

## `getPurchaseOrder().supplier.balance` returned the supplier row's stale stored `balance`

المصدر: plan 21 Part 04 Wave 2 parity run (lane L2), every `purchases/*` case that reads a purchase
detail — `steps.detail.value.supplier.balance`: mock `0`, Rust `9514.58` (and `unallocatedCredit`
present only on Rust).

### خطوات إعادة الإنتاج

1. On `demo-sa`, open any purchase order of `sup-1` (`getPurchaseOrder(id)`).
2. `detail.supplier.balance` is `0` — the seed fixture's stored field — while
   `getSupplier('sup-1').balance` is the real payable balance (`9514.58`), and
   `detail.supplier.unallocatedCredit` is missing.

The mock's `purchaseService.getPurchaseOrder` returned `clone(supplier)` (the raw `db.suppliers`
row) instead of the computed supplier every other reader returns (`partyService`'s
`withComputedSupplier`). The Rust backend reads it through `parties::service::read::get_supplier`
(07-purchases `PurchaseDetail.supplier`), so Rust was already right.

Not an accounting posting error — no journal line, balance or invariant is affected; only the DTO
field is stale (no screen reads `detail.supplier.balance` today). Hence `BUG-`, not `ACC-`.

### الإصلاح

`src/modules/purchases/services/purchaseService.ts` — `supplier` is now
`{ ...clone(supplier), balance: supplierBalance(id), unallocatedCredit: unallocatedCreditFor('supplier', id) }`,
the same shape `getSupplier` returns. Regression: the `purchases/*` parity cases (`detail` steps).

### ما جُرِّب ولم ينجح

(لا شيء.)

### الملفات ذات الصلة

- `src/modules/purchases/services/purchaseService.ts` (`getPurchaseOrder`)
- `src/modules/parties/services/partyService.ts` (`withComputedSupplier`)
- `src-tauri/src/domains/purchases/service/read.rs` (detail → `get_supplier`)
