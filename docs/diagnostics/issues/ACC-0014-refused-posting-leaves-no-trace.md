---
id: ACC-0014
kind: accounting
status: fixed
area: invoices
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0014-refused-posting-leaves-no-trace.json
---

## ترحيل مرفوض (فترة مقفلة) يترك مستندات ومخزوناً وأرقاماً بلا قيد

The mock backend has no transaction, and its sales, payment and voucher write paths saved their
document before `postJournal` ran its checks. A sale into a locked period (or a closed fiscal year, or
an entry that did not balance) saved the invoice, took an invoice number and moved stock, and only
then got refused — leaving an invoice with no journal entry (`inventory-gl`, `vat-output` and
`customer-allocation` all broke). Refunds, payments (`recordPayment` saved the payment and bumped the
invoices' `paidAmount`), allocate-later FX, the four general vouchers and card settlements did the
same, and every refusal burned a document number. The Rust backend runs each command in one
transaction and checks the period before numbering, so it never left a trace; the mock did.

**Rule now:** every write path runs every check that can refuse its posting — `preflightJournal`
(`src/mocks/backend/core.ts`: period lock / closed year, account resolution, balance, ≥ 2 lines) —
BEFORE it saves a row, takes a number, moves stock or touches another document. A refused posting
leaves no trace and consumes no number, in both backends.

- `recordSale`: preflight right after `prepareSale`, before the invoice is built/numbered.
- `recordRefund`: the restock/write-off values and the full posting are built first (a pure pass),
  preflighted, and only then the refund is numbered and saved and stock moves back.
- `recordPayment`: the number is taken after validation and the preflight; allocations touch the
  invoices only after that. `allocatePayment` preflights its FX entry before saving any allocation.
- Vouchers: `saveVoucher` (`vouchers.ts`) preflights the lines, then numbers and saves.
  `recordCardSettlement` (`settlements.ts`) likewise.
- Purchases are covered by ACC-0013 (same helper).

### خطوات إعادة الإنتاج

1. A company with stock and one credit invoice (inv-1), `lockDate` set far in the future.
2. Try: a credit sale, a cash sale, a receipt allocated to inv-1, a refund of inv-1, and one voucher
   of each kind. Every call is refused with "الفترة مقفلة".
3. Old code: after the first refused sale, `inventory-gl` (320 vs 280), `vat-output` and
   `customer-allocation` break — the invoice was saved and stock moved with no entry. New code: all
   eight refusals leave the books exactly as they were.

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0014-refused-posting-leaves-no-trace.json`.
The voucher refusals have no invariant that sees a burned number; they are pinned by the Rust tests
below (the next accepted voucher is `VCH-000001`).

### ما جُرِّب ولم ينجح

- Wrapping each write in a snapshot/rollback of the whole mock `db`: heavy, and it hides the rule
  (check first, then write) that the Rust port already follows.

### الملفات ذات الصلة

- `src/mocks/backend/core.ts` — `preflightJournal` (shared with ACC-0013).
- `src/mocks/backend/sales.ts` — `recordSale`, `recordRefund`.
- `src/mocks/backend/payments.ts` — `recordPayment`, `allocatePayment`.
- `src/mocks/backend/vouchers.ts` — `saveVoucher`; `src/mocks/backend/settlements.ts` — `recordCardSettlement`.
- Rust (already transactional, tests only): `src-tauri/tests/domain_invoices.rs`
  `refused_sale_leaves_no_invoice_stock_or_number`, `domain_payments.rs`
  `refused_payment_consumes_no_number_and_touches_no_invoice`, `domain_vouchers.rs`
  `refused_vouchers_leave_no_row_and_consume_no_number`.
