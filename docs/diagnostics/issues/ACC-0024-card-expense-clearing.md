---
id: ACC-0024
kind: accounting
status: fixed
area: expenses
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: posting
regression_test: scripts/verify/cases/ACC-0024-card-expense-clearing.json
---

## مصروف مدفوع بطريقة دفع بطاقة يُرحَّل دائناً على حساب تسوية البطاقات

A posting bug, reported by parity lane L4. The invariant was right. `recordExpense` / `record_expense`
credited the payment method's own account role. For a card (mada/visa) or wallet method that role is
the clearing account (1125 / 1126). A clearing account holds money customers paid by card or wallet
that the acquirer has not deposited yet (docs/v2/02 C3): only customer tenders go in, and only a card
settlement takes them out. A 115 expense paid "by card" left card clearing at -115 against 0
unsettled tenders (`card-clearing` failed). The next settlement of real tenders would have left it
wrong for good.

**Rule now:** an expense paid with a card- or wallet-clearing method is credited to the bank, where
money the business pays with its own card actually leaves. This is the same rule
`settlementAccountFor` / `settlement_account_for` uses to settle every non-cash payment. Cash and
bank methods are unchanged. Helpers: `expensePayoutRole` (expenses.ts) and `expense_payout_role`
(`domains/expenses/service/expenses.rs`).

**Not changed here (owned elsewhere):** general payment vouchers (`recordPaymentVoucher`, vouchers.ts,
and its Rust port) credit the method's role in the same way, and so does `payVatSettlement`, which
goes through it. A voucher paid with a card method has the same defect.

### خطوات إعادة الإنتاج

1. ACC-0009 base, 3900 closed, with an expense category on 6260.
2. Record an expense of 115 (tax invoice, VAT 15), paid with `pm-mada`. The old posting credits
   `cardClearing` 115 (`card-clearing -115 vs 0`). The new posting credits `bank` 115.

### الملفات ذات الصلة

- `src/mocks/backend/expenses.ts` — `recordExpense`, `expensePayoutRole`.
- `src-tauri/src/domains/expenses/service/expenses.rs` — `record_expense`, `expense_payout_role` (+ unit test).
- `src/mocks/backend/invariants.ts` / `src-tauri/src/shared/invariants/clearing.rs` — `card-clearing` (unchanged).
