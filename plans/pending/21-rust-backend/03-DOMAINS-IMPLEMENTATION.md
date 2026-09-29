# 21 · Part 03 — Domains implementation (every `port` function on Rust)

> **Status (2026-09-28):** **planned** — 23 domain files (~300 commands) written by parallel Opus planning agents; 48 Part 02 gaps collected in `03-domains/_part02-gaps.md` (Wave 0 fixes them); user decisions pending: G-25, G-31 (mock accounting bugs) and the per-file open questions. Starts while Part 02's final test pass is paused
> (user decision 2026-09-28, recorded in `00-MASTER-PLAN.md` "Next step": development continues,
> Part 02's tests resume later as their own step). No domain is flipped to Rust here — flipping is
> Part 04, per domain, after its parity cases pass (master §5).
>
> **✅ CODE COMPLETE (2026-09-29) — tests not yet run.** All 23 domain files implemented (Waves 0–6):
> **306 IPC commands** registered in `lib.rs` + `ipc_sig!` (`ipc_manifest_matches_handler` ✔), ts-rs
> bindings generated into `src/modules/*/types/gen/` (~580 files) with `contract.check.ts` identity
> checks. Gates green: `cargo check --workspace --all-targets` 0 errors / 0 warnings · `bun run build`
> 0 errors · `check` ✔ · `verify:mocks` 128/0 · `verify:replay` ✔ (new ACC-0001/0002 cases) ·
> `contract:check` ✔ · `memory:check` ✔ (0 seam violations) · `diag:check` ✔. Manager additions:
> `core/grants.rs` (G-P3), `SequenceLock::ProductCodes`, P2-52 pre-migration backup wiring, the
> repo-root `.cargo/config.toml` (cargo reads config from the run dir — `jobs = 4` and
> `TS_RS_EXPORT_DIR` were silently not applied from `src-tauri/.cargo/`).
> **Not done:** every `tests/domain_*.rs` DB test is written but **not run** (deferred, time-boxed
> pass); parity cases → Part 04; `RUST_DOMAINS` stays empty (no domain flipped). **User decisions
> pending:** G-25 (refund over-refunds VAT), G-31 (landed-cost split, non-stock receipt, draft
> adjustment PIN) — ported as-is with `// G-25`/`// G-31` markers; accounting open questions in
> 12b (VAT settled twice, reopen date, VAT reversal/cap); C-02, C-16.

## 1. Goal and scope

Implement, in Rust, every service function Part 01 marked **port** — one `domains/<d>/` module
per Vue module (master §4) — on top of Part 02's foundation (`with_tx`, entities, `shared::ledger`,
`shared::stock`, `shared::activity`, `shared::balances`, the IPC bridge). Each ported function
becomes one IPC command that returns **exactly** the DTO the mock returns today, so Part 04 can
diff mock vs Rust field by field.

- **In scope:** the D10 snapshot importer (step 00, Part 02 handoff §9), then every domain in
  master §7 order; each domain's commands, service logic, DTOs (ts-rs), `contract.check.ts`
  entries, the dormant frontend switch line in each ported service function, undo compensators
  (the five in phase-e E-5), and the DB tests + parity-case list for Part 04.
- **Out of scope:** flipping any domain (`RUST_DOMAINS` stays empty — Part 04), the parity harness
  itself (Part 04), new UI except where Part 02's handoff requires it (setup role step, LAN
  toggle/pairing/server-failure screen — see 02-setup and 01-settings), and all real testing
  (final testing plan, master §8).

## 2. Read first (in this order)

1. [`../../../CLAUDE.md`](../../../CLAUDE.md) — rules, "Accounting safety", "Rust ↔ Vue", ARCHITECTURAL AUTONOMY.
2. [`00-MASTER-PLAN.md`](00-MASTER-PLAN.md) — §3 rules 1–10, §4 layout, §5 switch, §7 order, §9 decisions.
3. [`02-CORE-AND-SHARED-ARCHITECTURE.md`](02-CORE-AND-SHARED-ARCHITECTURE.md) — §3 P2-xx decisions, §9 handoff.
4. [`01-frontend-analysis/cross-cutting.md`](01-frontend-analysis/cross-cutting.md) — auth/session, terminal id, settings split, `AppError`, events, paging/search, dates, the D10 spec (§9).
5. The domain's own analysis file `01-frontend-analysis/<module>.md` (§1 endpoints with their port/stay/drop disposition, §2 DTOs, §3 validation, §4 undo matrix, §5 concurrency, §6 events, §7 aggregations) — **the primary input for every domain file; don't re-derive it, cite it.**
6. The mock being ported: `src/mocks/backend/<file>.ts` + the module's `services/*.ts` and `types/index.ts`.

