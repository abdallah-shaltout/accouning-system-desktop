# 06 — Sales: POS, Full Invoice Form, Pricing & VAT Math, Returns, Shifts

There are two ways to sell, and **both produce the same `SalesInvoice`**. They share the math in
`modules/invoices/helpers/totals.ts` and post through the same rules.

| | **POS** (`/pos`) — for the cashier | **Invoice form** (`/sales/invoices/new`) — for the accountant |
|---|---|---|
| Speed | Scan-first, keyboard, one screen | Complete, spreadsheet-like grid |
| Customer | Optional (walk-in) | Required, with a details card |
| Payment | Must be settled now (tenders), or on credit | Usually on terms (due date); a payment can be taken now |
| Output | Thermal receipt (A4 on request) | A4 tax invoice (PDF) |
| Extras | Held sales, shifts, returns by scanning the receipt | Quotations → invoice, per-line description and tax, amount in words, journal preview |

## 1. POS upgrades

**Cart line**
- **Unit picker:** shown when the product has more than one unit. Scanning a unit's barcode selects
  that unit (the box barcode → علبة, the strip barcode → شريط).
- **Qty:** a stepper, plus the classic `3*` prefix (type `3*` then scan = qty 3).
- **Price:** click to edit when `pos.overridePrice` allows it. The floor is `minPrice`, or cost unless
  `pos.sellBelowCost`. A reason is required when the price is below the list price. The line shows a
  small "سعر معدّل" (price changed) tag; `listPrice` is kept for audit.
- **Line discount:** % or amount (F8 on the selected line; Shift+F8 = invoice discount). Over the
  user's `maxDiscountPct` → a **manager PIN approval overlay**, which is logged.
- **Batch:** auto-picked first-expiry-first-out (FEFO) for tracked products, and can be changed.
  An expired batch blocks the sale.
- **Weighted barcodes** (supermarket setting): prefix `2x` + item code + weight or price.

**Flow**
- **Customer (F4):** search by name or phone, or quick-add (name + phone). The card shows the balance
  and credit-limit use. The price list switches to the customer's.
- **Held sales (F6):** hold with an optional label. A badge shows the held count. They're saved per
  terminal and survive a reload; open holds show a warning at shift close.
- **Pay (F12)** opens the tender dialog:
  - Method buttons come from the payment methods marked *show in POS*.
  - **Split payment:** add tenders until the remaining amount is 0.
  - **Cash:** quick-amount buttons (exact, 50, 100, 200, 500) and the change.
  - **Card / wallet:** optional reference (last 4 digits, approval code).
  - **Credit:** needs a customer and checks the credit limit.
  - **Foreign cash** (if enabled): choose USD → shows the amount due in USD at today's rate; change is
    given in the base currency.
- **After the sale:**
  - The receipt prints silently if a printer is configured ([12](12-documents-pdf-excel.md) §5);
    otherwise it opens the preview.
  - Buttons: "طباعة A4" (print A4), "فاتورة جديدة" (new sale; Enter).
- **Return (F7):** scan the receipt QR or type its number → pick the lines → refund method.
  Returning without a receipt needs `sales.refundWithoutReceipt`.
- **Shift bar:** shows the shift start time and cash sales so far. F10 = pay-in/pay-out (a pay-out for
  a small expense creates an expense voucher paid from the drawer).

**Keys:**

| Key | Action |
|-----|--------|
| F1 | Help |
| F2 | Search |
| F4 | Customer |
| F6 | Hold / resume |
| F7 | Return |
| F8 | Line discount |
| Shift+F8 | Invoice discount |
| F9 | New sale |
| F10 | Cash in/out |
| F12 / Ctrl+Enter | Pay |
| Ctrl+P | Reprint last receipt |
| Del | Remove line |
| + / − | Change qty |
| `n*` | Qty multiplier |

