# 21 · 01.B — `diagnostics` contract

> **Status:** done (2026-09-27) · **Inventory:** `docs/backend/contract/diagnostics.md`
> (regenerate with `bun run contract`) · **Mock spec:** none — no dedicated `src/mocks/backend/*`
> file; every function reads `db.*`/the in-memory posting-trace ring directly through `@/mocks`'s
> public surface · **Services:** `src/modules/diagnostics/services/{accountingDebugService,
> auditService,supportBundleService}.ts` · **Types:** `src/modules/diagnostics/types/index.ts`
>
> **The last module in the 01.B build order** (the master plan's own build-order list ends here;
> `templates.md` was reviewed just before this one but is not actually last, despite an earlier
> status-note in this file briefly saying so — corrected there). This module is the read-side for
> CLAUDE.md's "Diagnostics" section (18.B/18.F): the accounting debugger (`/dev/diagnostics` →
> المحاسبة), the business-audit-log reader, and the "تصدير ملف التشخيص" support-bundle export.
> **Zero writes anywhere in this module** — every one of the 14 functions is a pure read (or, for
> the 3 repro-recording functions, a read/toggle of an **in-memory, session-local** ring buffer that
> itself never touches `db`). This task's own line in `01-FRONTEND-ANALYSIS.md` names its job
> explicitly: **"decide which read endpoints Rust serves in debug builds (F9)"** — that decision is
> this file's main deliverable, more than any mock-code fix.

## 1. Endpoints