## 3. Domain conventions (every domain file and every implementer follows these)

### 3.1 Layout

```text
src-tauri/src/domains/
  mod.rs                 manager-owned: `pub mod <d>;` per domain, `all_ipc_signatures()`, `register_undo()`
  <d>/mod.rs             `pub mod commands; pub mod service; pub mod dto;` + `pub fn ipc_signatures() -> Vec<IpcSig>`
                         + (only if the domain has compensators) `pub fn register_undo(r: &mut UndoRegistry)`
  <d>/commands.rs        thin IPC layer: args struct, `require`, `with_tx`/`with_read`, map error
  <d>/service.rs         the logic (split into `service/<area>.rs` files past ~400 lines)
  <d>/dto.rs             serde camelCase DTOs = the TS types, `#[derive(TS)] #[ts(export_to = "<module>/types/gen/")]`
src-tauri/tests/domain_<d>.rs   DB-backed tests (written now, run in the deferred test pass)
```

### 3.2 A command

- Name `<domain>_<function_snake>` (`invoiceService.createSale` → `invoices_create_sale`). One
  `#[derive(Deserialize, TS)] <Fn>Args` struct per command (camelCase), even for one field —
  `backendCall(cmd, args)` sends `{ args }`.
- Signature: `#[tauri::command] pub async fn <cmd>(state: State<'_, AppState>, args: <Fn>Args) -> Result<<Dto>, ApiErrorPayload>`
  (`crate::core::dto::ApiErrorPayload`, `From<AppError>`; see `core/status.rs`).
- Body: `with_tx(&state, TxOpts::default(), |tx, cx| Box::pin(async move { cx.require(tx, Area::X, Access::Write).await?; service::f(tx, cx, args).await }))`
  for writes; `with_read` for multi-statement reads (reports, statements). The `Area`/`Access`
  per command is decided in the domain file against the pages that call it (`core/auth.rs`
  `Area`: Dashboard, Pos, Sales, Inventory, Parties, Purchases, Expenses, Accounting, Payments,
  Reports, Analytics, Approvals, Users, Settings).
- Service functions: `pub async fn f<C: ConnectionTrait>(conn: &C, cx: &TxCtx, …) -> TxResult<T>`
  (return `TxResult` so a raw `DbErr` keeps its errno for deadlock retry — Part 02 convention).
- Every command gets an `ipc_sig!(<cmd>, <Fn>Args, <Dto>)` line in its domain's
  `ipc_signatures()`; the manager registers the command in `lib.rs` `generate_handler!`.

### 3.3 Behaviour parity (the rule that decides every detail)

- **Behaviour-exact port** of the mock: same validation order, same Arabic messages byte for
  byte, same `AppError` codes (Part 02 C-01: `VALIDATION`/`NOT_FOUND`/`CONFLICT`/`FORBIDDEN`/
  `UNAUTHORIZED`/`INTERNAL`), same rounding points (`round2`/`round4` only), same ordering of
  returned lists (the mock's array order = `ORDER BY created_at, id` unless the mock sorts).
  Where the mock is wrong, **don't fix it silently**: keep the behaviour, and list it under the
  domain file's "Known mock quirks" for a later decision (Part 01 already listed most).
- **Rule 3:** journal entries only via `shared::ledger`, stock/batches/avg cost only via
  `shared::stock`, audit/activity only via `shared::activity`. `architecture_rules` enforces it.
- **Rule 4:** one transaction per command. Locks in the **global order documented in
  `src-tauri/src/core/lock.rs`**: document row(s) → party rows (sorted) → product rows (sorted,
  then their batches) → settings (S) → fiscal year (S/X, via `assert_open_period`) →
  `document_counters` → `change_versions` (only at commit, inside `with_tx`). Every command that
  posts party-tagged journal lines share-locks those party rows first.
- **Events:** `cx.touch(ChangeCategory::…)` for every category the mock `emit`s (analysis §6).
- **Server-side authority:** every total, VAT, cost, balance and number is recomputed in Rust
  from ids and quantities — the DTO the frontend sends is never trusted for money (CLAUDE.md
  "ARCHITECTURAL AUTONOMY").

### 3.4 Frontend side (dormant until Part 04 flips the domain)

- At the top of each ported service function (inside its existing `wrap(...)`), add
  `if (usesRust('<domain>')) return backendCall('<cmd>', { … });` — the mock body below stays
  unchanged. Pages never change. `stay-frontend` and `drop` functions are not touched.
- The module's `types/contract.check.ts` gets one `Expect<Equals<Gen.X, X>>` per new DTO. If the
  generated type can't equal the hand-written one, change the **Rust** DTO (the TS type is the
  contract), or record why with a `// contract-ok: <reason>` comment.
