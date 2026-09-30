---
id: ACC-0017
kind: accounting
status: fixed
area: invoices
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0017-refund-gross-rounding.json
---

## مرتجع وحدة من سطر شامل الضريبة يرد هللة زائدة أو ناقصة

`refundLineShare` (ACC-0003) rounded a refund's net share and VAT share separately. For 2 × 100
tax-inclusive (line net 173.91, VAT 26.09) the first unit refunded 86.96 + 13.05 = 100.01 and the
last 86.95 + 13.04 = 99.99 — the customer got a halala more than the item's price, then a halala less.
Totals still reversed the sale exactly, so no invariant saw it, but the invoice's outstanding dropped
to 99.99 and a 100 receipt against the remaining unit was then refused as over-allocation.

**Rule now:** the GROSS share is rounded first — `round2(gross × q / qty)` with the same cumulative
(last-remainder) rule — then the VAT share `round2(vat × q / qty)`, and net = gross − VAT. Each unit
refunds exactly its gross price (100 = 86.95 + 13.05, then 86.96 + 13.04), refunded net + VAT still
equals refunded gross, and any sequence of partial refunds still ends on the line's net and VAT.
One helper, `refundLineShare` (`src/modules/invoices/helpers/totals.ts`), ported as
`refund_line_share` (`src-tauri/src/domains/invoices/service/refund.rs`); `RefundPage.vue`'s estimate
uses the same helper.

ACC-0003's numbers do not change: 3 × 99.99 with a 10% discount (net 234.76, VAT 35.21, gross 269.97)
gives 89.99 per unit either way (78.25 + 11.74, 78.26 + 11.73, 78.25 + 11.74), so its case and the
Rust unit test `partial_refunds_sum_to_the_line_snapshot` stand as they are.

### خطوات إعادة الإنتاج

1. Tax-inclusive store (15%), a credit sale of 2 × 100 to a customer (outstanding 200).
2. Refund one unit → old code: 100.01 (outstanding 99.99). A 100 receipt allocated to the invoice is
   refused ("أكبر من المتبقي عليه (99.99)"). New code: refund 100, the receipt settles the invoice,
   and refunding the last unit in cash gives back exactly 100.

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0017-refund-gross-rounding.json`
(old code breaks at step 1, the receipt).

### ما جُرِّب ولم ينجح

- Rounding net first and deriving VAT = gross − net: the VAT share then carries the drift and a
  partial refund's VAT no longer matches `vat × q / qty`.

### الملفات ذات الصلة

- `src/modules/invoices/helpers/totals.ts` — `refundLineShare`.
- `src-tauri/src/domains/invoices/service/refund.rs` — `refund_line_share` + unit test
  `each_unit_refunds_its_exact_gross`.
- `src-tauri/tests/domain_invoices.rs` — `unit_refunds_of_an_inclusive_line_are_each_exactly_the_price`.