| Function | Disposition (confirmed / changed + why) | Rust command | Request DTO | Response DTO | Writes (confirmed) | Shared | Undo | Notes |
|---|---|---|---|---|---|---|---|---|
| `getAuditEntries` | port (confirmed) — **NOT dev-only**, see rationale below | `diagnostics_get_audit_entries` | `{ filter?: AuditFilter }` | `AuditEntry[]` | — | — | n/a (read) | Reads `db.audit` (the business audit trail — `logAudit`/`logActivity`'s output, written by every other module's posting/mutation code, never by this module). **This is a production feature, not a dev tool**: the audit log page (`/dev/diagnostics`'s "Audit" tab, but also surfaced wherever the app shows "who did what" — CLAUDE.md explicitly separates business audit from diagnostic logging: "it is business data... and is never a diagnostic log"). Filters by `userId`/`entity`/`action`/date range/free-text search (over `message`/`entityId`/`entityLabel`). |
| `getAuditEntities` | port (confirmed) — **NOT dev-only** | `diagnostics_get_audit_entities` | — | `string[]` | — | — | n/a (read) | Distinct `entity` values seen in `db.audit`, sorted — feeds the audit-log filter dropdown. Deliberately derived from live data rather than a hard-coded list "that would drift from what the backend actually writes" (source comment) — Rust's version should be a `SELECT DISTINCT entity FROM audit ORDER BY entity`, not a maintained enum, to preserve that same guarantee. |
| `getAuditEntityKinds` | **not a separate endpoint** — a plain re-export alias (`export const getAuditEntityKinds = getAuditEntities`) for call sites written against either name | (same command as `getAuditEntities` — no second Rust command needed) | — | — | — | — | n/a | Confirmed via `AGENT_MEMORY.md`'s "Unwrapped service exports" list in an earlier session read (`diagnostics.getAuditEntityKinds` appears there) — not one of the 14 `wrap()`-registered functions, listed here only for completeness so a future reader doesn't go looking for a 15th endpoint. |
| `exportSupportBundle` | port (confirmed) — **NOT dev-only** | `diagnostics_export_support_bundle` | `{ options?: { includeDbSnapshot?: boolean } }` | `string \| true \| null` | — | — | n/a (read + local file write, not a `db` write) | The "تصدير ملف التشخيص" production support feature (Settings → حول/الدعم). Builds a zip (app version, OS string, redacted `settings`, every log channel's `.jsonl` entries, and an **explicit opt-in only** DB snapshot) and saves it via the shared `saveFile` helper (CLAUDE.md rule 21 — native Save dialog). **The actual log-file reading (`exportAll()` from `diagnosticsService.ts`) and the OS/app-version detection are frontend concerns** (reading `.jsonl` files off disk via `plugin-fs`, `@tauri-apps/api/app`'s `getVersion()`, `navigator.userAgent`) — only the **redacted settings snapshot** and optional **DB snapshot** genuinely need to come from the real backend once MariaDB replaces the mock. Rust's role here is narrower than the function's full scope: it supplies the settings/DB data; the frontend still owns log-file collection, zipping, and the save dialog, exactly as today. |
| `listRecentDocuments` | **dev-only** (changed — see §8) | `diagnostics_list_recent_documents` (debug builds only) | `{ limit?: number }` (default 100) | `AccountingDocSummary[]` | — | — | n/a (read) | Most recent posted journal entries for the debugger's document picker, joined with whether an in-memory posting trace exists for it. |
| `getPostingTrace` | **dev-only** (changed) | `diagnostics_get_posting_trace` (debug builds only) | `{ entryId: string }` | `PostingTrace \| undefined` | — | — | n/a (read) | Reads from an **in-memory ring buffer** (`recentPostingTraces()`/`postingTraceFor()`), not `db` at all — "works even with debug mode off... falls back to `undefined` when the entry predates this session (ring is in-memory only, cleared on reload)." **This is the one function in the module whose Rust equivalent needs its own in-process ring buffer inside `AppState`**, not a DB query — there is no `posting_traces` table to design; it's an ephemeral, session-lifetime debugging aid by design, and should stay that way (persisting it would turn a lightweight dev aid into a growing table nobody reads outside a debug session). |
| `getJournalEntryRaw` | **dev-only** (changed) | `diagnostics_get_journal_entry_raw` (debug builds only) | `{ id: string }` | `JournalEntry \| undefined` | — | — | n/a (read) | Looks in **both** `journalEntries` and `journalDrafts` (posted or not) — the one function in this module that reads draft data, since the debugger needs to inspect an entry regardless of its posted state. |
| `getBalancesAround` | **dev-only** (changed) | `diagnostics_get_balances_around` (debug builds only) | `{ entryId: string }` | `{ accountId, accountName, accountCode, before, after }[]` | — | — | n/a (read) | "Before"/"after" balances for every account an entry touches, computed by re-walking **every** journal entry on-or-before this one's date (tie-broken by `number`) — an O(all entries) scan per call, acceptable for a dev tool invoked on-demand, not something to optimize into a running-balance table (that would duplicate the ledger's own running-total logic for a feature only debug builds use). |
| `getInvariantResults` | **dev-only** (changed) | `diagnostics_get_invariant_results` (debug builds only) | — | `InvariantResult[]` | — | — | n/a (read) | Calls `runAllInvariants(db)` — **the exact same function** `bun run verify:mocks` calls (CLAUDE.md: "`runAllInvariants()`... is the one copy of the 14 invariants both `bun run verify:mocks` and that tab use — don't add a second copy of an invariant check anywhere else"). Rust's port of the 14 invariants (master plan §8, "The Rust port of the 14 invariants is green on every test database") is the single source this command must call — never a second, diagnostics-specific reimplementation. |
| `getDriftReport` | **dev-only** (changed) | `diagnostics_get_drift_report` (debug builds only) | — | `DriftRow[]` | — | — | n/a (read) | Subledger-vs-GL drift per customer/supplier/product. **Explicitly documented as duplicating a small amount of invariant math on purpose** ("this file never decides pass/fail, that stays in the shared invariant") — the source comment already states the boundary Rust must preserve: this command computes the same subledger/GL formula `checkPartyAllocation` (in `invariants.ts`) uses, for **display** (every row, not just failing ones), but the pass/fail judgment itself must still come from the one shared invariant function, never re-derived here independently. |
| `explainAccountBalance` | **dev-only** (changed) | `diagnostics_explain_account_balance` (debug builds only) | `{ accountId: string, partyId?: string }` | `ExplainLine[]` | — | — | n/a (read) | "اشرح هذا الرقم" — every journal line touching an account (optionally narrowed to a party), grouped by source document, sorted by date descending. A straightforward filter+sort over `journalEntries`, no special posting-math duplication concerns. |
| `startReproRecording` | **dev-only** (changed) | `diagnostics_start_repro_recording` (debug builds only) | — | — | — | — | n/a | Snapshots the **current in-memory `db`** as a recording's starting point, then records every subsequent service call. **This function's entire mechanism is mock-specific** (`actionJournal.ts`'s `startRecording(clone(db))`) — once the real backend exists, "recording every service call + a DB snapshot" needs a genuinely different Rust-side implementation (likely: snapshot the real DB via a transaction-consistent export, then tap the same command-dispatch layer every `#[tauri::command]` already goes through to log each call). **Flagged in §8/§9** as needing real design work in Part 02/04, not a straightforward 1:1 port — this is the sharpest "the mock's implementation strategy doesn't carry over" finding in the whole module. |
| `stopReproRecording` | **frontend** (confirmed, no change) | n/a | — | — | — | — | n/a | Purely toggles the client-side `actionJournal` ring's recording flag — no `db`/backend read at all. |
| `isReproRecording` | **frontend** (confirmed, no change) | n/a | — | — | — | — | n/a | Same — a local boolean flag check. |
| `exportReproBundle` | **frontend** (confirmed, no change) | n/a | — | — | — | — | n/a | Builds the recorded bundle from the client-side `actionJournal` ring and saves it via `saveFile` — no `db`/backend read. Once `startReproRecording`'s mechanism is redesigned for the real backend (see above), this function's shape may need to change too, but that's a consequence of the recording redesign, not a separate decision. |

## 2. DTOs → Rust

| Type | Field | Rust type | Column | Why |
|---|---|---|---|---|
| `AuditEntry` | `id` | `Uuid` | `UUID` | UUIDv7 (D2) — this table is written by every other module's `logAudit` calls, not by this module; listed here only because `diagnostics` is the one that reads it back. |
| `AuditEntry` | `action` | `enum AuditAction { Create, Update, Post, Void, Reverse, Delete, Login, Settings }` | `ENUM(...)` | `#[serde(rename_all = "snake_case")]` — reused everywhere `logAudit`/`logActivity` is called (every module reviewed this session), not redeclared per-module. |
| `AuditEntry` | `before`/`after` | `Vec<AuditFieldDiff>` (`{ field: String, before: Option<serde_json::Value>, after: Option<serde_json::Value> }`) | JSON columns, or a child table if field-level diff querying is ever needed (not today — every consumer reads the whole diff array per entry, never filters by field) | `unknown` in TS → `serde_json::Value` in Rust, since a diffed field's value can be any JSON-serializable type (string, number, boolean, nested object). |
| `AuditEntry` | `link` | `Option<RouteRef>` | — | Every other module's F7 path-string-link findings (`/customers/${id}`, `/accounting/journal/${id}`, etc.) ultimately land in **this exact field** — `AuditEntry.link` is the shared destination type every module's `logActivity(kind, msg, userId, date, link?)` call populates. Confirming this here closes the loop: F7's fix converts each call site's literal string into a `RouteRef`, and this is the one field type that absorbs all of them. |
| `AuditEntry` | `at` | `DateTime<Utc>` | `DATETIME(3)` | Instant (when the audited action happened), not a business-local date. |
| `PostingTrace` (not detailed in the generated inventory's type list since it's exported from `@/mocks`, not `diagnostics/types`) | — | Rust-side, this stays an **in-process, non-persisted struct** held in `AppState`'s ring buffer (see `getPostingTrace`'s note in §1) — no DB columns to design. | — | — |
| `PerfStat` | `budgetMs` | `Option<Decimal>` | — (not written by anything in this module — belongs to the perf-logging side of `logService.ts`, out of this module's endpoints; listed only because it's declared in `diagnostics/types/index.ts`) | Flagged as out-of-scope for this module's 14 functions, same treatment `templates.md` gave `LabelOptions`/`LabelPreset`. |
| `LogEntry`/`LogChannel`/`LogLevel`/`LogContext`/`LogErrorInfo`/`FingerprintGroup` | — | (same as `PerfStat` — types exported from this module's `types/index.ts` but never read/written by any of the 14 reviewed functions; they belong to `logService.ts`'s channel-logging system, a separate, larger diagnostics concern CLAUDE.md's "Diagnostics" section describes but which has no service-layer `wrap()`ped endpoints of its own to review here — `.diagnostics/logs/*.jsonl` files are read directly off disk by dev tooling, not through this module's service surface) | — | Noted for completeness; not this file's job to design a Rust schema for, since nothing in the 14-function inventory writes or reads these as a `db` table — they're an orthogonal file-based logging system. |

