# 01 — Personas: Using the App as Each Role

I used the v1 build as each role for a normal working day. Each section lists what that
person does, where v1 gets in the way, and what v2 must give them. The needs point to the feature
files where they're specified.

---

## 1. Cashier / seller (البائع — الكاشير)

**Day:** opens the drawer with a float → sells for hours (scan, scan, pay) → handles a return →
a customer wants a box *or* a single strip → a regular customer buys on credit → closes the drawer,
counts the cash and hands over.

**Where v1 gets in the way**
- There's no shift: the drawer is never opened or counted, so cash over/short is invisible.
- There's no way to park a sale while serving the next customer.
- The price is fixed. The cashier can't change it for a negotiated price, even with permission, and
  can't give a discount on one item, only a % on the whole invoice.
- One payment method per sale. A customer paying part cash and part mada can't be served.
- Products have one unit, so a box and a strip can't both be sold.
- Returns start from the invoice list. Scanning the receipt's QR or number from the POS would be faster.
- Printing goes through the browser print dialog, which is slow at the counter.

**v2 gives them** ([06](06-sales-and-pos.md))
- Shift open/close with a Z-report.
- Held sales (F6), per-line discount and custom price within limits, split payment, unit picker and
  unit barcodes.
- Return by scanning the receipt, and a silent receipt print.
- Home = the POS itself, plus a small "my shift" panel.

---

## 2. Storekeeper (أمين المخزن) — *new role*

**Day:** receives goods from a supplier against a purchase → prints shelf labels for what arrived →
sends stock to another branch and receives incoming transfers → counts a shelf → writes off
damaged or expired items → checks what's running low.

**Where v1 gets in the way**
- There's no storekeeper role. Today the job needs manager rights, which include accounting.
- Receiving is the same action as posting the supplier bill (purchase "confirm"). It can't be done
  without seeing or entering prices.
- No branches or transfers.
- No expiry dates or batch numbers.
- No barcode label printing.
- Stocktake is all-or-nothing. It can't count one shelf or category, and the variance is measured
  against quantities at *completion*, not at the time of the count.

**v2 gives them** ([07](07-products-and-inventory.md))
- Role preset: inventory write, purchase receiving, transfers, labels; no accounting and no costs
  unless allowed.
- Goods receipt with an optional price-hidden mode, labels printed straight from a receipt, transfers
  with send/receive, and a partial stocktake with snapshot quantities.
- Expiry list, and a home page with receiving to do, incoming transfers, low stock, expiring soon and
  open counts.

---

## 3. Accountant (المحاسب)

**Day:** issues a B2B tax invoice on terms (net 30) for a company customer → records the supplier's
bill with its invoice number → records the rent and electricity with the scanned bill attached →
posts salaries to cost centers → allocates a customer's bank transfer across three invoices →
checks that VAT reconciles → sends a customer statement → at month end, reviews the trial balance.

**Where v1 gets in the way**
- The only way to sell is the POS. There's no full invoice form with due date, terms, per-line tax,
  notes and currency.
- No expenses screen. Every expense is a manual journal entry.
- The journal pages are thin: no attachments, no cost center, no party, no templates, no drafts, no
  inline preview of an entry's lines from the list. Keyboard entry in the grid is weak.
- A payment settles exactly one document. On-account and multi-invoice receipts are impossible.
- Nothing stops posting into a closed year, or posting a manual entry to the customer/supplier
  control accounts, which breaks the sub-ledgers.
- There's no opening entry, no VAT settlement and no year-end closing entry.

**v2 gives them**
- Full invoice form ([06](06-sales-and-pos.md)); expenses and vouchers ([09](09-purchases-payments-expenses.md)).
- Journal redesign ([11](11-journal-dashboard-insights.md)); period locks and control-account rules
  ([02](02-accounting-review.md)).
- Allocations; aging, VAT-return and cash-flow reports ([13](13-reports.md)).
- Home = cash and banks, receivables and payables due, VAT status, drafts and to-dos.

---

## 4. Owner / manager (المدير — صاحب المحل)

**Day:** opens the app in the morning wanting three answers: *how did we do, where's the money,
what should I do today?* Compares branches. Approves a large discount or write-off. Checks that
the backup ran.

**Where v1 gets in the way**
- The home page is busy (KPIs, chart, activity feed, recent invoices, stock alerts) but doesn't
  answer "what should I do".
- There are no comparisons with the previous period, no gross profit and no branch view.
- Nothing is recommended. The data to spot overdue customers, dead stock or a VAT deadline exists
  but isn't turned into actions.
- No backup at all.

**v2 gives them** ([11](11-journal-dashboard-insights.md), [14](14-platform.md))
- A home page that opens with "يحتاج انتباهك" (up to 5 recommendations, each with one action button),
  then 4 KPIs with the change vs the previous period and one chart.
- A separate التحليلات page for depth, a branch switcher and comparison, approvals, and backup status.
- A command palette for jumping anywhere.

---

## 5. Role matrix v2

Access levels: `—` none · `R` read · `W` write. **Bold** marks what changed from v1.

| Area | admin | manager | accountant | cashier | **storekeeper** |
|------|:-:|:-:|:-:|:-:|:-:|
| dashboard | W | W | W | R (shift panel) | **R (stock panel)** |
| pos | W | W | — | W | — |
| sales (invoices, quotations, returns) | W | W | **W** | R own | — |
| inventory (stock ops, transfers, counts, labels) | W | W | R | R | **W** |
| catalog (products, units, prices) | W | W | R | R | **R (W on units/barcodes)** |
| parties | W | W | W | R + quick-add | R suppliers |
| purchases | W | W | W | — | **W (receiving)** |
| **expenses** | W | W | W | **W pay-out from drawer** | — |
| payments | W | W | W | — | — |
| accounting (journal, CoA, periods) | W | R | W | — | — |
| reports | W | W | W | — | **R stock reports** |
| **analytics** | W | W | R | — | — |
| users | W | — | — | — | — |
| settings (company, taxes, templates, backup) | W | W (no backup restore) | R | — | — |

### Special permissions (per user, on top of the role)

`pos.maxDiscountPct` · `pos.overridePrice` (+ `minPrice` floor) · `pos.sellBelowCost` ·
`sales.refundWithoutReceipt` · `sales.viewCost` · `inventory.approveAdjustment` (above a value
threshold) · `accounting.postToClosedPeriod` (admin only) · `accounting.reverse` ·
`branches.all` (otherwise the user sees only their assigned branches) · `settings.restoreBackup`.

Presets remain the default. Settings → Users & roles shows the matrix, and an admin can adjust a
preset or clone it into a custom role. Route meta and the sidebar keep using `auth.can(area, level)`;
special permissions add `auth.allows(key)`.
