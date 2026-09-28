# 21 · 03.14 — `analytics` + `dashboard` (read-only analytics tabs and home KPIs)

> **Status:** code complete 2026-09-28 (this implementer); `cargo check` not run (hard rule — no
> cargo from this agent; manager's wave-6 build will surface any remaining type errors). Tests
> written in `src-tauri/tests/domain_analytics.rs` but not executed (deferred time-boxed test
> pass). Frontend switch lines + `displayPrefs()` + `analytics/types/contract.check.ts` done.
> **Needs from manager**: `pub mod analytics;`/`pub mod dashboard;` + their `ipc_signatures()`/
> `export_bindings()` hooks in `domains/mod.rs`, the matching `commands::*` lines in `lib.rs`'s
> `generate_handler!`, and G-39's `products::service::transfers::to_dto` → a `pub` `transfer_dto`
> twin (`dashboard::service::feed::in_transit_transfers` needs it — see that file's doc comment).
> Also noted: `entities::sales::invoices::Model` has no `date_key` field even though the DB
> generated column exists (`m0008_sales.rs`'s `add_doc_date_key`) — `dashboard_get_recent_invoices`
> sorts in memory via `DocDate::key()` instead of `ORDER BY date_key`; a future entity update could
> restore the DB-level sort. Wave W6 (entry file §4). Depends on: 13
> (`service/common.rs` patterns, gaps G-32/G-33/G-8), 04-approvals, 06-products, 08-invoices (their
> public DTO assemblers, §3.0), and the new gaps G-36–G-39 below (registry: [`_part02-gaps.md`](_part02-gaps.md)).
> **Split:** the entry file's row 14 ("analytics, dashboard, insights") is too big for one file.
> **This file** holds the 3 analytics functions (`domains/analytics/`) and the 13 dashboard KPI /
> notification reads (`domains/dashboard/`). [`14b-insights.md`](14b-insights.md) holds the insight
> engine (21 rules, product inline hints, thresholds), also in `domains/dashboard/`. One implementer
> does both files, this one first.

**Goal.** Port `analyticsService.ts` (3 fns) and the 13 `port` functions of `dashboardService.ts`
to Rust reads that return the mock's DTOs exactly, including the Arabic insight sentences, without
changing a page.

**Read first.** [`01-frontend-analysis/analytics.md`](../01-frontend-analysis/analytics.md) (`AA§n`)
· [`01-frontend-analysis/core.md`](../01-frontend-analysis/core.md) §1a, §2, §7 (`AC§n`) ·
`src/modules/analytics/services/analyticsService.ts` (`as:<line>`) ·
`src/modules/core/services/dashboardService.ts` (`ds:<line>`) · `src/modules/core/types/index.ts:23-49`
· `src/modules/core/helpers/format.ts:44-105` (the formatters G-36 ports) · 13-reports §3.0 (helpers).

## Part 02 / cross-domain gaps (manager tasks, on top of 13's G-32, G-33, G-8)

| # | Gap | Why (evidence) | Needed by |
|---|---|---|---|
| G-36 | **`utils::format`**: `enum Numerals { Latn, Arab }`, `enum DateStyle { Dmy, Ymd }`, `format_money(d, n)`, `format_number(d, max_frac, n)`, `format_date_key(day, style, n)` — a port of `formatMoney`/`formatNumber`/`formatDate` (`format.ts:71-105`) for the `ar-SA-u-ca-gregory-nu-<n>` locale after `clean()`: Latn → `,` grouping and `.` decimal; Arab → digits `٠-٩`, `٬` (U+066C) grouping, `٫` (U+066B) decimal; minus `-`; Intl `halfExpand` rounding; `-0` prints as `-0`. Its unit test is a table of strings captured from `format.ts` in the browser (values 0, −0, 0.5, 1234.5, −1234.567, 1000000, 12.345 × both numeral systems × both date styles). | Analytics' `trendInsight` embeds `formatDate(...)` (`as:58`) and 14b's rule messages embed `formatMoney`/`formatNumber` (`insightRules.ts:26`). These depend on per-device display settings, so they come in as command args (decision A-2). | 14, 14b |
| G-37 | **Frontend `src/modules/core/services/backendMirror.ts`**: `mirrored<T>(key, load: () => Promise<T>, fallback: T, opts?: { ttlMs?: number }): T` and `clearMirrors(prefix?: string)`. A `shallowReactive(Map)` of `key → { value } \| { pending } \| { failed }`: a read returns the cached value (reactive dependency), else starts `load()` once and returns `fallback`; success stores the value (dependent `computed`s re-run); failure → `log.error('core.backendMirror', …)`, keep `fallback`, retry on the next clear. Every entry clears on `ledger:changed`/`catalog:changed`/`parties:changed` (the mock bus that `initBackendBridge` feeds) and after `ttlMs` when given. | Six dashboard functions (`ds:304-331`) and the insight engine (14b) are **synchronous** and are read inside `computed()` (`AccountantHome.vue:23`, `StorekeeperHome.vue:37-38`, `useNotifications.ts:66,81,95`, `useInsights.ts:15-18`). A sync function cannot `await` IPC; changing it to async would change pages and controllers (master §5). A reactive mirror keeps every caller unchanged. | 14, 14b |
| G-38 | **`Simplify<T>`** in `src/modules/core/types/contract.ts` (`{ [K in keyof T]: T[K] }`). | `Equals` (`contract.ts:11`) is an identity check, so `HomeKpi & { marginPct }` or `Invoice & { customerName? }` is not identical to a flattened ts-rs type. Checks compare `Simplify<Gen.X>` with `Simplify<X>`. | 14 |
| G-39 | 06-products exposes a public `Product` DTO assembler (`Vec<products::Model>` → `Vec<Product>`) and a `StockTransfer` one; 04-approvals an `ApprovalRequest` one; 08-invoices an `Invoice` one. This domain calls them and never re-maps those entities. | `getLowStockProducts`/`getInTransitTransfers`/`getPendingApprovalRequests`/`getRecentInvoices` return those modules' DTOs verbatim (AC§1a). | 14 |

## 1. Commands

Every command is a read (**tx = `with_read_ctx`**, 13 G-32), **events = none**, **not undoable**.

| # | Mock fn (line) | Disposition | Rust command | Args → Return | Area / Access |
|---|---|---|---|---|---|
| 1 | `getSalesAnalytics` (`as:42`) | port | `analytics_get_sales_analytics` | `{ days?: i32, dateStyle: DateStyle, numerals: Numerals }` → `SalesAnalytics` | Analytics / Read |
| 2 | `getProductAnalytics` (`as:129`) | port | `analytics_get_product_analytics` | `{ days?: i32, limit?: i32 }` → `ProductAnalytics` | Analytics / Read |
| 3 | `getCustomerAnalytics` (`as:180`) | port | `analytics_get_customer_analytics` | `{ days?: i32, limit?: i32 }` → `CustomerAnalytics` | Analytics / Read |
| 4 | `getDashboardSummary` (`ds:22`) | port | `dashboard_get_dashboard_summary` | `()` → `DashboardSummary` | Dashboard / Read |
| 5 | `getHomeKpis` (`ds:184`) | port | `dashboard_get_home_kpis` | `{ period?: HomePeriod }` → `HomeKpis` | Dashboard / Read |
| 6 | `getLowStockProducts` (`ds:59`) | port | `dashboard_get_low_stock_products` | `{ limit?: i32 }` → `Vec<Product>` | Dashboard / Read |
| 7 | `getRecentInvoices` (`ds:64`) | port | `dashboard_get_recent_invoices` | `{ limit?: i32 }` → `Vec<RecentInvoice>` | Dashboard / Read |
| 8 | `getRecentActivity` (`ds:72`) | port | `dashboard_get_recent_activity` | `{ limit?: i32 }` → `Vec<RecentActivityEntry>` | Dashboard / Read |
| 9 | `getTopProducts` (`ds:261`) | port | `dashboard_get_top_products` | `{ period?, limit? }` → `Vec<TopProductRow>` | Dashboard / Read |
| 10 | `getTopCustomers` (`ds:284`) | port | `dashboard_get_top_customers` | `{ period?, limit? }` → `Vec<TopCustomerRow>` | Dashboard / Read |
| 11 | `getInTransitTransfers` (`ds:304`, sync) | port (mirror) | `dashboard_get_in_transit_transfers` | `{ homeBranch?: Id }` → `Vec<StockTransfer>` | Inventory / Read |
| 12 | `getPendingApprovalRequests` (`ds:309`, sync) | port (mirror) | `dashboard_get_pending_approval_requests` | `()` → `Vec<ApprovalRequest>` | Approvals / Read |
| 13 | `getLastBackupFailedAt` (`ds:314`, sync) | port (mirror) | `dashboard_get_last_backup_failed_at` | `()` → `Option<String>` | Dashboard / Read |
| 14 | `getJournalDraftCount` (`ds:319`, sync) | port (mirror) | `dashboard_get_journal_draft_count` | `()` → `i32` | Dashboard / Read |
| 15 | `getStockValueSnapshot` (`ds:324`, sync) | port (mirror) | `dashboard_get_stock_value_snapshot` | `()` → `DecimalValue` | Dashboard / Read |
| 16 | `hasAnyProducts` (`ds:329`, sync) | port (mirror) | `dashboard_has_any_products` | `()` → `bool` | Dashboard / Read |
| — | `onLedgerChanged` (`ds:334`) | stay-frontend (AC§1a) | — | — | — |

Area choices: analytics tabs live on the `analytics` route (`analytics/routes.ts:5`); home widgets
need `dashboard` (every role has `dashboard: read`, `permissions.ts:15-36`); in-transit transfers are
inventory data (every role has inventory ≥ read); pending approvals are only read when the user
`canApprove` (`useNotifications.ts:80-81`), so the server asks for `Approvals / Read`.

## 2. DTOs

**`domains/analytics/dto.rs`**, `#[ts(export_to = "analytics/types/gen/")]` (13 §2 conventions):

| Rust DTO | TS type | Notes |
|---|---|---|
| `SalesTrendPoint { date, total }` | `as:26` | `date: String` (`YYYY-MM-DD`). |
| `WeekdayRow { day, total, avg }`, `PaymentMixRow { method, total, pct }` | inline in `SalesAnalytics`, `as:34,36` | |
| `SalesAnalytics` | `as:31` | |
| `ProductProfitRow` | `as:113` | `id: String` (product id or `freetext-<i>` key, 13b §3.0). |
| `ProductAnalytics` | `as:123` | |
| `CustomerShare { id, name, total }`, `CustomerAnalytics` | `as:171-178` | `id: Id`. |
| Args | — | `DateStyle` (`'dmy' \| 'ymd'`) and `Numerals` (`'latn' \| 'arab'`) from G-36, with `#[ts(type = …)]` literal unions. |

New `src/modules/analytics/types/contract.check.ts`: `Equals` for the 7 types, imported from
`../services/analyticsService` (the TS types are declared there, AA header).

**`domains/dashboard/dto.rs`**, `#[ts(export_to = "core/types/gen/")]` (the Vue module is `core`):

| Rust DTO | TS type | Notes |
|---|---|---|
| `SalesTrendDay { date, total }`, `DashboardSummary` | `core/types/index.ts:38-49` | counts `i32`. |
| `HomePeriod { Today, Week, Month }` lowercase | `ds:86` | |
| `HomeKpi { value, previous, changePct, sparkline }` | `ds:150` | `changePct: Option<Decimal>` serialized as `null` (**not** skipped — the TS type is `number \| null`, `ds:153`); the struct gets no `skip_serializing_none`; `#[ts(type = "number \| null")]`. `sparkline: Vec<Decimal>` (`number[]`). |
| `GrossProfitKpi` (+ `marginPct`), `ReceivablesKpi` (+ `overdue`) | `HomeKpi & {…}`, `ds:159-161` | Flat structs (all fields listed); checked with `Simplify` (G-38). |
| `HomeTrendPoint { date, total, previousTotal? }`, `HomeKpis` | `ds:157-164` | `previousTotal` always set; `#[ts(optional)]` to match `previousTotal?`. |
| `TopProductRow`, `TopCustomerRow` | `ds:246,254` | `TopProductRow.id: String` (product key). |
| `RecentInvoice` | `Invoice & { customerName?: string }` | `#[serde(flatten)] invoice: Invoice` (08) + `customer_name: Option<String>`; `Simplify` check. |
| `RecentActivityEntry` | `ActivityEntry & { userName?: string }` | flatten `core::dto::ActivityEntry` + `user_name`; `Simplify` check. |
| `DecimalValue(Decimal)` | `number` | `#[serde(transparent)]`, `#[ts(type = "number")]` — `ipc_sig!` needs a `TS` return type. |

`src/modules/core/types/contract.check.ts` gains the entries for the dashboard types
(`DashboardSummary`, `HomeKpi`, `HomeKpis`, `TopProductRow`, `TopCustomerRow`, and the two
`Simplify` checks); `HomeKpi`/`HomeKpis`/`TopProductRow`/`TopCustomerRow` are imported from
`../services/dashboardService` where they are declared (`ds:150-258`).

## 3. Service logic

### 3.0 Shared rules

- `today = ctx.clock.today()`; "N days ago" = `today − N` (the mock's `setDate(getDate() − N)` on a
  local `Date`, `as:47-49,61-63`); day keys are `%Y-%m-%d`.
- **Bounds (decision A-1):** `days` defaults 30, must be `1..=366`, else `VALIDATION "عدد الأيام يجب أن
  يكون بين 1 و 366"`; `limit` must be `1..=100`, else `VALIDATION "الحد الأقصى للنتائج يجب أن يكون بين 1 و 100"`
  (AA§8, AC§3: the IPC boundary re-validates; the UI only sends 30/8/6/5).
- `sold` invoices = `status ≠ DRAFT`, `(created_at, id)` order (`as:20-22`). Lookups are live rows
  with the mock's fallback text. Group-bys are `IndexMap` + stable sort (13 G-33).
- `outstanding(inv) = max(0, round2(grand − refunded − paid))` (`invoices/helpers/totals.ts:240-242`),
  in the invoice's own currency (no FX — quirk Q-2).
- `account_balance(role)` = `resolve_account(role, default ctx)` then `round2(SUM(debit) − SUM(credit))`
  over all its lines (`ds:14-19`); `account_balance_as_of(role, day)` adds `je.date_day <= day`
  (`ds:236-244`). A missing role propagates `resolve_account`'s `NOT_FOUND` (the mock's `accountFor`
  throws too).
- `js_num` for numbers interpolated with `${}`; `format_*` (G-36) only where the mock calls the formatters.
- Line key for product grouping = product id, or `freetext-<position>` (13b §3.0).

### 3.1 `analytics_get_sales_analytics` (`as:42-105`)

1. Validate `days`. `trend`: for `d = days−1 … 0`, `key = today − d`; `gross = sum2(sold with day = key,
   grand)`; `refunds = sum2(refunds with day = key, grand)`; push `{ key, round2(gross − refunds) }`
   (`as:46-53`). One SQL `GROUP BY date_day` per table over `[today − days + 1, today]` feeds it.
2. `avgTotal = Σ trend.total / (len or 1)` raw; `best` = first max of `trend` by total (stable sort desc,
   `as:55`). `trendInsight = best && avgTotal > 0 ? "أفضل يوم كان {format_date_key(best.date)} بمبيعات
   {js_num(round2(best.total))} ر.س — أعلى بنسبة {js_num(round2((best.total − avgTotal)/avgTotal × 100))}% من المتوسط"
   : "لا توجد بيانات كافية بعد لهذه الفترة"` (`as:56-59`; the date uses `args.dateStyle`/`args.numerals`).
3. `cutoff = today − days`; `recent` = sold with `day >= cutoff` (no upper bound, `as:61-64`).
4. `byWeekday`: weekday of `date_day` (0 = Sunday, the mock's `getDay()` of the local date, `as:69`);
   `totals[w] += grand` raw, `counts[w] += 1`; rows in label order `["الأحد","الاثنين","الثلاثاء","الأربعاء",
   "الخميس","الجمعة","السبت"]` → `{ day, total: round2(total), avg: count > 0 ? round2(total / count) : 0 }` (`as:66-77`).
5. `bestWeekday` = first max by `avg`; `restAvg = Σ(avg of the other rows) / 6` (always 6, `as:79`);
   `weekdayInsight = bestWeekday && best.avg > 0 && restAvg > 0 ? "{day} هو أعلى يوم مبيعاً بمتوسط
   {short(best.avg)} ر.س — أعلى بـ {js_num(round2((best.avg − restAvg)/restAvg × 100))}% من بقية الأيام"
   : "لا توجد بيانات كافية بعد"`, where `short(v) = v >= 1000 ? "{js_num(round2(v/1000))}K" : js_num(round2(v))` (`as:107-109`).
6. `paymentMix`: for each recent invoice's tenders (by `position`), label = the live payment method's
   `name` or `"أخرى"`; `total = round2(total + amount)` keyed by **label** (`as:85-92`); `methodTotal =
   Σ totals or 1`; rows `{ method, total, pct: round2(total / methodTotal × 100) }`, stable sort total desc.
7. `totalItems = sum2(per recent invoice sum2(lines qty))`; `avgInvoice = n ? round2(sum2(grand)/n) : 0`;
   `avgItemsPerInvoice = n ? round2(totalItems / n) : 0`; `returnsRatePct = n ? round2(count(refunds
   with day >= cutoff) / n × 100) : 0` (`as:98-102`).

### 3.2 `analytics_get_product_analytics` (`as:129-167`)

1. Validate `days`, `limit` (defaults 30, 8). `recent` as 3.1 step 3.
2. Per recent line: `net = line.net ?? round2(qty × price − discount)`; `qty += qty`, `revenue += net`,
   `profit += net − qty × costPrice` (the line's own cost snapshot, AA§7), all raw (`as:137-146`).
3. Row `{ id: key, name: live product name or "—", sku: live sku or "", qty: round2, revenue: round2,
   grossProfit: round2(profit), marginPct: revenue > 0 ? round2(profit/revenue × 100) : 0 }` (raw values
   in the ratio, `as:147-157`).
4. `sorted` = stable sort by `grossProfit` desc; `top = first limit`; `bottom = last limit, reversed` (`as:159-161`).
5. `insight = top non-empty ? "\"{top[0].name}\" هو الأعلى ربحاً بإجمالي {js_num(round2(top[0].grossProfit))} ر.س
   خلال آخر {days} يوماً" : "لا توجد مبيعات كافية بعد لهذه الفترة"` (`as:162-164`).

### 3.3 `analytics_get_customer_analytics` (`as:180-215`)

1. Validate `days`, `limit`. `cutoffKey = (today − days)` key. `recent` = sold with `day >= cutoff` and a customer.
2. `firstSale[c]` = the **minimum raw date key** over all sold invoices of `c` (`inv.date < prev`, a
   string compare — `MIN(date_key COLLATE utf8mb4_bin)` in SQL, `as:187-192`).
3. For each distinct recent customer (first-appearance order): `firstSale >= cutoffKey` as **byte
   strings** (an ISO instant key vs a day key, `as:198` — quirk Q-3) → new, else returning.
4. `segmentInsight = size > 0 ? "{js_num(round2(new/size × 100))}% من العملاء النشطين هذه الفترة عملاء جدد"
   : "لا توجد بيانات كافية بعد"`.
5. `byCustomer`: `round2(total + grand)` per customer over `recent`; rows `{ id, name: live or "—", total }`,
   stable sort total desc; `totalRevenue = Σ totals or 1`; `top10SharePct = round2(Σ first 10 / totalRevenue × 100)`;
   `concentrationInsight = rows ≥ 10 ? "أفضل 10 عملاء يمثلون {js_num(top10SharePct)}% من إجمالي الإيرادات"
   : "عدد العملاء غير كافٍ لحساب التركّز بدقة"`; `topCustomers = first limit` (`as:206-214`).

### 3.4 `dashboard_get_dashboard_summary` (`ds:22-53`)

1. `sold`; `todays` = sold with day = today; `todayRefunds` = refunds with day = today; `unpaid` = sold
   with `outstanding > 0` (includes `REFUNDED` status rows that still owe, as the mock).
2. `cashOnHand = account_balance(Cash)`, `bankBalance = account_balance(Bank)`.
3. `salesTrend`: 14 days, `d = 13 … 0`, same formula as 3.1 step 1.
4. Return `{ todaySales: round2(sum2(todays grand) − sum2(todayRefunds grand)), todayInvoiceCount,
   unpaidInvoiceCount, unpaidInvoiceTotal: sum2(outstanding), lowStockCount: low_stock().len(),
   cashPosition: round2(cash + bank), cashOnHand, bankBalance, salesTrend }`.
5. `low_stock()` (`ds:55-57`) = live products `active && type == "product" && stock_mode != "none"`
   (NULL passes) `&& stock_qty <= (min_stock ?? 0)`, `(created_at, id)` order.

### 3.5 `dashboard_get_home_kpis` (`ds:88-233`)

1. `period` default `today`. `period_range` (`ds:88-104`): `to = today`; `from = today` / `today − 6` /
   `today − 29`; `span = (to − from) + 1`; `prevTo = from − 1`; `prevFrom = prevTo − span + 1`.
2. `net_sales(a, b) = round2(sum2(sold with a ≤ day ≤ b, grand) − sum2(refunds with a ≤ day ≤ b, grand))` (`ds:110-114`).
3. `gross_profit(a, b)` (`ds:117-139`): over sold invoices in range, per line `net = line.net ?? round2(qty × price −
   discount)`, `revenue += net`, `profit += net − qty × costPrice` (raw); then per refund in range: its
   original invoice (any date), per refund line its original line (missing → skip): `unitNet = (orig.net ??
   round2(orig.qty × orig.price − orig.discount)) / (orig.qty or 1)`; `net = round2(unitNet × rl.qty)`;
   `revenue −= net`; `profit −= net − rl.qty × orig.costPrice`. Return `{ profit: round2, revenue: round2 }`.
4. `cash = round2(account_balance(Cash) + account_balance(Bank))`; `cashPrev = round2(as_of(Cash, prevTo) + as_of(Bank, prevTo))`.
5. `receivables = round2(sum2(outstanding of invoices with status ∉ {DRAFT, REFUNDED}))`; `receivablesPrev` = same
   with `day <= prevTo`; `overdue = round2(sum2(outstanding of those with due_date set, outstanding > 0 and
   due-date key < ISO(now)))` — the mock compares `YYYY-MM-DD` to `new Date().toISOString()` as strings
   (`ds:141-148`), so a due date of today is already overdue (quirk Q-4); `now = ctx.clock.now` via `format_iso_ms`.
6. `spark` = `net_sales(k, k)` for the 14 days ending today, oldest first (`ds:172-181`); the same vector
   feeds all four KPIs.
7. `salesTrend`: for `d = span−1 … 0`: `{ date: to − d, total: net_sales(that day), previousTotal: net_sales(prevTo − d) }` (`ds:202-212`).
8. `change_pct(c, p) = p == 0 ? (c == 0 ? Some(0) : None) : Some(round2((c − p)/|p| × 100))` (`ds:166-169`).
9. Return `netSales {value, previous, changePct, sparkline}`, `grossProfit {…, marginPct: revenue > 0 ?
   round2(profit/revenue × 100) : 0}`, `cash {…}`, `receivables {…, overdue}`, `salesTrend`.
   Performance: steps 2/6/7 are answered from one per-day `GROUP BY date_day` over
   `[min(prevFrom, today − 13), today]` for invoices and refunds, then summed in Rust per day with `sum2`.

### 3.6 `dashboard_get_low_stock_products` (`ds:59-62`)

1. `limit` default 6 (validated). `low_stock()` stable-sorted by `stock_qty / (min_stock or 1)` asc
   (`min_stock` NULL or 0 → 1).
2. First `limit` rows → the 06 `Product` assembler (G-39).

### 3.7 `dashboard_get_recent_invoices` (`ds:64-70`)

1. `limit` default 8. **All** invoices (drafts included, `ds:66`), stable sort by date key desc
   (`date_key COLLATE utf8mb4_bin DESC, created_at, id` — the key column exists on `invoices`,
   `migration/src/m0008_sales.rs:414`), first `limit`.
2. Map through the 08 `Invoice` assembler + `customerName` = live customer name (omitted when none).

### 3.8 `dashboard_get_recent_activity` (`ds:72-79`)

1. `limit` default 12. `activity` rows with `kind != auth`, stable sort by the `DocDate` key of their
   `date` desc, first `limit`.
2. Map to `core::dto::ActivityEntry { id, date: key, userId, kind, message, link? }` + `userName` = live
   user name (omitted when none). No second copy of this mapping: if 16-diagnostics already maps
   `activity::Model` → `ActivityEntry`, reuse it.

### 3.9 `dashboard_get_top_products` (`ds:261-281`)

1. `period` default `month`, `limit` default 5. Range = `period_range(period)` (current window only).
2. Per sold line in range: `qty += qty`, `profit += net − qty × costPrice` (raw, same `net` rule).
3. Rows `{ id: key, name: live or "—", sku: live or "", qty: round2, grossProfit: round2 }`, stable sort
   `grossProfit` desc, first `limit`.

### 3.10 `dashboard_get_top_customers` (`ds:284-296`)

1. Defaults `month`, 5. Per sold invoice in range with a customer: `round2(total + grand)` (gross, no
   refund netting — AC§9, kept). Rows `{ id, name: live or "—", total }`, stable sort desc, first `limit`.

### 3.11 The six notification reads (`ds:304-331`)

1. `in_transit_transfers(homeBranch)`: transfers `status = SENT` and (`homeBranch` absent or `to_branch_id
   = homeBranch`), `(created_at, id)` order → 06 `StockTransfer` assembler.
2. `pending_approval_requests`: `approval_requests` with `status = pending`, `(created_at, id)` order → 04 assembler.
3. `last_backup_failed_at`: `settings.backup.last_backup_failed_at` → `format_iso_ms`, or `None`.
4. `journal_draft_count`: `COUNT(*)` of live `journal_drafts`.
5. `stock_value_snapshot`: `round2(SUM(stock_value))` of live products `active && type == "product"`.
6. `has_any_products`: `EXISTS` a live product (active or not, `ds:330`).

## 4. Concurrency (D8)

Pure reads (AA§5, AC§5): one `with_read_ctx` snapshot per command, no locks. The mirrored functions
(G-37) show the last value until a change event or the TTL refreshes it — the same staleness model
as the mock's own cached computeds.

## 5. Undo

Not applicable: no writes.

## 6. Frontend switch lines

`src/modules/analytics/services/analyticsService.ts` (import `usesRust`, `backendCall`, and
`numeralSystem` from `@/modules/core/helpers/format`, `dateFormatStyle` via a new
`displayPrefs()` export in `format.ts` that returns `{ dateStyle: dateFormatStyle.value, numerals: numeralSystem.value }`):

- `getSalesAnalytics` (`as:42`): `if (usesRust('analytics')) return backendCall('analytics_get_sales_analytics', { days, ...displayPrefs() });`
- `getProductAnalytics` (`as:129`): `… ('analytics_get_product_analytics', { days, limit });`
- `getCustomerAnalytics` (`as:180`): `… ('analytics_get_customer_analytics', { days, limit });`

`src/modules/core/services/dashboardService.ts` (`usesRust('dashboard')`):

- async fns: `getDashboardSummary` → `backendCall('dashboard_get_dashboard_summary')`; `getHomeKpis` →
  `('dashboard_get_home_kpis', { period })`; `getLowStockProducts` → `(…, { limit })`; `getRecentInvoices`;
  `getRecentActivity`; `getTopProducts` → `{ period, limit }`; `getTopCustomers` → `{ period, limit }`.
- sync fns, through the mirror (G-37) with `ttlMs: 30_000`:
  `getInTransitTransfers(homeBranch)` → `return mirrored(\`dashboard:inTransit:${homeBranch ?? ''}\`, () => backendCall('dashboard_get_in_transit_transfers', { homeBranch }), [], { ttlMs: 30_000 });`
  and likewise `dashboard:pendingApprovals` (`[]`), `dashboard:lastBackupFailedAt`
  (`async () => (await backendCall(…)) ?? undefined`, fallback `undefined`), `dashboard:journalDraftCount` (`0`),
  `dashboard:stockValue` (`0`), `dashboard:hasAnyProducts` (`false`).

No page, component or controller changes.

## 7. Known mock quirks (kept) and decisions

**Quirks:**
- **Q-1** Every money sentence says `ر.س` (`as:58,82,163`), whatever the store currency.
- **Q-2** `unpaidInvoiceTotal`, receivables KPIs and outstanding use each invoice's own currency (no FX).
- **Q-3** "New customer" compares the first sale's raw date key (maybe an ISO instant) with a day key
  as strings (`as:198`).
- **Q-4** An invoice due today counts as overdue (`dueDate < now ISO`, `ds:144`).
- **Q-5** `getRecentInvoices` includes drafts; `getTopCustomers` ranks gross sales (AC§9).
- **Q-6** Payment-mix buckets merge methods that share a name (AA§7).

**Decisions (strictest option, logged):**
- **A-1** `days`/`limit` are bounded at the IPC boundary (§3.0), per AA§8/AC§3.
- **A-2** Device display settings (date style, numerals) travel as command args, because they are
  per-device (cross-cutting §3: appearance is device state) and the mock bakes them into returned
  strings (`as:58`). Rust formats with G-36. Only `getSalesAnalytics` needs them here.
- **A-3** Dashboard and insight commands are named `dashboard_*` in `domains/dashboard/`, not core.md's
  suggested `core_*`: master §4 names `dashboard` a domain, `BackendDomain` has `'dashboard'`
  (`backend.ts:26-44`), and `core_*` is Part 02's infrastructure namespace (`core_backend_status`).
  The switch is `usesRust('dashboard')`.
- **A-4** Synchronous service functions are served through a reactive mirror (G-37), not made async,
  so no page/controller changes (master §5).
- **A-5** Pending approval requests require `Approvals / Read` on the server (the only caller already
  gates on `canApprove`).

## 8. Tests

**(a) `src-tauri/tests/domain_analytics.rs` and `domain_dashboard.rs`** (fixtures are dated relative
to the DB's `UTC_TIMESTAMP()` read at test start — the business-clock source, P2-08 — with
`settings.timezone` set so "today" is unambiguous):
- sales analytics: trend length = `days`, same-day refunds netted; `trendInsight` text for both date
  styles and numeral systems; empty data → the "لا توجد…" texts; weekday buckets from `date_day`;
  payment mix by label with `"أخرى"`; `days = 0` and `367` → `VALIDATION`.
- product analytics: `net` fallback when `line.net` is NULL; historical line cost; top/bottom slices; insight text.
- customer analytics: new vs returning at the cutoff edge; top-10 share; `< 10` customers message.
- summary: 14-day trend; unpaid count/total; `NOT_FOUND` when no cash account exists.
- home KPIs: `period_range` for all three periods; `changePct` 0/null/value; refund reduces gross profit
  by the unit net; overdue includes a due-today invoice; `changePct` serializes as `null`.
- low stock: ratio order with `min_stock` NULL; `limit` bound.
- recent invoices/activity: order by key; `auth` excluded; names attached.
- top products/customers: ranking and limits.
- notification reads: SENT filter by branch; pending approvals `FORBIDDEN` for a cashier; draft count;
  stock value; `has_any_products` with only an inactive product → true.

**(b) Parity cases:** seeded demo data with the clock pinned to the seed's "today": commands 1–10
with defaults, plus `getHomeKpis` for `week` and `month`; command 1 with `ymd`+`arab`. Epsilon `1e-9`
on raw fields: none (every numeric field here is rounded).

## 9. Checklist

- [x] G-36, G-37, G-38 merged (manager); G-39 still **open** (`transfer_dto` pub twin needed — see status note above); 13's G-32, G-33, G-8 in place.
- [x] `domains/analytics/{mod,commands,service,dto}.rs`: 3 commands + `ipc_signatures()` + `export_bindings`.
- [x] `domains/dashboard/{mod,commands,dto}.rs` + `service/kpis.rs` (3.4–3.5, 3.9–3.10) and `service/feed.rs` (3.6–3.8, 3.11).
- [x] 16 commands with the §1 Area/Access; reported to the manager (see final report — 15 `dashboard_*` + 3 `analytics_*`, 14b's 2 counted in the 15).
- [x] `displayPrefs()` in `format.ts`; switch lines in `analyticsService.ts` and `dashboardService.ts` (§6).
- [x] `analytics/types/contract.check.ts` (new) and the dashboard entries in `core/types/contract.check.ts`.
- [x] Tests §8(a) written (`domain_analytics.rs`, 22 tests) — not run (deferred pass); parity cases §8(b) handed to Part 04 as a to-do (not separately enumerated — see final report).
- [x] Then [`14b-insights.md`](14b-insights.md).

## Gate

`cargo check` clean (manager's run after W6); tests and parity cases written (deferred run);
switch lines present; both `contract.check.ts` files compile; `bun run memory:check`: the 16
commands invoked + registered, 0 contract gaps.
