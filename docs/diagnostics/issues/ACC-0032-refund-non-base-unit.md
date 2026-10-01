---
id: ACC-0032
kind: accounting
status: verified
area: invoices
first_seen: 2026-09-30
last_seen: 2026-09-30
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0032-refund-non-base-unit.json
---

## مرتجع سطر مبيع بوحدة غير الأساسية يُرجع وحدة أساسية واحدة بدل محتوى الوحدة

A sale line in a non-base unit (a box of 4, `unitFactor` 4) takes `qty × unitFactor` base units out
of stock and posts COGS for them at the base-unit cost (`costPrice` is per base unit — the product's
average cost at sale time). The refund (`recordRefund` / `create_refund`) restocked `line.qty` base
units at `line.qty × costPrice` instead, ignoring the invoice line's stored `unitFactor`: refunding 1
box put 1 piece back and reversed 1 piece's COGS; a written-off box booked 1 piece's cost to 5120.
Stock and GL moved by the same (wrong) amount, so `inventory-gl` still held and no invariant saw it —
but the product lost 3 pieces per refunded box and COGS stayed overstated by their cost.

**Rule now:** a refund line's stock side works in base units — `baseQty = round2(qty × unitFactor)`
(the same `baseQty` / `base_qty` the sale used, factor 1 when absent), restock `baseQty` units at
`round2(baseQty × costPrice)`, and a write-off books that same value to `inventoryWriteOff`. The
money side (ACC-0003 / ACC-0017 `refundLineShare`, ACC-0009 FX) is unchanged: it already works on the
line's own qty and snapshotted net/VAT. The refundable qty (`remaining`) stays in the line's unit.

Batches: the refund has never returned units to a batch on either backend (restocked units become
unbatched stock, which `consumeFefo` tolerates) — unchanged by this fix, noted for a later decision.

### خطوات إعادة الإنتاج

1. A product with 8 pieces at cost 40 and a box unit of 4. Sell 2 boxes (all 8 pieces) for cash.
2. Refund 1 box with restock → old code: stock 1 (value 40); new code: stock 4 (value 160).
3. Sell 1 box again → old code: refused ("المتاح 1"); new code: accepted.
4. Write the second box off → 160 to 5120 (old code: 40).

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0032-refund-non-base-unit.json`
(old code breaks at step 2, the second sale).

### ما جُرِّب ولم ينجح

- Snapshotting `costPrice` per line unit (`costPrice × unitFactor`) at sale time instead: it changes
  the meaning of a stored field every print/report path reads, and legacy lines would be ambiguous.

### الملفات ذات الصلة

- `src/mocks/backend/sales.ts` — `recordRefund` (restock/write-off loop).
- `src-tauri/src/domains/invoices/service/refund.rs` — `create_refund` step 13.
- `src-tauri/tests/domain_invoices.rs` — `refund_of_a_non_base_unit_line_moves_its_base_units`.
- `scripts/parity/cases/invoices/sale-non-base-unit.ts` — `refund-box` / `write-off-box` steps.
