---
id: ACC-0015
kind: accounting
status: verified
area: payments
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0015-allocate-later-fx-tagging.json
---

## التخصيص اللاحق لدفعة بالدولار يرحّل فرق العملة بوسم عملة خاطئ

A USD receipt recorded unallocated posts its full base cash against the receivable with no FC tag
(it is not tied to a document's rate yet). Allocating it later to a USD invoice booked at a lower rate
(`allocatePayment`) posted a two-line FX entry — Dr AR 350 / Cr FX gain 350 — and tagged that 350 AR
line with the whole 500 USD settled at the payment's rate. `fx-conversion` broke (500 × 49.2 ≠ 350),
the customer's USD balance went UP by 500 instead of down (a debit tagged 500 USD), and
`one-active-entry` flagged the payment for having two active entries, although 09 §3.3 designs that
second entry. The Rust port (`allocate_existing_payment`) copied it.

A related sign bug surfaced on the supplier side: `validateAllocations` computed `fxGainLoss = cash −
AP` for PAID too, so paying a USD PO with dearer dollars was booked as a gain and the entry could not
balance (both `recordPayment` and allocate-later refused it).

**Rule now (docs/v2/10 §2, 09 §3.3):**
- The allocate-later FX entry reclassifies the slice being allocated: it releases the untagged cash
  from the control account (Dr AR / Cr AP, base cash), settles each FC document at ITS OWN rate,
  FC-tagged (Cr AR / Dr AP `amount`, `amountFc` at the document's `exchangeRate`), and books the gap
  to `fxGain`/`fxLoss`. Net GL effect is unchanged (control ± FX); the party's FC balance now drops
  by the FC settled and every tagged line converts at its own rate. Only FX-realizing rows are
  included — a zero-FX allocation still posts nothing (09 §3.3).
  500 USD received at 49.2 (24,600) allocated to a 500 USD invoice at 48.5: Dr AR 24,600 / Cr AR
  24,250 (500 USD @ 48.5) / Cr FX gain 350.
- `fxGainLoss` is "+ = gain" for both directions: RECEIVED `cash − AR`, PAID `AP − cash`.
- `one-active-entry` does not count a payment's allocate-later FX entry (an entry whose lines are
  only receivable/payable and fxGain/fxLoss — it moves no money); the payment's own entry, which
  always hits cash/bank, must still be unique. Key, order and message unchanged.

### خطوات إعادة الإنتاج

1. Credit sale of 500 USD at 48.5 to a USD customer; a 500 USD receipt at 49.2 (24,600), unallocated.
2. Allocate the receipt to the invoice (24,600). Old code: `one-active-entry`,
   `fx-conversion` (JE/jl fc 500 × 49.2 = 24,600 ≠ 350) break and the customer's USD balance is 1,000.
   New code: USD balance 0, base balance 0, FX gain 350.

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0015-allocate-later-fx-tagging.json`.
Note: the case also needs `allocations-within-total` to compare an FC allocation in the document's
currency — that check belongs to the invariants fix running in parallel (it compares the base
`amount` 24,250 to the USD `grandTotal` 500); until it lands, the replay fails on that key only.

### ما جُرِّب ولم ينجح

- Tagging the FX line with `amountFc: 0`: `customerBalanceFc` still never sees the settlement (the
  receipt's credit was untagged), so the USD balance stays at +500 on a settled invoice.
- Collapsing to one AR line of 350 with a negative FC amount: breaks `fx-conversion` and hides the
  settlement rate.

### الملفات ذات الصلة

- `src/mocks/backend/payments.ts` — `allocationFxLines`, `allocatePayment`, `validateAllocations`.
- `src/mocks/backend/invariants.ts` — `isAllocationFxEntry` (one-active-entry).
- `src-tauri/src/domains/payments/service/allocate.rs` — `allocation_fx_lines`; `common.rs` — FX sign.
- `src-tauri/src/shared/invariants/documents.rs` — `is_allocation_fx_entry`.
- `src-tauri/tests/domain_payments.rs` — `allocate_later_fx_entry_tags_the_settled_fc_at_the_invoice_rate`,
  `allocate_later_supplier_payment_books_fx_loss`.
- Parity: `scripts/parity/cases/payments/payment-allocate-later-fx.ts`.
