---
id: BUG-0021
kind: bug
status: fixed
area: parties
first_seen: 2026-09-30
last_seen: 2026-09-30
occurrences: 1
---

## `getLinkedNetBalance` returned an unrounded float (`-521.0100000000002`)

المصدر: plan 21 Part 04 Wave 2 parity run (lane L2), `parties/parties-link-unlink` step
`net-balance` — mock `-521.0100000000002`, Rust `-521.01`.

### خطوات إعادة الإنتاج

1. On `demo-sa`, link a customer and a supplier that both carry a balance (`linkPartyRecords`).
2. `getLinkedNetBalance(customerId, supplierId)` returns `customerBalance − supplierBalance` as a raw
   JS float subtraction, so two `round2`'d balances can give a value with float noise.

CLAUDE.md "One rounding rule": money goes through `round2` only. The Rust side
(`parties::service::link::get_linked_net_balance`) subtracts two exact `Decimal`s, which is already
the correctly rounded value. No posting, balance or invariant is affected (the value is only shown,
and `MoneyText` rounds for display) — hence `BUG-`, not `ACC-`.

### الإصلاح

`src/modules/parties/services/partyService.ts` — `round2(customerBalance(c) - supplierBalance(s))`.
Regression: parity case `parties/parties-link-unlink` (`net-balance`).

### ما جُرِّب ولم ينجح

(لا شيء.)

### الملفات ذات الصلة

- `src/modules/parties/services/partyService.ts` (`getLinkedNetBalance`)
- `src-tauri/src/domains/parties/service/link.rs` (`get_linked_net_balance`)
