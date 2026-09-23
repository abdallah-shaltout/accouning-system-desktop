# 13 — Reports v2

## 1. Shared report shell

All reports use the upgraded `ReportShell`:
- **Filter bar:** period with comparison (previous period / same period last year), branch, cost
  center, currency (only when relevant). Filters are kept in the URL.
- **Insights box:** under the title, following the reference `ReportInsights` shape: a headline,
  2–4 metrics and recommendations, all from the insight rules ([11 Part D](11-journal-dashboard-insights.md)).
- **Export:** **real PDF** (Typst report template), **Excel** (formatted, with SUM formulas) and print.
- **Drill-down:** every number links to the rows behind it:
  - An account balance opens the ledger for that account and period.
  - A ledger row opens the journal entry.
  - A journal entry opens its document.
- **Speed:** results are computed in the mock backend and are 200–500 ms at most on the demo seed.

## 2. Catalogue

**Status key:**
- ✅v1 = exists, keep with the shared shell
- ⬆ = exists, upgrade
- ✚ = new

### Financial statements

| Report | Status | Notes |
|--------|:-:|-------|
| Trial balance | ⬆ | Levels (headers collapsed or expanded), opening / movement / closing columns, comparison, FC column |
| Income statement (P&L) | ⬆ | Sections from subtypes: revenue → cost of sales → **gross profit** → operating expenses → other → zakat → net. Comparison, **by branch** and **by cost center** columns |
| Balance sheet | ⬆ | **Current / non-current** split (from subtypes); customers and suppliers with credit balances reclassified; comparison date |
| Cash-flow statement | ✚ | Indirect method from subtypes (working-capital changes, investing = fixed assets, financing = capital, loans, drawings) |
| General ledger / account statement | ⬆ | Party, cost center and branch columns; FC columns for FC accounts; opening balance row |
| Day book (دفتر اليومية) | ✚ | Printable journal for a period (PDF) |

### Receivables & payables

| Report | Status | Notes |
|--------|:-:|-------|
| Customer / supplier statement | ⬆ | Moved to the party page; bulk statements PDF |
| AR aging / AP aging | ✚ | Buckets 0–30 / 31–60 / 61–90 / 90+ by **due date**; per party, drillable |
| Overdue invoices | ✚ | With a contact action (WhatsApp link), sortable by days overdue |
| Customer balances summary | ✚ | Balance, limit, overdue, last payment date |

### Sales

| Report | Status | Notes |
|--------|:-:|-------|
| Sales summary | ⬆ | Grouping by day / week / month, **product, category, customer, cashier, branch, payment method, hour** |
| Gross profit | ✚ | Per invoice / product / category: sales − COGS, margin %; the cost comes from the posted `costValue` |
| Returns analysis | ✚ | By reason, product, cashier; returns rate |
| Discounts & price overrides | ✚ | Per cashier and product: list price vs charged, approvals |
| Shift / Z-report history | ✚ | Per shift: sales by method, expected vs counted, variance |
| Quotations conversion | ✚ | Sent → accepted → invoiced rate |

### Inventory

| Report | Status | Notes |
|--------|:-:|-------|
| Stock on hand & valuation | ⬆ | Per branch or all; value = qty × average cost; ties to the inventory GL (the difference is shown and should be 0) |
| Stock movements | ✅v1 → ⬆ | + branch, batch, value columns |
| Low stock / reorder | ✚ | With suggested qty and "create purchase draft" |
| Slow / dead stock | ✚ | Days since last sale, value |
| Expiry | ✚ | Buckets, actions |
| Stocktake variances | ✚ | Per count: qty and value variance, by category |
| Transfers | ✚ | In transit, received with shortages |

### Purchases & expenses

| Report | Status | Notes |
|--------|:-:|-------|
| Purchases summary | ✚ | By supplier / product / period; average purchase price trend per product |
| Expenses | ✚ | By category / branch / cost center / month, with the trend |
| Budget vs actual | ✚ | Per cost center |

### Tax

| Report | Status | Notes |
|--------|:-:|-------|
| VAT return | ⬆ | **SA boxes**: standard-rated sales, sales to registered persons in other GCC states (0 unless used), zero-rated domestic sales, exports, exempt sales; standard-rated domestic purchases, imports (VAT paid at customs / reverse charge), zero-rated purchases, exempt purchases; corrections from credit/debit notes and from previous periods; carried-forward credit; net due. **EG/AE variants** by country. Reconciles to the VAT accounts (keeps the v1 check) |
| VAT detail | ✚ | Line-level listing per document with category, net, VAT: the audit trail behind each box |

### Management

| Report | Status | Notes |
|--------|:-:|-------|
| Period comparison | ✚ | Any two periods, key lines side by side with Δ and Δ% |
| Branch comparison | ✚ | KPIs per branch |
| Business health | ✚ | From the reference: liquidity (current ratio), profitability (net margin), debt, collection (DSO), each scored 0–25, with an explanation in plain Arabic |
| Profit leakage | ✚ | Discounts + price overrides + returns + shrinkage + write-offs as % of sales |

## 3. Report menu

The reports hub is grouped by the sections above. There's a search box (also reachable from the
command palette), and each user can **pin** reports as favorites, which then show first.
