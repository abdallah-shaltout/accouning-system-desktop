---
id: ACC-0010
kind: accounting
status: verified
area: parties
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0010-refund-credit-allocation.json
---

## رصيد العميل من المرتجع لا يُحتسب رصيداً غير مخصص

A `customer_credit` refund on an already-paid invoice broke the `customer-allocation` invariant
(§4.6: Σ document outstanding − unallocated credit = sub-ledger balance). `recordRefund` credits the
customer's receivable with the refund's `creditedToAccount` — correct per docs/v2/02-accounting-
review.md D4 ("keep as customer credit — an unallocated credit on the party") and §3's credit-note row
("… *or* `receivable[p]` as credit"). But `unallocatedCreditFor` (`src/mocks/backend/payments.ts`) only
summed payments' unallocated money, so the party page never showed that credit and the invariant saw
a ledger balance of −89.99 against 0 outstanding and 0 credit.

**Decision: the posting was right; the credit reader was wrong.** A refund kept as customer credit is
the same kind of balance as an overpayment. `unallocatedCreditFor` now adds, for customers, every
refund's credit (`refundCreditedBase` in `sales.ts`: exactly what `recordRefund` credited — an FC
invoice's credit at the invoice's own rate, per ACC-0009). The invariant, the accounting debugger's
drift report and the party page (`partyService` → `unallocatedCredit`) all read it through that one
function; the Rust port adds the same to `shared::balances::unallocated_credit_for` and the batch
`unallocated_credits` (party list).

**Later (not in this fix):** applying that credit to a later invoice. Payments' credit can be allocated
from the payment page; a refund's credit has no allocation record yet, so it stays visible as the
party's unallocated credit and nets against open invoices on the balance, but cannot be linked to a
specific invoice.

### خطوات إعادة الإنتاج

1. Tax-inclusive store, customer, credit sale of 3 × 99.99 with a 10% invoice discount (269.97).
2. Receive 269.97 allocated to the invoice (fully paid).
3. Refund 1 unit with "رصيد للعميل" → old code: `customer-allocation` fails (outstanding 0,
   unallocated 0, ledger −89.99). Refund the other 2 units the same way → still failing.

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0010-refund-credit-allocation.json`;
on the old code it breaks at step 1.

### ما جُرِّب ولم ينجح

- Changing the posting instead (crediting a separate customer-advances account): contradicts D4/§3,
  which keep the credit on the customer's own receivable sub-ledger so the statement and balance show it.

### الملفات ذات الصلة

- `src/mocks/backend/payments.ts` — `unallocatedCreditFor`.
- `src/mocks/backend/sales.ts` — `refundCreditedBase`.
- `src/mocks/backend/invariants.ts` — `checkPartyAllocation` (`customer-allocation`).
- `src-tauri/src/shared/balances.rs` — `refund_credits`, `unallocated_credit_for`, `unallocated_credits`.
- `src-tauri/tests/domain_invoices.rs` — `customer_credit_refund_on_paid_invoice_is_unallocated_credit`.
