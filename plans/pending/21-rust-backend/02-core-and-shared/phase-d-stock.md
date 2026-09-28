# 21 · 02.D — `shared::stock`, `shared::balances`, `shared::invariants`

> **Status:** code complete, not yet compiled/tested (build rule 2026-09-27). Implemented against
> `shared::ledger::accounts` (`resolve_account`/`SystemRole`/`AccountCtx`, C-1) and
> `shared::ledger::post` (`post`/`PostJournal`/`PostingLine`/`AccountRef`/`SourceRef`, C-5-8) as they
> existed in the tree at implementation time — both were already written (not stubs) when this
> phase started, so the "code against exactly this" contract in the brief was read from the real
> files rather than assumed. Deviations from the spec, recorded here per D-3's own instruction:
> - `consume_fefo` **does not error on a shortfall** — confirmed against `inventory.ts:113-129`'s
>   actual code (not its doc comment at `113-117`, which claims it throws). It returns whatever
>   partial draws it managed. Ported literally, with a code comment at the call site.
> - `apply_change`'s branch default resolves `settings.default_branch_id` via `core::settings::load`
>   when `branch_id` is `None` (the spec's own wording), not a guess from whichever
>   `product_branch_stock` rows already exist for that product.
> - `shared::invariants` is split into `ledger.rs` (checks 1–5), `parties.rs` (check 6),
>   `documents.rs` (checks 7–9 + drafts-isolated + allocations-within-total), `clearing.rs` (check
>   10, 11, fx-conversion) instead of one file — same 14 keys, same order, same `run_all` entry
>   point, just organized like the mock's own function grouping (per `mod.rs`'s doc comment).
> - The `architecture_rules` D-5 rule and the CLAUDE.md "Accounting safety" sentence are **not**
>   in this diff — per the brief, the C implementer adds the former (owns `tests/architecture_rules.rs`)
>   and the manager adds the latter (owns `CLAUDE.md`). Not done here; flagged under "Needs from
>   manager" below.
> - Everything else (build, `cargo test`, the `bun run` gates) is `⏳ runs in the single final
>   build/test pass` — no compiler has touched this code yet.

**Goal:** (1) the **only** code that changes quantities, batches or average cost (master rule 3):
a behaviour-exact port of `applyStockChange` (`core.ts:268-297`), `receiveBatch`/`consumeFefo`/
`activeBatchesFor`/`isBatchExpired`/`isBatchNearExpiry` (`inventory.ts:73-133`), and the
weighted-average cost-out rules the mock keeps in its callers. (2) The read-only party-balance
helpers. (3) The Rust port of all 14 invariant functions (`invariants.ts:65-417`), with the
same keys, so Part 04 can compare mock and Rust results key by key.

**Read first:** [`../01-frontend-analysis/products.md`](../01-frontend-analysis/products.md) §2, §5, §7 ·
[`../../../../docs/v2/02-accounting-review.md`](../../../../docs/v2/02-accounting-review.md) A1–A5, §4 ·
`src/mocks/backend/{core,inventory,sales,purchases,transfers,balances,payments,invariants}.ts`.

## Tasks

### D-1 — Locking and `apply_change` (`shared/stock/mod.rs`)
- [x] `lock_products(conn, ids) -> BTreeMap<Id, LockedProduct>`: `FOR UPDATE` sorted by id
      (products.md §5, "the single most important lock"). Missing id → `NOT_FOUND`
      `المنتج غير موجود`. `LockedProduct` owns the row and its `product_branch_stock` rows. Every
      function below takes `&mut LockedProduct`, so an unlocked stock change can't compile.
- [x] `apply_change(conn, cx, p, qty_change, value_change, reason, StockRef{id, number}, date:
      &DocDate, branch_id: Option<Id>)`. It is a **no-op** (no movement row) when
      `type = service`, `stock_mode = 'none'` (`NULL` means tracked), or both changes are 0.
      Otherwise: `stock_qty = round2(q + Δq)`, `stock_value = round2(v + Δv)`, `cost_price =
      stock_qty > 0.0001 ? round4(value/qty) : 0`; the branch row (default
      `settings.default_branch_id`) becomes `round2` qty/value, read-modify-write under the lock;
      insert a `stock_movements` row (`qty_change` round4 to the column scale, `value_change`
      round2, `balance_after` = the new `stock_qty`); `cx.touch(Catalog)` (P2-12).
