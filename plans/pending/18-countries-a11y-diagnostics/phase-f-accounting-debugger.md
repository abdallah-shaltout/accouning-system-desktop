# 18.F — Accounting debugger

Accounting bugs are hard because the wrong number appears far from its cause (a report total, days later).
The goal: **from any wrong number, get back to the exact posting step that produced it, and turn it into a
permanent regression test.**

### F1. Posting trace
- [ ] The backend `postJournal` / `resolvePosting` path builds a `PostingTrace` for every document:
  `{ docType, docId, correlationId, steps: [...], lines, totals: { dr, cr }, balanced }`, where the steps are line discount → invoice discount allocation → VAT (base, rate, tax, rounding delta) → **account resolution** (role → candidates tried: product / category / settings default → chosen + why) → cost (WAC qty/avg before → after) → FX (rate, fc → base, rounding).
- [ ] Returned to the service, logged on the `accounting` channel, kept in an in-memory ring. In debug mode it is also stored beside the journal entry (never in backups).

### F2. Runtime invariants
- [ ] Move the pure checks from `scripts/verify/*` into `src/mocks/backend/invariants.ts`. `verify:mocks` and the app both use this one copy.
- [ ] Invariants: every posted entry balanced · trial balance nets to 0 · AR control = Σ customer balances · AP control = Σ supplier balances · inventory GL = Σ qty × WAC per branch · VAT accounts = VAT report · nothing posted before the lock date or in a closed year · drafts never affect balances · opening-balance equity 0 after close · allocations ≤ document total · shift expected cash = cash account movement · FX base = fc × rate within 0.01.
- [ ] Debug mode: run after each `mutate()` (debounced). The **first** failure logs the correlationId, the failing invariant, the expected/actual difference in the minor unit (هللة/قرش), and the last 20 actions, and opens an `ACC-` ledger issue stub.

### F3. Inspector page `/dev/accounting`
- [ ] Pick a document → tabs: posting trace · journal lines · account resolution · balances before/after · invariants.
- [ ] **"اشرح هذا الرقم"** (dev only): on a TB / P&L / balance sheet / party balance cell, list the contributing journal lines grouped by source document, each linking to its trace.
- [ ] Drift report: subledger vs GL per party and per product, with the first document where they diverge.

### F4. Repro bundles + replay
- [ ] In debug mode, record the **action journal**: every service call (name, args, returned id) on top of the start snapshot.
- [ ] "تصدير حالة لإعادة الإنتاج" → JSON via `saveFile`.
- [ ] `bun run verify:replay <bundle>`: headless replay against the mock backend, running the invariants after each step, and printing the first step where they break.
- [ ] After a fix, the bundle is minimized and moved to `scripts/verify/cases/` as a permanent regression case (the ledger issue's `regression_test`).

### F5. Rules
- [ ] CLAUDE.md "Accounting safety": every accounting bug fix ships with a `verify` case. An `ACC-` issue can only become `verified` when that case exists and is green.

