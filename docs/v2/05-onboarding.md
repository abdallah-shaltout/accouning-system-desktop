# 05 — Onboarding, Opening Entry & Moving From an Old System

**Goal:** a shop that already has customers, suppliers, stock, cash and a bank balance in another
system (or in a notebook) can go live in under an hour, with correct opening balances.

## 1. First run

A fresh install (no company in IndexedDB) opens a **welcome screen**, not the login screen. It has
two cards:
- **ابدأ شركتك** (start your company): runs the wizard, and the first user created there becomes admin.
- **استكشف ببيانات تجريبية** (explore with demo data): loads today's 75-day demo seed. A "بيانات
  تجريبية" badge stays in the topbar, and Settings → *Reset* returns to the welcome screen.

## 2. The wizard (`/setup`, blank layout, progress rail on the right)

Each step has a small "why this matters" note. **Required** steps block completion; the others
can be skipped and appear later in the setup checklist. Progress is saved after each step.

| # | Step | Req. | Contents |
|---|------|:---:|----------|
| 1 | **نوع النشاط** (business type) | ✓ | Clothing, pharmacy, supermarket, electronics, general retail, services or wholesale. Sets the defaults: units, product fields (expiry for pharmacy), CoA add-on and POS layout |
| 2 | **بيانات المنشأة** (company details) | ✓ | Name (Arabic + optional English), logo upload with crop, individual/company, VAT number (SA: 15 digits starting and ending with 3), CR number, national address, phones (country-code input), email |
| 3 | **الدولة والعملة والضريبة** (country, currency, tax) | ✓ | Country → base currency and tax templates. VAT registered? Prices include VAT? (default yes for retail). Extra currencies (optional) with a first rate |
| 4 | **السنة المالية وتاريخ البدء** (fiscal year, start date) | ✓ | Fiscal year start (default 1 Jan). **Go-live date** = the opening-balance date (default: today) |
| 5 | **الفروع** (branches) | ✓ | The main branch (name, number prefix, address). "Add another branch" is optional |
| 6 | **دليل الحسابات** (chart of accounts) | ✓ | Choose مبسّط / **قياسي** (recommended) / مفصّل with a live tree preview. Name the bank accounts, one row each (bank, IBAN, currency) |
| 7 | **طرق الدفع** (payment methods) | ✓ | Toggles: cash, mada, Visa/Mastercard, bank transfer, STC Pay, credit sale. Each is pre-mapped to an account from step 6 and can be edited |
| 8 | **الأرصدة الافتتاحية** (opening balances) | — | See §3. It can be done later from the checklist |
| 9 | **المستخدمون** (users) | — | Add a cashier, accountant or storekeeper (name, username, role, branch, POS PIN) |
| 10 | **الطباعة** (printing) | — | Invoice template (thumbnails), thermal width, default printer, and a test print |
| 11 | **جاهز** (ready) | ✓ | A summary, and the opening entry for review if step 8 was done. "ابدأ العمل" goes to the role's home |

**Afterwards:** the dashboard shows a **setup checklist card** until every skipped item is done or
dismissed: opening balances, first product import, users, template, first backup.

## 3. Opening balances (step 8, also at `/setup/opening`)

Tabs, each with **manual entry** or **Excel import**. Everything is dated the go-live date.

1. **النقدية والبنوك** (cash and banks): a balance per cash drawer and bank account. Foreign-currency
   accounts take the FC amount and a rate.
2. **العملاء** (customers): a row per customer with name, phone, balance and side (مدين = owes us /
   دائن = we owe them), plus optional currency and credit limit. Missing customers are created here.
   - **Detail per customer (optional):** instead of one balance, list the open invoices from the old
     system (number, date, due date, amount). Aging then works from day one. These become
     `opening` targets for payment allocation.
3. **الموردين** (suppliers): the same, mirrored.
4. **المخزون** (stock): per branch, a row per product with quantity (any unit), unit cost, and batch +
   expiry for tracked products. Products not in the catalog can be created inline or imported.
5. **أرصدة أخرى** (other balances): any other balance-sheet account (fixed assets at net book value,
   loans, accrued salaries, prepaid rent, owner current account).
6. **المراجعة** (review):
   - The **opening journal entry** built from tabs 1–5, with a line per party and product total.
   - A mini balance sheet: assets, liabilities, and the balancing figure in **3900 أرصدة افتتاحية**.
   - A question: "تحويل الفرق إلى: رأس المال / جاري المالك" (move the difference to capital or the
     owner's current account). Default: capital.

**Posting:**
- **What it creates:** one `OPENING` journal entry, per-party lines with party tags, stock movements
  with reason `opening` and the `stockValue` set for each product.
- **Closing 3900:** a second entry moves the 3900 balance to capital, which brings 3900 to zero
  (invariant 9).
- **Before first use:** both entries can be edited. Editing reverses and re-posts them until the
  first sale or purchase is posted. After that, changes go through the manual journal.

## 4. Adding an old customer or supplier later

The party form has an **"رصيد سابق من نظام قديم"** (balance from an old system) section: amount,
side, and as-of date (default: the go-live date).

**Posting:**
- **The entry:** a small `OPENING` entry, Dr `receivable[p]` / Cr `openingBalanceEquity` (mirrored
  for suppliers).
- **After go-live:** if the as-of date is after the go-live date, the credit side goes straight to
  **capital** instead of 3900, with a note. 3900 stays at zero.

**On the party:**
- **Statement:** the balance appears as a **"رصيد افتتاحي"** row.
- **Payments:** it can be allocated like an invoice.
- **Editing:** allowed until a payment is allocated to it. After that, the amount is locked and
  the correction is a journal adjustment.

## 5. Excel import (shared for all master data)

Used by the opening balances, products, customers, suppliers, price updates and stock counts.

1. **Download template:** an .xlsx with Arabic headers, a notes row, data validation (lists for
   unit, category, tax, side) and sample rows. The sheet is set right-to-left.
2. **Upload:** the user drops an .xlsx or .csv. Their own files work too.
3. **Map columns:** the app auto-maps by header name (Arabic and English synonyms), and the user can
   remap with dropdowns. The mapping is remembered per import type.
4. **Validate:** a table of rows with ✓ / ⚠ / ✗.
   - **Errors** (✗): per cell, e.g. "الوحدة 'كرتونه' غير موجودة".
   - **Duplicates:** matched by SKU or barcode or phone. The user chooses **update / skip / create**.
   - **Unknown categories or units** are offered for automatic creation.
5. **Import:** runs in batches with a progress bar. At the end there's a summary (created / updated /
   skipped) and a downloadable error file with the failed rows and a reason column.

**Implementation:** `exceljs` is lazy-loaded. There's one generic `ImportWizard` component, and each
import type has a descriptor:
```ts
{ key, title, columns: [{ key, label, synonyms, required, parse, validate }], dedupe, commit }
```

## 6. Settings touched by onboarding (editable later)

- **Company, taxes, branches, payment methods, numbering and templates:** under Settings.
- **Base currency:** locked after the first posting.
- **Go-live date:** locked after the opening entry is posted.
- **Business type:** can be changed later. It only changes defaults.
