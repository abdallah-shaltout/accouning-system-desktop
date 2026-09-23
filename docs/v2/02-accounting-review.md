# 02 — Accounting Review of v1 + Corrected Posting Rules

**Scope of the review:** every posting rule in `src/mocks/backend/` (sales, refunds, purchases,
purchase returns, payments, stock adjustments, manual journal, reversal) plus the chart of accounts
fixture and the balances module. I also ran `bun run verify:mocks` against the seeded data.

**How to read the status column:**
- ✅ correct, keep
- ⚠️ acceptable for a demo but wrong or limiting for a real shop
- ❌ a bug: produces wrong numbers or breaks an invariant

## 1. Summary

| # | Area | Finding | Status |
|---|------|---------|:-:|
| A1 | Inventory | The inventory account drifts away from Σ qty × average cost after returns (verify:mocks on the seed: GL **30,741.59** vs **30,742.20**) | ❌ |
| A2 | Inventory | Average cost is rounded to 2 decimals on every receipt, which adds to A1 | ⚠️ |
| A3 | Stock-in | Every STOCK_IN credits **Capital 3100**, whatever the reason | ❌ |
| A4 | Stocktake | Gains are credited to revenue (4400), which inflates income | ⚠️ |
| A5 | Stocktake | Variance is measured against quantities at *completion*, not at count time | ⚠️ |
| B1 | Journal | Manual entries can post to AR 1130 / AP 2100 / Inventory 1140, which breaks the sub-ledgers | ❌ |
| B2 | Periods | Nothing stops posting into a **closed fiscal year**; closing a year posts no closing entry | ❌ |
| B3 | Journal | A reversal is always dated "now"; the user can't choose the date or give a reason | ⚠️ |
| C1 | Payments | One payment settles exactly one document: no on-account, multi-invoice or advance payments | ❌ (blocks opening balances) |
| C2 | Balances | Party balances are Σ open documents, not the ledger, so they can't include opening balances or unallocated money | ❌ (blocks v2) |
| C3 | Payments | Card payments go straight to Bank 1120. Card money arrives a day or more later, minus fees, so it needs a clearing account | ⚠️ |
| D1 | Sales | One VAT rate for the whole store, applied to the invoice total. There are no exempt or zero-rated goods and no per-line VAT | ⚠️ |
| D2 | Sales | Prices are VAT-exclusive; Saudi B2C shelf prices must include VAT | ⚠️ |
| D3 | Sales | Customer credit limit is not enforced; credit sales have no due date, so aging is impossible | ⚠️ |
| D4 | Returns | Cash back is always credited to the original payment method's account | ⚠️ |
| D5 | Returns | A return isn't a legal credit note: its reason is optional and the credit-note wording is missing | ⚠️ |
| E1 | Purchases | A purchase return's cash back always debits **Cash 1110**, even for a bank refund | ❌ |
| E2 | Purchases | Service lines are forced to 5300 instead of the product's purchase account | ⚠️ |
| E3 | Purchases | No supplier invoice number or date; input VAT is claimed even from suppliers without a VAT number | ⚠️ |
| E4 | Purchases | No purchase discounts and no landed costs (freight or customs added to inventory cost) | ⚠️ |
| F1 | CoA | 5100 Purchases and 5150 Purchase Returns exist but never move (they're periodic-system accounts in a perpetual system), which confuses accountants | ⚠️ |
| F2 | CoA | Flat list with no header accounts, so the balance sheet can't split current from non-current | ⚠️ |
| F3 | CoA | Posting rules hard-code account codes (`'1130'`), so renumbering breaks posting | ⚠️ |
| F4 | CoA | Accounts are missing: opening-balance equity, card clearing, VAT settlement, customer advances, FX, cash over/short, write-offs, fixed assets… (see [03](03-chart-of-accounts.md)) | ❌ |

Correct in v1, and staying:
- **Double entry:** every entry balances, and zero lines are dropped.
- **Sale:** Dr cash/bank for the paid amount and AR for the remainder; Cr net sales and output VAT;
  COGS at average cost.
- **Return:** revenue, VAT and COGS are reversed at the original line cost. The last return on an
  invoice uses exact remainders, so no cents are left over.
- **Purchase:** Dr inventory and input VAT, Cr AP.
- **Receivables:** a partial or credit sale requires a customer.
- **Balance sheet:** the unclosed current-year result is shown inside equity.
- **VAT report:** it reconciles to the VAT accounts.

## 2. Findings in detail

### A1 + A2 — inventory drift (❌)

[`purchases.ts:77`](../../src/mocks/backend/purchases.ts#L77) re-averages cost on receipt and rounds it to 2 decimals.
Returns move value at a *different* cost and don't re-average:

- **Sales return** ([`sales.ts:246`](../../src/mocks/backend/sales.ts#L246)): the returned units come back at the *original line cost*.
- **Purchase return** ([`purchases.ts:175`](../../src/mocks/backend/purchases.ts#L175)): the units go out at the *purchase price*.

Example: stock is 9 units with an average cost of 50. A receipt of 10 @ 60 makes the average 55.26.
A customer then returns 1 unit that was sold at cost 50.
- The GL gets +50, for a total of 1,100.
- The stock list shows 20 × 55.26 = 1,105.26.
- The two are now **5.26 apart**. Across many returns the gap grows.

**Fix:**
- Keep a **`stockValue` per product** alongside `stockQty`, and derive
  `avgCost = stockValue / stockQty` (4 decimals, display only).
- Every movement changes `stockValue` by exactly the amount posted to the GL.
- **Receipt** and **sales return**: value += posted amount, which re-averages automatically.
- **Sale**: COGS = round2(qty × avgCost). When the sale empties the stock, COGS = the remaining
  `stockValue`, so nothing is left over.
- **Purchase return**: value −= qty × purchase price. If that would leave a negative value, or a
  non-zero value with zero quantity, the difference goes to *inventory variance* (5110).
- New invariant: `GL(inventory) = Σ product.stockValue`, exactly.

### A3 — every stock-in credited to capital (❌)

[`inventory.ts:49`](../../src/mocks/backend/inventory.ts#L49) credits Capital for every stock-in.
Opening stock, a gift from a supplier, found items and an owner bringing goods from home are
different events. **Fix:** the stock-in document gets a required `reason`:

| Reason | Credit account (role) |
|--------|--------------|
| Opening stock | `openingBalanceEquity` (3900) |
| Owner contribution | `ownerCurrent` (3150) |
| Free goods from a supplier or a gift | `otherIncome` (4300) |
| Found / surplus (not during a count) | `inventoryVariance` (5110, credit) |
| Other | a user-chosen account (not a control account) |

### A4 + A5 — stocktake (⚠️)

- **Gains:** recorded as revenue (4400) today. Book both gains and losses to **inventory variance
  5110** in *cost of sales*. That's what they are: corrections of COGS.
- **Write-offs** (damaged or expired): use their own account, **5120**, so the owner sees them.
- **Snapshot:** a count records `systemQty` at *count time* for each line. Completing it applies
  `countedQty − systemQtyAtCount` as a delta, so sales made between counting and approving aren't
  counted twice. [`inventory.ts:21`](../../src/mocks/backend/inventory.ts#L21) currently re-reads
  quantities at completion.

### B1 — control accounts open to manual entries (❌)

[`journal.ts:6`](../../src/mocks/backend/journal.ts#L6) accepts any active account. A manual Dr 1130
changes the GL without changing any customer's balance, and the "AR GL = Σ customers" check breaks.
**Fix:**
- Accounts get `requiresParty` and `allowManual` flags.
- **AR/AP:** a manual line needs a party. Allowed with a party, e.g. for write-offs or reclassification.
- **Inventory, VAT output and VAT input:** blocked in manual entries. Use a stock adjustment or a
  VAT settlement instead.
- Opening balances go through the wizard, not the manual journal.

### B2 — closed periods (❌)

[`core.ts:44`](../../src/mocks/backend/core.ts#L44) `postJournal` never checks the fiscal year, and the
Fiscal Years page just toggles `isClosed`. **Fix:**
- **Every posting** checks that its date is inside an open fiscal year and after the **lock date**
  (a settings value such as "no postings before 2026-06-30"). Only `accounting.postToClosedPeriod`
  can override the check.
- **Closing a year** is a wizard:
  1. Pre-checks: no drafts, trial balance balanced, 3900 is zero.
  2. The closing entry: revenue and expenses → retained earnings (3250).
  3. The year is locked.
  4. The next year is opened.
- **Reopening** reverses the closing entry and needs admin rights.

### B3 — reversal (⚠️)

Reversal is always dated now ([`journal.ts:34`](../../src/mocks/backend/journal.ts#L34)). **Fix:** a dialog with the date
(default today, or the original date if its period is open) and a required reason, which is stored
on both entries.

### C1 + C2 — payments and party balances (❌)

[`payments.ts:9`](../../src/mocks/backend/payments.ts#L9) links one payment to one document.
[`balances.ts:7`](../../src/mocks/backend/balances.ts#L7) sums open documents. Neither can represent an
opening balance, a deposit or an overpayment. **Fix:**
- **Party on the line:** every AR/AP journal line carries `partyType` + `partyId`.
- **Balance:** a party's balance is Σ(debit − credit) of the lines on its control account, per currency.
- **Payments:** a payment posts once (Dr method account / Cr AR). Its `allocations[]` link it to
  documents, including the opening balance, as a sub-ledger record only. Allocating or
  un-allocating later changes no GL entry.
- **Document outstanding:** a document's outstanding = total − Σ allocations − Σ credit notes.
- **Unallocated money:** shows as a credit on the party (see `customerAdvances` in [09](09-purchases-payments-expenses.md)).

### C3 — card clearing (⚠️)

**Fix:** each payment method has its own account. Mada/Visa go to **1125 card clearing**. A
*card settlement* voucher then posts Dr bank + Dr card fees 6140 / Cr 1125 when the bank deposit
arrives. The recommendations engine warns about clearing balances older than 3 days.

### D1 + D2 — VAT model (⚠️)

**Tax categories** (the ZATCA codes) replace the single store rate:

| Category | Code | Rate |
|----------|:-:|------|
| Standard | S | 15% |
| Zero-rated | Z | 0% (e.g. qualifying medicines, exports) |
| Exempt | E | 0% |
| Out of scope | O | — |

- **Categories:** each product has a sale tax and a purchase tax. Zero-rated and exempt both have
  0 VAT but go in **different VAT-return boxes**, so they can't be one "0% tax".
- **Per line:** each line computes and rounds its own VAT; the invoice VAT is the sum of the lines.
- **VAT-inclusive prices** are the retail default. The math is in [06 §3](06-sales-and-pos.md).

### D3 — credit control (⚠️)

- Customers get `paymentTermsDays` and `creditLimit`.
- **Due dates:** a credit invoice stores `dueDate`, which drives aging and "overdue".
- **Credit limit:** a sale that would take the balance over the limit is blocked unless the user has
  `sales.overrideCreditLimit`.

### D4 + D5 — returns (⚠️)

- **Refund method:** the return form asks how the money goes back: cash, card reversal, bank, or
  *keep as customer credit* (an unallocated credit on the party).
- **Credit note:** the printed return is titled **إشعار دائن**, references the original invoice
  number and date, and requires a reason. Both are ZATCA credit-note requirements.

### E1–E4 — purchases (❌ / ⚠️)

- **E1 (❌):** a refund from a supplier uses the chosen method's account, not always cash
  ([`purchases.ts:174`](../../src/mocks/backend/purchases.ts#L174)).
- **Line accounts:** non-stock and service lines post to the product's purchase account, falling
  back to the category, then to the settings default.
- **Supplier invoice:** the purchase stores `supplierInvoiceNo` and `supplierInvoiceDate`, and warns
  about a duplicate invoice number for the same supplier.
- **Input VAT:** input VAT is recoverable only when the supplier has a VAT number. Otherwise the VAT
  is added to the cost, with a warning.
- **Discounts:** line and invoice discounts on purchases use the same math as sales.
- **Landed costs:** optional freight, customs and clearing lines are spread over stock lines by
  value, or by quantity, and become part of inventory cost.

### F1–F4 — chart of accounts

Replaced by the v2 tree in [03](03-chart-of-accounts.md). Posting rules look accounts up by
**system role** (`accountFor('receivable')`), never by code.

## 3. Corrected posting rules (v2)

`[p]` = the line carries a party · `[b]` branch · `[cc]` cost center. Branch and cost center are
filled on every line from the document. Amounts are in the base currency; foreign amounts are
stored on the line too (see [10](10-branches-currencies-cost-centers.md)).

| Document | Debit | Credit |
|----------|-------|--------|
| **Sale / tax invoice** | each tender's method account; `receivable[p]` for the unpaid part | revenue per line (product → category → `sales`/`serviceRevenue`), net of discounts; VAT output per tax account |
| ↳ cost | `cogs` | `inventory` (stock value moved) |
| **Credit note (sales return)** | `salesReturns` (net); VAT output | `receivable[p]` up to what's outstanding, then the refund method account *or* `receivable[p]` as credit |
| ↳ cost | `inventory` (original line cost, re-averaged) | `cogs` |
| **Purchase invoice** | `inventory` (stock lines + landed cost); the purchase account for services/expenses; `vatInput` (recoverable only) | `payable[p]` |
| **Debit note (purchase return)** | `payable[p]` up to what's outstanding, then the refund method account | `inventory` at purchase price (with the variance guard); service accounts; `vatInput` |
| **Customer receipt** | method account | `receivable[p]` (± `fxGain`/`fxLoss` on settlement) |
| **Supplier payment** | `payable[p]` | method account (± FX) |
| **Card settlement** | `bank`, `cardFees` | `cardClearing` |
| **Expense voucher** | expense account `[cc]`, `vatInput` | method account *or* `payable[p]` |
| **Money transfer** (drawer → bank, bank → bank) | destination | source |
| **Stock-in** | `inventory` | by reason (table in A3) |
| **Write-off** (damaged/expired) | `inventoryWriteOff` | `inventory` |
| **Stocktake** | `inventory` / `inventoryVariance` | `inventoryVariance` / `inventory` |
| **Transfer: send** | `inventoryInTransit` [to-branch] | `inventory` [from-branch] |
| **Transfer: receive** | `inventory` [to-branch]; `inventoryVariance` for a shortage | `inventoryInTransit` |
| **Shift close** | cash over: `cash`; short: `cashShort` | over: `cashOver`; short: `cash` |
| **Opening entry** | asset balances; `receivable[p]` per customer | liability balances; `payable[p]` per supplier; the difference goes to `openingBalanceEquity` |
| **Close opening equity** | `openingBalanceEquity` (or credit, depending on its sign) | `capital` or `ownerCurrent` |
| **VAT settlement** | `vatOutput` | `vatInput`; `vatPayable` (net; a debit if refundable) |
| **VAT payment** | `vatPayable` | `bank` |
| **Year-end close** | each revenue account | each expense account; the net goes to `retainedEarnings` |
| **FX revaluation** (optional, period end) | party/bank FC accounts or `fxLoss` | `fxGain` or the accounts; auto-reversed on day 1 of the next period |

## 4. Invariants: `bun run verify:mocks` v2

Every one of these must print OK after seeding, and again after the e2e flows run:

1. Every entry: Σ debit = Σ credit; no line is both debit and credit; ≥ 2 lines.
2. Trial balance is balanced; balance sheet A = L + E, with the current result included.
3. `GL(receivable)` = Σ customer sub-ledgers, per currency; the same for payables and suppliers.
4. `GL(inventory)` = Σ `product.stockValue`, exactly. Each branch's quantity = Σ that branch's movements.
5. Output VAT GL for a period = Σ VAT on invoice lines − Σ VAT on credit-note lines. Input VAT: the
   same, for purchases, expenses and debit notes.
6. For every party: Σ document outstanding − unallocated credit ± opening balance = sub-ledger balance.
7. Every posted document has exactly one active entry (or an entry + reversal pair); every
   `sourceRef` resolves.
8. No entry is dated inside a locked period unless its creation time is before the lock.
9. `openingBalanceEquity` = 0 once onboarding is complete.
10. Card and wallet clearing balances match the unsettled tenders.
11. Every closed shift: expected cash − counted cash = the posted over/short amount.

## 5. Note for the real backend (later)

The mock uses JS numbers with `round2`. The real data layer must store money as **integer minor
units** (halalas/piasters) or as a decimal type, and compute per-line tax and rounding exactly as
[06 §3](06-sales-and-pos.md) specifies. That way the mock's results are the backend's test oracle.
