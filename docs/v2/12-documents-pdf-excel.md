# 12 — Documents: Real PDF Engine, Template Designer, Labels, Thermal, Excel

## 1. The PDF engine decision

v1's "PDF" is the browser print dialog: the user picks "Save as PDF" and the app never gets a
file. v2 needs a real generator: the app produces the file, saves it and attaches it, and the
template can be customized properly.

| Option | Arabic shaping & RTL | Customization | Output quality | Verdict |
|--------|---------------------|---------------|----------------|---------|
| jsPDF / pdfmake / pdf-lib (JS) | Weak: needs reshaping hacks, bidi breaks in tables | Code | OK | ✗ |
| Browser print (v1) | Good | CSS | The user drives the dialog; no file for the app | ✗ (the "fake" one) |
| Headless Chrome from Rust | Good | CSS | Excellent | ✗ needs Chrome installed or bundled (very large) |
| WebView2 `PrintToPdf` | Good (reuses the HTML templates) | CSS | Good | Windows only → **fallback** |
| **Typst, embedded in Rust** | Good: real shaping + bidi (`set text(lang: "ar")`) | Templates as code + JSON options, and a designer on top | Exact, deterministic, fast | **✓ chosen** |

**Why Typst:**
- **Arabic:** it's a typesetting engine with proper text shaping. Arabic works with `lang: "ar"`.
- **Rust library:** it can be embedded in Tauri (the `typst` + `typst-pdf` crates, or the
  `typst-as-lib` wrapper). No external program is needed, and fonts are bundled, so output is
  identical on every machine.
- **Previews:** it renders SVG/PNG pages for live previews and PNG for thermal printers.
- **ZATCA Phase 2:** it supports **PDF/A-3 with embedded file attachments**. That's the format for
  e-invoices (a PDF/A-3 with the XML inside), so it covers a future need.
- **Known gap:** Arabic *kashida justification* isn't supported. It doesn't matter for invoices,
  which don't justify Arabic text.

**Gate before building on it (spike, ~1 day):** render a two-page invoice with:
- Cairo and Noto Naskh fonts.
- Mixed Arabic, English and digits in table cells.
- A long product name that wraps, and the table header repeated on page 2.
- The amount in words, the QR image and a PDF/A-3b export.

**Pass** = correct shaping and order, rendering in under 200 ms, and the file opens in Acrobat and
Edge. **If it fails**, switch to WebView2 `PrintToPdf` with the existing HTML templates (Windows only).

## 2. Architecture

```text
Vue page ──► pdfService.render(kind, id, templateId?)          (modules/core/services/pdfService.ts)
               │ builds a DocumentPayload JSON (company, branch, party, lines, totals, qr svg, logo…)
               ▼
        invoke('render_pdf', { templateSource | templateId, payload, assets })   (Tauri command, Rust)
               │ typst World: virtual files data.json, logo.png, qr.svg, fonts (embedded)
               ▼
        PDF bytes ─► save dialog + fs write (binary) · open in the system viewer · attach to the document
        invoke('render_preview', …, format: 'svg') ─► PdfPreview.vue (page list, zoom, page nav)
```

- **Rust side:** `src-tauri/src/pdf/`:
  - Commands `render_pdf` and `render_preview`.
  - Templates in `src-tauri/templates/*.typ` with a shared `lib.typ` for the header, party box, lines
    table, totals, QR and footer.
  - Fonts embedded with `include_bytes!`: Cairo, IBM Plex Sans Arabic, Tajawal, Noto Naskh Arabic
    and a Latin companion.
  - Custom templates are passed as source strings.
- **Data in:** the template reads `json("data.json")`. Everything is prepared in TS: numbers
  formatted as the user's settings say, the amount in words, and the QR as SVG (existing `zatcaQr.ts`
  + `uqr`). The template only lays out. **No Typst packages**: everything must work offline.
- **Capabilities:** add `fs:allow-write-file` (binary) and `opener:allow-open-path`.
- **Browser dev mode** (no Tauri): `pdfService` falls back to the v1 print route with a notice:
  "ملف PDF الفعلي متاح في نسخة سطح المكتب" (the real PDF file is available in the desktop app).
  So `bun run dev` keeps working.
- **This is the only Rust work in v2.** It's a local rendering module, not a backend.

## 3. Templates & the designer (Settings → قوالب الطباعة)

**Document kinds:**
- Sales: tax invoice (standard), simplified invoice, quotation, credit note.
- Purchasing: purchase order, debit note.
- Vouchers and ledgers: receipt/payment voucher, journal voucher, party statement.
- Stock and shifts: transfer note, Z-report, labels.
- **Reports:** a generic report template (title, filter line, table with a repeating header, totals,
  page x of y).