## 3. Validation and errors

**None.** No function in this module throws any error under any input — `getJournalEntryRaw`/`getPostingTrace`/`getBalancesAround` all return `undefined`/`[]` for a missing/unknown id rather than a `NOT_FOUND`, matching `templates.md`'s "missing is a valid answer, not an error" pattern for a read-only inspector tool. No dedicated Zod schema, no `ApiError` import anywhere in the three service files.

## 4. Undo matrix (every function that writes)

Not applicable — zero writes to `db` anywhere in this module (confirmed: blank "Writes" and "Shared" columns in the generated inventory for all 14 functions; the 3 repro-recording functions mutate an in-memory, non-persisted client-side ring, not any table).

## 5. Concurrency under D8 (several terminals on one DB)

Mostly not applicable — pure reads, same structural reasoning as `reports.md`/`analytics.md`. The one caveat: `startReproRecording`'s in-memory snapshot-and-record mechanism is inherently **per-process, not per-branch** — recording on one terminal captures only that terminal's own service calls, never another terminal's concurrent activity on the same MariaDB. This is expected and fine for a debugging aid (a developer records what happens on the machine they're debugging), not a gap to fix — but worth stating explicitly since it's a departure from every other module's "shared DB, every terminal sees the same state" assumption.

## 6. Events and side effects

