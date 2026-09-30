---
id: ACC-0031
kind: accounting
status: fixed
area: vouchers
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0031-payment-voucher-card-payout.json
---

## سند الصرف المدفوع ببطاقة/محفظة يُقيَّد على حساب التسوية بدل البنك

A general payment voucher (`recordPaymentVoucher`) credited the chosen payment method's own account
role. Card and wallet methods carry a CLEARING role (`cardClearing` / `walletClearing`) — money
customers paid by card/wallet that the acquirer hasn't deposited yet (docs/v2/02 C3). Paying a
supplier fee or the VAT authority with the store's card therefore reduced the clearing balance, as if
a customer tender had been settled: `card-clearing` / `wallet-clearing` broke and the next card
settlement no longer matched the bank deposit. `payVatSettlement` pays through the same voucher, so
"سداد" of VAT by card had the same effect. The Rust port (`create_payment_voucher`) copied it. Same bug
as ACC-0024 for expenses.

**Rule now:** money the business pays out with a card/wallet method leaves the bank, so the voucher's
credit goes through the ACC-0024 helper — `expensePayoutRole` (`src/mocks/backend/expenses.ts`, now
exported) / `expense_payout_role` (`src-tauri/src/domains/expenses/service/expenses.rs`, now `pub`):
card/wallet clearing → `bank`, every other role unchanged. Receipt vouchers are untouched (money coming
in by card does go through clearing).

### خطوات إعادة الإنتاج

1. A card sale (150 mada + 50 STC Pay tenders) sitting in clearing.
2. A payment voucher of 40 by mada, one of 25 by STC Pay, and a VAT payment of 30 by mada.
3. Old code: `card-clearing` 110 vs 150 after the first voucher (80 vs 150 after the VAT payment),
   `wallet-clearing` 25 vs 50. New code: bank is credited; both clearing checks hold.

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0031-payment-voucher-card-payout.json`.

### الملفات ذات الصلة

- `src/mocks/backend/vouchers.ts` — `recordPaymentVoucher`; `src/mocks/backend/journal.ts` — `payVatSettlement`.
- `src/mocks/backend/expenses.ts` — `expensePayoutRole`.
- `src-tauri/src/domains/vouchers/service/general.rs` — `create_payment_voucher`.
- `src-tauri/src/domains/expenses/service/expenses.rs` — `expense_payout_role`.
- `src-tauri/tests/domain_vouchers.rs` — `payment_voucher_by_card_or_wallet_credits_bank_not_clearing`.