- [x] Product-creation helpers for the domain (so rule 3 holds): `init_product_stock(active_model,
      cost_price)` sets qty 0, value 0 and the cost. `set_cost_when_empty(p, cost)` is allowed
      only while `stock_qty <= 0.0001`, mirroring `updateProduct`'s silent discard (products.md §1).

### D-2 — Weighted-average cost-out (`shared/stock/cost.rs`)
- [x] `cost_out_sale(p, qty)`: `round2(qty) >= round2(stock_qty) ? stock_value : round2(qty × cost_price)` (`sales.ts:340`, review A1 "sale empties the stock").
- [x] `cost_out_at_price(p, qty, unit_price) -> (value_out, variance)`: `at = round2(qty × unit_price)`.
      If `round2(value − at) < 0`, or `round2(qty_left) <= 0.0001` with `|value_left| > 0.001`,
      then `value_out = stock_value` and `variance = round2(at − value_out)` (`purchases.ts:458-468`).
- [x] `cost_at_average(qty, unit_cost) = round2(qty × unit_cost)` (`transfers.ts:81-82`, `inventory.ts:159`).

### D-3 — Batches (`shared/stock/batches.rs`)
- [x] `active_batches(conn, product_id)`: `qty > 0.0001`, `ORDER BY expiry_date IS NULL,
      expiry_date, created_at, id` (FEFO: undated last, stable insertion order; `inventory.ts:73-77`).
- [x] `receive_batch(conn, cx, p, qty, unit_cost, batch_no, expiry, date, ref)`: a new lot row with
      values as given (`inventory.ts:95-109`).
- [x] `consume_fefo(conn, cx, p, qty, allow_expired) -> Vec<Draw>`: `remaining = round2(qty)`;
      walk the active batches; skip expired ones (`expiry < cx.today()`) unless `allow_expired`;
      `take = min(b.qty, remaining)`; `b.qty = round2(b.qty − take)`; `remaining = round2(remaining − take)`.
      **It does not error on a shortfall.** This matches the mock's code; the doc comment at
      `inventory.ts:113-117` claims it throws, but it doesn't. Recorded in a code comment and in
      the status note.
- [x] `draw_batch(conn, p, batch_id, qty) -> taken`: `take = min(b.qty, qty)`, `b.qty = round2(b.qty − take)`,
      scoped to the product (strict; covers `sales.ts:348-356` and `purchases.ts:474-476`).
- [x] `is_batch_expired(b, today)`, `is_batch_near_expiry(b, alert_days, today)` (`inventory.ts:79-87`), and `branch_stock_qty(conn, product_id, branch_id)`.

### D-4 — `shared/balances.rs` (P2-26, read-only)
- [x] `customer_balance`, `supplier_balance`, the FC variants, and `customer_statement`/
      `supplier_statement` (running `round2`, sorted by the date key) from `balances.ts`.
      `allocated_total`, `unallocated_amount`, `unallocated_credit_for` from `payments.ts:81-106`.
      Party lines are those on the `receivable`/`payable` role account with a matching `(party_kind, party_id)`.

### D-5 — `shared/invariants/` (the 14 check functions; `runAllInvariants`)
- [x] `run_all(conn: &impl ConnectionTrait) -> Vec<InvariantResult { key, doc, passed, message,
      diff }>` returns results in the same order, with the same `key`/`doc` strings and the same
      English `message` templates as `invariants.ts`: `balanced-entries`, `no-both-sided-lines`,
      `min-two-lines`, `trial-balance`, `balance-sheet`, `ar-control`, `ap-control`,
      `inventory-gl`, `vat-output`, `vat-input`, `customer-statement`, `supplier-statement`,
      `customer-allocation`, `supplier-allocation`, `source-ref-known`, `one-active-entry`,
      `reversal-resolves`, `lock-date`, `opening-balance-equity`, `card-clearing`,
      `wallet-clearing`, `shift-variance`, `drafts-isolated`, `allocations-within-total`,
      `fx-conversion`.
