# 08 — Customers & Suppliers

## 1. Party form v2

The v1 form had a name, phone, VAT number and a note. v2 adds the fields from the reference
system plus what B2B invoicing and statements need. It's a modal for quick-add (name + phone +
type only) and a full page (`/customers/new`) with sections:

| Section | Fields |
|---------|--------|
| **الأساسي** (basics) | Individual / company, name (ar), name (en, optional; used on bilingual invoices), code (auto `C-0001` / `S-0001`), group, tags, active |
| **التواصل** (contact) | Phones (several, each with a label: mobile / work / **WhatsApp**) using `AppPhoneInput`, email, contact people (name, role, phone, email) |
| **العنوان الوطني** (national address) | Country, city (searchable list per country), district, street, building no., additional no., postal code, unit no., short address (e.g. `RRRD2929`). A preview line shows how it prints on the invoice |
| **الضريبة والسجل** (tax & registration) | VAT number (SA: 15 digits, starts and ends with 3; EG: 9 digits), CR number, national ID (individuals). A VAT number → B2B standard invoices by default |
| **التعامل** (terms) | Currency (locked after the first document), price list, payment terms (days), credit limit (customers), salesperson, home branch |
| **البنك** (bank, suppliers mostly) | Bank name, IBAN (validated: SA = 24 characters, checksum), account name |
| **الحسابات** (accounts, advanced) | Suppliers: default expense account (e.g. the electricity company → 6230) |
| **رصيد سابق** (balance from an old system) | Amount, side, as-of date ([05 §4](05-onboarding.md)) |
| **ملاحظات ومرفقات** (notes & attachments) | Notes; attachments (a copy of the CR, the contract) |

**Checks:**
- **Duplicates:** a warning on a duplicate phone or VAT number, with a link to the existing party.
- **Both roles:** "Customer and supplier" is possible, e.g. a wholesaler we both buy from and sell
  to. It's two linked party records, and a **net balance** is shown on each.

## 2. `AppPhoneInput` (used by *every* phone field)

Modeled on the reference `AppInputPhone.vue`, adapted to our components:

- **Layout:** in RTL the country button is at the *end* of the field; the number is LTR inside it.
  - **Country button:** flag + dial code (`+966`). It opens a searchable list (Arabic and English
    names, dial code).
  - **Order of the list:** common countries first: SA, EG, AE, KW, QA, BH, OM, JO. The company's
    country is the default.
- **Number field:** `type="tel"`, `inputmode="numeric"`, digits only. The placeholder and max
  length come from the country, e.g. SA `5X XXX XXXX` (9 digits). Arabic-Indic digits are converted
  to Latin as the user types.
- **Paste:** removes spaces, dashes and a leading `00` or `+` with the country code. It switches the
  country if the pasted number carries a different code.
- **Value:** stored as **E.164** (`+966567891234`), not the reference's `+966-567891234`. It's
  formatted for display (`+966 56 789 1234`) with `libphonenumber-js/min`.
- **Validation:** `isValidPhoneNumber` shows an inline Arabic error ("رقم الجوال غير صحيح للسعودية").
- **Extras:** a WhatsApp action button on the party page (`https://wa.me/<digits>`); click-to-copy.
- **Country data:** `modules/core/helpers/countries.ts` (code, Arabic name, dial code, currency,
  flag emoji), ported from `references/.../assets/data/country.ts`.

## 3. The party page (`/customers/:id`, `/suppliers/:id`)

Every customer and supplier has a real profile page.

**Header card:**
- Name, type, group and tags; phone, WhatsApp and email actions.
- **Balance**, colored by side: "عليه" (owes us) / "له" (we owe them).
- A **credit-limit bar** and **overdue amount** for customers.
- Primary actions: فاتورة جديدة (new invoice) / أمر شراء (new purchase), سند قبض/صرف (receipt /
  payment voucher), كشف حساب (statement), تعديل (edit).

**Tabs:**

| Tab | Contents |
|-----|----------|
| **نظرة عامة** (overview) | 4 KPIs (sales or purchases this year, outstanding, overdue, average days to pay); a 12-month bar chart; last 5 documents; an insight line, e.g. "يسدد عادة خلال 18 يوماً" (usually pays within 18 days) |
| **المستندات** (documents) | Invoices, quotations, credit notes (or purchases and debit notes), with filters and status chips |
| **المدفوعات** (payments) | Payments with their allocations. **Allocate unallocated credit** to open invoices from here |
| **كشف الحساب** (statement) | Date range; opening balance; running balance; currency toggle (party currency / base). PDF / Excel / print / share (copies a WhatsApp message with the balance summary and saves the PDF) |
| **الأعمار** (aging) | 0–30 / 31–60 / 61–90 / 90+ with the invoices in each bucket |
| **المرفقات** (attachments) | Files |
| **السجل** (history) | Activity: created, edited fields, documents, notes (who and when) |

## 4. Lists

- **Columns** (selectable): code, name, phone, city, balance, overdue, last activity, group.
- **Filters:** has balance, overdue, over the credit limit, group, city, inactive.
- **Bulk actions:** export Excel, import from Excel, print statements (one PDF, a page per party),
  deactivate.

## 5. Groups & pricing

**Customer groups** (Settings → Parties): name, default price list, default payment terms,
default discount %. They're applied when a customer is created and can be overridden per customer.
Examples: "جملة" (wholesale), "شركات" (companies), "VIP".
