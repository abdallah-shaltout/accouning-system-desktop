---
id: ACC-0009
kind: accounting
status: verified
area: invoices
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0009-fx-refund-base-conversion.json
---

## مرتجع فاتورة بعملة أجنبية يُرحَّل بمبالغ العملة الأجنبية دون تحويل

A sales refund on a foreign-currency invoice posted the refund's invoice-currency amounts straight to
the base-currency ledger. Returning a 100 USD invoice sold at 48.5 debited sales returns 86.96 and
output VAT 13.04 and credited the receivable 100 — "SAR" amounts that were really dollars — while the
sale had posted 4,217.56 / 632.44 / 4,850. The customer was left owing 4,750 SAR on a fully returned
invoice, revenue and VAT were barely reversed, and a cash refund paid back 100 "SAR" for 100 USD.
The journal still balanced; `customer-allocation` caught the receivable only when nothing else was
open for that customer, and `vat-output` subtracted the same unconverted refund VAT on both sides,
so it never saw the gap. The Rust port (`create_refund`) did the same.

**Rule now (docs/v2/10 §2, docs/v2/02-accounting-review.md §3 "Credit note"):**
- An FC invoice's refund converts to base at the **original invoice's rate** (`invoice.exchangeRate`,
  the rate the sale posted with). The net/VAT split is converted cumulatively per invoice — base
  refunded so far = `convertLinesToBase([Σ refunded net, Σ refunded VAT], rate)`, and each refund
  takes the difference — so a full refund, or any sequence of partial refunds, reverses the sale's
  base revenue, VAT and receivable exactly.
- The receivable and customer-credit legs stay at the invoice's rate and carry `currency`/`amountFc`/
  `rate`, like the sale's receivable line and a payment allocation.
- Money paid back (cash/card/bank) goes out at the day's rate (`latestRate`, falling back to the
  invoice's rate) to the FC settlement account when one exists, like `recordPayment`. The gap between
  that and the obligation at the invoice's rate is realized FX (`fxGain` credit / `fxLoss` debit,
  "فرق عملة محقق") — the same way payments book it. The shift drawer movement records the base amount.
- The `vat-output` invariant now converts refunded VAT the same cumulative way, and `customer-
  allocation` compares an FC invoice's outstanding at the invoice's rate (as `openDocumentsFor` does).
- The mock `closeEnough` gained a 1e-9 epsilon: a difference of exactly 0.01 is 0.0100000000002 in
  floating point, while the Rust port compares exact decimals.

Helpers: `refundBaseSplit` / `cashBackBaseFor` (`src/mocks/backend/sales.ts`), ported as
`refund_base_split` / `refund_cash_back_base` (`src-tauri/src/shared/currency.rs`).

### خطوات إعادة الإنتاج

1. Base SAR, USD at 48.5 (invoice) and 50 (today). A USD customer.
2. Credit sale INV-A: 1 × 100 USD. Credit sale INV-B: 2 × 100 USD, fully received at 49.
3. Refund INV-A in full → old code: Dr sales returns 86.96, Dr VAT 13.04, Cr receivable 100
   (`customer-allocation` breaks: outstanding 0, ledger 4,750).
4. Refund 1 unit of INV-B in cash, then the last unit as customer credit → now: revenue/VAT reversed
   at 48.5 to the halala, cash out 5,000.50 at 50, FX loss 150.01, customer credit 4,849.51.

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0009-fx-refund-base-conversion.json`;
on the old code it breaks at step 0 (`customer-allocation`) and every later step.

### ما جُرِّب ولم ينجح

- Converting each refund on its own (`convertLinesToBase([net, VAT], rate)` per refund): partial
  refunds then drift a halala from the sale's base amounts (e.g. 4,850.49 + 4,849.52 = 9,700.01 on a
  9,700 sale). The cumulative conversion telescopes to the sale's own figures.
- Converting the cash leg at the invoice's rate: hides the real currency gain/loss and credits cash
  with an amount that did not leave the till.

### الملفات ذات الصلة

- `src/mocks/backend/sales.ts` — `recordRefund`, `refundBaseSplit`, `cashBackBaseFor`.
- `src/mocks/backend/invariants.ts` — `vat-output`, `customer-allocation`, `closeEnough`.
- `src/modules/diagnostics/services/accountingDebugService.ts` — drift report (same outstanding rule).
- `src-tauri/src/domains/invoices/service/refund.rs` — `create_refund`.
- `src-tauri/src/shared/currency.rs` — `refund_base_split`, `refund_cash_back_base`.
- `src-tauri/src/shared/invariants/ledger.rs`, `parties.rs`; `src-tauri/src/domains/diagnostics/service/debugger.rs`.
- `src-tauri/tests/domain_invoices.rs` — `fx_refund_posts_base_amounts_at_invoice_rate`.