**Speed** (targets are measured with the mock's latency switched off):
- **Scan → line in the cart < 50 ms:** a barcode index (`Map<barcode, {productId, unitId}>`) is built
  once and updated when products change. Nothing is awaited on add-to-cart. Totals are computed
  locally.
- **Product grid:** virtualized, with images lazy-loaded as thumbnails. Favorites and category
  tabs are cached in Pinia.
- **Checkout → receipt < 300 ms:** printing runs after the sale is saved and never blocks the
  next sale.
- **POS route:** preloaded right after login for cashiers.

## 2. Full invoice form (desk)

- **Header:**
  - Customer, with a card showing VAT number, address, balance, credit limit and overdue amount.
  - Invoice date, due date (date + customer terms), branch, salesperson, price list, currency + rate,
    and the customer's PO reference.
  - **Invoice type** is automatic: *فاتورة ضريبية* (standard) when the buyer has a VAT number,
    otherwise *فاتورة ضريبية مبسطة* (simplified). It can be changed.
- **Lines grid:** product/service search, editable description, unit, qty, price, discount %, tax,
  net, VAT and total.
  - **Enter** moves to the next cell, **Ctrl+Enter** adds a line, **Ctrl+D** duplicates, **Ctrl+Del**
    deletes.
  - **Paste from Excel** fills rows.
  - **Free-text lines** are allowed for non-stock services and need a revenue account.
- **Footer:** invoice discount, totals per VAT category, and the **amount in words**, e.g.
  "فقط مائة وواحد وعشرون ريالاً وخمسون هللة لا غير" (helper `tafqit()` in `core/helpers`, with the
  currency's unit names).
- **Payment now (optional):** tender rows as in the POS. Otherwise the invoice stays open until its
  due date.
- **Notes / terms:** pick from saved terms (Settings → Sales) or type. Attachments are allowed.
- **Actions:** save draft, post, post & print, post & new, journal preview (the existing
  `JournalPreview` component), duplicate.
- **Quotations (عروض الأسعار):** the same form at `/sales/quotations`. Statuses: draft, sent,
  accepted, rejected, expired. **Convert → invoice** copies everything.
- **Invoice detail page:**
  - Status, allocations and payments, credit notes, the journal link, attachments and the audit trail.
  - Print with a template picker, **download PDF**, receive payment, create credit note.

## 3. Pricing, discount & VAT math (the single source of truth)

Setting `company.pricesIncludeTax` (default **true** for retail): entered prices and discounts
include VAT. Every document stores its own copy of the setting.

For each line *i*, with rate `rᵢ` from its tax (0 for Z/E/O):

1. `Lᵢ = round2(qtyᵢ × unitPriceᵢ)`
2. Line discount: `dᵢ = pct ? round2(Lᵢ × p) : amount` → `L'ᵢ = Lᵢ − dᵢ`
3. Invoice discount `H` (% of Σ L', or an amount) is **spread over the lines in proportion to `L'ᵢ`**.
   Rounding leftovers go to the lines with the largest remainders, so Σ hᵢ = H exactly →
   `L''ᵢ = L'ᵢ − hᵢ`
4. VAT per line:
   - **Exclusive:** `netᵢ = L''ᵢ`, `vatᵢ = round2(netᵢ × rᵢ)`, `grossᵢ = netᵢ + vatᵢ`
   - **Inclusive:** `grossᵢ = L''ᵢ`, `netᵢ = round2(grossᵢ / (1 + rᵢ))`, `vatᵢ = grossᵢ − netᵢ`
5. Invoice totals = Σ of the lines. `vatByCategory` groups the lines by (category, rate).

**Rules:**
- **Discounts reduce the taxable amount; VAT is never discounted by itself.** This is the "discount
  on the product vs discount on the tax" question: a discount always applies to the product price,
  and VAT is recalculated on what remains.
- **Inclusive mode:** the customer pays exactly the shelf prices minus the discounts. VAT is derived
  from that, so no halala differences appear at the till.
- **Custom price:** it replaces `unitPrice` in step 1; `listPrice` is kept only for audit. A discount
  is `listPrice − unitPrice` only in reports. Discount and price-override reports show both.
- **Purchases:** use the same steps with the purchase tax and the supplier's price mode.

**Worked example** (inclusive prices; line B is a zero-rated medicine):

| | Line A (S 15%) | Line B (Z 0%) | Total |
|---|--:|--:|--:|
| qty × price | 2 × 57.50 = 115.00 | 1 × 23.00 = 23.00 | 138.00 |
| line discount | 10% → −11.50 | — | |
| L' | 103.50 | 23.00 | 126.50 |
| invoice discount 5.00, spread | −4.09 | −0.91 | −5.00 |
| gross | 99.41 | 22.09 | **121.50** |
| net | 99.41 / 1.15 = **86.44** | 22.09 | 108.53 |
| VAT | **12.97** | 0.00 | 12.97 |

Posting (paid in cash):
- Dr cash 121.50
- Cr 4100 sales 86.44
- Cr 4120 zero-rated sales 22.09
- Cr 2150 VAT output 12.97
- plus the cost lines (COGS / inventory)

**Tests:** `totals.spec` pins the example and edge cases: 0 qty, 100% discount, mixed categories,
the proportional spread when one line is 0, and exclusive vs inclusive giving the same net for the
same effective price.

## 4. Returns → credit notes (إشعار دائن)

- **Where they start:** the invoice detail page, the POS (F7), or the invoice list's row action.
- **Picking lines:** choose lines and quantities; the remaining returnable quantity is shown.
  - **Reason:** required, from a list (عيب مصنعي، مقاس غير مناسب، رغبة العميل، خطأ في الفاتورة) or
    free text.
  - **Restock toggle per line:** default on. Off = the item is damaged, so it's written off (5120)
    instead of going back to stock.
- **Refund method:**
  - Default: against the invoice's outstanding amount first.
  - Then the rest goes to cash, the original card, a bank transfer, or **customer credit**
    (unallocated, usable on a future invoice).
- **Printing:** the credit note is titled إشعار دائن, references the original invoice number and
  date, and shows the reason and the VAT breakdown.
- **Cost:** COGS is reversed at the original line cost, re-averaging the stock value (review A1).

## 5. Shifts (الورديات) and the cash drawer

- **Setting:** "يجب فتح وردية للبيع" (a shift must be open to sell). Default **on** when the POS is
  enabled.
- **Open:** pick the branch and cash drawer, then count the opening float (optionally by denomination).
- **During the shift:**
  - Every cash tender, cash refund, pay-in, pay-out and bank drop is recorded as a shift movement.
  - The **X-report** (mid-shift) shows sales by method and the expected cash.
- **Close:**
  - The cashier counts the cash (by denomination), and the app shows expected vs counted.
  - Any variance is posted to cash over (4330) or cash short (6320).
  - **Z-report** is printed (thermal or A4).
  - The cashier chooses to *hand over to the next shift* or *drop cash to the safe/bank*, which
    creates a transfer voucher.
- **Manager screen `/pos/shifts`:** open and closed shifts with variances. A manager can force-close
  a shift and reprint a Z-report.

## 6. Invoice list v2

- **Filters:** branch, source (POS/desk), type, payment status, **overdue**, customer, cashier, date
  and amount.
- **Saved views:** "آجل متأخر" (overdue credit), "اليوم — فرعي" (today, my branch).
- **Footer:** a totals row.
- **Bulk actions:** export Excel, download PDFs as a .zip, print.