- `src/modules/<module>/types/gen/` is generated by `bun run bindings` — never hand-edited
  (implementers write the Rust; the manager regenerates).

### 3.5 Undo

Only the five action types in [`phase-e-activity.md`](02-core-and-shared/phase-e-activity.md)
§E-5 record an `UndoSpec` and register a `Compensator` (accounting ×4, setup ×1). Every other write
records its audit row without `undo` ("not undoable via the registry" — corrections are new
opposite documents). Compensators pass `allow_closed_period = actor is admin` (D7).

### 3.6 Tests (written now, run later)

Per user decisions (2026-09-27/28): implementers **write** tests but never run cargo; the manager
runs one throttled `cargo check --workspace --all-targets` (no linking, below-normal priority,
`jobs = 4`) after each wave to catch compile errors; DB tests and the fast `bun run` gates run in
a separate, **time-boxed** pass later — one test binary at a time; a run that drags on is stopped,
recorded and resumed later (memory `feedback_dont-block-on-long-runs`). Each domain file lists:
(a) DB tests for `tests/domain_<d>.rs` (happy path, each validation message, each concurrency
rule, posting results checked with `shared::invariants::run_all`), and (b) the **parity cases**
Part 04 will replay on mock vs Rust.

### 3.7 Ownership during implementation

Manager-owned (implementers request changes in their report): `Cargo.toml`, `lib.rs`,
`domains/mod.rs`, `core/**`, `shared/**`, `entities/**`, `migration/**`, `CLAUDE.md`, plan entry
files, `AGENT_MEMORY.md`. An implementer owns `domains/<d>/**`, `tests/domain_<d>.rs`, the
module's `services/*.ts` switch lines and `types/contract.check.ts`, and its domain plan file.
If a domain needs a new shared helper, entity field or migration, it asks; the manager adds it
between waves (a new migration is `m0016+`, never an edit to m0001–m0015 once Part 02's tests
pass).

## 4. Phases (one file per domain, master §7 order)

