# 10 — Branches, Currencies & Cost Centers

These are the three "dimensions" of v2. Each stays invisible for a business that doesn't use it:
- **One branch:** no branch switcher or branch columns.
- **Base currency only:** no currency fields.
- **No cost centers:** the fields are hidden (the branch cost center is filled in silently).

## 1. Branches (الفروع)

**Settings → Branches:** name, code, number prefix (e.g. `RYD`), address and phones (printed on
receipts), cash drawer account (auto-created as `111x الصندوق — <branch>`), default bank, default
price list, receipt header text, active.

- **Users:** each user has *allowed branches* and a *home branch*.
  - **Cashiers** are locked to their shift's branch.
  - **Users with more than one branch** get a **branch switcher** in the topbar: one branch or "كل
    الفروع" (all branches; read views only).
- **Documents:** every document has a branch. Numbering follows the branch's series
  (`RYD-INV-00042`); a single-branch company can use one series with no prefix.
- **Ledger:** every journal line carries `branchId`. The trial balance, P&L and balance sheet filter
  by branch, and the P&L has a "compare branches" mode (a column per branch).
- **Stock:** quantity per branch; transfers are in [07 §4](07-products-and-inventory.md).
- **Dashboard:** follows the selected branch. "All branches" adds a branch comparison card: sales,
  gross profit and average invoice, per branch.
- **Deactivating** a branch requires zero stock and no open shifts.

## 2. Multiple currencies (العملات)

**Settings → Currencies:**
- The base currency is set in onboarding and locked after the first posting.
- Other currencies are enabled with a code, Arabic name, symbol and decimals.
- **Exchange rates:** a table (currency, date, rate = base per 1 unit). The latest rate is the
  default on documents. The rate entry form accepts either direction ("1 USD = 3.75 SAR" or
  "1 SAR = 0.2667 USD").
- **Fixed rates:** SAR/USD and AED/USD are pegged, so they get a fixed-rate option.

**Rules (decision 6):**
- **A party has one currency.** Its documents, statement and credit limit are in that currency.
  A business selling to USD and SAR customers creates the USD ones with currency USD.
- **Documents:** they carry `currency` + `exchangeRate`. Lines and totals are in the document
  currency, and posting converts each line to the base currency.
  - **Journal lines** store `amountFc`, `currency` and `rate` so the FC balance can be rebuilt.
  - **Rounding:** base amounts are `round2(fc × rate)` per line. Any cent difference between
    Σ lines and the converted total goes to the largest line, never to a separate account.
- **Cash and bank:** accounts can have a currency (e.g. "صندوق الدولار" / USD cash box, "حساب
  بنكي بالدولار" / USD bank account). Their balance shows in both the FC and the base currency.
- **VAT on a foreign-currency invoice:** in SA the invoice must also show the **VAT amount in SAR**
  (`totals.vatBase`), and the templates print it.
- **POS:** sells in the base currency. With "قبول عملات أجنبية نقداً" (accept foreign cash) on, a
  cash tender can be taken in USD at today's rate, with the change in the base currency.
- **Price lists:** can have a currency (e.g. "قائمة تصدير بالدولار" / USD export list); a USD
  customer defaults to it.

**Realized FX (on payment allocation):**
```text
Base currency EGP. Invoice USD 1,000 @ 48.50 → AR 48,500 EGP.
Receipt USD 1,000 @ 49.20 → cash 49,200 EGP.
  Dr Cash (USD) 49,200 / Cr AR[p] 48,500 / Cr 4310 FX gain 700
Party ledger in USD: 1,000 − 1,000 = 0 ✓   (partial allocations use the invoice's rate for AR)
```

**Unrealized FX (optional, month end):**
- **Wizard:** "إعادة تقييم العملات" (currency revaluation) takes a rate per currency and lists the
  open FC balances (parties, FC banks and cash) with their revalued base amounts.
- **Posting:** an `FX_REVAL` entry to FX gain/loss, auto-reversed on the first day of the next period.

**Reports:** always in the base currency. Party statements and FC cash/bank ledgers can switch to
the FC. The trial balance has an FC column for FC accounts.

## 3. Cost centers (مراكز التكلفة)

**Settings → Accounting → Cost centers:**
- **Tree:** code, name, type (branch / department / project / other), parent, manager, active.
- **Budget** per fiscal year (optional).
- **Branch cost centers** are created with each branch and can't be deleted.

- **Where they're used:** every journal line can carry `costCenterId`.
  - Documents fill it from the branch, or from the document or line if the user picked one.
  - Expense accounts can be marked `requiresCostCenter`.
- **Manual journal:** a cost center column. A "توزيع" (split) action on a line splits it by
  percentages across cost centers, e.g. rent 60% branch A, 40% branch B.
- **Reports:**
  - **P&L by cost center:** a column per center, with a drill-down.
  - **Cost center ledger.**
  - **Budget vs actual:** bars with variance %; an insight when a center passes 90% of its budget.

## 4. How the dimensions show in the UI

- **Line grids** (journal, purchase, expense) show branch, cost center and currency columns only when
  the feature is on and, for currency, only when the value differs from the header.
- **Report filter bar** (`ReportShell`): branch and cost center selects, and a currency toggle,
  each shown only when relevant.
- **Settings → Features:** "تفعيل الفروع / العملات / مراكز التكلفة" (enable branches / currencies /
  cost centers) switches, so the UI stays simple until they're needed. Onboarding sets them from
  the answers.
