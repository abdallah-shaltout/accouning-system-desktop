---
id: ACC-0020
kind: accounting
status: fixed
area: accounting
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0020-vat-settlement-control.json
---

## `vat-output` / `vat-input` تفشل بعد كل تسوية ضريبية صحيحة

A false alarm. The VAT control check compared the output/input VAT account balances with Σ document
VAT for all time. A VAT settlement (`postVatSettlement` / `submit_vat_settlement`) is the only thing
that legitimately moves those accounts without a document: Dr output VAT / Cr input VAT / Cr or Dr
`vatPayable`. So every valid settlement failed both checks. The parity cases
`accounting/vat-settlement-payable` and `-refundable` were red for this reason
(`30831.15 vs 46227.6`).

**Rule now:** the ledger side leaves out `VAT_SETTLEMENT` entries and any entry reversing one. These
are identified the same way `settledVatPeriods` / `settled_vat_periods` do, by the journal type
`VAT_SETTLEMENT`. The VAT payment (`payVatSettlement` / `pay_vat_settlement_now`) posts Dr
`vatPayable` / Cr cash or bank and never touches either VAT account, so it needs no exclusion. VAT
posted with no document behind it still fails the check.

Side effect on an existing case: `ACC-0007`'s hand-built snapshot books sales VAT through manual
entries with no invoice documents. B1 blocks manual lines on the VAT accounts in the app. Its
baseline now reports `vat-output 900 vs 0`, which is correct. Replay ignores baseline failures, so
the case is still green.

### خطوات إعادة الإنتاج

1. ACC-0009 base, 3900 closed.
2. Settle VAT for 2026-09-01 … 09-30. The old check reports `vat-output 0 vs 1933.02`; the new one
   passes.
3. Pay 100 of it to the authority by bank transfer. Both checks still pass.

### الملفات ذات الصلة

- `src/mocks/backend/invariants.ts` — `checkVatControl`, `vatSettlementEntryIds`, `glBalance(…, skipEntry)`.
- `src-tauri/src/shared/invariants/ledger.rs` — `check_vat_control`, `vat_settlement_entry_ids`, `gl_balance_excluding_entries`.
- `src/mocks/backend/journal.ts`, `src-tauri/src/domains/accounting/service/vat.rs`.
- `src-tauri/tests/shared_invariants.rs` — `vat_settlement_is_left_out_of_vat_control`.
