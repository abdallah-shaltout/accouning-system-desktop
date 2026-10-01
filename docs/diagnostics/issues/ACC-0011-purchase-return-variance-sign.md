---
id: ACC-0011
kind: accounting
status: verified
area: purchases
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0011-purchase-return-variance-sign.json
---

## فرق المخزون في مرتجع المشتريات مُرحَّل بالإشارة المعكوسة فيُرفض القيد

A purchase return that removes more value at the purchase price than the stock is carried at (the
inventory-variance guard, docs/v2/02-accounting-review.md A1/A2) posted the difference on the wrong
side. `recordPurchaseReturn` (`src/mocks/backend/purchases.ts`) booked `variance > 0 → Dr
inventoryVariance` and `variance < 0 → Cr inventoryVariance`. The supplier's side of a return is
always qty × purchase price (Dr payable / refund account), and the inventory side is only what is on
the books (`valueOut`), so a positive variance is a missing CREDIT. The entry came out
`Dr payable 115 + Dr variance 40 = 155` against `Cr inventory 60 + Cr vatInput 15 = 75`, and
`postJournal` refused it ("القيد غير متوازن: المدين 155 ≠ الدائن 75"). Because the mock was not atomic
(ACC-0013), the refused return also left the return row, the stock move and `returnedAmount` behind
without a journal (`inventory-gl`, `vat-input` and `supplier-allocation` broke). The Rust port
(`record_purchase_return`, 07-purchases §3 U-9 step 7) had copied the same inverted rule. The negative
branch was inverted the same way.

**Rule now:** positive variance (the supplier owes back more than the removed stock is carried at) →
`Cr inventoryVariance` (a gain); negative variance (more value on the books than the price) →
`Dr inventoryVariance` (a loss). The return entry balances by construction.

### خطوات إعادة الإنتاج

1. A VAT-registered supplier and a stock product.
2. Buy 2 @ 50 (received), sell 1 (leaves at its 50 carrying value), buy 1 @ 10: on hand 2 units carried
   at 60.
3. Return both 50-units against the first order (credit to the supplier's account).
4. Old code: refused, `القيد غير متوازن: المدين 155 ≠ الدائن 75`, with the return row and stock move left
   behind. New code: `Dr payable 115 / Cr inventory 60 / Cr vatInput 15 / Cr inventoryVariance 40`,
   stock 0 / 0, every invariant green.

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0011-purchase-return-variance-sign.json`
(one step: `purchases.createPurchaseReturn`); on the old code it breaks at that step. Parity case
`purchases/p-p7-return-variance-guard` covers the same flow.

### ما جُرِّب ولم ينجح

- Nothing else was tried: the sign follows from the entry having to balance (payable side fixed at the
  purchase price, inventory side fixed at the carrying value).

### الملفات ذات الصلة

- `src/mocks/backend/purchases.ts` — `recordPurchaseReturn`.
- `src-tauri/src/domains/purchases/service/returns.rs` — `record_purchase_return` step 7.
- `src-tauri/tests/domain_purchases.rs` — `acc_0011_return_variance_is_credited_when_the_supplier_owes_more_than_the_stock_is_carried_at`,
  `return_variance_guard_when_stock_value_is_below_what_return_would_remove`.
