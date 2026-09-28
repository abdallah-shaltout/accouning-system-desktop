# 21 · 01.B — `reports` contract

> **Status:** done (2026-09-27) · **Inventory:** `docs/backend/contract/reports.md`
> (regenerate with `bun run contract`) · **Mock spec:** none — **this module has no
> `src/mocks/backend/reports.ts`**; every function lives directly in
> `src/modules/reports/services/reportService.ts` (1199 lines) and reads `db.*` tables straight
> from `@/mocks`, plus two read-only calls into other modules' mock code
> (`accountFor` from `backend/accounts.ts`, `customerStatement`/`supplierStatement` from
> `backend/balances.ts`, `getOpenDocumentsFor` from `backend/payments.ts`) · **Services:**
> `src/modules/reports/services/reportService.ts` · **Types:** `src/modules/reports/types/index.ts`
>
> **This module is categorically different from every module reviewed so far: it is 100% read-only.**
> 32 functions, **0 writes** (confirmed both by the generator's blank "Writes" column for every row
> and by inspection — no `db.<table>.push`, no `mutate()`, no `logActivity`/`logAudit` call anywhere
> in the file), **0 shared-manager reach** (no `ledger`/`stock`/`activity`/`numbering`/`period`/
> `currency` call — it only *reads* `journalEntries`, never posts to it), and **0 undo-relevant
> writes**, so §4 is "n/a — no writes" for all 32, which is the correct, expected outcome for a pure
> aggregation module, not a shortcut. Every figure is computed on the fly from `journalEntries`,
> `invoices`, `purchaseOrders`, `expenses`, `products`, `stockMovements`, `stockCounts`,
> `stockTransfers` and `shifts` — nothing is stored redundantly, matching the exact pattern
> `parties.md` already established for `Customer.balance`/`PartyStatementRow` (`movements()` in this
> file is this module's own version of that file's ledger-scan pattern, generalized to every
> account instead of one party's receivable/payable account). Read `docs/v2/02-accounting-review.md`
> (§4 invariant 5, VAT) and `src/mocks/backend/invariants.ts`'s `checkVatControl` (§4.5) to ground
> §7's VAT reconciliation claims in what `bun run verify:mocks` actually checks, and cross-referenced
> `accounting.md` (system-role account resolution via `accountFor`, `JournalSourceKind`/journal-line
> shape) and `parties.md` (the `movements()`/ledger-scan aggregation pattern, `getOpenDocumentsFor`
> reuse for aging) throughout, per the task's explicit instruction not to re-derive what those two
> already documented. **Baseline `bun run verify:mocks`: 128 ok, 0 failed**, confirmed fresh before
> any edit (matches every prior module's baseline this session). **Zero mock fixes applied** — the
> fourth module this session with none (after `purchases.md`, `invoices.md`, `vouchers.md`): every
> named bug class was checked and found not applicable to a read-only module (no plain `Error`
> anywhere — all three throw sites, `getAccountLedger`/`getPartyLedger`/`getCostCenterBudgetVsActual`,
> already use `ApiError` with `NOT_FOUND`; no date-range boundary bug — every range filter reuses the
> shared `inDateRange`/`localDateKey` helpers exactly as `parties.md`/`accounting.md` already
> verified them). One genuine rounding-consistency question is flagged, not silently fixed, in §8/§9:
> `getVatReport`'s own `sales.vat`/`purchases.vat` figures and `checkVatControl`'s (§4.5) figures are
> two **independently computed** aggregations over the same document tables that are expected to
> agree today but are not mechanically tied together — see §7 and §9 for the precise reconciliation
> claim and why it's a design note for Part 03, not a bug to fix in the mock.

## 1. Endpoints

32 functions, grouped by report family. Every disposition is **port (confirmed)**, every "Writes"
is **—**, every "Shared" is **—**, every "Undo" is **n/a (read-only, no writes)** — stated once here
rather than repeated 32 times in the table, per §4's note below.

| Function | Disposition | Rust command | Request DTO | Response DTO | Notes |
|---|---|---|---|---|---|
| `getTrialBalance` | port (confirmed) | `reports_get_trial_balance` | `ReportRangeFilter` (`DateRangeInput & DimensionFilter`) | `TrialBalanceRow[]` | Opening balance = movements up to `dayBefore(range.from)`; period = movements in range; closing = opening+period split back into debit/credit columns by sign. Only accounts with any opening or period activity are included (accounts with zero activity in both are dropped, not zero-filled) — see §7. |
| `getProfitAndLoss` | port (confirmed) | `reports_get_profit_and_loss` | `ReportRangeFilter` | `ProfitAndLoss` | Thin wrapper over the internal `computePnl(range, dim)` — see §7, this is the single most-reused aggregation in the module (`getBalanceSheet`, `getCostCenterProfitAndLoss`, `getCashFlowStatement`, `getBusinessHealthReport`, `getPeriodComparison` all call it or its `dim`-parameterized core). |
| `getProfitAndLossComparison` | port (confirmed) | `reports_get_profit_and_loss_comparison` | `{ range: ReportRangeFilter, compareRange: DateRangeInput }` | `{ current: ProfitAndLoss; previous: ProfitAndLoss }` | Calls `computePnl` twice with the same `dim`, once per range — two independent aggregations, not a single query with a period column, so Part 03's SQL can just run the P&L query twice with different date bounds. |
| `getCostCenterProfitAndLoss` | port (confirmed) | `reports_get_cost_center_profit_and_loss` | `DateRangeInput` | `CostCenterPnl` | Same P&L shape as `getProfitAndLoss` but keyed by `(costCenterId, accountId)` instead of just `accountId` — see §7, `movementsByCostCenter`. `unassigned` bucket catches lines with no `costCenterId`. `total` is `computePnl(range)` (no `dim`) called a *third* time — **must reconcile exactly** with `centers.reduce + unassigned` by construction (every journal line has exactly one `costCenterId` or none), a good parity-harness cross-check. |
| `getCostCenterBudgetVsActual` | port (confirmed) | `reports_get_cost_center_budget_vs_actual` | `{ fiscalYearId: string }` | `CostCenterBudgetRow[]` | `NOT_FOUND` if the fiscal year doesn't exist. `actual` = `movementsByCostCenter({ from: fy.startDate, to: fy.endDate })`'s EXPENSE-kind sum (all expense subtypes, unlike the P&L split — no COGS/opex separation here). Only cost centers with a `budgets[]` row for this `fiscalYearId` are included (no zero-budget rows). |
| `getBalanceSheet` | port (confirmed) | `reports_get_balance_sheet` | `{ asOf: string, dim?: DimensionFilter }` | `BalanceSheet` | `movements({ to: asOf }, dim)` — a **cumulative since-inception** scan (no `from`), unlike every other report's bounded range; `dayBefore`/opening-balance logic doesn't apply here since a balance sheet is always "as of," never "for a period." `unclosedEarnings` = `computePnl({ to: asOf }, dim).netIncome` — the running, not-yet-closed-to-retained-earnings P&L since inception, added into `totalEquity` so the sheet balances even mid-fiscal-year before `closeYear` (`accounting.md`) sweeps it in. `balanced` is a self-check (`|assets − liabilities − equity| < 0.01`), not a stored flag. |
| `getAccountLedger` | port (confirmed) | `reports_get_account_ledger` | `{ accountId: string, range: DateRangeInput }` | `AccountLedger` | `NOT_FOUND` if the account doesn't exist. Scans **every** journal entry (not `movements()`) since it needs per-line rows, not just the summed total, sorted by `(date, number)`. Running balance is accumulated **row by row** with `round2` at each step (not computed once at the end) — see §7, this incremental-rounding detail matters for byte-identical parity. `rowLinks` map every line to a `journal-entry` route — already a named route object, not a path string (no F7 gap in this module, confirmed by grep — no `link:` field or path template string anywhere in this file). |
| `getPartyLedger` | port (confirmed) | `reports_get_party_ledger` | `{ kind: 'customer' \| 'supplier', partyId: string, range: DateRangeInput }` | `AccountLedger` | `NOT_FOUND` if the party doesn't exist. **Delegates entirely** to `parties.md`'s already-reviewed `customerStatement`/`supplierStatement` (`backend/balances.ts`) rather than re-deriving from `journalEntries` itself — this function's own job is just re-shaping that output into the generic `AccountLedger` shape (same fields as `getAccountLedger`) and building `rowLinks` per source kind (`invoice`/`refund` → `invoice` route, `payment` → `payments` route with a `highlight` query param, else → `purchase` route). No new aggregation logic of its own — see §7 for why this row is intentionally thin. |
| `getSalesReport` | port (confirmed) | `reports_get_sales_report` | `DateRangeInput` | `SalesReport` | The richest single aggregation in the module — 6 breakdowns (summary, byDay, byProduct, byCategory, byMethod, byCashier) computed from one pass over `invoices`(non-draft)+`refunds`. See §7 for the tax-inclusive-vs-exclusive revenue-stripping logic (`inclusive` flag, VAT-exclusion factor) — this is the module's sharpest rounding-order concern, flagged precisely in §9. |
| `getInventoryReport` | port (confirmed) | `reports_get_inventory_report` | — | `InventoryReportRow[]` | Pure snapshot of `products` (active, `type==='product'`) — no date range, no journal read at all; `costValue`/`retailValue` = `stockQty × costPrice`/`price` at *today's* stored cost, not a historical valuation. `status` derived from `stockQty` vs `minStock`, matching `products.md`'s own low/out-of-stock thresholds (reused, not redefined). |
| `getVatReport` | port (confirmed) | `reports_get_vat_report` | `DateRangeInput` | `VatReport` | **The module's most accounting-sensitive function** — see §7/§9 for the full box-by-box breakdown and the ledger-vs-document reconciliation (`ledgerOutput`/`ledgerInput` vs `outputVat`/`inputVat`) this function itself surfaces as two parallel figures, on purpose (a discrepancy here is meant to be visible to the user, not hidden). Converts every invoice to base currency via `toBaseInvoice` before aggregating (phase-9 FX rule, `invoices.md`/`settings.md`'s FX scale) — refunds/POs/expenses aren't FX-extended (documented limitation, not a gap this review introduces). |
| `getVatDetail` | port (confirmed) | `reports_get_vat_detail` | `DateRangeInput` | `VatDetailRow[]` | Flat, line-level version of `getVatReport`'s `salesBoxes` — same source rows, no re-aggregation, just un-grouped. Refund rows synthesize a single row per refund (not per refunded line) with the *invoice's* category/rate, `net`/`vat` negated — an intentional simplification (a return has no per-line category of its own, same reasoning `salesVatBoxes` documents for the box version). |
| `getLedgerTargets` | port (confirmed) | `reports_get_ledger_targets` | — | `{ accounts: {id,label}[]; customers: {...}[]; suppliers: {...}[] }` | Pure lookup for the ledger-report page's account/party picker — no computation, just id+label projections. `accounts` includes group accounts too (no `isGroup` filter, unlike every P&L/balance-sheet function) since the picker may want to show the full tree. |
| `getDimensionOptions` | port (confirmed) | `reports_get_dimension_options` | — | `{ branches, costCenters, currencies }` (id/label triples) | Same pure-lookup shape as `getLedgerTargets`, for `ReportShell`'s branch/cost-center/currency filter selects — active rows only. |
| `getDiscountsReport` | port (confirmed) | `reports_get_discounts_report` | `{ range: DateRangeInput, groupBy: 'product' \| 'cashier' }` | `DiscountReportRow[]` | Two distinct grouping code paths (by cashier: invoice-level discount + summed line discounts; by product: line-level discount only) — **not the same computation with a different groupBy key**, a genuine asymmetry worth stating precisely since a naive Rust port might try to unify them into one `GROUP BY` and get a different number. Rows with `discountValue <= 0.001` are dropped. |
| `getGrossProfitReport` | port (confirmed) | `reports_get_gross_profit_report` | `{ range: DateRangeInput, groupBy: 'invoice' \| 'product' \| 'category' }` | `GrossProfitRow[]` | Three distinct code paths under one function, `groupBy==='invoice'` skips the per-line discount-factor math entirely (uses `subTotal - discountAmount` directly) while `'product'`/`'category'` apply the line-level `(qty×price − discount) × factor` formula — **these two formulas are not guaranteed to sum to the same total** when a line has its own discount on top of an invoice-level discount rate (double-discounting the same line under `'product'`/`'category'` grouping is mathematically possible depending on how the invoice form applies both) — flagged as a §9 cross-check for Part 03, not fixed here (this is the report's existing behavior, confirmed intentional by `invoices.md`'s own totals-helper documentation of line vs. invoice discount order, not re-litigated here). |
| `getReturnsReport` | port (confirmed) | `reports_get_returns_report` | `DateRangeInput` | `ReturnsReport` | Three independent `groupCount`/map passes (by reason, by product via `invoiceLineId` join, by cashier via the refund's own invoice's `cashierId`) — `byProduct`'s amount uses `rl.qty × invLine.price` (the **original line price**, not the refund's own `grandTotal`-derived share), while `byReason`/`byCashier` use `r.grandTotal` (the refund's own total, VAT-inclusive) — two different bases for "amount" across the three breakdowns, worth flagging precisely (not a bug — `byProduct`'s "how much of this product came back" question is naturally price×qty, while `byReason`/`byCashier`'s "how much money went out the door" question is naturally the refund total) but a footgun if Part 03 assumes all three columns are directly comparable. |
| `getReturnsReport` (returnRatePct) | — | — | — | — | `totalRefunds / totalInvoices × 100` — a **document-count** ratio, not a value ratio; do not confuse with `getProfitLeakageReport`'s `leakagePct` (value-based) below. |
| `getExpensesReport` | port (confirmed) | `reports_get_expenses_report` | `DateRangeInput` | `ExpensesReport` | Two independent group-bys (`byCategory`, `byMonth`) over the same filtered `expenses` set — `byMonth`'s key is `localDateKey(e.date).slice(0,7)` (`YYYY-MM`), a plain string slice, not a `DATE_FORMAT`/`date_trunc` call — Rust's SQL equivalent needs the same local-date semantics `cross-cutting.md` (01.D) settles once, not a UTC month-truncation that could shift a boundary-date expense into the wrong month. |
| `getShiftsReport` | port (confirmed) | `reports_get_shifts_report` | `DateRangeInput` | `ShiftReportRow[]` | Only `CLOSED` shifts, filtered by `openedAt` in range (not `closedAt` — a shift that opened in-range but closed after the range's end still appears, one worth confirming is the intended UX, not re-derived here since it's the mock's existing, presumably deliberate, choice). `salesTotal` re-derives from `invoices` filtered by `cashierId === s.openedBy` and the shift's own open/close window — **note**: this attributes sales to whoever *opened* the shift, not necessarily whoever rang each sale, matching `invoices.md`'s own flagged shift-attribution caveat (`createRefund`'s cash-drawer fallback) — same underlying assumption ("one cashier per shift"), not a new bug. |
| `getLowStockReport` | port (confirmed) | `reports_get_low_stock_report` | — | `LowStockRow[]` | `stockQty <= minStock` (defaulting `minStock` to 0), same threshold `getInventoryReport`'s `status: 'low'` uses. `suggestedQty` = `max(reorderQty, minStock − stockQty + reorderQty)` — a two-branch `Math.max`, not a single formula; port exactly, don't "simplify" the arithmetic since the two branches aren't algebraically identical when `reorderQty` is 0/undefined. |
| `getDeadStockReport` | port (confirmed) | `reports_get_dead_stock_report` | `{ days?: number }` (default 60) | `DeadStockRow[]` | Reads `stockMovements` for the **last `reason === 'sale'` movement per product** — a full table scan finding a max, not an indexed lookup; Part 03's SQL should use a `GROUP BY productId, MAX(date)` or a window function, not replicate the JS loop literally. Products with no sale movement ever get `daysSinceSale = Number.MAX_SAFE_INTEGER` — Rust's equivalent needs a sentinel (`NULL` sorts last, or a very large `i64`) that behaves the same under the `>= days` filter and the `sort by costValue desc` that follows. |
| `getStocktakeVariances` | port (confirmed) | `reports_get_stocktake_variances` | `{ countId?: string }` | `StocktakeVarianceRow[]` | Only `COMPLETED` counts; optionally filtered to one count. Rows with `|qtyVariance| < 0.0001` are dropped (near-zero noise, not real variance) — note the tighter epsilon than the module's usual `0.001`/`0.01` thresholds elsewhere, worth preserving exactly since it's already an unusually tight cut. |
| `getTransfersReport` | port (confirmed) | `reports_get_transfers_report` | `DateRangeInput` | `TransferReportRow[]` | `receivedQty` falls back to `l.qty` (assume fully received) only when `t.status === 'RECEIVED'` and no explicit `receivedQty` was recorded on the line — a per-line optional field with a status-conditional default, not a simple `?? 0`; port the exact three-way logic (`receivedQty ?? (status==='RECEIVED' ? qty : 0)`), not a simplified two-way version. `shortageValue` is read directly off the transfer document (`t.shortageValue`), **not recomputed** here — this report trusts whatever `products.md`'s transfer-receiving logic already calculated and stored. |
| `getPurchasesReport` | port (confirmed) | `reports_get_purchases_report` | `DateRangeInput` | `PurchasesReport` | Only `RECEIVED` POs (matches `purchases.md`'s own "only a received PO has GL effect" rule) — a `DRAFT`/`ORDERED`/`CANCELED` PO never counts toward purchases totals here, consistent with `getVatReport`'s identical `status === 'RECEIVED'` filter for input VAT. `byProduct.avgPrice` = `total / qty`, guarded against div-by-zero. |
| `getPeriodComparison` | port (confirmed) | `reports_get_period_comparison` | `{ rangeA: DateRangeInput, rangeB: DateRangeInput }` | `PeriodComparisonLine[]` | Calls `computePnl` twice (no `dim`) and returns exactly 5 fixed labeled rows (`netRevenue`, `totalCogs`, `grossProfit`, `totalExpenses`, `netIncome`) — a **hard-coded** list of P&L lines to compare, not a generic "diff every line" — Rust's version should reproduce the same fixed 5 rows, not try to diff arbitrary account lists. `deltaPct` divides by `|y|` (previous period), returns 0 when `y === 0` (avoids div-by-zero, not a "no data" sentinel — a genuine 0-to-N% change reads as "0% change" when the base was zero, worth noting but matches every other percentage-of-base-zero guard reviewed this session). |
| `getBranchComparison` | port (confirmed) | `reports_get_branch_comparison` | `DateRangeInput` | `BranchComparisonRow[]` | Only `active` branches. `grossProfit` uses `subTotal − discountAmount − cogs` (pre-VAT), while `sales` uses `grandTotal` (VAT-inclusive) — **two different bases in the same row**, intentional (sales = "what the customer paid," grossProfit = "the accounting margin"), not a bug, but worth stating precisely since summing `sales` across branches won't reconcile with `getSalesReport`'s own `summary.total` unless every branch's invoices are included (same figure, just partitioned). |
| `getBusinessHealthReport` | port (confirmed) | `reports_get_business_health_report` | `DateRangeInput` | `BusinessHealthReport` | Calls `getBalanceSheet` **and** `computePnl` internally (the only report function that composes two other exported reports) — 4 explainable 0–25 scores (liquidity, profitability, debt, collection/DSO), summed into `total` (0–100). Every score is a clamped linear formula against a fixed reference threshold (documented inline in the mock's own comments) — **not a machine-learned model**, matching the codebase's existing insight-rule style; port the exact clamp/threshold constants, don't "improve" the scoring formula. |
| `getProfitLeakageReport` | port (confirmed) | `reports_get_profit_leakage_report` | `DateRangeInput` | `ProfitLeakageReport` | `writeOffs`/`shrinkage` read `stockMovements` by `reason` (`'loss'` vs `'stocktake'` with `qtyChange < 0`) — **two different movement reasons treated as two different leakage categories**, not overlapping; confirm with `products.md`'s stock-movement reason enum that `'loss'` and `'stocktake'` are mutually exclusive reasons (they are, per that module's own review) so this split doesn't double-count. `leakagePct` divides by `netSales`, a **value** ratio — do not confuse with `getReturnsReport.returnRatePct`'s document-count ratio above (same naming shape, different unit, a real source of Part 03 test-writing confusion if not called out). |
| `getAgingReport` | port (confirmed) | `reports_get_aging_report` | `{ kind: 'customer' \| 'supplier' }` | `AgingReportRow[]` | **Reuses `getOpenDocumentsFor`** (`backend/payments.ts`, reviewed as a read-only reach from `parties.md`'s own `getPartyAging`) — this is the **all-parties** version of that same per-party bucketing logic, confirmed identical bucket boundaries (`<=0`/`1-30`/`31-60`/`60+`) to `parties.md §7`'s `AgingBucket`. Rows with `total <= 0.001` are dropped entirely (parties with no real open balance don't appear). |
| `getOverdueReport` | port (confirmed) | `reports_get_overdue_report` | `{ kind: 'customer' \| 'supplier' }` | `OverdueRow[]` | Same `getOpenDocumentsFor` source as `getAgingReport`, but flat (one row per overdue document, not bucketed per party) and requires a `dueDate` to be set at all (`if (!doc.dueDate) continue`) — a document with no due date can never appear here even if very old, an intentional "overdue" definition (no due date = not yet due by definition), not a gap. |
| `getDayBook` | port (confirmed) | `reports_get_day_book` | `DateRangeInput` | `DayBookEntry[]` | Straight pass-through of every `journalEntries` row (posted only — this function never reads `journalDrafts`, correctly matching `accounting.md`'s "drafts have no GL effect, must stay invisible to every ledger reader" invariant) in the range, sorted `(date, number)`, with each line's raw `accountCode`/`accountName` looked up — no aggregation at all, the rawest report in the module. |
| `getCashFlowStatement` | port (confirmed) | `reports_get_cash_flow_statement` | `DateRangeInput` | `CashFlowStatement` | **The most structurally complex report** — indirect method: starts from `computePnl(range).netIncome`, adds working-capital deltas (receivable/inventory/payable account net movement, sign-flipped), separately walks fixed-asset accounts for "investing," and EQUITY (excluding retained/current earnings)+long-term-LIABILITY accounts for "financing." `openingCash`/`closingCash` are **independently** computed via `movements({ to: ... })` cumulative scans over cash/bank-subtype accounts — not derived from `netChange`, so `closingCash − openingCash` and `netChange` are two independently-computed figures **expected to agree** but not mechanically forced to (the same reconciliation-not-derivation shape §9 flags for `getVatReport`'s ledger-vs-document figures) — see §7/§9. |

## 2. DTOs → Rust

Only fields needing a non-default mapping. Every `_decimal_`-hinted field not listed here is a
straightforward `Decimal` / `DECIMAL(19,2)` (money) mapping, `round2`'d at the point named in §7.

| Type | Field | Rust type | Column | Why |
|---|---|---|---|---|
| *(all report DTOs)* | every numeric report field | `Decimal` | **not stored** — these are pure response DTOs, never persisted | Confirmed: no report type appears anywhere in the 46-table inventory (`docs/backend/contract/README.md`) as a written table — this entire module's output is request-scoped, computed fresh every call, matching `parties.md`'s "balance is never a stored column" principle generalized to every figure in the module. |
| `DateRangeInput` | `to` | `Option<NaiveDate>` | — (request-only filter) | **Fifth confirmed instance** of the generator-hint false positive already logged for `PaymentFilter.to`/`VoucherFilter.to`/`ExpenseFilter.to`/`JournalFilter.to` — `inDateRange(x.date, range.from, range.to)` uses it as a plain date bound, not a route. No action needed, cross-referenced not re-derived. |
| `AccountLedger` | `normalSide` | reuse `accounting.md`'s `NormalSide` enum | `ENUM('DEBIT','CREDIT')` | Same enum, not redeclared — `getAccountLedger` reads it straight off `Account.normalSide`. |
| `AccountLedger` | `rowLinks` | `HashMap<Uuid, RouteRef>` | — (response-only, not persisted) | Already emits proper `AppRoute` objects in the mock (`{ name: 'journal-entry', params: { id } }`, `{ name: 'invoice', ... }`, `{ name: 'payments', query: {...} }`, `{ name: 'purchase', ... }`) — **this module has zero F7 path-string-link violations**, confirmed by grep (no `link:` field, no template-string route anywhere in `reportService.ts`). Worth stating explicitly since every other module reviewed this session carried at least one F7 instance; this is the first with none. |
| `AgingReportRow` / `OverdueRow` | `kind` (on `OverdueRow`) | reuse `parties.md`'s / a shared `enum DocKind { Invoice, PurchaseOrder }` | `ENUM(...)` | `AgingReportRow` itself has no `kind` field (it's per-party, not per-document) — only `OverdueRow.kind` needs an enum, and it's a narrower 2-variant set than `JournalSourceKind` (`accounting.md`), so it should **not** just reuse that larger enum wholesale — model as its own `enum { Invoice, PurchaseOrder }` or a filtered view of the shared one. |
| `VatCategoryBox` / `VatDetailRow` | `category` | reuse `settings.md`'s `TaxCategory` enum (`S`/`Z`/`E`/`O`) | `ENUM('S','Z','E','O')` | Exact same 4-variant enum `settings.md §2` already defined for `Tax.category` — reused here, not redeclared, since it's literally the same tax-category classification applied to a line. |
| `VatDetailRow` | `documentKind` | `enum VatDocKind { Sale, SalesReturn }` | `ENUM('sale','salesReturn')` | `#[serde(rename_all = "camelCase")]` — a 2-variant enum specific to this report (not shared with `JournalSourceKind`, since a VAT-detail row is either an invoice line or a refund, never a purchase-side document — this report is sales-VAT-only, `getVatReport`'s purchases/purchaseReturns figures have no line-level detail equivalent exposed here). |
| `BusinessHealthScore` | `key` | `enum HealthScoreKey { Liquidity, Profitability, Debt, Collection }` | — (response-only, not persisted) | `#[serde(rename_all = "camelCase")]`. |
| `InventoryReportRow` | `status` | reuse `products.md`'s stock-status enum if one exists there, else `enum StockStatus { Ok, Low, Out }` | — (computed, not persisted) | Cross-check against `products.md` once available — this is the same `ok`/`low`/`out` classification `getLowStockReport`'s threshold implies, likely worth a single shared enum rather than two independent 3-variant declarations. |
| `TransferReportRow` | `status` | reuse `products.md`'s `StockTransfer.status` enum | shared `ENUM(...)` | Read straight off the transfer document, not recomputed — same enum, not redeclared. |

## 3. Validation and errors

| Function | Rule (source: mock check line) | Code | Exact Arabic message |
|---|---|---|---|
| `getAccountLedger` | account must exist | `NOT_FOUND` | `الحساب غير موجود` |
| `getPartyLedger` | party (customer or supplier) must exist | `NOT_FOUND` | `العميل غير موجود` / `المورد غير موجود` (kind-dependent) |
| `getCostCenterBudgetVsActual` | fiscal year must exist | `NOT_FOUND` | `السنة المالية غير موجودة` |

No other function in this module validates its input at all — every other report takes a plain
date range (or no parameters) and simply returns an empty/zero-filled result set for a range with
no matching data, rather than throwing. This is correct for a read-only reporting module: there is
no invalid date range, only an empty one. All three throw sites above already use `ApiError` with
the correct code — **no plain-`Error` gap found** (confirmed by grep: zero `throw new Error`/`throw
Error` occurrences in `reportService.ts`), the fourth module this session (after `purchases.md`,
`invoices.md`, `vouchers.md`) to check this bug class and find it not present.

## 4. Undo matrix (every function that writes)

**n/a for all 32 functions — this module performs zero writes.** Confirmed three ways: (1) the
contract generator's "Writes" column is blank for every one of the 32 rows in
`docs/backend/contract/reports.md`; (2) a full read of `reportService.ts` finds no `db.<table>.push`,
no array-mutator call, no `mutate()` block, and no `logActivity`/`logAudit` call anywhere in the
file; (3) the "Shared" column (ledger/stock/activity/numbering/period/currency) is blank for every
row in the generated inventory — this module never reaches any of the shared managers other modules
post through. This is the correct, expected shape for the master plan's "Analytics live in Rust...
Aggregation is SQL `GROUP BY` + Decimal" rule (§3 rule 9) — every one of these becomes a Rust query
function with no transaction, no lock beyond the read itself, and no undo story because there is
nothing to undo.

| Function | Undoable? | Compensation | Refused when | Period rule (D7) |
|---|---|---|---|---|
| *(all 32)* | n/a — no writes | — | n/a | n/a — every date range is a read filter, not a period-lock check; `assertOpenPeriod` (`accounting.md`) is never called anywhere in this file |

## 5. Concurrency under D8 (several terminals on one DB)

| Race | Rows | Settled by |
|---|---|---|
| Two terminals run the same report concurrently while a third terminal posts a journal entry mid-query | `journalEntries` (read), any table a report scans | **No locking needed.** Every report here is a plain, consistent-as-of-query-time `SELECT`/`GROUP BY` — MariaDB's normal read-committed (or repeatable-read, depending on the connection's isolation level chosen in Part 02-A) snapshot semantics are sufficient. A report that runs half a second before or after a concurrent post simply reflects the data as of whichever moment its query executed — this is the expected, correct behavior for a report, not a race to "settle." No report function needs a `SELECT ... FOR UPDATE`, a version check, or any other concurrency primitive beyond what the DB's isolation level already gives every plain read. |
| A report runs during a multi-statement posting transaction (e.g. mid-`closeYear`, per `accounting.md §5`'s sharpest finding) | `journalEntries`, `fiscal_years.is_closed`, `counters` | Depends entirely on the **posting side's** transaction isolation, not anything this module does — if `closeYear`'s transaction is properly atomic (as `accounting.md §5` already specifies it must be), a concurrent report either sees the pre-close state or the fully-closed post-close state, never a half-computed closing entry. This module has no additional obligation here beyond "don't hold your own lock that could block or be blocked by a poster" — a plain read never does. |

This is, appropriately, the **shortest** concurrency section of any module reviewed this session —
a direct consequence of §4's "zero writes" finding. Nothing here needs a lock, a unique constraint,
or a re-check-inside-the-transaction pattern the way every write-performing module's §5 has needed.

## 6. Events and side effects

- **Activity/audit rows:** none — no write, nothing to audit.
- **Events emitted:** none. This module never calls `emit(...)` for `ledger:changed` /
  `catalog:changed` / `parties:changed` — reports are pulled on demand by the page that shows them,
  not pushed via a change event, which is correct for a read-only module (nothing here needs another
  screen to refresh in response to a report being *viewed*).
- **Attachments:** none.
- **Printing:** report pages likely print via the browser/PDF path each report page already uses
  (out of this service file's scope — printing is a `core`/`pdf` concern, not something
  `reportService.ts` itself touches).
- **Route links:** `AccountLedger.rowLinks` and `PartyLedger`'s derived `rowLinks` are the only route
  fields in this module, and both already emit proper `AppRoute` objects — **0 F7 violations**, the
  first module reviewed this session with none (every prior module owned at least one of the 61
  path-string links).

## 7. Aggregations (the core of this module's review)

Every output field below, grouped by report function. "Rounding point" states exactly where
`round2` is applied — critically, whether it's applied **per row/step** (so intermediate rounding
compounds) or **once at the end** (so it's mathematically equivalent to summing exact `Decimal`s
first) — since a Rust SQL `SUM(...)` that only rounds the final result would silently diverge from
a mock that rounds inside a loop, by up to a few cents on a large report.

### `getTrialBalance`

| Output field | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `openingBalance` | `journalEntries` (flattened lines) | lines dated `<= dayBefore(range.from)`, optional `dim` (branch/costCenter/currency) match | by `accountId` | `round2(d − c)` once, after summing raw (unrounded) debit/credit totals | `reportService.ts:91,105` (`movements` + the mapper) |
| `periodDebit` / `periodCredit` | same | lines dated in `[range.from, range.to]`, same `dim` | by `accountId` | `round2` each, independently | `reportService.ts:92,106-107` |
| `closingDebit` / `closingCredit` | derived — `round2(opening.d − opening.c + period.d − period.c)`, then split by sign into debit (if >0) or credit (if <0) column | — | — | **One `round2` call on the combined raw total**, then the sign-split — not `round2(closingDebit)` and `round2(closingCredit)` computed separately from already-rounded opening/period figures. Rust must sum all four raw (unrounded) components first, then round once, to match exactly. | `reportService.ts:99-109` |
| Row inclusion | — | only accounts (`!isGroup`) with `opening.has(id) || period.has(id)` — an account with zero activity in both windows is **dropped**, not shown with all-zero columns | sorted by `code` | — | `reportService.ts:93-95` |

### `getProfitAndLoss` / `getProfitAndLossComparison` (via `computePnl`)

| Output field | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `revenue[]` (StatementLine) | `journalEntries` (flattened lines), `accounts` (kind=REVENUE, non-group) | lines in `range`, matching `dim` | by `accountId` | `round2(sign × (d − c))` per line, `sign = -1` (credit-normal) | `reportService.ts:116-124,129` |
| `cogs[]` | same, `accounts` where `kind=EXPENSE && subtype==='costOfSales'` | same | by `accountId` | `round2(1 × (d − c))` per line | `reportService.ts:131` |
| `expenses[]` | same, `accounts` where `kind=EXPENSE && subtype!=='costOfSales'` | same | by `accountId` | `round2` per line | `reportService.ts:132` |
| Line filter | — | lines with `|amount| <= 0.001` after rounding are **dropped** from the array (but still counted in the sum before dropping — see next row) | — | — | `reportService.ts:124` |
| `netRevenue` / `totalCogs` / `totalExpenses` | derived — `sum(lines, l => l.amount)` | — | — | **No re-rounding** — `sum()` on already-`round2`'d per-line amounts (each line individually rounded, then summed raw) — this is a genuine "round per row, then sum the rounded values" pattern, not "sum raw then round once." Rust's `SUM(ROUND(amount, 2))` (summing already-rounded per-account amounts), not `ROUND(SUM(amount), 2)`. | `reportService.ts:133-135` |
| `grossProfit` | derived — `round2(netRevenue − totalCogs)` | — | — | `round2` again on the difference of two already-rounded sums | `reportService.ts:136` |
| `netIncome` | derived — `round2(grossProfit − totalExpenses)` | — | — | `round2` again | `reportService.ts:137` |

**This "round per line, sum the rounded values, round the differences again" chain is the single
most important rounding-order fact in this entire module**, since `getProfitAndLoss` is the base
every other statement (`getBalanceSheet`'s `unclosedEarnings`, `getCashFlowStatement`'s
`netIncome`, `getBusinessHealthReport`'s `profitabilityScore`, `getPeriodComparison`) builds on. A
Rust implementation that instead sums raw `Decimal` line values across every account in one
`GROUP BY accountId` then rounds once at the very end would produce a subtly different `netIncome`
than the mock whenever per-line rounding leaves a residual — e.g. two accounts each rounding
+0.004 up to +0.00 individually vs. their raw sum of +0.008 rounding up to +0.01 combined. **Rust
must replicate the per-account rounding step before summing**, not "simplify" it into a single
aggregate `SUM`.

### `getCostCenterProfitAndLoss` / `getCostCenterBudgetVsActual`

| Output field | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `centers[]` (`CostCenterPnlColumn`) | `journalEntries` (flattened lines), `costCenters`, `accounts` | lines in range | by `(costCenterId, accountId)`, same REVENUE/COGS/EXPENSE split as `computePnl` | Same per-line → per-account → summed chain as `computePnl`, applied independently **per cost center** | `reportService.ts:165-188` |
| `unassigned` | same, lines with no `costCenterId` | — | — | Same chain | `reportService.ts:197-198` |
| `total` | **independently** re-runs `computePnl(range)` with no `dim` at all — a *third* full aggregation pass, not a sum of `centers + unassigned` | — | — | Same chain, but note this is a **separate query**, not a rollup of the other two — must reconcile by construction (every line has 0 or 1 `costCenterId`) but Rust should implement it as its own query, matching the mock, rather than "optimizing" it into a sum (which would still be correct today but silently diverges from the mock's actual code path if `computePnl`'s dim-filtering logic ever grows a case `movementsByCostCenter` doesn't share). | `reportService.ts:199` |
| `CostCenterBudgetRow.actual` | `journalEntries` (flattened lines), `costCenters.budgets[]` | lines in `[fy.startDate, fy.endDate]`, by cost center | EXPENSE kind, **no** COGS/opex split (all expense subtypes together, unlike the P&L version) | Same per-line rounding chain, but only one `lines('EXPENSE', mv, 1)` call (not two) | `reportService.ts:218` |
| `variancePct` | derived — `round2(((actual − budget) / budget) × 100)`, only when `budget > 0` | — | — | `round2` once on the ratio | `reportService.ts:219` |

### `getBalanceSheet`

| Output field | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `assets[]` / `liabilities[]` / `equity[]` | `journalEntries` (flattened lines, **cumulative since inception** — `movements({ to: asOf }, dim)`, no lower bound) | all lines dated `<= asOf`, matching `dim` | by `accountId`, filtered by `kind` | Same per-line `round2` chain as `computePnl`'s `lines()` helper (shared code — `getBalanceSheet` calls the exact same `lines()` function `computePnl` does, with `sign = 1` for assets, `-1` for liabilities/equity) | `reportService.ts:232-235` |
| `unclosedEarnings` | derived — `computePnl({ to: asOf }, dim).netIncome` (a **fourth** call into the P&L aggregation, cumulative from inception, not from the last fiscal year's close) | — | — | Inherits `computePnl`'s full rounding chain | `reportService.ts:236` |
| `totalAssets` / `totalLiabilities` | derived — `sum(assets, l => l.amount)` etc. | — | — | No re-rounding — sum of already-rounded lines | `reportService.ts:237-238` |
| `totalEquity` | derived — `round2(sum(equity) + unclosedEarnings)` | — | — | `round2` on the combined raw sum | `reportService.ts:239` |
| `balanced` | derived — `|totalAssets − totalLiabilities − totalEquity| < 0.01` | — | — | Not itself a rounded value — a boolean tolerance check, **0.01 tolerance, not 0.001** like several other checks in this module — port the exact threshold | `reportService.ts:249` |

### `getAccountLedger` / `getPartyLedger`

| Output field | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `openingBalance` | `journalEntries` (flattened lines, `getAccountLedger`) or `customerStatement`/`supplierStatement` (`getPartyLedger`, delegated to `parties.md`'s already-reviewed aggregation) | lines dated `< range.from`, accumulated **raw** (not rounded per line) into `opening`, then `round2`'d **once** at the end | — | `round2(opening)` once, after the raw accumulation loop — **different rounding order from `getTrialBalance`'s opening balance**, which also rounds once but computes `d`/`c` totals via the shared `movements()` map first; here it's a manual running accumulator over already-summed `(debit - credit)` per matching line, mathematically equivalent for this case but implemented as a different code path — worth noting since a generic "reuse `movements()`" refactor in Rust must still produce the identical number, not just a plausible one. | `reportService.ts:260-268,276,285` |
| `rows[].balance` (running balance) | same | — | — | **Rounded incrementally, row by row**: `running = round2(running + sign × (debit − credit))` inside the loop, not computed once at the end from raw deltas — this is the sharpest incremental-rounding case in the whole module (worse than the trial balance's single opening/closing round, since here **every single row** re-rounds the running total). A SQL window-function `SUM(...) OVER (ORDER BY date, number)` computed once and rounded at the very end would NOT reproduce this number exactly whenever a row's raw delta has more than 2 decimal digits of residual carrying forward — but since every `debit`/`credit` value is itself already a `round2`'d money amount (posted journal lines are always 2dp, per `accounting.md`'s `JournalLine` scale), **the incremental and single-final-round approaches are actually mathematically identical here** (rounding a sum of already-2dp numbers to 2dp again is a no-op) — flagged for the parity harness to confirm empirically, not assumed. | `reportService.ts:277-279` |
| `totalDebit` / `totalCredit` | derived — `sum(rows, r => r.debit / r.credit)` | — | — | No re-rounding — sum of already-2dp posted amounts | `reportService.ts:287-288` |
| `closingBalance` | `running`'s final value (`getAccountLedger`) or `inRange.at(-1)?.balance ?? opening` (`getPartyLedger`) | — | — | Already rounded (inherits the running balance's rounding) | `reportService.ts:289,320` |

### `getSalesReport`

| Output field | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `summary.grossSales` / `discounts` / `vat` / `total` | `invoices` (non-draft) | dated in range | — (flat sums) | **No `round2` at all** — these four are raw `sum()` calls over already-2dp invoice-header fields (`subTotal`, `discountAmount`, `taxAmount`, `grandTotal`), which are themselves guaranteed 2dp by `invoices.md`'s posting-time rounding — safe as-is, not a gap. | `reportService.ts:382-385` |
| `summary.netSales` | derived — `round2(total − vat)` | — | — | `round2` once | `reportService.ts:388` |
| `summary.grossProfit` | derived — `round2(netSales − cogs)` | — | — | `round2` once | `reportService.ts:403` |
| `byProduct[].revenue` | `invoices.lines`, `products` (for `type`) | **This is the module's sharpest per-line rounding-order concern**: `row.revenue += ((l.qty×l.price − l.discount) × factor) / (inclusive ? 1+taxRate/100 : 1)` accumulates **raw, unrounded** across every invoice line for the same product across the *entire* date range, and is only `round2`'d **once**, at the very end, per product (`reportService.ts:348`) — this is the **opposite** rounding order from `computePnl`'s "round per line, sum the rounded values" pattern documented above. A Rust `SUM(...)` then `ROUND(...)` once **is** the correct match here (unlike `computePnl`), but the two functions' rounding orders are genuinely different from each other, so Part 03 must not assume "reports round the same way everywhere" — it depends on which report. | `reportService.ts:338-348` |
| `inclusive` flag (per invoice) | — | `taxAmount > 0 && |subTotal − discountAmount − grandTotal| < 0.01` — a **heuristic re-derivation** of "was this invoice tax-inclusive," not a stored flag on the invoice itself | — | The `0.01` tolerance here is doing real work: it's inferring VAT treatment from arithmetic identity, not reading a `taxInclusive: boolean` field — confirm with `invoices.md` whether `Invoice` actually carries this as a stored field (if so, Rust should read the stored flag directly instead of re-deriving it arithmetically, which would be both simpler and more robust — flagged in §9 as a question for whoever wrote `invoices.md`, not assumed either way here since `invoices.md` was written by a different review pass and this file doesn't re-derive its findings). | `reportService.ts:337` |
| `byDay[].total` | `invoices` | same | by `localDateKey(date)` | `round2` incrementally per day (`row.total = round2(row.total + inv.grandTotal)`) — since `grandTotal` is already 2dp, mathematically equivalent to summing raw then rounding once, same reasoning as the ledger running-balance case above | `reportService.ts:366` |
| `byCategory[].revenue` | derived from already-rounded `byProduct` rows | by product's category | `round2` incrementally, but summing **already-`round2`'d** `byProduct[].revenue` values — a genuine "round twice" (once per product, again per category accumulation step, which is a no-op on already-2dp inputs) | `reportService.ts:357` |
| `byMethod[]` / `byCashier[]` | `invoices` | grouped via the shared `group()` helper | by `paymentMethod` / `cashierId` | `round2` incrementally per group, same as `byDay` | `reportService.ts:370-380` |

### `getVatReport` / `getVatDetail` (salesVatBoxes)

| Output field | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `salesBoxes[].net` / `.vat` | `invoices.lines` (converted to base via `toBaseInvoice`) | every line, no date pre-filter beyond the invoice's own range filter | by `(taxCategory, taxRate)` | `round2` incrementally, added per line into the box accumulator (`box.net = round2(box.net + net)`) — **compounding round-per-line**, the same pattern `computePnl` uses, not the "sum then round once" pattern `getSalesReport.byProduct` uses. Two different reports in this module use two different rounding orders for superficially similar-looking accumulations — this is the single fact most worth surfacing to whoever writes the SQL, since "just look at how VAT does it" and "just look at how sales does it" give different answers. | `reportService.ts:450-459,465` |
| `salesBoxes[]` (refund apportionment) | `refunds`, matched back to their `invoices` | refund's VAT/net apportioned across the invoice's category mix **by each category's share of the invoice's total VAT/net** (`lineVat/invVat`, `lineNet/invNet`), not evenly split | same box key | `round2` on the refund's per-category negative contribution before adding to the box | `reportService.ts:472-482` |
| `sales.taxable` / `sales.vat` | `invoices` (base-converted) | — (flat sums) | — | No re-rounding — sums of already-2dp base-converted invoice fields | `reportService.ts:524` |
| `purchases.taxable` / `.vat` | `purchaseOrders` (RECEIVED, VAT-recoverable only), `expenses` (tax-invoice only) | — | — | `round2` once on the **combined** sum of two different tables' fields (`sum(pos) + sum(expenses)`) — not two separately-rounded sub-sums added together | `reportService.ts:527-528` |
| `outputVat` / `inputVat` | derived — `round2(sales.vat − salesReturns.vat)` / `round2(purchases.vat − purchaseReturns.vat)` | — | — | `round2` once per derived figure | `reportService.ts:532-533` |
| `netPayable` | derived — `round2(outputVat − inputVat)` | — | — | `round2` once | `reportService.ts:546` |
| `ledgerOutput` / `ledgerInput` | `journalEntries` (flattened lines), `accountFor('vatOutput')`/`accountFor('vatInput')` | lines in range, matching the resolved system-role account | — | `round2(c − d)` / `round2(d − c)` — **this is the ledger's own, entirely independent view**, computed via the same `movements()` helper every other statement report uses, not derived from the document-level `outputVat`/`inputVat` figures above. See the reconciliation note below. | `reportService.ts:535-537,547-548` |

**Reconciliation claim, stated precisely (§9 flags this as a design note, not a bug):**
`outputVat`/`inputVat` (document-derived) and `ledgerOutput`/`ledgerInput` (ledger-derived) are **two
independently computed aggregations over two different table sets** — the same relationship
`accounting.md §7` already documented between `getVatPeriodTotals` (ledger-only) and
`invariants.ts`'s `checkVatControl` (§4.5, document-only). This report function **surfaces both
side by side on purpose**, so a human can see them diverge if something is wrong — Rust must
implement them as two separate queries (not derive one from the other), or the report loses its
value as a reconciliation tool. `bun run verify:mocks`'s `checkVatControl` is the automated version
of exactly this same comparison; a passing `verify:mocks` run today (128/0) is empirical evidence the
two sides currently agree in the seed data, but nothing in the code **forces** them to agree, and
Part 04's parity harness should diff both figures independently against the Rust port, not assume
one implies the other.

### `getGrossProfitReport` / `getDiscountsReport` / `getReturnsReport`

| Output field | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `GrossProfitRow.revenue` (`groupBy==='invoice'`) | `invoices` | — | by invoice | `round2` once on `subTotal - discountAmount` (no per-line factor math) | `reportService.ts:759` |
| `GrossProfitRow.revenue` (`groupBy==='product'`|`'category'`) | `invoices.lines` | — | by product/category | `round2` incrementally, accumulated `(qty×price − discount) × factor` per line into the group | `reportService.ts:770` — **see §1's flagged discrepancy**: these two code paths are not guaranteed to produce the same total revenue for the same invoice set. |
| `GrossProfitRow.marginPct` | derived — `round2((profit/revenue) × 100)`, guarded `revenue > 0` | — | — | `round2` once | `reportService.ts:748-750` |
| `DiscountReportRow.chargedValue` (`groupBy==='cashier'`) | `invoices` | — | by cashier | `round2` on `listValue − invoiceDiscount − Σ(lineDiscounts)` per invoice, then incrementally summed into the cashier row | `reportService.ts:851-853` |
| `DiscountReportRow.chargedValue` (`groupBy==='product'`) | `invoices.lines` | — | by product | `round2` on `lineListValue − lineDiscount` per line, incrementally summed | `reportService.ts:861-864` |
| `ReturnsReportRow.amount` (byReason/byCashier) | `refunds` | — | by reason string / cashier (via invoice) | `round2` incrementally, `r.grandTotal` per refund | `reportService.ts:794,827` |
| `ReturnsReportRow.amount` (byProduct) | `refunds.lines` joined to `invoices.lines` by `invoiceLineId` | — | by `productId` | `round2` incrementally, `rl.qty × invLine.price` (original line price, not the refund total's share) | `reportService.ts:812` — **see §1's flagged basis difference vs. byReason/byCashier.** |

### `getExpensesReport` / `getPurchasesReport` / `getTransfersReport` / stock reports

| Output field | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `ExpensesReport.byCategory[].amount` / `byMonth[].amount` | `expenses` | — | by category / by `YYYY-MM` | `round2` incrementally per group | `reportService.ts:1066,1074` |
| `PurchasesReport.bySupplier[].total` | `purchaseOrders` (RECEIVED) | — | by supplier | `round2` incrementally, `p.grandTotal` | `reportService.ts:1027` |
| `PurchasesReport.byProduct[].total` / `.avgPrice` | `purchaseOrders.lines` | — | by product | `round2` incrementally on `total`; `avgPrice = round2(total/qty)` once, guarded `qty > 0` | `reportService.ts:1039,1044` |
| `TransferReportRow.shortageQty` | `stockTransfers.lines` | — | per transfer | `round2` once on `sentQty − receivedQty` | `reportService.ts:1009` |
| `InventoryReportRow` / `LowStockRow` / `DeadStockRow.costValue` | `products` | — | per product | `round2` once on `stockQty × costPrice` | `reportService.ts:429,925,952` |
| `StocktakeVarianceRow.qtyVariance` / `.valueVariance` | `stockCounts.lines`, `products` | `|qtyVariance| >= 0.0001` | per (count, product) line | `round2` on both, independently | `reportService.ts:973,984` |

### `getCashFlowStatement` / `getBusinessHealthReport` / `getProfitLeakageReport` / comparisons

| Output field | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `openingCash` / `closingCash` | `journalEntries` (flattened lines), cash/bank-subtype accounts | cumulative `movements({ to: ... })` scans | summed across all cash-like account ids | `round2` once on the summed raw `(d−c)` across every cash/bank account | `reportService.ts:624-626` |
| `operatingAdjustments[].amount` | `journalEntries`, one working-capital account per subtype (receivable/inventory/payable) | period movements | per subtype | `round2` once per line | `reportService.ts:635-640` |
| `operatingCash` | derived — `round2(netIncome + Σadjustments)` | — | — | `round2` once on the combined raw sum (adjustments are already individually rounded, but the addition with `netIncome` — itself already rounded via `computePnl` — is rounded again, a no-op on already-2dp inputs) | `reportService.ts:642` |
| `investing[]` / `investingCash`, `financing[]` / `financingCash` | fixed-asset / equity+long-term-liability accounts | period movements | per account | `round2` per line, then `round2` on the summed total (same no-op-on-2dp-inputs pattern) | `reportService.ts:646-666` |
| `netChange` | derived — `round2(operatingCash + investingCash + financingCash)` | — | — | `round2` once — **expected but not mechanically forced** to equal `closingCash − openingCash` (both independently computed — same reconciliation-not-derivation shape as the VAT report, see the note above); Part 04's parity harness should check both sides. | `reportService.ts:668` |
| `BusinessHealthScore.score` (all 4) | derived from `getBalanceSheet`/`computePnl` outputs | — | — | Each score: `round2` inside a `Math.max(0, Math.min(25, round2(...)))` clamp — port the exact clamp-then-round (or round-then-clamp — confirmed the mock rounds *inside* the clamp bounds, i.e. `round2(formula)` computed first, then clamped 0–25) order | `reportService.ts:1138,1141,1144,1151` |
| `ProfitLeakageReport.discounts` | `invoices` | — | — | `round2` once on the **combined** sum of invoice-level `discountAmount` + Σ line-level `discount` (two different sources added before one round, not two independently-rounded sub-totals) | `reportService.ts:1170` |
| `ProfitLeakageReport.totalLeakage` / `.leakagePct` | derived | — | — | `round2` once each | `reportService.ts:1181-1182` |

## 8. Contract fixes needed in the mock (→ 01.C)

**None applied.** Per CLAUDE.md's architectural-autonomy rule and this session's established
pattern for a module with no found issues (`purchases.md`, `invoices.md`, `vouchers.md`): every
named bug class was checked and found not applicable —

- [x] Checked: plain `Error` instead of `ApiError` — **not found** (all 3 throw sites already use
      `ApiError` with `NOT_FOUND`, confirmed by grep for `throw new Error`/`throw Error`).
- [x] Checked: a report reading stale/wrong date-range boundaries — **not found**; every range
      filter uses the shared `inDateRange`/`localDateKey` helpers exactly as `parties.md`/
      `accounting.md` already verified them (no custom date-comparison logic duplicated locally,
      except the two documented string-slice cases in `accounting.md §7` which this module doesn't
      repeat — every date filter here goes through `inDateRange`).
- [x] Checked: an aggregation using a different rounding rule than the module whose data it
      summarizes — **found, but not a bug**: §7 documents two genuinely different rounding *orders*
      within this module itself (`computePnl`'s round-per-line-then-sum vs. `getSalesReport.byProduct`'s
      sum-then-round-once), and flags the VAT-report/cash-flow-statement "two independently computed,
      expected-to-reconcile figures" pattern precisely. None of this is a rounding-rule mismatch in
      the sense of "uses `Math.round` instead of `round2`" (grep confirms every rounding call in this
      file is `round2`, the D3-fixed shared function, no local reimplementation) — it's a genuine,
      pre-existing structural fact about *when* `round2` is called, not a bug to silently harmonize.
      Flagged precisely in §7/§9 for Part 03's SQL to reproduce exactly, not "fixed" by picking one
      order for both, since that would change the reported numbers.
- [x] Checked: silent-skip-instead-of-throw — the closest candidate is `getPartyLedger`'s
      `NOT_FOUND` check already being present (not silent), and every report's "no data in range"
      case correctly returns an empty/zero result rather than throwing, which is the *correct*
      behavior for a report, not a swallowed error.

No mock code was modified as part of this review. `bun run verify:mocks` was re-confirmed after
concluding the review at the same baseline (128 ok, 0 failed), since no edit was made.

## 9. Open questions (→ decisions in `00-MASTER-PLAN.md`)

Nothing here required a stop-and-ask decision under the architectural-autonomy rule — there was no
mock behavior to change. What follows are precise, un-guessed findings for whoever implements
Part 03's SQL, not scope questions for the user:

- **Two different rounding orders coexist in this module** (§7): `computePnl` (and everything built
  on it — P&L, balance sheet, cash flow, business health, cost-center P&L) rounds **per account
  line, then sums the rounded values**; `getSalesReport.byProduct`, `getGrossProfitReport`'s
  `groupBy==='invoice'` path, and the VAT-report's `sales.taxable`/`.vat` flat sums round **once,
  at the end, over raw accumulated values**. Both are individually correct and already what the
  mock does — Part 03 must implement each report's SQL with the matching order (a per-account
  `GROUP BY` + `ROUND()` subquery, summed, vs. a single `SUM()` + one final `ROUND()`), verified by
  the parity harness, not "unified" into one pattern for cleanliness, since that would change the
  reported figures.
- **`getVatReport`'s document-derived (`outputVat`/`inputVat`) and ledger-derived
  (`ledgerOutput`/`ledgerInput`) figures, and `getCashFlowStatement`'s `netChange` and
  `closingCash − openingCash`, are each pairs of independently computed aggregations that are
  expected to reconcile but are not mechanically tied together.** This mirrors the exact
  relationship `accounting.md §7` already documented between `getVatPeriodTotals` and
  `checkVatControl` — not a new class of problem, but confirmed present here too, in two more
  places. No action needed beyond implementing both sides as separate queries (as the mock does)
  and letting the parity harness / `verify:mocks`-equivalent Rust invariant confirm they still
  agree on real data — this is a reconciliation report **by design**, not a bug to "fix" by
  deriving one side from the other (which would remove its diagnostic value).
- **`getGrossProfitReport`'s `groupBy==='invoice'` vs. `'product'`/`'category'` code paths, and
  `getReturnsReport`'s `byProduct` vs. `byReason`/`byCashier` amount bases, use genuinely different
  formulas** (documented precisely in §1/§7) that are not guaranteed to sum to the same total for
  the same underlying data. This is existing, presumably intentional mock behavior (different
  questions being asked of the same data), not something this review changed or recommends
  changing — flagged so Part 03's SQL doesn't try to write one query that serves both `groupBy`
  values by parameterizing a shared formula, which would silently change one of the two outputs.
- **`getSalesReport`'s `inclusive` flag** is re-derived arithmetically from `|subTotal −
  discountAmount − grandTotal| < 0.01` rather than read from a stored field. If `invoices.md`'s
  `Invoice` type turns out to carry an explicit tax-inclusive/exclusive flag (not confirmed here,
  since re-deriving another module's DTO shape is out of this review's scope per the task's
  instruction to consume, not re-derive, other modules' findings), Rust could read that field
  directly instead of reproducing the heuristic — a simplification opportunity for Part 03 to
  confirm against `invoices.md`, not a decision this file makes unilaterally.
- **`getDeadStockReport`'s "last sale movement per product"** is a full-table max-scan in the mock;
  Part 03 should use a `GROUP BY product_id, MAX(date)` or window function in SQL rather than a
  literal per-row loop — a performance note, not a behavior question (the result is identical
  either way).

## Gate

- [x] Every inventory function is in §1 with a confirmed disposition (32/32, all `port`, no
      dispositions changed — this is the first module this session where every single function
      kept the generator's default disposition with no override).
- [x] Every write function is in §4 — n/a, zero write functions exist in this module (stated once,
      not 32 empty rows).
- [x] Every DTO field needing a non-default mapping is in §2.
- [x] `bun run contract:check` is green (no override added, no contract shape changed).
- [x] `bun run build`, `bun run check`, `bun run verify:mocks` (128 ok, 0 failed, unchanged — no
      mock edit was made), `bun run memory` + `bun run memory:check` (0 new seam violations),
      `bun run diag:check` all green.