- **No activity/audit rows written by this module** — correct, since this module only *reads* the audit trail, never writes to it (per CLAUDE.md's own rule: "A new backend write that changes a business entity should call `logAudit`... at the same point it always did" — this module has no such write to begin with).
- **No events emitted.**
- **File I/O**: `exportSupportBundle` writes a zip file via `saveFile` (native Save dialog, not silent) — the one "write" in this module, and it's to the filesystem, not `db`. `exportReproBundle` (frontend-only) does the same for a JSON repro bundle.

## 7. Aggregations (reports / analytics / dashboard / insights only)

Not a reports module in the `reports.md`/`analytics.md` sense (no chart data, no rounding-order concerns of the kind those files documented) — but `getDriftReport`/`getBalancesAround` are worth a table since they re-derive ledger math for display:

| Output (DTO field) | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `DriftRow[]` (customer) | `invoices`, `refunds`, `journalEntries`, `unallocatedCreditFor` (payments module, already reviewed) | non-`DRAFT` invoices, net of refunds/unallocated credit | by `customerId` | `round2` at each step (outstanding, subledger, gl, diff) — `accountingDebugService.ts:155-165` | `accountingDebugService.ts:150-166` |
| `DriftRow[]` (supplier) | `purchaseOrders`, `purchaseReturns`, `journalEntries` | `RECEIVED` POs, net of returns/unallocated credit | by `supplierId` | same `round2` pattern | `accountingDebugService.ts:168-182` |
| `DriftRow[]` (product) | `journalEntries` (inventory-role lines), `products.stockValue` | only emitted when the **aggregate** invariant already fails (no per-product GL breakdown is derivable from journal lines alone, per the source comment) | — (flat, one row per product, only when the aggregate check fails) | `round2` on the aggregate `invGl`/`invSum` | `accountingDebugService.ts:184-198` |
| `getBalancesAround` before/after | `journalEntries` (all entries on-or-before the target, tie-broken by `number`) | per `accountId` touched by the target entry | by `accountId` | `round2` on each account's running total | `accountingDebugService.ts:87-98` |

## 8. Contract fixes needed in the mock (01.C — resolved 2026-09-27)

- [x] **8 functions overridden to `dev-only`** in `scripts/contract/config.ts` (`diagDevOnly` constant, applied to `listRecentDocuments`, `getPostingTrace`, `getJournalEntryRaw`, `getBalancesAround`, `getInvariantResults`, `getDriftReport`, `explainAccountBalance`, `startReproRecording`). **Rationale**: CLAUDE.md's own "Diagnostics" section states the accounting debugger lives at "`/dev/diagnostics` (dev builds)" and its tabs (Errors/Performance/Debug/Audit/**Accounting**) are explicitly a development surface — the "Accounting" tab specifically is 18.F3/18.F4's posting-trace/invariants/drift/repro-recording tooling, none of which a production build should expose as a callable command a non-developer could reach. This is the direct answer to this module's own stated task: **"decide which read endpoints Rust serves in debug builds (F9)."** `getAuditEntries`/`getAuditEntities`/`exportSupportBundle` were deliberately **left as `port`** (production), since they back genuinely user-facing features (the audit log, and the "تصدير ملف التشخيص" support-bundle export a non-technical owner uses per CLAUDE.md's "User-facing" diagnostics note) — not everything under `src/modules/diagnostics/` is a dev tool, and conflating the two would have wrongly hidden a real production feature behind a debug-build flag. `bun run contract` re-run: `diagnostics` module is now 3 port / 3 frontend / 8 dev-only (was 11 port / 3 frontend).
- [ ] **`startReproRecording`'s snapshot-and-record mechanism doesn't have an obvious 1:1 Rust equivalent** (see §1) — flagged as a genuine design task for Part 02/04, not a mock fix. The mock's approach (clone the whole in-memory `db`, then record every subsequent `wrap()`-intercepted service call) only works because the mock's entire dataset fits in memory and every "backend call" already funnels through one interception point (`defineService.ts`'s `wrap`). Under a real MariaDB backend, "recording" needs either (a) a consistent snapshot export (e.g. a point-in-time dump via a `START TRANSACTION WITH CONSISTENT SNAPSHOT` read, or a full logical export) plus tapping the Tauri command-dispatch layer to log each subsequent IPC call, or (b) some other strategy — this needs real design, not a guess, and is called out precisely so Part 04 (which owns "Reproduce with... replay it headlessly" per CLAUDE.md) picks it up deliberately.
- No other findings — no plain `Error`/`ApiError` gaps (nothing throws at all), no reference-check gaps (nothing deletes), no missing audit trail (nothing writes to `db`), no F7 path-string links owned by this module directly (though `AuditEntry.link`'s type, as noted in §2, is the shared destination every *other* module's F7 fixes converge into).

## 9. Open questions (→ decisions in `00-MASTER-PLAN.md`)

- **`startReproRecording`'s Rust-side redesign** (see §8) — this is the one substantive open item from this module. Not urgent for 01.B's own gate (the disposition is confirmed `dev-only`, and no accounting risk exists in leaving the *implementation strategy* undecided for now), but Part 04 ("Reproduce with... replay it headlessly") should not start designing the Rust repro-recording mechanism without first reading this note, since the mock's "clone the whole DB" trick has no direct equivalent once the DB is real and potentially large.

## Gate

- [x] Every inventory function is in §1 with a confirmed disposition (14/14 `wrap()`ped functions, plus `getAuditEntityKinds` noted as a non-endpoint alias for completeness; 8 overridden to `dev-only`, applied and regenerated).
- [x] Every write function is in §4 — n/a, no writes to `db` anywhere in this module.
- [x] Every DTO field needing a non-default mapping is in §2.
- [x] `bun run contract:check` is green after the overrides (`diagnostics` module now 3 port / 3 frontend / 8 dev-only). Also green: `bun run build`, `bun run verify:mocks` (128 ok, 0 failed, unchanged — no mock code was touched, only `scripts/contract/config.ts`), `bun run check`, `bun run memory:check` (0 new seam violations), `bun run diag:check`.

---

## 01.B complete

This was the last module in the 01.B per-module review order (`01-FRONTEND-ANALYSIS.md` §5's list:
settings → setup → users → approvals → parties → products → purchases → invoices → payments →
vouchers → expenses → accounting → reports → analytics → core → templates → **diagnostics**, 17
files in total). Every module file has its Gate section fully checked. Part 01's own overall gate
(`01-FRONTEND-ANALYSIS.md` §6) still needs: 01.C's shared path-string-link pass (F7 — every module
this session flagged its own instances, none yet converted in bulk) and 01.D's cross-cutting doc
(auth/session, terminal identity, settings split, `AppError` catalogue, events, paging, dates,
table→entity map, import spec) — neither is written yet. `01-FRONTEND-ANALYSIS.md`'s own status
note and phase checklist should be updated to reflect 01.B's completion before either of those
starts.