| # | File | Domain / what | Depends on | Status |
|---|---|---|---|---|
| 00 | [`03-domains/00-import.md`](03-domains/00-import.md) | D10 snapshot importer (`infrastructure/import/`), demo data in desktop dev | Part 02 | code complete |
| 01 | [`03-domains/01-settings.md`](03-domains/01-settings.md) | settings (store settings, branches, currencies, taxes, payment methods, LAN toggle + server screen) | 00 | code complete |
| 02 | [`03-domains/02-setup.md`](03-domains/02-setup.md) | setup wizard (first-run role step, provisioning, pairing, party openings) | 01 | code complete |
| 03 | [`03-domains/03-users.md`](03-domains/03-users.md) | users, login/session, roles | 01 | code complete |
| 04 | [`03-domains/04-approvals.md`](03-domains/04-approvals.md) | approval requests | 03 | code complete |
| 05 | [`03-domains/05-parties.md`](03-domains/05-parties.md) | customers/suppliers, statements, credit limit | 01 | code complete |
| 06 | [`03-domains/06-products.md`](03-domains/06-products.md) | catalog, prices, inventory, adjustments, counts, transfers | 01 | code complete |
| 06b | [`03-domains/06b-inventory.md`](03-domains/06b-inventory.md) | inventory half of products: adjustments, movements, batches/expiry, counts, transfers (same domain, same implementer) | 06 | code complete |
| 07 | [`03-domains/07-purchases.md`](03-domains/07-purchases.md) | purchase orders/invoices/returns | 05, 06 | code complete |
| 08 | [`03-domains/08-invoices.md`](03-domains/08-invoices.md) | sales, POS, shifts, quotations, returns | 05, 06 | code complete |
| 08b | [`03-domains/08b-pos-shifts.md`](03-domains/08b-pos-shifts.md) | POS half of invoices: shifts, cash in/out, X/Z report, held sales (same domain, same implementer) | 08 | code complete |
| 09 | [`03-domains/09-payments.md`](03-domains/09-payments.md) | receipts/payments, allocations | 07, 08 | code complete |
| 10 | [`03-domains/10-vouchers.md`](03-domains/10-vouchers.md) | vouchers, card settlements | 09 | code complete |
| 11 | [`03-domains/11-expenses.md`](03-domains/11-expenses.md) | expenses and categories | 01 | code complete |
| 12 | [`03-domains/12-accounting.md`](03-domains/12-accounting.md) | chart of accounts, journal, drafts, recurring, the 4 accounting undo compensators | all writers | code complete |
| 12b | [`03-domains/12b-period-close.md`](03-domains/12b-period-close.md) | FY close/reopen, lock date, VAT settlement (same domain, same implementer; revaluation + openings live in 01-settings/02-setup) | 12 | code complete |
| 13 | [`03-domains/13-reports.md`](03-domains/13-reports.md) | reports engine (read-only) | all writers | code complete |
| 13b | [`03-domains/13b-reports-operational.md`](03-domains/13b-reports-operational.md) | operational reports: sales, purchases, stock, aging, shifts | 13 | code complete |
| 14 | [`03-domains/14-analytics.md`](03-domains/14-analytics.md) | analytics, dashboard, insights (read-only) | 13 | code complete |
| 14b | [`03-domains/14b-insights.md`](03-domains/14b-insights.md) | the 21 insight rules + product hints | 14 | code complete |
| 15 | [`03-domains/15-templates.md`](03-domains/15-templates.md) | print templates (D9) | 01 | code complete |
| 16 | [`03-domains/16-diagnostics.md`](03-domains/16-diagnostics.md) | the 3 production `port` fns + support bundle DB snapshot | 01 | code complete |
| 17 | [`03-domains/17-backup.md`](03-domains/17-backup.md) | backup/restore (`infrastructure/backup/`, P2-52) | 00 | code complete |

**Implementation waves** (parallel Sonnet implementers, disjoint files, manager in between):
W1 = 00 · 01 · 03 · 15 · 16 → W2 = 02 · 04 · 05 · 06 · 11 · 17 → W3 = 07 · 09 · 10 → W4 = 08
(08 calls into 09/10's services, so they land first) → W5 = 12 (+12b) → W6 = 13 · 14. After each wave: manager registers commands/modules, runs one throttled
`cargo check`, fixes compile errors, regenerates `AGENT_MEMORY.md`.

## 5. Definition of done (Part 03)

- Every Part 01 `port` function has a Rust command with a registered handler, an `ipc_sig!`, a
  generated TS binding with a passing `contract.check.ts` entry, and the dormant switch line.
- `AGENT_MEMORY.md` IPC table: every command invoked and registered, **0 contract gaps**.
- `cargo check` clean; `architecture_rules` green. DB tests and parity cases written for every
  domain (run in the deferred, time-boxed test pass; Part 04 gates each flip on them).
- `bun run build`, `check`, `verify:mocks` (128/0), `contract:check`, `memory:check`, `diag:check`
  green (run once at the end of Part 03, time-boxed).
- Status note at the top of every domain file; master §6 row 03 updated.

## 6. Part 02 gaps found while planning (manager tasks before/between waves)

Collected in [`03-domains/_part02-gaps.md`](03-domains/_part02-gaps.md) — each gap names the
domain(s) that need it and is fixed by the manager (new migration `m0016+` or a shared helper)
before the wave that depends on it.

## 7. Decisions and open questions

- **Carried from Part 02, still open for the user:** C-02 (`INTERNAL` error code), C-16
  (attachment blob storage under D8 — the domains that store attachments keep the mock's current
  data-URL-in-row behaviour until it's decided; recorded per domain).
- New decisions made while writing the domain files are logged in each file's "Decisions"
  section (architectural autonomy: strictest option, logged, not asked).
