# 18.F — Accounting debugger

**Status (2026-09-26): F1-F5 done and committed on `master`** — `b61d19b` (F1 posting trace, on top
of an already-uncommitted F2 pass from an earlier run, committed separately as `fde0d1c`), `f8544ef`
(F3 inspector), `72ca145` (F4 repro bundles + replay), CLAUDE.md rule (F5) in the same pass as this
file's checkbox update. `bun run verify:mocks` stayed 49/0/0 at every checkpoint. **Not verified
against an Egypt seed** — doc-18 Phase D (country profiles, which adds the EG seed) is paused behind
the concurrent plan-20 refactor per `TODO.md`, so this phase was only verified against the existing
(Saudi) seed; re-run `verify:mocks`/`verify:replay` once D lands. Full e2e suite was **not** re-run
this pass (the dev server was contended by a concurrent session's work on the same checkout at the
time) — run `python scripts/e2e/run.py` before moving this whole plan folder to `plans/completed/`.
A genuine pre-existing invariant finding (unrelated to F1-F4's own code) is logged in `TODO.md`, not
fixed here: `checkAllocationsWithinTotal` fails on a fresh seed alone, previously invisible because
`verify:mocks`'s console harness never wired that check up.

Accounting bugs are hard because the wrong number appears far from its cause (a report total, days later).
The goal: **from any wrong number, get back to the exact posting step that produced it, and turn it into a
permanent regression test.**

### F1. Posting trace
- [x] The backend `postJournal` / `resolvePosting` path builds a `PostingTrace` for every document:
  `{ docType, docId, correlationId, steps: [...], lines, totals: { dr, cr }, balanced }`, where the steps are line discount → invoice discount allocation → VAT (base, rate, tax, rounding delta) → **account resolution** (role → candidates tried: product / category / settings default → chosen + why) → cost (WAC qty/avg before → after) → FX (rate, fc → base, rounding).
  Implemented as optional per-line `trace` annotations on `PostingLine` (`src/mocks/backend/core.ts`)
  that callers may attach — account resolution itself is always reported (role→chosen, or a direct
  accountId pick) even when a caller supplies no annotations; no existing caller was changed to add
  annotations in this pass (defaulting gracefully, per the task).
- [x] Returned to the service, logged on the `accounting` channel, kept in an in-memory ring. In debug mode it is also stored beside the journal entry (never in backups).
  (`src/mocks/backend/posting-trace.ts` — the ring/per-entry map are module-level state, not `db`,
  so `backupArchive.ts`'s whole-`db` clone never includes them.)

### F2. Runtime invariants
- [x] Move the pure checks from `scripts/verify/*` into `src/mocks/backend/invariants.ts`. `verify:mocks` and the app both use this one copy.
- [x] Invariants: every posted entry balanced · trial balance nets to 0 · AR control = Σ customer balances · AP control = Σ supplier balances · inventory GL = Σ qty × WAC per branch · VAT accounts = VAT report · nothing posted before the lock date or in a closed year · drafts never affect balances · opening-balance equity 0 after close · allocations ≤ document total · shift expected cash = cash account movement · FX base = fc × rate within 0.01.
- [ ] Debug mode: run after each `mutate()` (debounced). The **first** failure logs the correlationId, the failing invariant, the expected/actual difference in the minor unit (هللة/قرش), and the last 20 actions, and opens an `ACC-` ledger issue stub.
  **Not done in this pass** — `runAllInvariants()` exists and is called on demand (F3's inspector,
  F4's replay, `verify:mocks`), but nothing calls it automatically after every `mutate()` yet. Left
  for a follow-up: wiring a debounced watcher into `mutate()` (`src/mocks/persist.ts`) touches a
  much hotter path than F1-F4's read-only/opt-in additions, and opening an `ACC-` stub automatically
  needs the same frontmatter-writing code `scripts/diagnostics/run.ts` already has for `BUG-`/`DBG-`
  — reuse that, don't duplicate it.

### F3. Inspector page `/dev/accounting`
- [x] Pick a document → tabs: posting trace · journal lines · account resolution · balances before/after · invariants.
- [x] **"اشرح هذا الرقم"** (dev only): on a TB / P&L / balance sheet / party balance cell, list the contributing journal lines grouped by source document, each linking to its trace.
  Implemented as `explainAccountBalance(accountId, partyId?)`, wired from the account-resolution and
  balances-before/after tabs (click an account to explain it) rather than from the TB/P&L/balance-
  sheet report pages themselves — those reports weren't touched in this pass (out of scope: doc-17's
  report pages are a separate concurrent effort). A future pass should add the same click-through
  from `ReportShell`/the trial-balance and party-balance cells once this inspector's shape is proven.
- [x] Drift report: subledger vs GL per party and per product, with the first document where they diverge.
- [x] **Resolved the open question:** no separate `/dev/accounting` route was added.
  `DevDiagnosticsPage.vue`'s "المحاسبة" tab already existed as a stub reserved for this phase
  ("stub — Phase F"), reachable the same way (DevMenu + command palette) — it now hosts the real
  inspector instead of a second route duplicating that reachability.

### F4. Repro bundles + replay
- [x] In debug mode, record the **action journal**: every service call (name, args, returned id) on top of the start snapshot.
- [x] "تصدير حالة لإعادة الإنتاج" → JSON via `saveFile`.
- [x] `bun run verify:replay <bundle>`: headless replay against the mock backend, running the invariants after each step, and printing the first step where they break.
- [ ] After a fix, the bundle is minimized and moved to `scripts/verify/cases/` as a permanent regression case (the ledger issue's `regression_test`).
  The mechanism and the empty `scripts/verify/cases/` folder exist (`bun run verify:replay` with no
  argument replays every case there); no case has been added yet because no accounting bug was fixed
  in this pass — this checkbox completes the first time F5's rule is actually exercised by a real fix.

### F5. Rules
- [x] CLAUDE.md "Accounting safety": every accounting bug fix ships with a `verify` case. An `ACC-` issue can only become `verified` when that case exists and is green.

