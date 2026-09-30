---
id: ACC-0033
kind: accounting
status: fixed
area: invoices
first_seen: 2026-09-30
last_seen: 2026-09-30
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0033-quotation-unit-conversion.json
---

## تحويل عرض سعر بوحدة غير الأساسية إلى فاتورة يبيع وحدات أساسية بعدد العلب

`saveQuotation` / `save_quotation` accept the same lines as a sale (`SaleInput['lines']`, with
`unitId`/`unitFactor`) but stored the quotation line without its unit, and
`convertQuotationToInvoice` / `convert_quotation_to_invoice` built the sale lines without it too. A
quotation for 2 boxes of 4 at the box price became an invoice of 2 **pieces** at the box price: the
revenue was right, but stock and COGS moved 2 pieces instead of 8.

**Rule now:** a quotation line keeps `unitId` and `unitFactor` exactly as given, and conversion passes
them to the sale, so the invoice takes `qty × unitFactor` base units out of stock and posts their
COGS — the same as selling the line directly. (The desk form sends base-unit lines today, so only API
callers hit this; the stored contract was still wrong.)

### خطوات إعادة الإنتاج

1. A product with 8 pieces and a box unit of 4. Save a quotation for 2 boxes, convert it (cash).
2. Sell 1 piece → old code: accepted (the conversion took only 2 pieces, 6 left); new code: refused
   ("المتاح 0" — the conversion took all 8).

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0033-quotation-unit-conversion.json`
(old code breaks at step 2, the follow-up sale that must be refused).

### ما جُرِّب ولم ينجح

- Nothing else — the unit columns already existed on `quotation_lines` (free text since m0020).

### الملفات ذات الصلة

- `src/modules/invoices/services/invoiceService.ts` — `saveQuotation`, `convertQuotationToInvoice`.
- `src-tauri/src/domains/invoices/service/quotations.rs` — `save_quotation`, `convert_quotation_to_invoice`.
- `src-tauri/tests/domain_invoices.rs` — `quotation_in_a_non_base_unit_converts_to_its_base_units`.
- `scripts/parity/cases/invoices/sale-non-base-unit.ts` — `quotation-save` / `quotation-convert` steps.
