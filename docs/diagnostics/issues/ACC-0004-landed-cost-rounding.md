---
id: ACC-0004
kind: accounting
status: verified
area: purchases
first_seen: 2026-09-28
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0004-landed-cost-rounding.json
---

## تكلفة الشحن التي لا تنقسم بالتساوي تجعل قيد الاستلام غير متوازن (G-31a)

Receiving a purchase order with a landed cost that does not split evenly over its stock lines was
refused as unbalanced. `allocateLandedCosts` (`src/mocks/backend/purchases.ts`) rounded each line's
share with `round2` and dropped the remainder, so 100.00 over three equal-qty lines gave
33.33 × 3 = 99.99 on the inventory debit while AP was credited the full 100.00 → `postJournal`
refused the entry ("القيد غير متوازن"). The mock is not atomic, so the refused receipt also left
stock value moved without its GL posting (`inventory-gl` and the supplier sub-ledger drifted). The
Rust port (`allocate_landed_costs`, plan 21 part 03, 07 Q-U1) had copied the rule verbatim.

**Rule now (docs/v2/02-accounting-review.md E4):** each landed-cost line is spread by value or qty,
every share is `round2`'d, and the rounding remainder (`round2(amount) − Σ shares`) goes to the
largest-weight line (ties → the first line). The shares add up exactly to the landed cost, so stock
value, `GL(inventory)` and AP agree to the halala.

### خطوات إعادة الإنتاج

1. A supplier and three stock products, cost 10 each.
2. Save a purchase order with `confirm: true`, one unit of each product (0% tax), and a landed cost
   of 100.00 spread by qty.
3. Old code: `القيد غير متوازن: المدين 129.99 ≠ الدائن 130`. New code: shares 33.34 / 33.33 / 33.33,
   stock values 43.34 / 43.33 / 43.33, balanced entry, every invariant green.

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0004-landed-cost-rounding.json`
(one step: `purchases.savePurchaseOrder`); on the old code it breaks at that step.

### ما جُرِّب ولم ينجح

- Largest-remainder (Hamilton) apportionment: correct too, but the user decision for G-31 was the
  simpler "remainder to the largest line" rule, which also matches how FX line conversion absorbs its
  rounding gap (docs/v2/10 §2).

### الملفات ذات الصلة

- `src/mocks/backend/purchases.ts` — `allocateLandedCosts`.
- `src-tauri/src/domains/purchases/service/receive.rs` — `allocate_landed_costs`.
- `src-tauri/tests/domain_purchases.rs` — `landed_costs_split_100_over_three_equal_lines_puts_remainder_on_first_largest_line`.
