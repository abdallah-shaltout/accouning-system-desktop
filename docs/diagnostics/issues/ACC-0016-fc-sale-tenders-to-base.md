---
id: ACC-0016
kind: accounting
status: fixed
area: invoices
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0016-fc-sale-tenders-to-base.json
---

## بيع بعملة أجنبية مدفوع نقداً/بطاقة يرحّل مبالغ الدفع دون تحويل

A foreign-currency sale converted its revenue, VAT and receivable to base at the sale's rate, but
posted each tender (cash, card, bank transfer) at its raw invoice-currency amount. A 300 USD sale at
48.57 with 250 USD tendered debited cash/bank "250" against 14,571 of credits: the entry was
unbalanced and the sale was refused ("القيد غير متوازن"). The Rust port (`prepare_sale`) did the same.

**Rule now (docs/v2/10 §2):**
- A tender is in the sale's currency. On an FC sale each tender posts its base amount at the sale's
  rate, tagged `currency`/`amountFc`/`rate` like a payment's settlement line, to the method's account
  for that currency when one exists (`accountFor(role, { branchId, currency })` — the resolution
  `recordPayment` uses; a base-currency sale is unchanged).
- `saleTenderBases` (`sales.ts`, ported as `shared::currency::sale_tender_bases`): the receivable
  keeps `toBase(receivable)` (what its FC tag and `openDocumentsFor` use); the tenders share the rest
  of `toBase(grandTotal)`, each `toBase(amount)`, any rounding gap on the largest tender — so the
  entry balances to the halala. It is recomputable from the saved invoice.
- Card settlements read an FC invoice's clearing tenders through the same helper, so the settlement
  clears the base amounts the sale posted. The drawer (shift movement) records the cash tender's base
  amount, as ACC-0009 set for FC cash refunds.

### خطوات إعادة الإنتاج

1. A USD customer; a 3 × 100 USD sale at 48.57, tenders 150.33 cash + 99.67 bank transfer, 50 on
   credit; and a 100 USD cash sale with no customer.
2. Old code: both refused as unbalanced (the replay flips `ok`). New code: cash 7,301.53, bank
   4,840.97, receivable 2,428.50 (50 USD) = 14,571; all invariants hold.

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0016-fc-sale-tenders-to-base.json`.

### ما جُرِّب ولم ينجح

- Converting tenders and receivable together with `convertLinesToBase`: the receivable could absorb
  the rounding cent and drift from `toBase(receivable)`, which the open-documents list and the
  `customer-allocation` check use.

### Open follow-up (not in this fix)

- The `card-clearing` / `wallet-clearing` invariants (`checkClearingAccounts`, owned by the
  invariants work) still sum raw tender amounts. An FC sale with a card/wallet tender will show a gap
  there until that check reads FC tenders through `saleTenderBases` as the settlement list now does.
  The regression case uses cash + bank tenders only for that reason.

### الملفات ذات الصلة

- `src/mocks/backend/sales.ts` — `saleTenderBases`, `prepareSale`, `recordSale` (drawer amount).
- `src/mocks/backend/settlements.ts` — `unsettledTenderGroups`.
- `src-tauri/src/shared/currency.rs` — `sale_tender_bases`; `src-tauri/src/domains/invoices/service/sale.rs`;
  `src-tauri/src/domains/vouchers/service/settlements.rs`.
- `src-tauri/tests/domain_invoices.rs` — `fc_sale_tenders_post_base_amounts`;
  `domain_vouchers.rs` — `unsettled_groups_read_fc_invoice_tenders_in_base`.
