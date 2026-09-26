# 21 · Part 01 — Frontend analysis (the contract the Rust backend must honour)

> **Status (2026-09-26):** 01.A done. Tooling is in place and D3 rounding is applied to the mock.
> 01.B–01.D pending. 01.B needs D8 and D9 answered first (§4). Parts 02–04 are not written until
> this part's gate is green.

## 1. Purpose

Turn the frontend and its mock backend into a **written, generated contract**: for every one of the
339 service functions, what it takes, what it returns, what data it reads and writes, which shared
manager it needs (ledger / stock / activity / numbering / currency / period), and what the Rust
side does with it. Part 02 designs entities and `shared/` from this, and Part 03 ports domains from
it. Nothing here is typed from memory. The generator produces the facts and reviewers confirm
them.

**Source of truth, in order:** the mock backend's behavior (`src/mocks/backend/*`,
`docs/v2/02-accounting-review.md`) → the service function's signature and return shape
(`src/modules/<m>/services/*`) → the module types (`src/modules/<m>/types`). Pages are never read
for contract purposes, since they only consume services.

## 2. Inputs (all generated or already in the repo)

| Input | What it gives | How to refresh |
|---|---|---|
| [`docs/backend/contract/README.md`](../../../docs/backend/contract/README.md) | Index: per-module counts, shared-manager matrix, table read/write map, path-string links | `bun run contract` |
| `docs/backend/contract/<module>.md` | Every service fn: params, return type (inferred by the TS checker), suggested disposition, tables written/read (transitive), shared managers, DTO types, plus every module type with field hints (_decimal_, _uuid_, _date_, _route_, _enum_) | `bun run contract` |
| [`docs/backend/contract/mocks.md`](../../../docs/backend/contract/mocks.md) | All 240 mock-engine functions: tables read/written, mock calls, **round2/round4 counts** (the exact rounding points Rust must reproduce) | `bun run contract` |
| `docs/backend/contract/contract.gen.json` | Everything above, machine-readable (Part 04 diffs and checks against it) | `bun run contract` |
| `scripts/contract/config.ts` | Heuristic settings, capability entry points, and **`overrides`**, where every changed disposition is recorded with a reason | edit by hand |
| `AGENT_MEMORY.md` | Routes, module deps, IPC table, boundary report | `bun run memory` |

How the generator works (`scripts/contract/`: `extract → analyze → render → run`, all settings in
`config.ts`): it builds one TypeScript program over `src/`, finds every `wrap('<module>.<fn>', …)`,
and follows each function's calls **transitively through the checker**, including re-exports
through `@/mocks`. For each body it records `db.<table>` reads and writes (explicit array mutators,
references obtained via `find`/`at`, and anything inside a `mutate()` callback), non-db mock state
(`session`), `invoke('<cmd>')` calls, and browser/plugin APIs. **Writes are a lower bound**, since a
write through an object passed into a helper is not tracked. Reviewers confirm them in 01.B.
`bun run contract:check` fails when the generated files are stale.

## 3. What the first run found (2026-09-26)

**339** functions: **298 port**, **8 rust-existing** (they already `invoke` `render_pdf` /
`render_preview`), **33 frontend**. **46** MockDb tables, **254** module types, **240** mock-engine
functions, **61** path-string links.

| # | Finding (evidence) | Consequence |
|---|---|---|
| F1 | Shared managers are reached by: **ledger 42** fns, **stock 18**, **activity 74**, **numbering 46**, **period 42**, **currency 4** (README "Shared managers"). | That is the exact caller list for `shared::ledger/stock/activity`. Part 02 sizes and tests those modules against it. |
| F2 | **Templates are per-device.** All 11 `templateService` fns use `localStorage` (`templateService.ts:14`, key `pdf_templates_v1`). The same holds for insight dismiss/snooze (`insightEngine`) and backup history (`backupService`, IndexedDB). | Under D1, a second terminal would print with different templates. Decision **D9**. |
| F3 | **Terminal identity already exists.** `terminalId` is on held sales and shifts (`invoices/types/index.ts:163,208,229`). | The Rust `AppState` needs a stable terminal id per install. D8 decides how terminals reach the DB. |
| F4 | **Auth is a mock.** `session = { userId: '' }` (`mocks/db.ts:257`), and `db.credentials` holds plain-text passwords (`mocks/db.ts`). | Rust owns the session and stores password + manager-PIN hashes (argon2). The `authService` contract (login/restore/logout/verifyManagerPin) stays the same. |
| F5 | **Error codes** are a closed set: `NOT_FOUND` 92, `CONFLICT` 23, `FORBIDDEN` 16, `VALIDATION` 7, `UNAUTHORIZED` 2 (`mocks/utils.ts:52`). The UI shows the Arabic message. | `AppError` serializes to `{ code, message }` with the same five codes and the same Arabic messages (listed per module in 01.B). |
| F6 | **Change events** drive page refresh: `ledger:changed` (16), `catalog:changed` (19), `parties:changed` (11), consumed via `onLedgerChanged` / `onCatalogChanged`. | Rust emits the same three names as Tauri events. With several terminals, other terminals must learn about changes too (D8). |
| F7 | **61 activity/audit links are path strings** (e.g. `backend/journal.ts:160`, `backend/branches.ts:126`). They break CLAUDE.md rule 25 for service link fields. | Fix in the **mock first** (01.C), so the parity harness compares route objects on both sides. |
| F8 | **Server-side validation** exists only as frontend Zod schemas (`parties/validators/partySchema.ts`, `products/validators/productSchema.ts`, `users/validators/userSchema.ts`) plus `ApiError('…','VALIDATION')` checks inside the mock. | Rust must re-validate every command input. The Zod schemas and mock checks are the rule list. |
| F9 | **Dev-only surface:** `devToolsService.resetToEmpty/reloadDemoData`, and `accountingDebugService` (posting trace, invariants, drift, repro bundles). | These become debug-build-only Rust commands. A **snapshot importer** (MockDb JSON → MariaDB) serves demo data, the Part 04 parity harness and D10. |
| F10 | **Backup** (`backupService`) snapshots the IndexedDB MockDb through `plugin-fs`. | Under MariaDB, backup/restore moves to `infrastructure/backup`. The service contract stays, and the dispositions flip to `port` in 01.B. |

