# 09 — Purchases, Payment Methods, Payments & Expenses

## 1. Purchase invoice v2 (`/purchases`)

**Flow:** `DRAFT` → (optional) `ORDERED` → `RECEIVED` (posted), or `CANCELED`.

| Step | Who | What happens |
|------|-----|--------------|
| Draft / order | manager, accountant | Supplier, date, lines (product, **unit**, qty, price, discount, tax), invoice discount, landed costs. "إرسال للمورد" (send to supplier) prints or exports a purchase order PDF and sets `ORDERED` |
| Receive | storekeeper (or anyone with purchases W) | The receiving screen shows the lines with **received qty** (default = ordered). For tracked products: batch + expiry. **Prices are hidden** unless the user has `sales.viewCost`. *Confirm receipt* posts stock + AP at the order prices |
| After receipt | accountant | Enters the supplier invoice number and date (a warning appears while they're missing). Price differences on the supplier's invoice go through a debit note or a supplementary purchase invoice |

- **Short delivery:** receiving less than ordered → "إنشاء أمر متبقٍ" (create a backorder draft) for
  the difference. Each document is received once; this keeps one posting per document.
- **Landed costs:** rows for freight, customs and clearing, each an amount + supplier (optional) +
  spread by value or qty.
  - **Same supplier:** added to the AP total.
  - **Another supplier** (e.g. the shipping company): a separate AP line for that supplier.
  - **Stock cost:** each stock line's cost includes its share (review E4).
- **VAT:** input VAT is recoverable only if the supplier has a VAT number. Otherwise the VAT is added
  to the cost, with a banner. Imports with customs VAT go to an optional line on the `vatInput`
  account.
- **Debit notes (مرتجع مشتريات / إشعار مدين):**
  - Reason required; refund method (against AP, cash, or bank).
  - Batch picking for tracked products.
  - "Return expiring batch" shortcut from the expiry report.

## 2. Payment methods (Settings → طرق الدفع)

**List:** drag to reorder; toggle *show in POS* / *show in payments*.

**Form:**
- Name, type, icon, account (and an account override per branch), fee %, requires reference,
  currency, active.

**Presets:**

| Method | Type | Account |
|--------|------|---------|
| نقداً (cash) | cash | the branch cash drawer |
| مدى (mada) | card | 1125, fee 0.8% (editable) |
| فيزا/ماستر (Visa/Mastercard) | card | 1125, fee 2.2% |
| تحويل بنكي (bank transfer) | bank_transfer | 1120 |
| STC Pay | wallet | 1126 |
| آجل (credit) | credit | the party's AR |
| رصيد العميل (customer credit) | store_credit | uses unallocated credit |

**Card settlement** (`/payments/settlements`):
- **Pick tenders:** unsettled card/wallet tenders grouped by day and method, with totals. Select the
  days being settled.
- **Enter the deposit:** the amount the bank received. The fee = the difference, pre-filled from fee %.
- **Post:** Dr bank, Dr card fees / Cr clearing.

## 3. Payments with allocation (سند قبض / سند صرف)

**Form:**
- **Header:** party, date, branch, method, amount, currency + rate (the party's currency), reference,
  attachments.
- **Allocation grid:**
  - Rows: open documents + the opening balance, oldest first.
  - Columns: number, date, due date, total, outstanding, **allocate** (editable).
  - Buttons: "تخصيص تلقائي" (auto allocate: oldest first, the default) and "مسح" (clear).
  - The footer shows allocated / unallocated.
- **Unallocated:** allowed. It stays on the party as a credit (a customer deposit or supplier
  advance) and shows on the party page with an "allocate" action.
- **FX:** if the party's currency isn't the base currency, the posted base amount uses the payment
  rate. Each allocation compares it with the invoice's rate, and the difference posts to FX gain or
  loss (see [10](10-branches-currencies-cost-centers.md) §2).

**Printing and lists:**
- **Voucher:** A5/A4 سند قبض/صرف with the amount in words and signature boxes.
- **List:** allocation status (fully / partly / unallocated), method and branch filters.

## 4. Expenses (المصروفات) — *new module*

**Expense categories (Settings → Expenses):** name, expense account, default tax, default cost
center, icon. Presets from the CoA template (rent, electricity, internet, packaging, maintenance…).

**Expense form (`/expenses/new`):**
- Date, branch, category, amount, **"فاتورة ضريبية؟"** (is it a tax invoice? → VAT split, supplier
  VAT number and invoice number), cost center.
- **Paid from:** a payment method or cash drawer, *or* on credit to a supplier.
- Description, **attachments** (photo of the bill), repeat monthly.
- Posting: Dr expense [cc] + VAT input / Cr method account or AP.

**Recurring expenses:** a template with the next date. On the due date an insight appears: "إيجار
أكتوبر مستحق — سجّله" (October rent is due — record it). The accountant posts it in one click;
`autoPost` is optional.

**List:** category and branch filters, a month total by category (mini bar chart), export.

## 5. General vouchers (السندات العامة) — the reference's "fast journal"

For money movements that aren't invoices:

| Voucher | Example | Posting |
|---------|---------|---------|
| **سند قبض عام** (general receipt) | Scrap sale, a refund from a government fee | Dr method account / Cr chosen account |
| **سند صرف عام** (general payment) | Paying a one-off fee | Dr chosen account / Cr method account |
| **تحويل بين الحسابات** (transfer) | Drawer → bank deposit, bank → bank | Dr destination / Cr source (with a fee line) |
| **مسحوبات / إضافة رأس مال** (owner) | The owner takes cash or adds cash | Dr drawings / Cr cash, or the reverse to capital / owner current |

Each voucher prints as a PDF and creates a normal system journal entry with a source link.

## 6. Cost centers in these documents

- **Default:** every document has a cost center, taken from the branch.
- **Where it can be changed:**
  - Expense and general-voucher forms: the header.
  - The purchase invoice: per line (e.g. shared costs).
  - The manual journal: per line.
- **Required:** accounts with `requiresCostCenter` refuse to post without one. Details are in
  [10](10-branches-currencies-cost-centers.md) §3.
