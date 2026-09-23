# 07 — Products & Inventory

## 1. Product form v2 (`/catalog/products/new`, tabbed)

| Tab | Fields |
|-----|--------|
| **أساسي** (basics) | Type (stock item / service / non-stock item such as bags), name (ar/en), SKU (auto-suggested), category, brand, tags, images (drag to reorder; the first is the POS thumbnail), description, active |
| **الوحدات والباركود** (units & barcodes) | Base unit + more units: a table of unit, *contains N base units*, barcodes (several), default for sale, default for purchase. "Generate EAN-13" button (internal prefix `2`/`628…`) |
| **الأسعار** (prices) | A matrix of price list × unit. Units beyond the base default to base price × factor, marked "تلقائي" (auto) until edited. Also: minimum price (floor), average cost (read-only), margin % per cell (hover). Price-mode note: "الأسعار شاملة الضريبة" (prices include VAT) |
| **الضريبة والحسابات** (tax & accounts) | Sale tax and purchase tax (category + rate; exemption reason shown for Z/E). *Advanced:* revenue, COGS and purchase accounts, each with "افتراضي من التصنيف" (default from category) shown in grey |
| **المخزون** (stock) | Per branch: on-hand (read-only), reorder point, reorder qty. Also: allow negative stock, shelf location, preferred supplier. Tracking: batches, expiry, alert days |
| **إضافي** (additional) | Warranty (months, manufacturer/store), weight, **custom fields** (defined in Settings → Products: text, number, date, list, yes/no, e.g. "المادة الفعالة" active ingredient, "الموسم" season) |

**Account resolution:** product → category → settings default. Categories get the same optional
account and tax defaults, so most products need no per-item configuration.

## 2. Complex units (the pharmacy case)

**Rule:** stock and cost are always in the **base (smallest) unit**. Every other unit is a factor.

```text
Product: بانادول 500 — base unit: شريط (strip)
  علبة (box)   factor 3   barcode 6281…01   price 24.00   default purchase unit
  شريط (strip) factor 1   barcode 6281…02   price  8.50   default sale unit
(optional 3rd level: قرص / tablet as base, factor strip = 10, box = 30)
```

- **Purchasing:** buying 10 boxes at 60.00 → stock +30 strips, cost per strip = 20.00.
- **Selling:** 1 box → −3 strips. Selling 1 strip when only whole boxes are on the shelf is fine,
  because stock is in strips (the box is "opened" implicitly). An optional setting logs a "فتح علبة"
  (box opened) event for pharmacies that want it.
- **Display:** quantities show as mixed units, "4 علبة + 2 شريط", with a toggle to base units. Stock
  reports have a unit selector.
- **Price per unit:** set explicitly. It doesn't have to be price × factor, because boxes are often
  cheaper per strip.
- **Validation:** factors are positive integers (decimals allowed for weight units). Exactly one unit
  has factor 1. The factor can't change once stock has moved; instead, add a new unit and deactivate
  the old one.

**Unit master (Settings → Catalog):** name, symbol, allows decimals (kg, m).
**Presets by business type:** pharmacy (علبة، شريط، قرص، زجاجة، أمبول), clothing (قطعة، طقم،
درزن), supermarket (حبة، كرتون، كيلو، جرام، لتر).

## 3. Batches & expiry (optional per product)

- **Receiving:** each purchase line or opening-stock line asks for a batch number and expiry date.
  Several batches per line are allowed.
- **Selling:** FEFO allocation (first expiry, first out). An expired batch can't be sold. A batch
  within `expiryAlertDays` shows an amber chip in the POS.
- **Stock views:** per batch. The product page has a "التشغيلات" (batches) tab.
- **Expiry report:** expired / ≤30 / ≤60 / ≤90 days, grouped by supplier. Actions: *return to
  supplier* (creates a debit-note draft) or *write off* (a write-off with reason `expired`).

## 4. Branch stock & transfers

- **Stock:** each product has a quantity per branch. The stock value is company-wide at average
  cost (decision 7). Lists show *this branch* by default, with a toggle "كل الفروع" (all branches)
  that adds a column per branch.
- **Transfer (`/inventory/transfers`):**
  1. **Draft:** from-branch, to-branch, lines (unit + qty; batch for tracked products).
  2. **Send:** the stock leaves the source (Dr in-transit / Cr inventory) and the destination gets a
     notification and an insight.
  3. **Receive:** the destination confirms the received quantity per line.
     - **Shortage** → inventory variance.
     - **Receive all** is the one-click path.
  4. **Reject:** the stock returns to the source.
- **Print:** a transfer note (A4/thermal) with a QR code that opens the transfer, for the driver.

## 5. Stock operations

| Screen | Who | Notes |
|--------|-----|-------|
| **إدخال مخزون** (stock-in) | storekeeper, manager | A reason is required (review A3); unit cost defaults to the average cost |
| **إتلاف / فاقد** (write-off / loss) | storekeeper → approval | Reasons: damaged, expired, theft, sample. Above a value threshold, the entry stays *pending* until a manager approves it |
| **الجرد** (stocktake, v2) | storekeeper | Scope: all, or chosen categories / shelf / products. The system quantity is snapshotted at count time. Count by scanning (each scan adds 1, or type the qty). *Blind count* option hides the system quantities. Review screen with variances and their value → approve |
| **حركة المخزون** (stock movements) | all with inventory access | Filters: branch, product, batch, reason, date, user. Columns include the value change. Export Excel |

## 6. Barcode / QR label printing (`/catalog/labels`)

- **Entry points:**
  - The product list (bulk select), which opens the label builder.
  - The product page: "طباعة ملصقات" (print labels).
  - A purchase or stock-in: "طباعة ملصقات للكميات المستلمة" (labels for the received quantities),
    the storekeeper's most common path.
- **Builder:** a table of product · unit · batch · number of copies.
  - **Copies:** "نسخة لكل قطعة في المخزون" (one per unit in stock, for the current branch), "عدد
    ثابت" (a fixed number), or "حسب الكمية المستلمة" (the received quantity).
- **Label templates:**
  - **Sheets:** A4 sheets (e.g. 3×8 at 70×37 mm, 4×10 at 48.5×25.4 mm) with a starting cell, so a
    half-used sheet can be reused.
  - **Thermal label printers:** one label per page (40×25, 50×30, 58×40 mm).
  - **Fields** (toggle and position): store name, product name, price (inclusive, with the currency
    symbol), unit, barcode (EAN-13 / Code-128), **QR** (encodes the barcode, or a URL/JSON the user
    picks), SKU, batch + expiry, size/color (custom fields).
- **Rendering:**
  - Barcodes are made by `bwip-js` as SVG and passed to the Typst label template. The output is a
    real PDF (see [12](12-documents-pdf-excel.md)).
  - **Preview:** the first page is drawn live.
  - **Why PDF:** exact sizes, so the labels line up with the label stock.

## 7. Variants (clothing), *proposed, later phase*

A clothing shop needs size × color. The plan is a **product template with attributes** that
generates child products (each with its own SKU, barcode and stock) and a grid entry form
(sizes across, colors down). It isn't in the phases of [15](15-action-plan.md) yet. It's listed so
the product model (§1) leaves room for `parentId` and `attributes`.
