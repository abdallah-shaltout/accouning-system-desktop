# 21 — Real backend: Tauri + Rust + SeaORM + MariaDB (master plan)

> **Status (2026-09-27):** planning is closed. Decisions D1–D3 and D8–D10 are answered (§9).
> **Part 01 is fully done and its gate (`01-FRONTEND-ANALYSIS.md` §6) is green**: 01.A (contract
> generator + D3 rounding), 01.B (all 17 per-module contract reviews), 01.C (the shared F7
> path-string-link pass — 0 path-string links remain), and 01.D (`01-frontend-analysis/cross-cutting.md`
> — auth/session, terminal identity, the settings branch/device split, the `AppError` catalogue,
> events, paging/search, dates/timezone, the 46-table ownership map, and the D10 import spec) are all
> complete — see `01-FRONTEND-ANALYSIS.md`'s status notes for the full history of mock fixes,
> disposition overrides and cross-cutting design decisions. Every fast gate (`contract:check`,
> `verify:mocks`, `build`, `check`, `memory:check`, `diag:check`) was re-run fresh at the end of the
> 01.D session and is green; no open `ACC-` issue exists. **Part 01 is not yet moved to
> `plans/completed/`** — that move, and writing Part 02, are left to the user/orchestrating session
> (out of 01.D's own scope). Parts 02–04 are written one at a time, each after the part before it
> passes its gate, so every plan file uses real findings instead of guesses.

## 1. Goal

Replace the in-browser mock backend (`src/mocks/`) with a real Rust backend inside the Tauri app,
**without editing a single page or component**. The Vue screens stay the source of truth for what
the backend returns. The mock's accounting engine stays the source of truth for what the numbers
must be. We move one domain at a time, and every step is checked against the mock by an automated
parity harness. There is no big-bang switch.

## 2. What the repo already gives us (and why it shapes this plan)

| Fact (from `AGENT_MEMORY.md` / code) | Consequence for the plan |
|---|---|
| Pages call only `modules/*/services/*`. **339** service functions, all wrapped with `wrap('<module>.<fn>', …)` (`diagnostics/services/defineService.ts`). Boundary report: **0** seam violations. | The seam already exists. The backend swap happens **inside service files only**. The `wrap` registry gives us a machine-readable list of every endpoint to port. |
| `src/mocks/backend/*` (20 files, ~5.7k lines) is a full posting engine: `postJournal`, `resolvePosting`, `assertOpenPeriod`, `applyStockChange`, FEFO batches, weighted-average cost, FX, shifts, settlements. | This is the **executable spec** for `shared/` and `domains/`. We port its behavior. We don't redesign it. Posting rules come from `docs/v2/02-accounting-review.md`. |
| `backend/invariants.ts` → `runAllInvariants()` (14 invariants). `bun run verify:mocks` checks them, and `scripts/verify/replay.ts` replays recorded service calls (`ReproBundle`). | The same bundles can run against **both** backends and be compared field by field. That parity harness is how we prove financial accuracy (Part 04). |
| `MockDb` has **46** tables with types in each module's `types/`. | This is the entity list for Part 02. Every table maps to an entity or is dropped with a stated reason. |
| Business audit already exists: `logAudit` / `logActivity` (`backend/core.ts:331-413`) → `AuditEntry { entity, entityId, action, before/after diffs, userId, branchId, at, reason, message, link }` + the `activity` feed + `partyHistory`. | The Rust `ActivityTracker` **emits this exact shape** and adds `payload`, `is_undoable`, `undo_of`, `undone_by`. The audit log page and dashboard feed keep working unchanged. |
| Reversal is already domain-specific: `reverseJournal` only reverses MANUAL entries. Automatic entries "are reversed from the source document (refund/cancel)" (`backend/journal.ts:133`). The frontend also has `createRefund`, `cancelPurchaseOrder`, `reversePartyOpening`, `removeAllocation`, `rejectTransfer`, `reopenYear`. | "Undo" = calling the **existing** compensating operation for that document type through an undo registry. We invent no new accounting semantics, and nothing is ever hard-deleted. |
| Money is JS `number` + `round2`, cost/qty/rate use `round4`. Before D3 there were ~12 copies written as `Math.round(n + EPSILON)`, which rounded a negative half **toward +∞** (−0.125 → −0.12). | **Fixed in 01.A (D3):** one rule in `src/modules/core/helpers/numbers.ts` (half away from zero, same as `rust_decimal` `MidpointAwayFromZero`). `verify:mocks` → `rounding` pins it. `rust_decimal` must round at the **same points** (listed in `docs/backend/contract/mocks.md`). |
| IDs are deterministic prefixed strings (`inv-12`, `uid()` in `mocks/utils.ts:72`). Document numbers come from `nextNumber` (`mocks/db.ts`). | UUIDs change the id format. The frontend treats ids as opaque strings, so screens are safe. The parity harness must map mock id ↔ UUID. Document numbers (`INV-0001`) stay a separate sequence. |
| Existing Rust: `diag/`, `pdf/` (Typst), `print/` (ESC/POS). 12 IPC commands, contract gaps `greet` and `render_pdf_spike`. `tauri-plugin-sql` (SQLite) is registered but never used from the frontend. | `pdf/` and `print/` move into `infrastructure/` with **unchanged command names**. `diag/` goes to `core/`. `greet` and `render_pdf_spike` get deleted. `tauri-plugin-sql` gets removed, since SeaORM replaces it. |
| e2e (`scripts/e2e/run.py`) drives the **browser** on `localhost:1420`, and there Tauri `invoke` does not exist. | The mock stays as the browser/e2e backend until a Tauri-driven e2e path exists (Part 04). Cutover is per domain, behind one switch. |
| No sync feature exists anywhere in the frontend (no `syncStatus` in any type). | `sync_status` gets added as a column (rule 5) with a local-only default. A sync API has **no frontend requirement yet**, so it stays out of scope (decision D6). |
| Service link fields must be named route objects (CLAUDE.md rule 25), but some mock links are still path strings (e.g. `backend/journal.ts:160`). | Rust DTOs emit `RouteRef { name, params }`. Part 01 lists every path-string link as a contract fix. |

## 3. The architectural rules (acknowledged and pinned to this repo)

These are **binding for every file in Parts 01–04.** Each one names where it lives in code.

1. **Frontend-driven contracts.** Every Rust DTO is derived from an existing TS type in
   `src/modules/<m>/types` and the return shape of a service in `src/modules/<m>/services`. A
   generated TS binding (tauri-specta or ts-rs, chosen in Part 02) is **type-checked against the
   existing TS type** by `vue-tsc`, so any drift fails `bun run build`.
2. **No repository pattern.** `domains/<d>/service.rs` uses SeaORM entities directly. All entities
   live in `entities/`.
3. **Centralized operations in `shared/`.** Only `shared::ledger` posts journal entries. Only
   `shared::stock` changes quantities, batches or average cost. Only `shared::activity` writes
   audit/activity. Any other code that touches those tables is a review failure.
4. **One DB transaction per command.** Every `shared::*` function takes `&impl ConnectionTrait`
   (the caller's `DatabaseTransaction`) and **never opens its own**. The command handler begins
   the transaction, calls the domain service, and commits. A document, its journal, its stock
   movements and its audit row commit together or not at all.
5. **Zero floats.** `rust_decimal::Decimal` everywhere money, qty, cost, rate or percentage
   appears, and `DECIMAL(p,s)` columns in MariaDB. Rounding goes through one module
   (`utils::money`) with `round2` / `round4` = `round_dp_with_strategy(_, MidpointAwayFromZero)`,
   the same rule as `core/helpers/numbers.ts` (D3): `round2` for money **and quantity** (the mock
   rounds quantities to 2 dp — corrected in Part 02, C-03), `round4` for cost/rate. A CI grep
   fails on `f32` / `f64` in `entities/`, `shared/`, `domains/`.
6. **Balanced or refused.** `shared::ledger::post` checks Σdebit = Σcredit (after rounding),
   the open period (`assert_open_period`, ported from `backend/core.ts:103`) and the account
   rules **before** any insert. It returns `AppError::unbalanced(…)` (code `VALIDATION`) /
   `AppError::period_locked_*` (code `FORBIDDEN`) — the mock's exact codes and Arabic messages
   (corrected in Part 02, C-01) — and never "fixes" numbers.
7. **Undo by compensation.** Every state-changing command records an activity row. If its action
   type is registered as undoable, undo calls that type's **existing** reversal operation (§2)
   inside a new transaction and links both rows (`undo_of` / `undone_by`). Nothing gets deleted.
   Soft delete (`deleted_at`) is only for master data (products, parties, accounts…), never for
   posted documents.
8. **Keys and metadata.** **UUIDv7** primary keys (D2), stored in MariaDB's native `UUID` type
   (≥ 10.7), plus `created_at`, `updated_at`, `deleted_at`, `sync_status` on every syncable table.
   Document numbers (`INV-0001`) stay a separate per-branch sequence.
9. **Analytics live in Rust.** Reports, dashboard KPIs, insights and analytics return
   ready-to-render DTOs (same shapes as `reportService` / `analyticsService` /
   `dashboardService` today). Aggregation is SQL `GROUP BY` + `Decimal`. Summary tables only get
   added when a perf-baseline measurement asks for them.
10. **Repo rules still apply.** Commands are registered in `generate_handler!` and called only from
    a module **service** (CLAUDE.md "Rust ↔ Vue"). Errors carry Arabic user messages plus a code
    that flows into `log.error` and the `E-XXXX` toast code. `bun run memory` runs after every
    structural change.

## 4. Target Rust layout (`src-tauri/src/`)

```text
core/            db.rs (MariaDB pool, migrations run on boot), error.rs (AppError/AppResult →
                 serializable {code, messageAr, details}), state.rs (AppState, current user/branch),
                 tx.rs (with_tx helper), diag/ (moved from src/diag, commands unchanged)
entities/        one SeaORM entity per table (+ migration crate `src-tauri/migration/`)
shared/          ledger/ (post, reverse, resolve_account, assert_open_period, posting_trace)
                 stock/ (apply_change, receive_batch, consume_fefo, weighted_avg_cost)
                 activity/ (record, undo registry, diff_fields)
                 numbering.rs (document sequences), currency.rs (to_base, require_rate),
                 balances.rs (read-only party balances/statements, P2-26)
                 invariants/ (Rust port of the 14 invariants — used by tests + diagnostics tab)
domains/<d>/     commands.rs (IPC, thin) · service.rs (logic) · dto.rs (serde, camelCase, = TS type)
                 <d> = the Vue module names, 1:1: accounting, analytics, approvals, dashboard,
                 diagnostics (its 3 production `port` fns — Part 02 C-10), expenses, invoices,
                 parties, payments, products, purchases, reports, settings, setup, templates,
                 users, vouchers
infrastructure/  database/ (bundled MariaDB: payload, provisioning, supervisor — D11), pdf/ (moved),
                 print/ (moved), backup/ (DB export/restore), sync/ (placeholder — D6)
utils/           money.rs (round2/round4 equivalents), dates.rs (local_date_key, ranges), text.rs
```

Domain names match the Vue modules exactly, so `invoiceService.createSale` maps to
`domains/invoices::create_sale`. That makes every lookup a one-step search for an agent.

## 5. How the frontend switches (strangler, per domain)

- One switch point in `src/modules/core/services/` (built in Part 02, phase F) decides per domain
  whether a service calls the mock or `invoke('<domain>_<fn>')`. **Pages never change.**
- A domain flips to Rust only after its parity cases pass (Part 04). Browser dev and the e2e suite
  keep using the mock until Tauri e2e exists. What happens to the mock after full cutover is D5.
- Frontend-only services stay frontend-only: `logService`, `actionJournal`, `saveFile`, geo,
  appearance. Part 01 marks each of the 339 functions as **port** / **stay-frontend** / **drop**.

## 6. Parts and files

Each part has an UPPERCASE entry file (overview, checklist, gate). When a part is too large for
one agent session, its details go into a lowercase sibling folder with **one self-contained file
per unit**. Each such file lists the exact TS files to read, the Rust files to create, and its own
gate, so an agent can execute one file without loading the others.

| # | Entry file | Split into | What | Size | Status |
|---|---|---|---|---|---|
| 00 | `00-MASTER-PLAN.md` | — | This index: rules, layout, order, decisions | S | done |
| 01 | [`01-FRONTEND-ANALYSIS.md`](01-FRONTEND-ANALYSIS.md) | `01-frontend-analysis/<module>.md` × 17 + `cross-cutting.md` | Contract from code: the `bun run contract` generator (→ `docs/backend/contract/`) plus a per-module review (disposition, DTO → Rust types, undo matrix, validation, concurrency, aggregations) and spec fixes in the mock | L | done (01.A–01.D; gate §6 green 2026-09-27) |
| 02 | [`02-CORE-AND-SHARED-ARCHITECTURE.md`](02-CORE-AND-SHARED-ARCHITECTURE.md) | [`02-core-and-shared/phase-a…f.md`](02-core-and-shared/phase-a-foundation.md) | A foundation (workspace, MariaDB pool, infra migration, `AppError`, `AppState`, `with_tx`) · A2 bundled MariaDB server (D11) · A3 Main-PC LAN hosting · B entities + migrations for the 46 tables (+ `print_templates`) · C `shared::ledger` + numbering + currency · D `shared::stock` + `shared::balances` + `shared::invariants` · E `shared::activity` + undo registry · F IPC bridge (ts-rs bindings, `backendCall`, frontend switch, change poller) | L | **Code complete** (all phases A–F written, 2026-09-28); final test pass paused — see the entry file's ⏸ note |
| 03 | [`03-DOMAINS-IMPLEMENTATION.md`](03-DOMAINS-IMPLEMENTATION.md) | `03-domains/<nn>-<domain>.md` | One file per domain, in dependency order (§7): DTOs, service, commands, undo matrix, parity cases. Reports/analytics engine last. | XL | **Code complete** (2026-09-29): 306 commands, all gates green; DB tests written, not run |
| 04 | [`04-INTEGRATION-AND-AUDIT.md`](04-INTEGRATION-AND-AUDIT.md) | [`04-integration-and-audit/phase-a…e.md`](04-integration-and-audit/phase-a-contract-checks.md) | A static contract checks · B parity harness (the real TS services run on mock vs Rust via a `tauri::test` host, diff DTOs + books) · B2 ~160 parity cases in 5 parallel domain lanes + fix loop · C Rust invariants on every test DB · D Tauri e2e path (CDP; run in the final testing plan) · E **single cutover** (all domains flip together — P4-1 reverses §1/§5's per-domain cutover, see the entry file §3) + final audit | L | planned (2026-09-29) |

## 7. Build order (dependencies, not preference)

```text
01 analysis ─► 02 A ─► 02 A2 (bundled DB; unblocks every DB test) ─► 02 A3 (LAN hosting)
          ─► 02 B (entities) ─► 02 C-E (ledger, stock, activity) ─► 02 F (bridge)
          ─► 03 domains: settings/setup → users/approvals → parties → products (catalog, inventory,
             transfers) → purchases → invoices (sales, POS, shifts, quotations) → payments →
             vouchers (incl. card settlements) → expenses → accounting (journal, FY close, VAT)
             → reports → analytics/dashboard/insights → templates/backup
          ─► 04 runs alongside from the first domain (each domain's parity cases gate its cutover)
```

Reason for the order: every later domain posts through accounts, parties, products and settings
that the earlier ones create. Reports come last because they only read what the others wrote.

## 8. Definition of done (whole plan)

> **e2e timing (user decision, 2026-09-26):** the full e2e suite runs **once, when everything in
> this plan is finished**, not at the end of each phase or part, because it takes too long. Phase
> gates use the fast checks only (`build`, `check`, `verify:mocks`, `contract:check`,
> `memory:check`, `diag:check`, and later `cargo test` + the parity harness). This overrides
> CLAUDE.md's "full suite at the end of every phase" for plan 21 only.
>
> **Testing plan (user decision, 2026-09-27):** all real testing — the full e2e suite, real
> `bun run desktop` checks, `bun tauri build` installer checks, clean-Windows-VM install/uninstall
> and second-PC LAN checks — goes into **one separate testing plan written at the very end**,
> outside Parts 01–04. Phase gates list those items as "deferred to the final testing plan"
> instead of blocking on them. Automatable checks (`cargo build`, `cargo test` against
> `bun run db:dev`, and the fast `bun run` gates) still run in every phase gate.

- Every service function marked **port** in Part 01 runs on Rust, and every domain is flipped.
- Parity harness: all `scripts/verify/cases/*.json` plus the per-domain parity cases give
  **identical** DTOs (ids mapped) and identical GL/stock/party balances on mock vs Rust.
- The Rust port of the 14 invariants is green on every test database. `cargo test` is green.
- `cargo build`, `bun run build`, `bun run check`, `bun run verify:mocks`, `bun run memory:check`
  and `bun run diag:check` are green, the full e2e suite is green, and a real `bun run desktop`
  pass has been done on Windows against MariaDB.
- `AGENT_MEMORY.md` shows the new Rust commands in the IPC table with **no contract gaps**.
- CLAUDE.md is updated wherever this plan changes a rule. Examples: "UI-only against the mock
  backend", the single copy of the invariants moving to Rust, the backup path.

## 9. Decisions

### Answered (2026-09-26)

| # | Decision | What it means in the plan |
|---|---|---|
| D1 | **Branch Master Server.** Each branch has one Main PC that runs the app and hosts the branch's MariaDB. Other devices in the branch connect to it over the LAN. The branch works 100% offline. Later, only the Main PC syncs with the central cloud server (Coolify). | One MariaDB per branch. Installer and backup target the Main PC. `sync_status` exists from day one. The sync worker (Main PC only) is a later part. How terminals reach the DB is D8. |
| D2 | **UUIDv7** for every primary key. | Rule 8 above. Time-ordered inserts, no clustered-index fragmentation, collision-free across branches for the cloud sync. |
| D3 | **Half away from zero**, in both backends. | Applied to the mock in 01.A before any porting (rule 5 above). |
| D8 | **Every terminal runs its own Tauri/Rust app**, which connects straight to the Main PC's MariaDB over the LAN (a connection string). There is no intermediary HTTP server. Concurrency relies on MariaDB connections and row-level locks. | 02-A: a connection-string setting (Main PC = `127.0.0.1` — refined from `localhost` in Part 02 C-23 —, terminals = the Main PC's LAN address), a pool per app. Races between terminals are settled in the DB: numbering and stock rows use `SELECT … FOR UPDATE` inside the command's transaction, and unique constraints guard document numbers. Cross-terminal refresh (`ledger:changed` etc.) comes from a small change-version table that each app polls. |
| D9 | **Print templates move to the database, shared per branch.** Every cashier prints the same formats. | The 11 `templateService` functions are `port` (overrides in `scripts/contract/config.ts`). A `print_templates` entity per branch. The existing `localStorage` templates (`pdf_templates_v1`) are imported once by the D10 importer. Insight dismiss/snooze stays per device (personal UI state). |
| D10 | **Real/beta data exists in browser storage.** Build the snapshot importer, and ship it. | One importer (MockDb snapshot JSON → MariaDB, ids remapped to UUIDv7 with a kept mapping) does three jobs: (1) a one-time "import from the previous version" step shipped to users, reading the IndexedDB snapshot (`src/mocks/persist.ts`, DB `mock-db`, store `snapshot`, key `current`, schema `version`) plus `localStorage` templates; (2) demo data in dev; (3) the Part 04 parity harness. It must be idempotent, run in one transaction, and end with the Rust invariants green, or roll back. |
| D11 | **Bundled, app-managed MariaDB on the Main PC** (user instruction 2026-09-27: "زي ما عملنا مع Webview كل دا هينزل مع التطبيق"). Every installer carries a pinned MariaDB 11.4 LTS no-install payload, the same way it carries the WebView2 fixed runtime (`scripts/fetch-webview2.js`). Choosing "Main PC" in the app provisions and runs it. Terminals never run it. No manual DB install, no Windows service, and no admin rights, except one explained UAC prompt when LAN sharing is turned on. | Part 02 phases A2 (payload, installer, provisioning, supervision, upgrades, uninstall safety, dev/test DB) and A3 (LAN sharing, firewall, pairing, tray/autostart). Business data lives in `%ProgramData%\com.abdallah.accounting-app\database\` and is never deleted by the app or its uninstaller. Backup reads through SQL (no `mariadb-dump`). Embedded SQLite was considered and not reopened: D8's multi-terminal LAN access needs a real database server, and bundling is solved for MariaDB. |
| D4 | **Undo = backend capability only** (adopted 2026-09-27 in Part 02 per the recommendation, architectural autonomy — C-04). Registry + compensation + `undo_of`/`undone_by` links; **no undo IPC command** until a later small UI plan. | 02-E builds the mechanism (`shared::activity::undo`); Part 03 registers the five compensators listed in phase-e E-5. |
| D7 | **Undo in a closed period: refused unless admin** (adopted 2026-09-27 in Part 02 per the recommendation). Compensators pass `allow_closed_period = actor is admin`, matching `reverseJournal`'s `allowClosedPeriod: isAdmin` today. | 02-E `undo()` + every Part 03 compensator. |

### Open

| # | Question | Recommendation | Blocks |
|---|---|---|---|
| D5 | **Mock after cutover.** Keep it for browser dev/e2e (kept equal by the parity harness) or delete it? | Keep until Tauri e2e covers the full suite, then delete. | 04-E |
| D6 | **Sync payload.** What the Main PC sends to and receives from the cloud. | Column only (`sync_status`) now. The sync design becomes its own part once the cloud API exists. | none now |

## 10. Risks we plan for

- **The spec has bugs.** 6 open `BUG-` issues exist (none `ACC-`). Part 01's gate requires
  `verify:mocks` green and no open `ACC-` issues, so we don't port a known wrong number.
- **Float → decimal drift.** Handled by D3, the single `utils::money`, and the parity harness
  diffing at the cent.
- **Scope creep.** Anything not reachable from an existing service function goes to a "later"
  note in its part, not into the tasks.

## Next step

**Part 01 is done** (gate green). **Part 02 is code complete** (2026-09-28): phases A, A2, A3, B,
C, D, E and F are all written and the whole workspace compiles with no warnings. Its single final
test pass was **paused by the user** ("testing takes too long — continue development, we'll come
back to it"): lib unit tests 137/137, `architecture_rules` 8/8 and `db_foundation` 7/7 are green
against the real bundled MariaDB, ~20 real bugs found by that pass are fixed, and the remaining DB
suites, the server suites and the fast `bun run` gates are still to run. The exact resume steps
are in the ⏸ note at the top of
[`02-CORE-AND-SHARED-ARCHITECTURE.md`](02-CORE-AND-SHARED-ARCHITECTURE.md).

1. **Next real work: Part 03** (domains). User decision 2026-09-28 overrides the earlier "write
   Part 03 only after Part 02's gate is green": development continues while Part 02's test pass
   waits. Start from Part 02's handoff list (entry file §9; the D10 importer comes first).
2. Part 02's test pass resumes as its own step later (one test binary at a time, bounded in time);
   only then set row 02 in §6 to `done`.
3. Decisions still open for the user (entry file §4): the added `INTERNAL` error code (C-02) and
   the attachment-storage question (C-16, not blocking).
4. Use only the fast gates between steps (now including `cargo test` against MariaDB and
   `bun run bindings:check`). The full e2e suite runs once, at the very end (§8).