**Base layouts:** كلاسيكي (classic; boxed table, header band), عصري (modern; large logo, accent
bar), مضغوط (compact; dense, many lines), ثنائي اللغة (bilingual; Arabic + English labels),
حراري (thermal; 58/80 mm), ملصقات (labels).

**TemplateOptions** (all shown in the designer):
- **Brand:**
  - **Logo:** upload + crop; position start/center/end; size S/M/L.
  - **Look:** accent color (presets + hex), font family and size.
  - **Stamp and signature:** image uploads.
- **Header:** toggles for company name, branch address (instead of the company's), VAT number, CR,
  phones, email and website; the document title in Arabic/English.
- **Party box:** which buyer fields to show; for standard invoices the buyer's VAT number and address
  are required.
- **Columns:** # · SKU · barcode · description · unit · qty · price · discount · net · VAT % · VAT ·
  total. Toggle, reorder (drag), relabel.
- **Totals:**
  - VAT by category, amount in words, paid/remaining.
  - **Previous balance / current balance** for credit customers, a common request in the region.
- **Footer:** terms (rich text), bank details (IBAN), signature lines (المستلم / المحاسب / المدير),
  thank-you line, page numbers.
- **QR:** position and size. It's **locked on** for VAT-registered SA simplified invoices.
- **Watermark:** مسودة (draft), مدفوعة (paid), نسخة (copy) on reprints.
- **Paper:** A4 / A5 / Letter / 80 mm / 58 mm / custom label size; margins; orientation.

**Designer page** (`/settings/templates/:id`):
- **Live preview:** SVG pages, re-rendered 300 ms after changes stop. Uses sample data or a real
  document chosen from a picker.
- **Options panel:** grouped accordions, as listed above.
- **Top bar:** name, document kind, *set as default* (per branch), duplicate, reset, **export/import
  template (.json)** so templates can move between installs.
- **Advanced tab (admin):** the Typst source for the template, with inline compile errors
  (line + message) and "العودة للقالب المولّد" (back to the generated template).
  - **Preserved options:** editing the source keeps the brand and paper options as variables, so
    logo and color changes still apply.
- **Everyday use:** documents print with the default template. The print button has a dropdown to
  pick another template.

## 4. Labels

Label templates are Typst too (sheet grid or one-per-page), with barcodes from `bwip-js` (SVG).
The builder UI and copy rules are in [07 §6](07-products-and-inventory.md).

## 5. Thermal receipts: fast & silent

Printing a receipt through a dialog is slow at the counter.

- **Rendering:** Typst → PNG at the printer's resolution (80 mm = 576 dots, 58 mm = 384 dots,
  203 dpi) → 1-bit dithering → **ESC/POS raster** (`GS v 0`) → paper cut → optional **cash-drawer
  kick** on cash sales.
- **Why raster:** Arabic prints perfectly, because it's an image, whatever code pages the printer
  supports.
- **Transport:**
  - A Windows printer queue in raw mode (`windows` crate: `OpenPrinter`/`WritePrinter`).
  - Or a network printer (`IP:9100`).
- **Settings → الطابعات (printers):**
  - Receipt printer (from `list_printers`), connection, width, dpi, cut, open drawer, copies.
  - A4 printer (system default), label printer.
  - **Test print** buttons.
- **Speed and fallback:** printing is asynchronous and never blocks the next sale. On failure: a
  toast + "إعادة الطباعة" (reprint) + the PDF fallback.
- **Mock phase:** the settings UI and a browser fallback (the v1 thermal HTML route) come first. The
  native transport is its own phase in [15](15-action-plan.md).

## 6. Excel

**Export:** `exceljs`, lazy-loaded, through one helper:
```ts
exportXlsx({ fileName, meta: { company, title, period, filters, user, generatedAt },
  sheets: [{ name, columns: [{ key, label, type: 'text'|'money'|'qty'|'date'|'percent', width? }],
             rows, totals?: 'sum' | Record<string, 'sum'|'avg'> }] })
```
- **Layout:**
  - The sheet is **right-to-left**, with a header block (company, title, period, filters) above the
    table.
  - The header row is frozen and bold, with a fill and an auto-filter.
- **Values:**
  - **Values are real numbers and dates with number formats**, never pre-formatted strings.
  - **The totals row uses SUM formulas**, so accountants can check it in Excel.
- **Printing:** column widths are set, landscape is used for wide sheets, and the print scales to
  fit the page width.
- **Where:**
  - Every `DataTable` gets an "تصدير" (export) menu: Excel / CSV. It exports **all rows matching the
    current filters**, not just the visible page, so services accept `{ all: true }`.
  - Every report, every statement, the journal (entries or entries with lines), and stock movements.
  - **Round-trip:** the product and price-list exports use the import-template columns, so a user can
    export, edit prices in Excel and import back ("تحديث الأسعار" / update prices).

**Import:** the shared wizard in [05 §5](05-onboarding.md).
