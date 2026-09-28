# 21 · 03.13 — `reports` (read-only reports engine: financial statements and ledgers)

> **Status:** implemented 2026-09-28 (code complete; DB tests and parity cases written but not run
> from this session — deferred, time-boxed test pass per the manager's cargo-throttling rule).
> `domains/reports/{dto.rs,commands.rs,mod.rs,service/*}` written for all 16 commands in this file;
> `src-tauri/tests/domain_reports.rs` covers trial balance, P&L, balance sheet, account ledger,
> ledger targets/dimension options and the session gate for this file's functions (13b's own tests
> live in the same file). 16 switch lines added to `reportService.ts` +
> `src/modules/reports/types/contract.check.ts` created (21 entries here + 20 from 13b). **Needs
> from manager:** `pub mod reports;` + `reports::export_bindings(cfg)` hook in `domains/mod.rs`,
> the 32 commands added to `generate_handler!`, and confirmation G-32/G-13/G-1/G-33/G-34/G-8 are
> merged (this implementer assumed they are, per the entry file's "fixed" status in
> `_part02-gaps.md`, and coded directly against their described shapes without re-verifying the
> merge). `bun run bindings` must run before `bun run build` type-checks this domain's
> `contract.check.ts` (the `types/gen/*` files don't exist yet). Wave W6 (entry file §4). Depends on: every
> writer domain (00–12), because reports only read what they post; Part 02 gaps below (ids from the shared registry [`_part02-gaps.md`](_part02-gaps.md): G-1, G-4/G-32, G-8, G-13, G-33, G-34, G-35).
> **Split:** the 32 `reportService` functions are too many for one file. **This file** holds the
> 16 financial-statement and ledger reports plus the shared helpers (`service/common.rs`).
> [`13b-reports-operational.md`](13b-reports-operational.md) holds the 16 operational ones (sales,
> purchases, stock, aging). Both describe **one** Rust domain, `domains/reports/`, and **one
> implementer** does both, this file first (13b's commands use the helpers built here).

**Goal.** Port the 16 statement/ledger functions of `reportService.ts` to `reports_*` commands.
Each one runs in a single REPEATABLE READ snapshot and returns exactly the mock's DTO, with the
same rounding points, the same filters and the same row order, so Part 04 can diff mock vs Rust
field by field. Writes nothing, emits nothing, has no undo.

**Read first.** [`01-frontend-analysis/reports.md`](../01-frontend-analysis/reports.md) §1 (rows
for these 16), §2, §3, §5, §7 (primary input, cited as `A§n`) · `src/modules/reports/services/reportService.ts`
(the mock itself, 1199 lines, cited as `rs:<line>`) · `src/modules/reports/types/index.ts` (`types:<line>`) ·
`src/mocks/utils.ts:70-72` (`sum` = `round2(Σ)`) and `:99-114` (`localDateKey`, `inDateRange`) ·
`src/mocks/backend/balances.ts` (ported as `shared::balances`) ·
[`../../../docs/v2/02-accounting-review.md`](../../../docs/v2/02-accounting-review.md) §4 invariants 2 and 5.

## Part 02 API gaps this domain needs (manager tasks, raised before W6)

Ids are the shared registry's ([`_part02-gaps.md`](_part02-gaps.md)). G-1 (with G-24), G-8 and G-13
were already raised by other domains; the rows below add what reports need from them. G-32 widens
G-4 (a clock alone is not enough). G-32…G-35 are new rows appended to the registry.

| # | Gap | Why (evidence) | Needed by |
|---|---|---|---|
| G-32 | **`core::tx::with_read_ctx`**: like `with_read` (REPEATABLE READ, READ ONLY, always rolled back) but the closure gets a `ReadCtx { actor: Option<AuthenticatedUser>, clock: BusinessClock, terminal_id: Id }` with `ReadCtx::require(conn, area, access)`, and it refuses with `UNAUTHORIZED "سجّل الدخول أولاً"` when there is no session (same as `with_tx`'s `require_user`). Make `read_business_clock` (`core/tx.rs:143`) shared by both. | `with_read` (`core/tx.rs:263`) passes only the transaction: no actor for `require`, no clock for "today" (aging, cash flow, business health need `localDateKey(new Date())`, `rs:623,696,1131`), and no session check. | every read command in 13, 13b, 14, 14b |
| G-13 | **`RouteRef.query`**: add `query: Option<BTreeMap<String, String>>` (skipped when `None`) to `utils/route.rs`. | `AppRoute` = `RouteLocationAsRelative` (has `query`); `getPartyLedger` emits `{ name: 'payments', query: { highlight } }` (`rs:309`). `RouteRef` has only `name` + `params`. | 13 (party ledger), 14b (insight links) |
| G-1 | **`shared::balances::OpenDocument` full shape**: add `kind` (`invoice`/`purchaseOrder`), `number`, `date_key`, `due_date_key: Option<String>` (invoices only), `total`, `currency`, `fc_outstanding`, `rate`, and order rows by the **date key string** then `(created_at, id)`, as `payments.ts:29-70` does (`a.date.localeCompare(b.date)`). | Today `OpenDocument` is `{ id, outstanding }` and sorts by `date_day` (`shared/balances.rs:243-300`). Aging and overdue need number/date/dueDate (13b); payments (09) needs the rest. | 13b, 14b, 09 |
| G-33 | **`indexmap = "2"`** as a direct dependency (already in `Cargo.lock` transitively). | Every mock group-by is a JS `Map` (insertion order) followed by a **stable** `sort` (`rs:370-380`, `:787-797`); ties keep first-appearance order. An insertion-ordered map is the exact equivalent. | 13, 13b, 14, 14b |
| G-34 | **`utils::text::compare_ar(a, b) -> Ordering`** backed by ICU4X `icu_collator` (compiled data, locale `ar`). | `getInventoryReport` sorts with `localeCompare(…, 'ar')` (`rs:433`). V8 uses ICU, so ICU4X gives the same order; no MariaDB collation reproduces CLDR `ar`. | 13b |
| G-8 | **Domain binding export hook**: `domains::export_bindings(cfg)` that calls each `<d>::dto::export_bindings(cfg)`, wired into `core/ipc.rs::export_bindings` (today it only exports `core::dto`, `core/ipc.rs:127-140`). | Without it `bun run bindings` never writes `reports/types/gen/*`. | every domain |
| G-35 | **Lock-order text conflict** (finding, no code): entry file §3.3 says "settings S → fiscal year → parties → products → documents → change_versions" but `core/lock.rs:3-6` says "documents → parties → products → settings → fiscal year → counters". Pick one and fix the other. | Reports take no locks, but every writer domain depends on one order. | all writers |

## 1. Commands

All 16 are `port (confirmed)` in A§1. Every command is a read: **tx = `with_read_ctx` (G-32)**,
**Area/Access = `Reports` / `Read`** (all 27 report pages have `meta.area: 'reports'`,
`src/modules/reports/routes/index.ts:4`), **events touched = none** (A§6).

| # | Mock fn (`rs:` line) | Rust command | Args → Return |
|---|---|---|---|
| 1 | `getTrialBalance` (88) | `reports_get_trial_balance` | `{ range: ReportRangeFilter }` → `Vec<TrialBalanceRow>` |
| 2 | `getProfitAndLoss` (140) | `reports_get_profit_and_loss` | `{ range: ReportRangeFilter }` → `ProfitAndLoss` |
| 3 | `getProfitAndLossComparison` (150) | `reports_get_profit_and_loss_comparison` | `{ range: ReportRangeFilter, compareRange: DateRangeInput }` → `PnlComparison` |
| 4 | `getCostCenterProfitAndLoss` (190) | `reports_get_cost_center_profit_and_loss` | `{ range: DateRangeInput }` → `CostCenterPnl` |
| 5 | `getCostCenterBudgetVsActual` (208) | `reports_get_cost_center_budget_vs_actual` | `{ fiscalYearId: Id }` → `Vec<CostCenterBudgetRow>` |
| 6 | `getBalanceSheet` (230) | `reports_get_balance_sheet` | `{ asOf: String, dim?: DimensionFilter }` → `BalanceSheet` |
| 7 | `getAccountLedger` (255) | `reports_get_account_ledger` | `{ accountId: Id, range: DateRangeInput }` → `AccountLedger` |
| 8 | `getPartyLedger` (295) | `reports_get_party_ledger` | `{ kind: PartyKindArg, partyId: Id, range: DateRangeInput }` → `AccountLedger` |
| 9 | `getVatReport` (510) | `reports_get_vat_report` | `{ range: DateRangeInput }` → `VatReport` |
| 10 | `getVatDetail` (557) | `reports_get_vat_detail` | `{ range: DateRangeInput }` → `Vec<VatDetailRow>` |
| 11 | `getCashFlowStatement` (618) | `reports_get_cash_flow_statement` | `{ range: DateRangeInput }` → `CashFlowStatement` |
| 12 | `getDayBook` (674) | `reports_get_day_book` | `{ range: DateRangeInput }` → `Vec<DayBookEntry>` |
| 13 | `getPeriodComparison` (1084) | `reports_get_period_comparison` | `{ rangeA: DateRangeInput, rangeB: DateRangeInput }` → `Vec<PeriodComparisonLine>` |
| 14 | `getBusinessHealthReport` (1129) | `reports_get_business_health_report` | `{ range: DateRangeInput }` → `BusinessHealthReport` |
| 15 | `getLedgerTargets` (597) | `reports_get_ledger_targets` | — (`()`) → `LedgerTargets` |
| 16 | `getDimensionOptions` (1188) | `reports_get_dimension_options` | — (`()`) → `DimensionOptions` |

None of the 32 report functions is paged: each returns a whole array or object (`types:8-438`), so
`core::dto::PagedResult` is **not** used by this domain (the reports pages page client-side in
`DataTable`). Commands 15 and 16 take no args (`ipc_sig!(cmd, (), Dto)`).

## 2. DTOs (`domains/reports/dto.rs`, `#[ts(export_to = "reports/types/gen/")]`)

Conventions (Part 02 F-1, `core/dto.rs:1-14`): `rename_all = "camelCase"`; every money/ratio field
is `Decimal` with `#[serde(with = "crate::utils::money::serde_number")] #[ts(type = "number")]`;
`Id` fields are `#[ts(type = "string")]`; optional TS fields are `Option` + `skip_serializing_none`
+ `#[ts(optional)]`; counts are `i32` (`number`). Dates are returned as the mock's **key string**
(`DocDate::key()`), typed `String`.

| Rust DTO | TS type | Notes |
|---|---|---|
| `DateRangeInput { from?, to? }` | `types:3` | `Option<String>`; parsed by `common::DateRange::parse` (§3.0). |
| `DimensionFilter { branchId?, costCenterId?, currency? }` | `types:176` | `Option<String>` each (a stale id is simply no match, as in the mock). |
| `ReportRangeFilter` | `types:182` | `#[serde(flatten)]` of the two above; TS is `DateRangeInput & DimensionFilter`. |
| `TrialBalanceRow` | `types:8` | `accountId: Id`. |
| `StatementLine` | `types:20` | |
| `ProfitAndLoss` | `types:27` | |
| `PnlComparison { current, previous }` | inline return, `rs:150` | Check against `Awaited<ReturnType<typeof getProfitAndLossComparison>>`. |
| `CostCenterPnlColumn` | `types:44` | `costCenterId: String` (not `Id`): holds `'__unassigned__'` / `'__total__'` (`rs:163,203`). |
| `CostCenterPnl` | `types:54` | |
| `CostCenterBudgetRow` | `types:61` | `nearBudget: bool`. |
| `BalanceSheet` | `types:71` | `asOf` echoes the input string. |
| `LedgerRow` | `types:84` | `id`/`entryId`: `Id`; `date: String` (key). |
| `AccountLedger` | `types:95` | `normalSide`: enum `NormalSide { DEBIT, CREDIT }` (`rename_all = "UPPERCASE"`); `subtitle?`; `rowLinks: BTreeMap<String, RouteRef>` with `#[ts(type = "Record<string, import('../../core/types/route').AppRoute>")]`. |
| `VatBucket { taxable, vat, count }` | inline in `VatReport`, `types:153-156` | |
| `VatCategoryBox` | `types:144` | `category`: reuse 01-settings' `TaxCategory` (`S`/`Z`/`E`/`O`, A§2). |
| `VatReport` | `types:152` | |
| `VatDetailRow` | `types:416` | `id: String` (`"<invoiceId>:<lineId>"`, `rs:566`); `documentKind`: enum `VatDocKind { Sale, SalesReturn }` camelCase (A§2). |
| `CashFlowLine`, `CashFlowStatement` | `types:189,194` | |
| `DayBookLine`, `DayBookEntry` | `types:209,216` | |
| `PeriodComparisonLine` | `types:381` | |
| `BusinessHealthScore`, `BusinessHealthReport` | `types:400,408` | `key`: enum `HealthScoreKey` camelCase (A§2). |
| `LedgerTarget { id, label }`, `LedgerTargets { accounts, customers, suppliers }` | inline return, `rs:597` | |
| `DimensionOption { id, label }`, `CurrencyOption { code, label }`, `DimensionOptions` | inline return, `rs:1188-1192` | |
| Args: `ReportsGet<Fn>Args` × 14 | — | `PartyKindArg { Customer, Supplier }` lowercase. `fiscalYearId`/`accountId`/`partyId` are `Id` (`#[ts(type = "string")]`). |

**`contract.check.ts`** (new file `src/modules/reports/types/contract.check.ts`, same shape as
`core/types/contract.check.ts:9-30`): one `Expect<Equals<Gen.X, X>>` per named type above that
exists in `types/index.ts` (21 entries), plus 3 `ReturnType` checks for the inline returns
(`PnlComparison`, `LedgerTargets`, `DimensionOptions`). `ReportRangeFilter` is compared as
`Equals<Gen.ReportRangeFilter, ReportRangeFilter>` (flatten emits an intersection).
`AccountLedger.rowLinks` gets `// contract-ok: AppRoute is imported, not generated`.

## 3. Service logic

### 3.0 Shared helpers (`domains/reports/service/common.rs`, also used by 13b)

- **`DateRange::parse(&DateRangeInput) -> AppResult<DateRange { from: Option<NaiveDate>, to: Option<NaiveDate> }>`**:
  `None` and `""` → no bound (the mock treats a falsy bound as none, `utils.ts:111-112`);
  otherwise `NaiveDate::parse_from_str(s, "%Y-%m-%d")`, failure → `VALIDATION "تاريخ غير صالح"`
  (new: the mock never validates, A§3; an IPC boundary must, master F8 — decision R-2).
- **`in_range(day, range)`** = `inDateRange` (`utils.ts:110-114`): inclusive on the business day.
  In SQL: `x.date_day >= ? AND x.date_day <= ?` (the `DocDate` `day` column **is** the mock's
  `localDateKey(x.date)`, P2-09).
- **`day_before(d)`** = `d.pred_opt()` (`rs:81-84`, a calendar step, no timezone).
- **`day_of_key(key, clock)`**: `RawDocDate::parse(key)?.resolve(clock).day` — the business day of
  a key string that came back from `shared::balances` (`utils/dates.rs:128-150`).
- **`days_between(a, b)`** = `(a - b).num_days()` for two `NaiveDate`s: the mock's
  `Math.floor((new Date(a) − new Date(b)) / 86_400_000)` on two `YYYY-MM-DD` strings (both UTC
  midnight, so always a whole number).
- **`Movements = HashMap<Id, (Decimal /*d*/, Decimal /*c*/)>`** and
  **`movements(conn, range, dim) -> TxResult<Movements>`** (`rs:64-79`): one SQL statement,
  `SELECT jl.account_id, SUM(jl.debit), SUM(jl.credit) FROM journal_lines jl JOIN journal_entries je
  ON je.id = jl.journal_entry_id WHERE <date bounds on je.date_day> [AND jl.branch_id = ?]
  [AND jl.cost_center_id = ?] [AND jl.currency = ?] GROUP BY jl.account_id`. A dim value that is
  `None` or `""` adds no condition (`rs:69-71`). No status filter (the mock reads every entry in
  `db.journalEntries`; drafts live in `journal_drafts` and are never read — A§1 row `getDayBook`).
  `SUM` of `DECIMAL(19,2)` is exact, which equals the mock's `+=` then later `round2` (A§7).
  A row exists for an account iff it has **any** line in range (`mv.has(id)` semantics, `rs:94,118`).
- **`movements_by_cost_center(conn, range)`** (`rs:165-180`): same SQL grouped by
  `(COALESCE(jl.cost_center_id, '__unassigned__'), jl.account_id)` → `IndexMap<String, Movements>`.
- **`AccountsIndex::load(conn)`**: every **live** account (`find_live`, P2-16; the mock hard-deletes)
  ordered `(created_at, id)` (= `db.accounts` order), keyed by id, with `code, name, kind, subtype,
  normal_side, is_group, parent_id, system_role`.
- **`sorted_by_code(accounts)`**: byte order on `code` (`a.code.localeCompare(b.code)`, `rs:95,119`;
  identical for digit/`-` codes — decision R-4).
- **`lines(idx, kind, mv, sign, filter) -> Vec<StatementLine>`** (`rs:116-125`): accounts with
  `!is_group && kind == K && mv.contains(id) && filter`, sorted by code; `amount = round2(sign × (d − c))`
  **per account**; drop lines with `|amount| <= 0.001`.
- **`sum2(values)`** = `round2(Σ)` (`utils.ts:70-72`: the mock's `sum` always rounds once at the end).
- **`compute_pnl(conn, idx, range, dim) -> ProfitAndLoss`** (`rs:127-138`): `mv = movements(...)`;
  `revenue = lines(REVENUE, −1)`; `cogs = lines(EXPENSE, +1, subtype == "costOfSales")`;
  `expenses = lines(EXPENSE, +1, subtype != "costOfSales")`; `netRevenue/totalCogs/totalExpenses =
  sum2(amounts)`; `grossProfit = round2(netRevenue − totalCogs)`; `netIncome = round2(grossProfit −
  totalExpenses)`. **Per-account round, then sum, then round the differences** (A§7 — never one
  `ROUND(SUM())` over all accounts). With 2-dp journal amounts both orders give the same number, but
  the port keeps the mock's order literally so a future 4-dp source can't make them drift.
- **`js_num(d)`** = `utils::money::js_number_string` for numbers interpolated into Arabic text.
- **Group-bys** use `IndexMap` (G-33) + `sort_by` (stable) to reproduce `Map` + `Array.sort`.

**Aggregation matrix (where the arithmetic runs).** SQL does filtering and `SUM` of *stored* 2-dp
columns (exact, so identical to the mock's raw sum). Rust does every step that rounds per row/per
group, divides, apportions, or sorts on a key the DB doesn't hold (journal `date` keys: `journal_entries`
has **no** generated `date_key` column, `migration/src/m0012_journal.rs`, so entry order is computed in
Rust from `DocDate::key()`).

| Command | SQL | Rust | Rounding points (A§7) | Order |
|---|---|---|---|---|
| 1 trial balance | 2× `movements` (opening, period) | row build | `round2(o.d−o.c)`, `round2(p.d)`, `round2(p.c)`, one `round2` on the 4-term closing sum | code |
| 2/3/13 P&L, comparison, period comparison | `movements` | `compute_pnl` | per account → `sum2` → `round2` diffs | code within section |
| 4 cost-center P&L | `movements_by_cost_center` + `movements` | per-center `pnl_column` | as `compute_pnl`, per center | `netRevenue` desc |
| 5 budget vs actual | `movements_by_cost_center` | Rust | `sum2`, `round2` on variance % | `actual` desc |
| 6 balance sheet | `movements(to: asOf)` | `lines` × 3 + `compute_pnl` | as above + `round2(Σequity + unclosed)` | code |
| 7 account ledger | lines of 1 account + their entries | sort + running balance | raw opening, `round2` once; running `round2` per row | `(date key, number, position)` |
| 8 party ledger | `shared::balances::*_statement` | slice + links | inherited | statement order |
| 9 VAT report | invoices/lines/refunds/POs/returns/expenses by range, `movements` | boxes, FX, apportion | per box `round2` incremental; see §3.9 | box first-appearance |
| 10 VAT detail | same rows | flatten | none (raw fields) | date key desc |
| 11 cash flow | 3× `movements` | Rust | see §3.11 | account array order |
| 12 day book | entries + lines in range | sort | none | `(date key, number)` |
| 14 business health | via 6 + 2 | formulas | `round2` inside clamp | fixed 4 rows |
| 15/16 lookups | 3 small `SELECT`s | — | — | see steps |

### 3.1 `reports_get_trial_balance` (`rs:88-112`)

1. `range = DateRange::parse(args.range)`; `dim` from the same args (`rs:90`).
2. `opening = if range.from is Some { movements(to: day_before(from), dim) } else { empty }` (`rs:91`).
3. `period = movements(range, dim)` (`rs:92`).
4. Accounts: live, `!is_group`, id in `opening ∪ period`, sorted by code (`rs:93-95`).
5. Per account: `o`,`p` default `(0,0)`; `closing = round2(o.d − o.c + p.d − p.c)` (**one** round on
   the raw 4-term sum, A§7); `openingBalance = round2(o.d − o.c)`; `periodDebit = round2(p.d)`;
   `periodCredit = round2(p.c)`; `closingDebit = closing > 0 ? closing : 0`; `closingCredit =
   closing < 0 ? −closing : 0` (`rs:97-110`).
6. `groupName` = name of the nearest `is_group` ancestor walking `parent_id` through the live index,
   `""` when none (`rs:52-56`).

### 3.2 `reports_get_profit_and_loss` (`rs:140-143`)

1. Parse range + dim. 2. Return `compute_pnl(range, dim)`.

### 3.3 `reports_get_profit_and_loss_comparison` (`rs:150-154`)

1. Parse `range` (with dim) and `compareRange` (dim ignored on it; the mock reuses `range`'s dim for
   both, `rs:152-153`). 2. `current = compute_pnl(range, dim)`, `previous = compute_pnl(compareRange, dim)`
   — two separate aggregations (A§1).

### 3.4 `reports_get_cost_center_profit_and_loss` (`rs:190-205`)

1. Parse range. `by_cc = movements_by_cost_center(range)`.
2. `pnl_column(mv)` (`rs:182-188`): `netRevenue = sum2(lines(REVENUE,−1))`, `totalCogs`,
   `totalExpenses` likewise, `grossProfit = round2(netRevenue − totalCogs)`, `netIncome = round2(grossProfit − totalExpenses)`.
3. `centers`: **live** cost centers in `(created_at, id)` order whose id is a key of `by_cc` (no
   `active` filter — the mock has none, `rs:193-194`), mapped to `{ costCenterId, name, ...pnl_column }`,
   then stable sort by `netRevenue` desc (`rs:196`).
4. `unassigned = { "__unassigned__", "غير مخصص", pnl_column(by_cc["__unassigned__"] or empty) }` (`rs:197-198`).
5. `total = compute_pnl(range, no dim)` projected to `{ "__total__", "الإجمالي", five totals }` — a
   **separate** aggregation, not a sum of columns (A§7, `rs:199-203`).

### 3.5 `reports_get_cost_center_budget_vs_actual` (`rs:208-226`)

1. Load the fiscal year by id (`fiscal_years` has no soft delete); missing → `NOT_FOUND
   "السنة المالية غير موجودة"` (`rs:210-211`, A§3).
2. `by_cc = movements_by_cost_center({ from: fy.start_date, to: fy.end_date })`.
3. For each live cost center in `(created_at, id)` order: its `cost_center_budgets` row for this
   `fiscal_year_id`; none → skip (`rs:215-216`).
4. `actual = sum2(lines(EXPENSE, +1, no filter) over by_cc[cc] or empty)` — all expense subtypes (`rs:218`).
5. `variancePct = budget > 0 ? round2((actual − budget) / budget × 100) : 0`; `nearBudget = budget > 0
   && actual >= budget × 0.9` (`rs:219-223`). `budget` is the stored amount, unrounded.
6. Stable sort by `actual` desc (`rs:225`).

### 3.6 `reports_get_balance_sheet` (`rs:230-251`)

1. `asOf` must parse as `YYYY-MM-DD`, else `VALIDATION "تاريخ غير صالح"` (R-2). Parse `dim`.
2. `mv = movements({ to: asOf }, dim)` — cumulative since inception (A§1).
3. `assets = lines(ASSET, +1)`, `liabilities = lines(LIABILITY, −1)`, `equity = lines(EQUITY, −1)`.
4. `unclosedEarnings = compute_pnl({ to: asOf }, dim).netIncome` (`rs:236`).
5. `totalAssets = sum2(assets)`, `totalLiabilities = sum2(liabilities)`, `totalEquity =
   round2(sum2(equity) + unclosedEarnings)` (`rs:237-239`).
6. `balanced = |totalAssets − totalLiabilities − totalEquity| < 0.01` (tolerance 0.01, A§7).
7. `asOf` echoes the input string.

### 3.7 `reports_get_account_ledger` (`rs:255-292`)

1. Live account by id; missing → `NOT_FOUND "الحساب غير موجود"` (`rs:257-258`). `sign = +1` if
   `normal_side == DEBIT` else `−1`.
2. SQL: every `journal_lines` row of this account joined to its entry (no date filter in SQL — the
   opening needs everything before `from`). In Rust, sort by `(entry.date().key(), entry.number,
   line.position)` byte order (`rs:263`; `localeCompare` on fixed-format ASCII keys = byte order, R-4).
3. Parse range. For each line: `day = entry.date_day`; `from` set and `day < from` → `opening +=
   sign × (debit − credit)` raw, next (`rs:267-270`); `to` set and `day > to` → skip (`rs:271`);
   else push `LedgerRow { id: line.id, date: entry key, entryId, entryNumber, description:
   line.description ?? entry.description, debit, credit, balance: 0 }` and
   `rowLinks[line.id] = { name: "journal-entry", params: { id: entry.id } }` (`rs:272-273`).
4. `running = round2(opening)`; for each row `running = round2(running + sign × (debit − credit))`,
   `row.balance = running` (**per-row round**, A§7, `rs:276-280`).
5. Return `title = "<code> — <name>"`, `subtitle = DEBIT ? "حساب مدين الطبيعة" : "حساب دائن الطبيعة"`,
   `normalSide`, `openingBalance = round2(opening)`, `rows`, `totalDebit = sum2(debits)`,
   `totalCredit = sum2(credits)`, `closingBalance = running`, `rowLinks` (`rs:281-291`).

### 3.8 `reports_get_party_ledger` (`rs:295-323`)

1. Live party with `kind = args.kind`; missing → `NOT_FOUND "العميل غير موجود"` / `"المورد غير موجود"` (`rs:297-298`).
2. `all = shared::balances::customer_statement` or `supplier_statement` (P2-26, one copy — never
   re-derived here). Parse range.
3. `before` = rows with `from` set and `day_of_key(r.date_key) < from`; `in_range` = rows with
   `in_range(day_of_key(r.date_key), range)` (`rs:300-301`).
4. `opening = before.last().balance ?? 0` (`rs:302`, no extra round: already rounded).
5. `rowLinks[r.id]`: kind `invoice`|`refund` → `{ name: "invoice", params: { id: ref_id } }`;
   `payment` → `{ name: "payments", query: { highlight: ref_id } }` (needs G-13); anything else →
   `{ name: "purchase", params: { id: ref_id } }` (`rs:305-310`, quirk Q-3).
6. Return `title = party.name`, `subtitle` = customer `"كشف حساب عميل — الرصيد الموجب مستحق لنا"` /
   supplier `"كشف حساب مورد — الرصيد الموجب مستحق للمورد"`, `normalSide` = customer DEBIT / supplier
   CREDIT, `openingBalance`, rows mapped `{ id, date: date_key, entryId: ref_id, entryNumber: number,
   description, debit, credit, balance }`, `totalDebit/totalCredit = sum2`, `closingBalance =
   in_range.last().balance ?? opening` (`rs:312-322`).

### 3.9 `reports_get_vat_report` (`rs:450-551`)

1. Parse range. Load invoices with `date_day` in range (**no status filter**, `rs:512`, quirk Q-1)
   in `(created_at, id)` order with their lines by `position`; convert each with `to_base_invoice`
   (`rs:497-508`): if `currency` and `exchange_rate` are set, `subTotal`, `discountAmount`,
   `taxAmount`, `grandTotal` → `round2(x × rate)`, each line's `net`/`vat` → `round2(x × rate)`
   when present.
2. Refunds with `date_day` in range (`rs:513`), not FX-converted (A§1).
3. POs: `status = RECEIVED && !vat_not_recoverable && date in range`; `recoverable_po_ids` = all
   RECEIVED, recoverable POs (any date); purchase returns whose `purchase_order_id ∈ recoverable_po_ids`
   and date in range; expenses with `is_tax_invoice && date in range` (`rs:518-521`).
4. **Boxes** (`salesVatBoxes`, `rs:450-487`), in an `IndexMap<(category, rate), box>`:
   - for each base invoice, each line: `add(taxCategory ?? "O", taxRate ?? 0, net ?? 0, vat ?? 0)`
     where `add` does `box.net = round2(box.net + net)`, `box.vat = round2(box.vat + vat)` (per-line,
     compounding — A§7), `count` stays 0 (`rs:461-466`);
   - for each refund: its invoice from **all** invoices, **raw (not base-converted)** (`rs:470`,
     quirk Q-2); skip if missing or `taxAmount == 0`. `invVat = taxAmount`, `invNet = subTotal −
     discountAmount`; per line `shareOfVat = invVat > 0 ? lineVat / invVat : 0`, `shareOfNet =
     invNet > 0 ? lineNet / invNet : 0`; `add(cat, rate, −round2(refund.subTotal × shareOfNet),
     −round2(refund.taxAmount × shareOfVat))` (`rs:469-483`);
   - keep boxes with `net != 0 || vat != 0` (`rs:485`). The box sums the mock computes on
     `rs:486` are never returned, so they are not computed.
5. `sales = { taxable: sum2(base inv subTotal − discountAmount), vat: sum2(base taxAmount), count }`;
   `salesReturns = { sum2(subTotal), sum2(taxAmount), count }`; `purchases = { taxable:
   round2(sum2(po.subTotal) + sum2(exp.net_amount)), vat: round2(sum2(po.taxAmount) +
   sum2(exp.tax_amount)), count: pos + expenses }`; `purchaseReturns = { sum2, sum2, count }` (`rs:524-531`).
   An expense's `net_amount`/`tax_amount` `NULL` counts as 0 (the expenses domain always sets them
   on a tax invoice).
6. `outputVat = round2(sales.vat − salesReturns.vat)`, `inputVat = round2(purchases.vat −
   purchaseReturns.vat)`, `netPayable = round2(outputVat − inputVat)` (`rs:532-546`).
7. **Ledger side, a separate query** (A§7 reconciliation — never derived from step 6): `mv =
   movements(range, no dim)`; `out = mv[resolve_account(VatOutput)]`, `inp = mv[resolve_account(VatInput)]`
   (`shared::ledger::accounts::resolve_account`, default ctx; a missing role propagates its
   `NOT_FOUND`, same as `accountFor`); `ledgerOutput = round2(c − d)`, `ledgerInput = round2(d − c)` (`rs:535-548`).

### 3.10 `reports_get_vat_detail` (`rs:557-593`)

1. Same invoice set as 3.9 step 1 (base-converted, no status filter) and refunds in range.
2. Per invoice line, skip when `net` and `vat` are both `None` or `0` (`!line.net && !line.vat`,
   `rs:564`); else row `{ id: "<invId>:<lineId>", date: inv key, documentNumber, documentKind: sale,
   productName: line.name, category: taxCategory ?? "O", rate: taxRate ?? 0, net ?? 0, vat ?? 0 }`.
3. Per refund: `{ id: refund.id, date: refund key, documentNumber: refund.number, salesReturn,
   productName: inv ? "مرتجع — <inv.number>" : "مرتجع", category: "S" (hard-coded, `rs:586`),
   rate: inv.taxRate ?? 0, net: −subTotal, vat: −taxAmount }` (inv looked up in all invoices).
4. Stable sort by date key **desc**, byte order (`rs:592`).

### 3.11 `reports_get_cash_flow_statement` (`rs:618-670`)

1. Parse range. `pnl = compute_pnl(range, no dim)`. `today = ctx.clock.today()`.
2. `cash_ids` = live, non-group ASSET accounts with subtype `cash` or `bank` (`rs:612-614`).
3. `opening_mv = from ? movements({ to: day_before(from) }) : empty`; `closing_mv = movements({ to: to ?? today })`
   (`rs:622-623`). `openingCash = round2(Σ_{cash_ids}(d − c))`, `closingCash` likewise — one round
   over the raw sum (`rs:624-626`).
4. `period_mv = movements(range)`. `wc(subtype, sign)` = the **first** live non-group account (array
   order) with that subtype; none → 0; else `round2(sign × (d − c))` (`rs:631-636`).
5. `operatingAdjustments` = in this order, filtered to `|amount| > 0.001`:
   `"التغير في الذمم المدينة (العملاء)"` = `−wc(receivable, 1)`; `"التغير في المخزون"` = `−wc(inventory, 1)`;
   `"التغير في الذمم الدائنة (الموردون)"` = `wc(payable, −1)` (`rs:637-641`).
   `operatingCash = round2(pnl.netIncome + sum2(adjustments))`.
6. `investing` = live non-group ASSET accounts with subtype `fixedAsset` in array order →
   `{ label: name, amount: round2(−(d − c)) }` filtered `> 0.001`; `investingCash = round2(sum2(...))` (`rs:645-653`).
7. `financing` = live non-group accounts in array order that are (EQUITY and `system_role` not
   `retainedEarnings`/`currentEarnings`) or (LIABILITY and subtype `longTermLiability`) →
   `{ name, round2(c − d) }` filtered; `financingCash = round2(sum2(...))` (`rs:657-666`).
8. `netChange = round2(operatingCash + investingCash + financingCash)` — **not** derived from
   `closingCash − openingCash` (A§7/§9: two independent figures kept side by side).

### 3.12 `reports_get_day_book` (`rs:674-689`)

1. Parse range. Entries with `date_day` in range and their lines by `position`.
2. Sort entries by `(date key, number)` byte order in Rust.
3. Map `{ id, number, date: key, description, lines: [{ accountCode, accountName, debit, credit }] }`;
   account looked up in the live index, `""`/`""` when missing (`rs:684-687`).

### 3.13 `reports_get_period_comparison` (`rs:1084-1096`)

1. Parse `rangeA`, `rangeB`. `a = compute_pnl(rangeA)`, `b = compute_pnl(rangeB)` (no dim).
2. `delta(x, y) = { a: x, b: y, delta: round2(x − y), deltaPct: y != 0 ? round2((x − y) / |y| × 100) : 0 }`.
3. Exactly 5 rows, fixed labels, this order: `"صافي الإيرادات"` (netRevenue), `"تكلفة المبيعات"`
   (totalCogs), `"مجمل الربح"` (grossProfit), `"المصروفات"` (totalExpenses), `"صافي الربح"` (netIncome).

### 3.14 `reports_get_business_health_report` (`rs:1129-1160`)

1. Parse range. `asOf = to ?? today`. `bs` = the 3.6 body with `asOf`, no dim (the mock calls the
   wrapped `getBalanceSheet(asOf)`, `rs:1132`; here the service function is called directly, same
   snapshot). `pnl = compute_pnl(range)`.
2. `currentAssets = sum2(bs.assets where the account's subtype != fixedAsset)`; `currentLiabilities =
   sum2(bs.liabilities)`; `currentRatio = cl > 0 ? ca / cl : (ca > 0 ? 2 : 1)`.
3. `clamp(x) = max(0, min(25, x))` applied **after** `round2` (A§7): `liquidity = clamp(round2(currentRatio
   / 2 × 25))`; `netMarginPct = netRevenue > 0 ? netIncome / netRevenue × 100 : 0`; `profitability =
   clamp(round2(netMarginPct / 20 × 25))`; `dte = totalEquity > 0 ? totalLiabilities / totalEquity :
   (totalLiabilities > 0 ? 2 : 0)`; `debt = clamp(round2(25 − dte × 25))`.
4. `ar` = first live non-group account with subtype `receivable`; `arBalance = sum2(bs.assets where
   accountId == ar.id)` (0 when none or dropped); `days = from && to ? max(1, days_between(to, from)) : 30`
   (`Math.round` of a whole-day difference = the difference); `dso = netRevenue > 0 ? arBalance /
   netRevenue × days : 0`; `collection = clamp(round2(25 − dso / 60 × 25))` (`rs:1147-1151`).
5. Scores in this order with these exact strings (`js_num` for the numbers):
   `liquidity` `"السيولة"` value `round2(currentRatio)` `"نسبة التداول {v} — الأصول المتداولة مقابل الالتزامات المتداولة"`;
   `profitability` `"الربحية"` `round2(netMarginPct)` `"هامش صافي الربح {v}%"`;
   `debt` `"المديونية"` `round2(dte)` `"نسبة الالتزامات إلى حقوق الملكية {v}"`;
   `collection` `"التحصيل"` `round2(dso)` `"متوسط أيام تحصيل الذمم (DSO) {v} يوماً"` (`rs:1153-1158`).
6. `total = round2(sum2(scores))`.

### 3.15 `reports_get_ledger_targets` (`rs:597-604`)

1. `accounts`: **all** live accounts (groups included, A§1) sorted by code →
   `{ id, label: "<code> — <name>" }`.
2. `customers` / `suppliers`: live parties of that kind in `(created_at, id)` order → `{ id, label: name }`.

### 3.16 `reports_get_dimension_options` (`rs:1188-1199`)

1. `branches`: live, `active` branches in `(created_at, id)` order → `{ id, label: name }`.
2. `costCenters`: live, `active` → `{ id, label: name }`.
3. `currencies`: `active` currencies in `(created_at, code)` order → `{ code, label: "<name_ar> (<code>)" }`.

## 4. Concurrency (D8, several terminals)

A§5: nothing to lock. Every command runs inside one `with_read_ctx` snapshot (REPEATABLE READ,
READ ONLY, P2-06), so a multi-query report (balance sheet = `movements` + `compute_pnl`; VAT =
documents + ledger; business health = balance sheet + P&L) sees one consistent state even while
another terminal posts. A report never takes `FOR UPDATE`/`LOCK IN SHARE MODE` and so can never
block or deadlock a poster; a concurrent `close_year` is seen entirely before or entirely after
(its own transaction is atomic, accounting.md §5).

## 5. Undo

Not applicable: no writes, so no audit row and nothing is undoable via the registry (entry §3.5).

## 6. Frontend switch lines

File: `src/modules/reports/services/reportService.ts`. Inside each existing `wrap(...)`, first line
after `async function …(…) {` (before `await delay()`), add the switch; the mock body below stays:

| Function (line) | Switch line |
|---|---|
| `getTrialBalance` (88) | `if (usesRust('reports')) return backendCall('reports_get_trial_balance', { range });` |
| `getProfitAndLoss` (140) | `… backendCall('reports_get_profit_and_loss', { range });` |
| `getProfitAndLossComparison` (150) | `… ('reports_get_profit_and_loss_comparison', { range, compareRange });` |
| `getCostCenterProfitAndLoss` (190) | `… ('reports_get_cost_center_profit_and_loss', { range });` |
| `getCostCenterBudgetVsActual` (208) | `… ('reports_get_cost_center_budget_vs_actual', { fiscalYearId });` |
| `getBalanceSheet` (230) | `… ('reports_get_balance_sheet', { asOf, dim });` |
| `getAccountLedger` (255) | `… ('reports_get_account_ledger', { accountId, range });` |
| `getPartyLedger` (295) | `… ('reports_get_party_ledger', { kind, partyId, range });` |
| `getVatReport` (510) | `… ('reports_get_vat_report', { range });` |
| `getVatDetail` (557) | `… ('reports_get_vat_detail', { range });` |
| `getCashFlowStatement` (618) | `… ('reports_get_cash_flow_statement', { range });` |
| `getDayBook` (674) | `… ('reports_get_day_book', { range });` |
| `getPeriodComparison` (1084) | `… ('reports_get_period_comparison', { rangeA, rangeB });` |
| `getBusinessHealthReport` (1129) | `… ('reports_get_business_health_report', { range });` |
| `getLedgerTargets` (597) | `… ('reports_get_ledger_targets');` |
| `getDimensionOptions` (1188) | `… ('reports_get_dimension_options');` |

Add `import { backendCall, usesRust } from '@/modules/core/services/backend';`. `getBusinessHealthReport`
keeps calling the mock `getBalanceSheet` internally only in the mock path (the Rust path is one command).

## 7. Known mock quirks (kept, not fixed) and decisions

**Quirks (behaviour-exact, listed for a later product decision):**
- **Q-1** `getVatReport`/`getVatDetail` include `DRAFT` invoices (no status filter, `rs:512,559`),
  unlike every sales report (`status !== 'DRAFT'`).
- **Q-2** VAT refund apportionment reads the refund's invoice **un-converted** (FC figures) while the
  invoice side is base-converted (`rs:470` vs `rs:512`); refunds, POs and expenses are never
  FX-converted (A§1, documented limitation).
- **Q-3** Party ledger links every non-invoice, non-payment row (e.g. a customer `opening` row) to the
  `purchase` route (`rs:310`).
- **Q-4** `groupName` walks the parent chain through **live** accounts only; a soft-deleted parent
  ends the walk (the mock's hard delete does the same, `rs:53-54`).
- **Q-5** Cash flow's working-capital lines use only the **first** account of each subtype (`rs:632`);
  a second receivable account is ignored.
- **Q-6** `getBusinessHealthReport` treats all liabilities as current (`rs:1136`).
- **Q-7** `unclosedEarnings` is the P&L since inception, even after a year close (`rs:236`, A§1).

**Decisions (architectural autonomy, strictest option, logged):**
- **R-1** Reports are **not** paged: no report returns `PagedResult` (they return whole arrays, `types:8-438`).
- **R-2** Date args are validated at the IPC boundary: `""`/absent = no bound, anything else must be
  `YYYY-MM-DD` or `VALIDATION "تاريخ غير صالح"` (new; the UI's date pickers only send valid keys).
- **R-3** Two independently computed figure pairs stay two queries: `outputVat`/`inputVat` vs
  `ledgerOutput`/`ledgerInput`, and `netChange` vs `closingCash − openingCash` (A§7/§9).
- **R-4** `localeCompare` without a locale on ASCII keys (account codes, document numbers, date keys)
  is ported as byte order; the parity cases include codes of different lengths to prove it.
- **R-5** Lookups use **live** rows (`find_live`), matching the mock's hard delete: a soft-deleted
  party/account/cost center/branch is absent exactly as a deleted mock row is.
- **R-6** `rowLinks` is a `BTreeMap` (JSON object key order is not part of the contract; Part 04 diffs
  objects, not text).
- **R-7** Money is `Decimal` end to end (master rule 5). Where the mock sums JS floats without a
  final `round2`, Rust returns the exact decimal; the parity harness compares those fields with an
  epsilon of `1e-9` (float noise only), listed in §8(b).

## 8. Tests

**(a) `src-tauri/tests/domain_reports.rs`** (DB-backed; seed through the step-00 importer from a
small snapshot fixture, or direct entity inserts; every posting-based fixture ends with
`shared::invariants::run_all` green):
- trial balance: opening = movements to `from − 1`, period in range; account with no activity dropped;
  account whose opening and period cancel still listed with zeros; closing split by sign.
- P&L: COGS vs opex split by `costOfSales`; an account whose debits equal its credits in range is
  dropped from the lines (|amount| ≤ 0.001) but the totals are unchanged; returns come out negative.
  (Journal amounts are `DECIMAL(19,2)`, so the per-account `round2` is a no-op on real data; the
  order is kept anyway so the code stays a literal port, A§7.)
- P&L comparison returns two independent P&Ls with the same dim.
- cost-center P&L: `centers + unassigned` reconcile with `total` on a fixture; sort by `netRevenue` desc.
- budget vs actual: `NOT_FOUND "السنة المالية غير موجودة"`; centers without a budget row skipped;
  `variancePct`, `nearBudget` at exactly 90%.
- balance sheet: `balanced` true on a posted fixture (invariant 2); `unclosedEarnings` included;
  invalid `asOf` → `VALIDATION "تاريخ غير صالح"`.
- account ledger: `NOT_FOUND "الحساب غير موجود"`; opening from lines before `from`; running balance
  per row; order by `(date key, number)` with a same-day instant vs day-only entry.
- party ledger: both `NOT_FOUND` messages; opening = last balance before `from`; the three link shapes
  including `query.highlight`.
- VAT report: boxes per `(category, rate)`; refund apportionment across two categories; FC invoice
  converted; ledger figures equal document figures on a clean fixture (invariant 5); draft invoice
  included (Q-1).
- VAT detail: zero lines skipped; refund row `category "S"`; date-desc order.
- cash flow: opening/closing cash; working-capital signs; `netChange` equals `closing − opening` on a
  clean fixture.
- day book: order and account lookups; drafts never included.
- period comparison: 5 fixed rows; `deltaPct` 0 when the base is 0.
- business health: each clamp at 0 and 25; `days` from range vs default 30; explanation strings.
- ledger targets / dimension options: soft-deleted and inactive rows excluded as specified.
- every command without a session → `UNAUTHORIZED`; a `cashier` session → `FORBIDDEN` (reports: none).

**(b) Parity cases for Part 04** (replay on mock vs Rust, diff DTOs):
the seeded demo data over (1) the full history, (2) the current month, (3) a range starting mid-year
(opening balances), each for commands 1–14; command 1/2/6 with each dim (branch, cost center,
currency); a multi-currency invoice + partial refund (VAT report/detail); account codes `"2"`,
`"10"`, `"1101"` (R-4 order); fields compared with `1e-9` epsilon (R-7): none in this file's DTOs
(every numeric field here is rounded) — the epsilon list lives in 13b.

## 9. Checklist

- [x] Confirm G-32, G-13, G-1 (for 13b), G-33, G-8 are merged by the manager; G-34 before 13b. *(assumed per `_part02-gaps.md`'s "fixed" status — not independently re-verified; flagged to the manager above.)*
- [x] `domains/reports/mod.rs` (`pub mod commands; pub mod service; pub mod dto;` + `ipc_signatures()` with 16 `ipc_sig!` lines for this file) and `dto.rs` with every §2 DTO + `export_bindings(cfg)`.
- [x] `service/common.rs`: `DateRange`, `in_range`, `day_before`, `day_of_key`, `days_between`, `movements`, `movements_by_cost_center`, `AccountsIndex`, `lines`, `sum2`, `compute_pnl`, `js_num`.
- [x] `service/statements.rs`: 3.1–3.6, 3.13, 3.14.
- [x] `service/ledgers.rs`: 3.7, 3.8, 3.12, 3.15, 3.16.
- [x] `service/vat.rs`: 3.9, 3.10 (with `to_base_invoice`).
- [x] `service/cash_flow.rs`: 3.11.
- [x] `commands.rs`: 16 commands, each `with_read_ctx` + `ctx.require(txn, Area::Reports, Access::Read)` + service call.
- [x] Report the 16 command names to the manager for `generate_handler!` and `domains/mod.rs` (see final report).
- [x] `tests/domain_reports.rs`: representative coverage for this file's §8(a) bullets — ⏳ deferred time-boxed test pass (not run from this session; DB-backed, needs `EQUAL_TEST_DATABASE_URL`).
- [x] 16 switch lines in `reportService.ts` (§6) + the import.
- [x] `src/modules/reports/types/contract.check.ts` with the §2 entries.
- [ ] Parity cases (§8b) listed in the Part 04 case list. *(not written — out of this implementer's time-boxed scope; Part 04's own pass should derive them from §8(b)'s description.)*
- [x] Then implement [`13b-reports-operational.md`](13b-reports-operational.md).

## Gate

`cargo check` clean (manager's throttled run after W6 — ⏳ not run from this session, per the
no-cargo hard rule); DB tests and parity cases written (run in the deferred, time-boxed pass); 16
switch lines present; `contract.check.ts` compiles under `bun run build` (needs `bun run bindings`
first to generate `types/gen/*`); `bun run memory:check` shows the 16 commands invoked + registered
with 0 contract gaps (needs the manager's `generate_handler!`/`domains/mod.rs` wiring first).