- [x] Tolerances (`closeEnough`: 0.01, balanced: 0.001) and the rounding points (per-account
      `round2` before summing in `checkTrialBalance`, `invariants.ts:113`) are copied exactly.
      `deltaMinorUnits` uses JS `Math.round` semantics (`floor(x + 0.5)`). `checkLockDate` and the
      trial balance use the **UTC** `LEFT(date_key, 10)`, as the mock does (`invariants.ts:281`).
- [x] It takes a connection, so it runs inside `with_read` (one RR snapshot) **and** inside the
      D10 importer's own write transaction before commit (cross-cutting §9).
- [ ] `architecture_rules`: only `src/shared/stock/**` (added by the C implementer, per the brief — not this diff) may write `products.{stock_qty,stock_value,cost_price}`,
      `product_branch_stock`, `product_batches.qty`, or insert `stock_movements`/`product_batches`.
- [ ] CLAUDE.md "Accounting safety" (added by the manager, per the brief — not this diff): add one sentence saying `shared::invariants` is the Rust port
      of `runAllInvariants` and that the two must keep the same keys until D5 retires the mock.

## Tests
- [x] The review A1 worked example (test written; ⏳ runs in the final build/test pass): 9 @ 50 (value 450), then a receipt of 10 @ 60 → qty 19, value
      1050, `cost_price = 55.2632`; sell 1 → value out 55.26; a return at the original cost of 50
      re-averages; after each step `GL(inventory) = Σ stock_value` (`inventory-gl` passes).
- [x] Selling the whole remaining stock (test written; ⏳ runs in the final build/test pass) takes exactly `stock_value`; the purchase-return variance guard; the branch rows always sum to the product totals.
- [x] FEFO: earliest expiry first (test written; ⏳ runs in the final build/test pass); undated last; insertion-order tie; expired skipped unless
      `allow_expired`; draws returned; a shortfall returns partial draws with no error; 2dp qty rounding.
- [x] Two concurrent `apply_change` calls (test written; ⏳ runs in the final build/test pass) on the same product serialise: the final qty, value and cost equal the sequential result.
- [x] Invariants: a scenario built only through `ledger::post` + `stock::*` + fixture document rows
      (an opening entry + a cash sale, both posted through `ledger::post`, with matching
      `stock::apply_change` calls) → `full_scenario_every_invariant_passes` asserts every one of the
      14 keys passes. Three targeted corruptions are covered (not all 14 — the remaining ones are
      exercised at the unit level in `mod.rs`'s/`cost.rs`'s/`batches.rs`'s own `#[cfg(test)]`
      blocks, not as DB-level corruption cases): a raw `UPDATE journal_lines SET debit = ...` fails
      `trial-balance`/`balance-sheet` while `balanced-entries` (which reads the entry header's own
      totals, untouched) still passes; a `stock_value` nudge fails only `inventory-gl`; a draft id
      copied straight into `journal_entries` fails `drafts-isolated`. A payment-allocation, closed
      shift and card-settlement scenario (mentioned in the spec) was not built — the two-document
      scenario above already exercises every checked table `run_all` reads, and building all four
      document kinds would need domain-level fixtures (invoice tenders, shifts, card settlements)
      this phase doesn't own; flagged for the parity harness (Part 04) to extend if a gap surfaces.

## Gate
- [ ] `cargo build` + `cargo test` green (`architecture_rules` includes D-5's rule). ⏳ runs in the single final build/test pass — not run this session.
- [ ] `bun run build`, `check`, `verify:mocks` (128/0), `contract:check`, `memory:check`, `diag:check`. ⏳ runs in the single final build/test pass — not run this session.
- [x] Status note at the top of this file.
