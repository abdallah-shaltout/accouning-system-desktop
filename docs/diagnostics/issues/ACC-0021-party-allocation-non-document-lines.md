---
id: ACC-0021
kind: accounting
status: fixed
area: parties
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0021-party-allocation-non-document-lines.json
---

## `customer-allocation` / `supplier-allocation` تتجاهل الأرصدة الافتتاحية والمصروفات الآجلة والقيود اليدوية للطرف

A false alarm, reported by parity lane L4. The §4.6 check reconciles a party's ledger balance with
Σ open document outstanding − unallocated credit. It only counted invoices, refunds, purchase orders,
returns and payments. Several valid postings put lines on a party's control account with no such
document behind them: a party opening balance and its reversal (docs/v2/05 §4), a credit expense
(`paidFrom.kind = 'credit'`, payable to the supplier), a manual AR/AP line (B1 allows it with a
party, for write-offs or reclassification), and an FX revaluation. Each of these made the party fail
the check. The parity cases `setup/setup-party-opening-*` and `expenses/create-credit-supplier` were
red for this reason.

**Rule now (docs/v2/02 §4.6 "± opening balance"):** balance = Σ document outstanding + non-document
net − unallocated credit. Non-document net is Σ of the party's control-account lines in entries whose
source is not `invoice` / `refund` / `payment` / `purchaseOrder` / `purchaseReturn` (customer
debit − credit, supplier credit − debit), less what payments allocated to an opening balance
(`targetKind: 'opening'`). Such an allocation also reduced the payment's unallocated credit.
Messages are unchanged.

### خطوات إعادة الإنتاج

1. ACC-0009 base, 3900 closed, onboarding finished with go-live 2026-09-01. Add supplier `sup-1` and
   an expense category.
2. Customer `cus-1` opening balance of 500 (after go-live, to capital). The old check fails
   `customer-allocation`.
3. A credit expense of 230 on `sup-1`, then a cash payment of 230 to `sup-1` (unallocated). The old
   check fails `supplier-allocation`.
4. A manual bad-debt write-off: Dr bad debt 100 / Cr receivable[`cus-1`] 100. With the fix, every
   step reconciles.

### الملفات ذات الصلة

- `src/mocks/backend/invariants.ts` — `checkPartyAllocation`, `nonDocumentNet`, `DOCUMENT_SOURCE_KINDS`.
- `src-tauri/src/shared/invariants/parties.rs` — `check_party_allocation`, `non_document_nets`.
- `src/mocks/backend/opening.ts`, `src/mocks/backend/expenses.ts`, `src/mocks/backend/journal.ts`.
- `src-tauri/tests/shared_invariants.rs` — `party_lines_without_documents_reconcile`.
