---
id: ACC-0003
kind: accounting
status: verified
area: invoices
first_seen: 2026-09-28
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0003-refund-vat-inclusive.json
---

## مرتجع المبيعات يرد ضريبة زائدة على الأسعار الشاملة للضريبة (G-25)

A sales refund on a tax-inclusive sale (the default, `settings.pricesIncludeTax`) refunded more than
the customer paid: returning a 115 item (net 100 + VAT 15) paid back 130. `recordRefund`
(`src/mocks/backend/sales.ts`) took `qty × (price − discount/qty)` — the tax-INCLUSIVE gross — as the
refund's net, then added `taxRate %` on top again (and the final-refund branch did the same with
`invoice.subTotal − invoice.discountAmount`, which is also gross under inclusive prices). It also
ignored a flat invoice discount (`discountAmount` with `discountRate = 0`) on partial refunds and used
the first line's rate (`invoice.taxRate`) for every line. The journal still balanced (Dr sales returns
+ Dr output VAT = Cr receivable/cash), so none of the 14 invariants caught it; the damage was an
over-debited sales-returns account, output VAT reduced by more than the sale charged, and too much
cash/credit handed back. The Rust port (`create_refund`, plan 21 part 03) had copied the formula
verbatim behind a `// G-25:` marker.

**Rule now (docs/v2/02-accounting-review.md, docs/v2/06-sales-and-pos.md §3):** a refund reverses
exactly its proportional share of each returned line's net and VAT as the sale's VAT engine
(`computeInvoiceTotals` / `shared::totals`) snapshotted them on the line (`lines[].net`/`vat`, after
line discount → invoice discount → VAT). No VAT is recomputed, so refunded net + refunded VAT =
refunded gross. Shares use cumulative rounding — the amount refunded for `q` of `qty` units is
`round2(total × q / qty)` (the whole snapshot at `q = qty`) and each refund takes the difference — so
any sequence of partial refunds reverses the sale's revenue and VAT exactly. One helper:
`refundLineShare` (`src/modules/invoices/helpers/totals.ts`), ported as `refund_line_share`
(`src-tauri/src/domains/invoices/service/refund.rs`); `RefundPage.vue`'s estimate uses the same helper.

### خطوات إعادة الإنتاج

1. Tax-inclusive store, a customer, a product priced 99.99.
2. Credit sale of 3 units with a 10% invoice discount → gross 269.97 (line net 234.76, VAT 35.21).
3. Return 1 unit → old code refunded 103.49 (89.99 + 13.50 VAT on top) instead of 89.99
   (net 78.25 + VAT 11.74); the invoice's outstanding dropped to 166.48 instead of 179.98.
4. Receive a payment of the true outstanding 179.98 → refused ("المبلغ المخصص … أكبر من المتبقي
   عليه (166.48)"). Returning all 3 units refunded 305.18 on a 269.97 sale.

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0003-refund-vat-inclusive.json`
(steps: refund 1/3 → payment 179.98 → refund 2/3 → final refund 3/3); on the old code it breaks at the
payment step.

### ما جُرِّب ولم ينجح

- Re-deriving net as `gross / (1 + rate)` per refund: a second copy of the VAT math, and it drifts
  from the sale's own rounding (the sale split VAT per line after the invoice-discount spread), so a
  full refund would not reverse the sale to the halala. Pro-rating the stored per-line snapshot avoids
  both problems.

### الملفات ذات الصلة

- `src/mocks/backend/sales.ts` — `recordRefund`.
- `src/modules/invoices/helpers/totals.ts` — `refundLineShare`.
- `src/modules/invoices/pages/RefundPage.vue` — the refund estimate.
- `src-tauri/src/domains/invoices/service/refund.rs` — `create_refund` / `refund_line_share` (+ unit tests).
- `src-tauri/tests/domain_invoices.rs` — `refund_partial_then_stock_restocked` (partial + final refund amounts).

### التحقق (2026-09-30، الجزء 04 الموجة 2، L3)

`verify:replay` of `ACC-0003-refund-vat-inclusive.json` is green, and the parity case
`scripts/parity/cases/invoices/acc-0003-refund-inclusive-vat-exact.ts` (a partial then a final refund
on a tax-inclusive sale with a line discount and an invoice discount, plus 1 × 115 → 115) passes
mock-vs-Rust with 0 unexplained diffs. Before this pass every Rust refund failed with `INTERNAL`:
`RefundInput.lines[].invoiceLineId` was decoded as a UUID, but the UI sends the invoice line's DTO id
(`{invoiceId}-l{n}`); it is now resolved by that id (`common::line_display_id`).
