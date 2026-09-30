---
id: ACC-0005
kind: accounting
status: verified
area: purchases
first_seen: 2026-09-28
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0005-non-stock-receipt-inventory.json
---

## استلام صنف غير مخزني يُحمَّل على حساب المخزون (G-31b)

Receiving a non-stock product (`type: 'product'` with `stockMode: 'none'`) debited `inventory`, but
`applyStockChange` ignores non-stock items, so no stock value was added. `GL(inventory)` then
exceeded `Σ product.stockValue` by the line's value and the `inventory-gl` invariant failed.
`receivePurchase` (`src/mocks/backend/purchases.ts`) only sent `type === 'service'` lines to the
expense path. The Rust port (`receive_purchase`, plan 21 part 03, 07 Q-U2) had copied it verbatim.

**Rule now (docs/v2/02-accounting-review.md E1–E4, "Line accounts"):** a line that is not
stock-tracked (a service, or a product with `stockMode: 'none'`) takes the service-line path. It is
debited to its purchase account through the existing `purchaseLineAccountId` / `purchase_line_account`
resolution: the product's `purchaseAccountId`, then the category's, then
`settings.accounting.defaultPurchaseAccountId`, then the `freightIn` role. It never touches
`inventory`, and like a service line it takes no landed-cost share. No new system role was added.
The Rust side uses `LockedProduct::is_untracked()`, which is the same test `applyStockChange` uses.

### خطوات إعادة الإنتاج

1. A supplier and one product with `stockMode: 'none'`, cost 10.
2. Save a purchase order with `confirm: true` for 3 units (0% tax).
3. Old code: the receipt posts `Dr inventory 30 / Cr payable 30` and `inventory-gl` fails
   (`30 vs 0`). New code: `Dr freightIn 30 / Cr payable 30` (the fallback purchase account), and
   every invariant stays green.

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0005-non-stock-receipt-inventory.json`
(one step: `purchases.savePurchaseOrder`); on the old code the step breaks `inventory-gl`.

### ما جُرِّب ولم ينجح

- Letting non-stock goods keep their landed-cost share and expensing it with the line: it widens
  the fix past "route them like service lines" (07 O-U3) and changes how landed costs are spread
  across the receipt. Not needed for the invariant.

### الملفات ذات الصلة

- `src/mocks/backend/purchases.ts` — `receivePurchase`, `purchaseLineAccountId`.
- `src-tauri/src/domains/purchases/service/receive.rs` — `receive_purchase` (step 5 `is_service`).
- `src-tauri/src/domains/purchases/service/mod.rs` — `purchase_line_account`.
- `src-tauri/tests/domain_purchases.rs` — `non_stock_product_receipt_debits_purchase_account_not_inventory`.
- Still open, not part of this fix: a received non-stock item can never be returned (07 Q-U7: the
  return checks stock on hand).
