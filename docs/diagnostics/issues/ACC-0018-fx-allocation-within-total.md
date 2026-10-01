---
id: ACC-0018
kind: accounting
status: verified
area: payments
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0018-fx-allocation-within-total.json
---

## `allocations-within-total` يقارن مبلغ التخصيص بالعملة الأساسية بإجمالي فاتورة بعملة أجنبية

A false alarm. The invariant compared a payment allocation's `amount` against the target document's
`grandTotal`. For a foreign-currency document those are in different currencies: `amount` is the base
amount posted to AR/AP at the document's rate, while `grandTotal` is in the document's own currency.
The demo seed's own FX worked example failed it (`pay-135->inv-634 (48500 > 1000)`: USD 1,000 at
48.50, settled in full), so every FX receipt looked like an over-allocation and a real one would hide
among them.

**Rule now:** both sides are compared in the document's currency: `amountFc ?? amount` against
`grandTotal`. That is the figure `applyAllocationToDocument` (payments.ts) / `apply_allocation`
(payments/service/common.rs) adds to `paidAmount`, and it matches ACC-0009's rule that an FC
document's figures stay in its own currency until they are converted at the document's rate.
`opening` allocations have no document total and are skipped, and the target is looked up by
`targetKind` (the mock used to match an id in either table). The offender detail now prints the
settled amount (`pay->inv (settled > total)`); the message template is unchanged.

### خطوات إعادة الإنتاج

1. A USD invoice of 100 at 48.5 (the ACC-0009 base snapshot).
2. A USD receipt of 100 at 50 (5,000 base), allocated in full to it. The allocation stores
   `amount` 4,850 (AR at the invoice's rate) and `amountFc` 100.
3. Old check: `4850 > 100`, fails. New check: `100 ≤ 100`, passes.

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0018-fx-allocation-within-total.json`.

### ما جُرِّب ولم ينجح

- Converting `grandTotal` to base at the invoice's rate and comparing it with `amount`. This is
  equivalent for AR, but it adds a rounding step on each side. `amountFc` is the stored figure the
  document itself is settled by.

### الملفات ذات الصلة

- `src/mocks/backend/invariants.ts` — `checkAllocationsWithinTotal`.
- `src-tauri/src/shared/invariants/documents.rs` — `check_allocations_within_total`.
- `src/mocks/backend/payments.ts` — `validateAllocations`, `applyAllocationToDocument`.
