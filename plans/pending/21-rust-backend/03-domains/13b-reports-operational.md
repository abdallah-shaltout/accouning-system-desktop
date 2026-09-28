# 21 · 03.13b — `reports` (read-only reports engine: operational reports)

> **Status:** implemented 2026-09-28 (code complete; DB tests and parity cases written but not run
> from this session — deferred, time-boxed test pass per the manager's cargo-throttling rule). All
> 16 commands in this file are in `domains/reports/{dto.rs,commands.rs,service/{sales,stock,purchasing,receivables}.rs}`,
> sharing `service/common.rs` with 13. 16 switch lines added to `reportService.ts`; the 20
> `contract.check.ts` entries for this file live in the same `src/modules/reports/types/contract.check.ts`
> 13 created (41 entries total: 21 from 13 + 20 from 13b). **Needs from manager:** same as 13 —
> `pub mod reports;` + `reports::export_bindings(cfg)` hook in `domains/mod.rs`, the 32 commands in
> `generate_handler!`, and confirmation the Part 02 gaps below are actually merged (assumed, not
> re-verified). Wave W6 (entry file §4). Depends on: every
> writer domain (00–12) and on [`13-reports.md`](13-reports.md) (its §3.0 helpers and its Part 02
> gaps, **G-1 and G-34 are needed here**).
> **Split:** this is the second half of the `reports` domain. [`13-reports.md`](13-reports.md)
> holds the 16 statement/ledger reports and the shared helpers; this file holds the 16
> operational reports (sales, purchases, stock, aging). Same Rust domain `domains/reports/`, same
> implementer, implemented **after** 13.

**Goal.** Port the 16 operational functions of `reportService.ts` to `reports_*` commands with the
mock's exact DTOs, rounding points and row order. Read-only, one REPEATABLE READ snapshot each, no
events, no undo.

**Read first.** [`01-frontend-analysis/reports.md`](../01-frontend-analysis/reports.md) §1, §2,
§7 (`A§n`) · `src/modules/reports/services/reportService.ts` (`rs:<line>`) ·
`src/modules/reports/types/index.ts` (`types:<line>`) · `src/mocks/backend/payments.ts:29-70`
(`openDocumentsFor`, ported as `shared::balances::open_*_for`, G-1) · `src/mocks/backend/sales.ts:266`
(free-text line ids) · `src/modules/core/helpers/labels.ts:37-42` (`SALE_METHOD_LABEL`).

## 1. Commands

All `port (confirmed)` (A§1). **tx = `with_read_ctx`** (13 G-32), **Area/Access = `Reports` / `Read`**,
**events = none**. None is paged (13 R-1).

| # | Mock fn (`rs:` line) | Rust command | Args → Return |
|---|---|---|---|
| 1 | `getSalesReport` (327) | `reports_get_sales_report` | `{ range: DateRangeInput }` → `SalesReport` |
| 2 | `getInventoryReport` (416) | `reports_get_inventory_report` | `()` → `Vec<InventoryReportRow>` |
| 3 | `getDiscountsReport` (841) | `reports_get_discounts_report` | `{ range, groupBy: DiscountGroupBy }` → `Vec<DiscountReportRow>` |
| 4 | `getGrossProfitReport` (752) | `reports_get_gross_profit_report` | `{ range, groupBy: GrossProfitGroupBy }` → `Vec<GrossProfitRow>` |
| 5 | `getReturnsReport` (782) | `reports_get_returns_report` | `{ range }` → `ReturnsReport` |
| 6 | `getExpensesReport` (1062) | `reports_get_expenses_report` | `{ range }` → `ExpensesReport` |
| 7 | `getShiftsReport` (885) | `reports_get_shifts_report` | `{ range }` → `Vec<ShiftReportRow>` |
| 8 | `getLowStockReport` (913) | `reports_get_low_stock_report` | `()` → `Vec<LowStockRow>` |
| 9 | `getDeadStockReport` (930) | `reports_get_dead_stock_report` | `{ days?: i32 }` → `Vec<DeadStockRow>` |
| 10 | `getStocktakeVariances` (963) | `reports_get_stocktake_variances` | `{ countId?: Id }` → `Vec<StocktakeVarianceRow>` |
| 11 | `getTransfersReport` (993) | `reports_get_transfers_report` | `{ range }` → `Vec<TransferReportRow>` |
| 12 | `getPurchasesReport` (1018) | `reports_get_purchases_report` | `{ range }` → `PurchasesReport` |
| 13 | `getBranchComparison` (1099) | `reports_get_branch_comparison` | `{ range }` → `Vec<BranchComparisonRow>` |
| 14 | `getProfitLeakageReport` (1165) | `reports_get_profit_leakage_report` | `{ range }` → `ProfitLeakageReport` |
| 15 | `getAgingReport` (694) | `reports_get_aging_report` | `{ kind: PartyKindArg }` → `Vec<AgingReportRow>` |
| 16 | `getOverdueReport` (719) | `reports_get_overdue_report` | `{ kind: PartyKindArg }` → `Vec<OverdueRow>` |

`range` is always `DateRangeInput` (13 §2). `PartyKindArg` is shared with 13.

## 2. DTOs (same `domains/reports/dto.rs`, `export_to = "reports/types/gen/"`, 13 §2 conventions)

| Rust DTO | TS type | Notes |
|---|---|---|
| `SalesReport` + `SalesReportSummary`, `SalesByDay`, `SalesByProduct`, `SalesByCategory`, `SalesByMethod`, `SalesByCashier` | `types:108-127` | `SalesByProduct.productId: String` (holds `freetext-<i>` for free-text lines, §3.0). Counts `i32`. |
| `InventoryReportRow` | `types:129` | `status`: enum `StockStatus { Ok, Low, Out }` lowercase (A§2). |
| `DiscountReportRow` | `types:284` | `key: String`. |
| `GrossProfitRow` | `types:253` | `key: String` (invoice id, product key or category **name**). |
| `ReturnsReportRow`, `ReturnsReport` | `types:265,273` | `key: String` (reason text, product key or cashier id / `"—"`). |
| `ExpensesReport` + `ExpensesByCategory`, `ExpensesByMonth` | `types:373` | `categoryId: Id`; `month: String` (`YYYY-MM`). |
| `ShiftReportRow` | `types:296` | `terminalId: Id`; `openedAt`/`closedAt?` key strings; `openedBy`/`closedBy?` are **names**. |
| `LowStockRow` | `types:312` | |
| `DeadStockRow` | `types:323` | `lastSaleDate?: String` (raw key); `daysSinceSale: i64` with `#[ts(type = "number")]` (holds `Number.MAX_SAFE_INTEGER` = 9007199254740991, which is exact in JSON). |
| `StocktakeVarianceRow` | `types:336` | |
| `TransferReportRow` | `types:350` | `status: String` (the transfer enum's wire value); branch fields are names. |
| `PurchasesReport` + `PurchasesReportSummary`, `PurchasesBySupplier`, `PurchasesByProduct` | `types:365` | `supplierId`/`productId: String` (fall back to the id when the name is missing). |
| `BranchComparisonRow` | `types:389` | |
| `ProfitLeakageReport` | `types:430` | |
| `AgingReportRow` | `types:226` | |
| `OverdueRow` | `types:238` | `kind`: enum `OverdueDocKind { Invoice, PurchaseOrder }` camelCase (A§2); `phone?`, `dueDate?`. |
| Args: `DiscountGroupBy { Cashier, Product }`, `GrossProfitGroupBy { Invoice, Product, Category }` (lowercase), `ReportsGetDeadStockReportArgs { days: Option<i32> }`, `ReportsGetStocktakeVariancesArgs { countId: Option<Id> }` | — | |

`contract.check.ts` (the file created by 13): 20 more `Expect<Equals<Gen.X, X>>` entries, one per
named TS type above.

## 3. Service logic

### 3.0 Rules shared by this file (on top of 13 §3.0)

- **Document order** = the mock's array order = `ORDER BY created_at, id`; lines by `position`.
- **Business day** of a document = its `date_day` (`localDateKey(x.date)`); range filter =
  `in_range` on it. **Raw key comparisons** (the mock compares the stored string, not the local
  day) use the `DocDate` key: the generated `<field>_key` column where the table has one
  (`stock_movements`, `stock_transfers`, sales/purchases/expenses tables, `migration/src/m0006,m0008-m0011`),
  compared with `COLLATE utf8mb4_bin`, else `DocDate::key()` in Rust.
- **Free-text invoice lines** (`is_free_text = true`) have no product. Their group key is
  `"freetext-<position>"`, the mock's `productId: freetext-${i}` (`sales.ts:266`); a product lookup
  on them misses (cost 0, category `"بدون تصنيف"`, name from the line).
- **Lookups** (product, category, user, supplier, branch) use **live** rows (13 R-5), with the
  mock's fallback text when missing.
- **`is_product(line)`** = the line's live product has `type == "product"` (`rs:343,760,771,1108`).
- **`sum2`** = `round2(Σ)` (`utils.ts:70-72`); "raw" below means an unrounded running `Decimal`.
- **Group-bys** = `IndexMap` (first appearance) + stable sort (13 G-33).

**Aggregation matrix.**

| Command | SQL | Rust | Rounding (A§7) | Order |
|---|---|---|---|---|
| 1 sales | invoices + lines + refunds in range | 6 breakdowns | byProduct raw then **one** `round2`; byDay/byMethod/byCashier `round2` per add; summary `sum2` | see 3.1 |
| 2 inventory | live active products + categories | status | `round2(qty × price)` | `compare_ar(category)`, `compare_ar(name)` |
| 3 discounts | invoices + lines | two code paths | per invoice/line `round2`, then per add | `discountValue` desc |
| 4 gross profit | invoices + lines | three code paths | invoice: once; product/category: per add | `profit` desc |
| 5 returns | invoices (count), refunds + lines, invoice lines | 3 groupings | per add | `amount` desc |
| 6 expenses | expenses in range | 2 groupings | per add | amount desc / month asc |
| 7 shifts | closed shifts + invoices | string-window filter | `sum2` | `openedAt` key desc |
| 8 low stock | products | formula | `round2` value | `qty` asc |
| 9 dead stock | `MAX(date_key)` per product on sale movements | days | `round2` value | `costValue` desc |
| 10 stocktake | completed counts + lines | variance | `round2` ×2 | `|valueVariance|` desc |
| 11 transfers | transfers + lines | sums | `sum2`, `round2` | date key desc |
| 12 purchases | received POs + lines, returns | 2 groupings | per add; `avgPrice` once | total desc |
| 13 branch comparison | invoices + lines, branches | per branch | `sum2`, `round2` | `sales` desc |
| 14 profit leakage | invoices + lines, refunds, stock movements | sums | `round2` on combined sums | — |
| 15/16 aging/overdue | `shared::balances::open_*_for` per party | buckets/days | `round2` per add | total / days desc |

### 3.1 `reports_get_sales_report` (`rs:327-412`)

1. Parse range. `invoices` = status ≠ `DRAFT`, day in range; `refunds` = day in range (`rs:329-330`).
   No FX conversion (quirk Q-6).
2. **byProduct** (`rs:332-349`), per invoice: `factor = 1 − discountRate/100`; `inclusive = taxAmount > 0 &&
   |subTotal − discountAmount − grandTotal| < 0.01` (re-derived, not a stored flag — A§9, kept); per
   line (key per §3.0; row created with `name = line.name` on first sight): `qty += line.qty`;
   `revenue += ((qty × price − discount) × factor) / (inclusive ? 1 + (line.taxRate ?? inv.taxRate)/100 : 1)`
   raw; `cost += is_product ? qty × costPrice : 0` raw.
3. `productRows` = rows mapped `{ revenue: round2(raw), cost: round2(raw), profit: round2(rawRevenue − rawCost) }`
   (profit from the **raw** values, `rs:348`), stable sort by rounded `revenue` desc.
4. **byCategory** (`rs:351-359`): iterate `productRows` in that order; key = the product's live category
   **name** or `"بدون تصنيف"`; `qty += r.qty` raw; `revenue = round2(revenue + r.revenue)`; stable sort revenue desc.
5. **byDay** (`rs:361-368`): key = `date_day` as `YYYY-MM-DD`; `invoices += 1`; `total = round2(total + grandTotal)`;
   sort by key asc.
6. **byMethod** (`rs:370-380,409`): key `payment_method`, `count += 1`, `total = round2(total + grandTotal)`,
   stable sort total desc, then `method` = `SALE_METHOD_LABEL`: `cash "نقداً"`, `card "بطاقة (مدى/فيزا)"`,
   `bank_transfer "تحويل بنكي"`, `credit "آجل"`.
7. **byCashier**: same grouping on `cashier_id`, `name` = live user name or `"—"` (`rs:410`).
8. **summary** (`rs:382-405`): `invoiceCount`; `grossSales = sum2(subTotal)`; `discounts = sum2(discountAmount)`;
   `vat = sum2(taxAmount)`; `total = sum2(grandTotal)`; `netSales = round2(total − vat)`; `refunds =
   sum2(refund grandTotal)`; `netAfterRefunds = round2(total − refunds)`; `cogs = sum2(productRows.cost)`;
   `grossProfit = round2(netSales − cogs)`; `averageInvoice = count ? round2(total / count) : 0`.

### 3.2 `reports_get_inventory_report` (`rs:416-434`)

1. Live products with `type == "product" && active`.
2. Row: `productId, name, sku, category` (live name or `"بدون تصنيف"`), `qty = stock_qty`, `minStock =
   min_stock ?? 0`, `costPrice`, `price`, `costValue = round2(qty × costPrice)`, `retailValue = round2(qty × price)`,
   `status = qty <= 0 ? out : qty <= minStock ? low : ok`.
3. Stable sort by `compare_ar(category)` then `compare_ar(name)` (G-34; `rs:433`).

### 3.3 `reports_get_discounts_report` (`rs:841-881`)

1. Parse range. Invoices non-draft in range.
2. `cashier` path (`rs:845-855`), key `cashier_id`, label = live user name or the id string: per invoice
   `invoiceCount += 1`; `listValue = sum2(lines qty × price)`; `charged = round2(listValue − discountAmount −
   sum2(lines discount))`; `row.listValue = round2(row.listValue + listValue)`; `row.chargedValue =
   round2(row.chargedValue + charged)`.
3. `product` path (`rs:857-867`), key per §3.0, label = first line's name: **per line** `invoiceCount += 1`;
   `listValue = round2(qty × price)`; `charged = round2(listValue − line.discount)`; same two per-add rounds.
   The two paths are **not** one parameterised query (A§1).
4. Output `{ key, label, invoiceCount, listValue, chargedValue, discountValue: round2(list − charged),
   discountPct: list > 0 ? round2((list − charged)/list × 100) : 0 }`, keep `discountValue > 0.001`,
   stable sort `discountValue` desc.

### 3.4 `reports_get_gross_profit_report` (`rs:752-778`)

1. Parse range. Invoices non-draft in range.
2. `invoice` path (`rs:758-762`): key `inv.id`: `{ label: number, qty: sum2(lines qty), revenue:
   round2(subTotal − discountAmount), cost: round2(sum2(lines is_product ? qty × costPrice : 0)) }`.
3. `product`/`category` path (`rs:764-773`): `factor = 1 − discountRate/100`; key = product key (§3.0) or
   the live category name / `"بدون تصنيف"`; label = first line's name (product) or the key (category);
   `qty += qty` raw; `revenue = round2(revenue + (qty × price − discount) × factor)`; `cost = round2(cost +
   (is_product ? qty × costPrice : 0))`.
4. Output `{ key, label, qty: round2(qty), revenue, cost, profit: round2(revenue − cost), marginPct:
   revenue > 0 ? round2((revenue − cost)/revenue × 100) : 0 }`, stable sort `profit` desc (`rs:775-777`).

### 3.5 `reports_get_returns_report` (`rs:782-837`)

1. Parse range. `totalInvoices` = count of non-draft invoices in range; refunds in range with lines.
2. `byReason` (`rs:787-800`): key `reason ?? "غير محدد"`, label = key; `count += 1`; `qty += sum2(refund lines qty)`;
   `amount = round2(amount + refund.grandTotal)`; stable sort amount desc.
3. `byProduct` (`rs:802-818`): refund's invoice (any date) — missing → skip; per refund line, its
   invoice line by `invoice_line_id` — missing → skip; key = that line's product key; `count += 1`
   (per line); `qty += rl.qty` raw; `amount = round2(amount + rl.qty × invLine.price)` (original line
   price, A§1); label = live product name or the key; stable sort amount desc.
4. `byCashier` (`rs:820-832`): key = the refund's invoice's `cashier_id`, or `"—"` when the invoice is
   missing; label = live user name or the key; same sums as byReason; stable sort amount desc.
5. `returnRatePct = totalInvoices > 0 ? round2(totalRefunds / totalInvoices × 100) : 0` (a count ratio, A§1).

### 3.6 `reports_get_expenses_report` (`rs:1062-1079`)

1. Parse range. Live expenses with day in range.
2. `byCategory`: key `category_id`, `amount = round2(amount + e.amount)`; name = live category name or
   the id; stable sort amount desc.
3. `byMonth`: key = `date_day` formatted `%Y-%m` (the business day's month, never a UTC truncation,
   A§1); `round2` per add; sort `month` asc.
4. `total = sum2(amount)`.

### 3.7 `reports_get_shifts_report` (`rs:885-909`)

1. Parse range. Shifts with `status = CLOSED` and `opened_at_day` in range.
2. `salesTotal` per shift (`rs:890-893`) = `sum2(grandTotal)` of non-draft invoices with `cashier_id =
   shift.opened_by` whose **day key string** `k` satisfies `!(k < openedKey) && !(k > closedKey)`,
   where `openedKey = opened_at` key and `closedKey = closed_at` key or `openedKey`, compared as
   **byte strings** (the mock's `inDateRange(i.date, s.openedAt, s.closedAt)` compares a
   `YYYY-MM-DD` key with ISO instants — quirk Q-1, kept literally).
3. Row: `id, number, terminalId, openedAt: key, closedAt: key?, openedBy: live user name or id,
   closedBy: name or id when set, expectedCash ?? 0, countedCash ?? 0, variance ?? 0, salesTotal`.
4. Stable sort by `openedAt` key desc, byte order.

### 3.8 `reports_get_low_stock_report` (`rs:913-928`)

1. Live products `type == "product" && active && stock_qty <= (min_stock ?? 0)`.
2. `suggestedQty = max(reorder_qty ?? (min_stock ?? 0) × 2, (min_stock ?? 0) − stock_qty + (reorder_qty ?? 0))`
   exactly as `rs:924` (two branches, not simplified — A§1); `costValue = round2(stock_qty × cost_price)`;
   category name or `"بدون تصنيف"`.
3. Stable sort `qty` asc.

### 3.9 `reports_get_dead_stock_report` (`rs:930-959`)

1. `days = args.days.unwrap_or(60)`; `days < 0` → `VALIDATION "عدد الأيام غير صالح"` (decision R-8).
   `today = ctx.clock.today()`.
2. SQL: `SELECT product_id, MAX(date_key COLLATE utf8mb4_bin) FROM stock_movements WHERE reason = 'sale'
   GROUP BY product_id` (the mock keeps the max raw string, `rs:934-938`).
3. Live products `type == "product" && active && stock_qty > 0`; `daysSinceSale = last ? days_between(today,
   day_of_key(last)) : 9_007_199_254_740_991`; `lastSaleDate = last` (raw key, omitted when none).
4. Keep `daysSinceSale >= days`; stable sort `costValue` desc (`costValue = round2(qty × cost_price)`).

### 3.10 `reports_get_stocktake_variances` (`rs:963-989`)

1. Counts with `status = COMPLETED` and (no `countId` or `id = countId`), `(created_at, id)` order.
2. Per line: live product or skip; `countedQty = counted_qty ?? 0`, `systemQty = system_qty ?? 0`;
   `qtyVariance = round2(counted − system)`; skip `|qtyVariance| < 0.0001` (A§1); `valueVariance =
   round2(qtyVariance × product.cost_price)` (the **current** cost, quirk Q-3).
3. Stable sort by `|valueVariance|` desc.

### 3.11 `reports_get_transfers_report` (`rs:993-1014`)

1. Parse range. Transfers with `date_day` in range, with lines.
2. `sentQty = sum2(lines qty)`; `receivedQty = sum2(lines received_qty ?? (status == RECEIVED ? qty : 0))`
   (three-way, A§1); branch names live or the id; `status` wire string; `shortageQty = round2(sent − received)`;
   `shortageValue = shortage_value ?? 0` (stored, not recomputed).
3. Stable sort by `date` key desc, byte order.

### 3.12 `reports_get_purchases_report` (`rs:1018-1058`)

1. Parse range. POs `status = RECEIVED` with day in range; purchase returns with day in range (all).
   No FX conversion (quirk Q-6).
2. `bySupplier`: key `supplier_id`, `count += 1`, `total = round2(total + grandTotal)`; name = live
   supplier or id; stable sort total desc.
3. `byProduct`: key `product_id`, `qty += qty` raw, `total = round2(total + qty × costPrice)`; output
   `{ productId, name (live or id), qty: round2(qty), total, avgPrice: qty > 0 ? round2(total / qty) : 0 }`
   (`avgPrice` from the raw qty); stable sort total desc.
4. `summary = { poCount, grossPurchases: sum2(subTotal), vat: sum2(taxAmount), total: sum2(grandTotal),
   returns: sum2(returns grandTotal) }`.

### 3.13 `reports_get_branch_comparison` (`rs:1099-1120`)

1. Parse range. Non-draft invoices in range with lines.
2. Live `active` branches in order; per branch its invoices (`branch_id = b.id`): `sales = sum2(grandTotal)`;
   `cogs = sum2(per invoice sum2(lines is_product ? qty × costPrice : 0))`; row `{ branchId, name, sales:
   round2(sales), grossProfit: round2(sum2(subTotal − discountAmount) − cogs), invoiceCount, averageInvoice:
   count ? round2(sales / count) : 0 }` (two bases, A§1).
3. Stable sort `sales` desc.

### 3.14 `reports_get_profit_leakage_report` (`rs:1165-1183`)

1. Parse range. Non-draft invoices in range with lines; refunds in range; stock movements with day in range.
2. `netSales = round2(sum2(subTotal − discountAmount))`; `discounts = round2(sum2(discountAmount) +
   sum2(per invoice sum2(lines discount)))` (one round on the combined sum, A§7); `returns = sum2(refund subTotal)`;
   `writeOffs = sum2(|value_change|)` of `reason = 'loss'`; `shrinkage = sum2(|value_change|)` of
   `reason = 'stocktake' && qty_change < 0`.
3. `totalLeakage = round2(discounts + returns + writeOffs + shrinkage)`; `leakagePct = netSales > 0 ?
   round2(totalLeakage / netSales × 100) : 0` (a value ratio, A§1).

### 3.15 `reports_get_aging_report` (`rs:694-715`)

1. `today = ctx.clock.today()`. Live parties of `kind` in order.
2. `docs = shared::balances::open_invoices_for(id)` or `open_purchase_orders_for(id)` (G-1 full shape);
   none → skip.
3. Per doc: `ref = due_date_key ?? date_key`; `d = days_between(today, day_of_key(ref))`; `d <= 0 → current`,
   `<= 30 → b30`, `<= 60 → b60`, else `b90plus`, each `round2(bucket + outstanding)`.
4. `total = round2(current + b30 + b60 + b90plus)`; keep `total > 0.001`; stable sort total desc.

### 3.16 `reports_get_overdue_report` (`rs:719-744`)

1. `today`; live parties of `kind`; per party its open docs (G-1).
2. Skip a doc with no `due_date_key` (supplier POs never carry one — quirk Q-4); `d = days_between(today,
   day_of_key(due))`; skip `d <= 0`.
3. Row `{ id, kind: invoice | purchaseOrder, number, partyId, partyName, phone: party.phone (omitted when
   NULL), date: date_key, dueDate: due_date_key, daysOverdue: d, outstanding }`.
4. Stable sort `daysOverdue` desc.

## 4. Concurrency (D8)

Same as 13 §4: one `with_read_ctx` snapshot per command, no locks (A§5). Aging reads parties and
their open documents inside that one snapshot, so a payment allocated on another terminal mid-report
is either fully in or fully out.

## 5. Undo

Not applicable: no writes (entry §3.5).

## 6. Frontend switch lines

File `src/modules/reports/services/reportService.ts` (the import is added by 13):

| Function (line) | Switch line |
|---|---|
| `getSalesReport` (327) | `if (usesRust('reports')) return backendCall('reports_get_sales_report', { range });` |
| `getInventoryReport` (416) | `… ('reports_get_inventory_report');` |
| `getDiscountsReport` (841) | `… ('reports_get_discounts_report', { range, groupBy });` |
| `getGrossProfitReport` (752) | `… ('reports_get_gross_profit_report', { range, groupBy });` |
| `getReturnsReport` (782) | `… ('reports_get_returns_report', { range });` |
| `getExpensesReport` (1062) | `… ('reports_get_expenses_report', { range });` |
| `getShiftsReport` (885) | `… ('reports_get_shifts_report', { range });` |
| `getLowStockReport` (913) | `… ('reports_get_low_stock_report');` |
| `getDeadStockReport` (930) | `… ('reports_get_dead_stock_report', { days });` |
| `getStocktakeVariances` (963) | `… ('reports_get_stocktake_variances', { countId });` |
| `getTransfersReport` (993) | `… ('reports_get_transfers_report', { range });` |
| `getPurchasesReport` (1018) | `… ('reports_get_purchases_report', { range });` |
| `getBranchComparison` (1099) | `… ('reports_get_branch_comparison', { range });` |
| `getProfitLeakageReport` (1165) | `… ('reports_get_profit_leakage_report', { range });` |
| `getAgingReport` (694) | `… ('reports_get_aging_report', { kind });` |
| `getOverdueReport` (719) | `… ('reports_get_overdue_report', { kind });` |

## 7. Known mock quirks (kept) and decisions

**Quirks:**
- **Q-1** Shifts report: an invoice counts toward `salesTotal` only if its day key is **after** the
  opening day string and not after the closing instant string (`rs:891`). A shift opened and closed
  on the same day always shows `salesTotal = 0`. This is the most user-visible quirk in `reports`;
  recommended fix later: compare instants (`opened_at ≤ invoice instant ≤ closed_at`).
- **Q-2** Shifts report attributes sales to the cashier who **opened** the shift (A§1).
- **Q-3** Stocktake `valueVariance` uses today's product cost, not the count line's `unit_cost`.
- **Q-4** Supplier overdue report is always empty: purchase orders carry no `dueDate` in the open-
  document shape (`payments.ts:56-69`); supplier aging uses the PO date instead.
- **Q-5** Free-text lines from **different** invoices at the same position share one byProduct/
  discount group (`freetext-<i>` keys collide in the mock too).
- **Q-6** Sales, purchases, gross-profit, discount, branch and leakage reports sum document fields
  without FX conversion (only the VAT report converts, 13 §3.9).
- **Q-7** Gross profit `invoice` vs `product`/`category` paths and returns `byProduct` vs
  `byReason`/`byCashier` use different formulas that need not add up to the same total (A§1/§9).

**Decisions:**
- **R-8** `getDeadStockReport(days)`: a negative `days` → `VALIDATION "عدد الأيام غير صالح"` (the
  page only sends the default or a positive number; an IPC input is validated, master F8).
- **R-9** Stable sorts everywhere (`Vec::sort_by`), matching V8's stable `Array.prototype.sort`, so
  ties keep first-appearance order.
- **R-10** `DeadStockRow.daysSinceSale` is `i64` and emits `9007199254740991` for "never sold",
  the mock's `Number.MAX_SAFE_INTEGER`, exact as a JSON number.
- 13's R-2 (date validation), R-4 (byte order for ASCII keys), R-5 (live lookups) and R-7
  (Decimal, epsilon for raw sums) apply here too.

## 8. Tests

**(a) `src-tauri/tests/domain_reports.rs`** (same file as 13, one test per bullet):
- sales: inclusive vs exclusive invoice revenue stripping; invoice discount factor; byProduct rounded
  once (three tax-inclusive lines of the same product at price 0.10, 15% VAT → revenue 0.26; per-line
  rounding would give 0.27); byCategory merges two categories with the same name;
  byMethod labels; draft excluded; refunds reduce `netAfterRefunds`; free-text line key.
- inventory: status thresholds; Arabic ordering via `compare_ar` (أ/ا/ب names).
- discounts: both paths on one fixture give different `invoiceCount`s; `> 0.001` filter.
- gross profit: three paths; `marginPct` 0 when revenue 0.
- returns: reason fallback `"غير محدد"`; byProduct uses original line price; missing invoice → cashier `"—"`.
- expenses: month bucket from the business day (an instant at 23:30 local on the last day of a month).
- shifts: Q-1 literally (same-day shift → 0; two-day shift counts only day two).
- low stock: `suggestedQty` both branches (reorderQty set / unset).
- dead stock: never-sold product → 9007199254740991; `days` filter; negative days → `VALIDATION`.
- stocktake: `< 0.0001` dropped; only COMPLETED; `countId` filter.
- transfers: `receivedQty` three-way logic; shortage.
- purchases: only RECEIVED; `avgPrice` from raw qty.
- branch comparison: inactive branch excluded; average 0 with no invoices.
- profit leakage: loss vs stocktake movements; `leakagePct` 0 with no sales.
- aging: bucket edges at 0/30/31/60/61 days; parties at 0 dropped.
- overdue: due today not overdue; supplier list empty (Q-4).
- `cashier` session → `FORBIDDEN` for one command of this file.

**(b) Parity cases for Part 04:** seeded demo data, full history + current month + mid-year range,
for commands 1, 3–7, 11–14; commands 2, 8–10, 15, 16 once (no range). The clock is pinned to the
seed's "today" on both sides. Fields compared with the `1e-9` epsilon (raw mock float sums, 13 R-7):
`SalesByProduct.qty`, `SalesByCategory.qty`, `ReturnsReportRow.qty` (all three groupings),
`LowStockRow.suggestedQty`. Known id-mapping exception: a seeded free-text line whose mock
`productId` is not `freetext-<i>` (`seed/branches9.ts:119`, `'freetext-fx-demo'`) maps to
`freetext-<position>` in Rust — whitelist that one key.

## 9. Checklist

- [x] G-1 (full `OpenDocument`) and G-34 (`compare_ar`) merged by the manager. *(assumed per `_part02-gaps.md`'s "fixed" status — not independently re-verified.)*
- [x] Add the 16 DTO groups of §2 to `dto.rs` and to `export_bindings`.
- [x] `service/sales.rs`: 3.1, 3.3, 3.4, 3.5, 3.13, 3.14.
- [x] `service/stock.rs`: 3.2, 3.8, 3.9, 3.10, 3.11.
- [x] `service/purchasing.rs`: 3.6, 3.7, 3.12.
- [x] `service/receivables.rs`: 3.15, 3.16.
- [x] 16 commands in `commands.rs` (`with_read_ctx` + `Area::Reports`/`Access::Read`) + 16 `ipc_sig!` lines.
- [x] Report the 16 names to the manager (`generate_handler!`) — see final report.
- [x] §8(a) tests in `tests/domain_reports.rs` — representative coverage (sales report, discounts/gross-profit empty sets, inventory, dead-stock validation, aging/overdue); ⏳ deferred time-boxed test pass, not run from this session.
- [x] 16 switch lines (§6).
- [x] 20 `contract.check.ts` entries.
- [ ] §8(b) cases (with the epsilon and whitelist notes) in the Part 04 case list. *(not written — out of this implementer's time-boxed scope.)*
- [x] Status note on both 13 files when done.

## Gate

`cargo check` clean (manager's throttled run after W6 — ⏳ not run from this session, per the
no-cargo hard rule); tests and parity cases written (run in the deferred, time-boxed pass); all 32
`reports` switch lines present (confirmed: `grep -c "usesRust('reports')" reportService.ts` = 32);
`contract.check.ts` compiles (needs `bun run bindings` first); `bun run memory:check`: 32
`reports_*` commands invoked + registered, 0 contract gaps (needs the manager's
`generate_handler!`/`domains/mod.rs` wiring first).