## 4. Decisions needed for this part

D1–D3 are answered and recorded in `00-MASTER-PLAN.md`. New decisions:

| # | Question | Recommendation | Blocks |
|---|---|---|---|
| D8 | **How does a terminal reach the data?** (a) Every terminal runs its own Rust backend, connected over the LAN to the Main PC's MariaDB. (b) Terminals call the Main PC's app through an HTTP API. | **(a).** No second server to build. MariaDB already gives transactions and row locks between terminals, and the Main PC just hosts the DB plus the future sync worker. Cost: numbering and stock need row locks (`SELECT … FOR UPDATE`), and cross-terminal refresh needs a small change-version table that terminals poll. | 01.B concurrency notes, 02-A |
| D9 | Should print **templates** be shared per branch (DB), or stay per device? | Templates go to the DB, shared per branch. Insight dismiss/snooze stays per device (personal UI state). | 01.B `templates`, `core` |
| D10 | Does anyone have **real data** in the current IndexedDB store that must move into MariaDB? | Build the snapshot importer anyway (F9). Ship it as a one-time "import from previous version" step only if the answer is yes. | 02, 04 |

## 5. Phases

| Phase | File(s) | What | Size | Status |
|---|---|---|---|---|
| 01.A | this file, `scripts/contract/*`, `scripts/verify/rounding.ts` | Tooling + D3 applied to the mock | M | done |
| 01.B | `01-frontend-analysis/<module>.md` × 16, from [`TEMPLATE.md`](01-frontend-analysis/TEMPLATE.md) | Per-module contract review | L | pending |
| 01.C | mock + types (see tasks) | Contract fixes in the spec (path links → route objects, anything 01.B finds) | M | pending |
| 01.D | `01-frontend-analysis/cross-cutting.md` | Auth/session, terminal id, errors, events, paging, dates/timezone, table → entity ownership map | M | pending |

### 01.A — Tooling and rounding (done)

- [x] `scripts/contract/` generator (`types`, `config`, `extract`, `analyze`, `render`, `run`) →
      `docs/backend/contract/*`, plus `bun run contract` and `contract:check` in `package.json`.
- [x] Landmarks for the rounding helper and the contract inventory in `scripts/memory/config.ts`.
- [x] **D3:** one rounding rule, `roundHalfAwayFromZero` / `round2` / `round4` in
      `src/modules/core/helpers/numbers.ts`. It rounds on the shortest decimal form, which matches
      `rust_decimal` `MidpointAwayFromZero`. Every copy now re-exports or imports it:
      `mocks/utils.ts`, `mocks/backend/core.ts` (`round4`), `mocks/backend/transfers.ts`,
      `mocks/backend/invariants.ts`, `invoices/helpers/totals.ts`, `scripts/verify/shared.ts`,
      `scripts/totals.spec.ts`. Inline `Math.round(x*100)/100` was replaced in `invoiceService`,
      `partyService`, `inventoryService`, `tafqit`, `ExpenseListPage`, `ProfitLossPage`,
      `CloseShiftDialog`, `PriceMatrix`.
- [x] Regression: a new `rounding` area in `bun run verify:mocks` (`scripts/verify/rounding.ts`)
      pins the cases the old code got wrong (−0.125 → −0.13, 1.005 → 1.01, 1234.565 → 1234.57,
      float noise) and checks that every exported `round2`/`round4` **is the same function**, so a
      future local copy fails the gate. Result: 128 ok / 0 failed on both SA and EG seeds, and
      `scripts/totals.spec.ts` passes 39/39. `build`, `check`, `contract:check`, `memory:check`
      and `diag:check` are green. **The e2e suite has not been run for this change.** It runs once
      at the end of the plan (master plan §8).

### 01.B — Per-module contract review

One file per module, copied from `01-frontend-analysis/TEMPLATE.md`. Order follows the master plan
build order, so Part 03 can start on the first module while later ones are still reviewed:

- [ ] `settings.md` (42) · [ ] `setup.md` (21) · [ ] `users.md` (9) · [ ] `approvals.md` (5)
- [ ] `parties.md` (16) · [ ] `products.md` (51) · [ ] `purchases.md` (12) · [ ] `invoices.md` (26)
- [ ] `payments.md` (7) · [ ] `vouchers.md` (11) · [ ] `expenses.md` (11) · [ ] `accounting.md` (33)
- [ ] `reports.md` (32) · [ ] `analytics.md` (3) · [ ] `core.md` (35) · [ ] `templates.md` (11, after D9)
- [ ] `diagnostics.md` (14): decide which read endpoints Rust serves in debug builds (F9)

For each module file, the reviewer must:
- [ ] Confirm or change every function's disposition. A change goes into
      `scripts/contract/config.ts` → `overrides` with a reason, followed by `bun run contract`.
- [ ] Confirm the write set against the mock code. The generator's writes are a lower bound.
- [ ] Map every DTO field to a Rust type and column type. Every _decimal_ gets a **scale**, taken
      from the mock's rounding point in `mocks.md` (2 = money, 4 = cost/qty/rate). Every
      _route_ becomes `RouteRef`. Every _enum_ becomes a Rust enum with serde `rename`.
- [ ] Fill the undo matrix: every write function is either **undoable via `<existing reversal fn>`**
      or **not undoable because …**. No new accounting semantics (master plan §3 rule 7).
- [ ] List validation rules (Zod schema + mock `VALIDATION`/`CONFLICT` checks) and error cases
      with their exact Arabic messages.
- [ ] Concurrency notes under D8: which rows two terminals can race on (numbering, stock, shift,
      allocations), and the lock or constraint that settles it.
- [ ] Reports, analytics, dashboard and insights modules only: for every output, the aggregation
      spec (source tables, filters, grouping, rounding point), so Part 03 writes SQL, not a loop.

### 01.C — Contract fixes in the spec (mock first, so the parity harness compares like with like)

- [ ] F7: turn the 61 path-string links in `src/mocks/**` and services into named route objects.
      Type the `link` fields on `ActivityEntry` / `AuditEntry` as `AppRoute`, update their
      readers, and get `bun run check` and `verify:mocks` green. When the inventory shows
      **0 path-string links**, this is done.
- [ ] Every other mismatch 01.B records under "Contract fixes needed in the mock". Each is one
      small change with `verify:mocks` green. An accounting one also ships a
      `scripts/verify/cases/*.json` case (CLAUDE.md "Accounting safety").

### 01.D — Cross-cutting contract (`01-frontend-analysis/cross-cutting.md`)

- [ ] Auth + session (F4): login/restore/logout/manager PIN, password hashing, role checks (the
      same `area` roles as `core/helpers/navigation.ts`).
- [ ] Terminal identity (F3) and what is per terminal (held sales, shift) vs. per branch.
- [ ] `AppError` catalogue (F5): code → HTTP-like meaning → how the UI shows it (`E-XXXX` toast).
- [ ] Events (F6): the three names, payloads, and the cross-terminal refresh mechanism (D8).
- [ ] Paging (`core/types/paging.ts` `PagedQuery`/`PagedResult`), sorting and search semantics
      (`core/helpers/search.ts` `matchesSearch` — Arabic normalization must match in SQL or Rust).
- [ ] Dates: `localDateKey` / `inDateRange` (`mocks/utils.ts`). Decide the timezone rule
      (business-local dates as `DATE`, instants as UTC `DATETIME(3)`).
- [ ] **Table → entity ownership map** for all 46 tables: owning module, which become child
      tables (invoice lines, journal lines, allocations), and which are settings blobs (`settings`,
      `counters`). This is the direct input to Part 02-B.

## 6. Gate (Part 01 is done when all of these hold)

- [ ] D8, D9, D10 answered and recorded in `00-MASTER-PLAN.md`.
- [ ] 16 module files + `cross-cutting.md` complete, with every checklist box ticked.
- [ ] Every service function has a confirmed disposition, either the heuristic's or an override
      with a reason.
- [ ] 01.C done: the inventory shows **0 path-string links**.
- [ ] `bun run contract:check`, `bun run verify:mocks` (0 failed), `bun run build`,
      `bun run check`, `bun run memory:check` and `bun run diag:check` are green.
- [ ] No open `ACC-` issue in `docs/diagnostics/ISSUES.md`, since we don't port a known wrong
      number.
