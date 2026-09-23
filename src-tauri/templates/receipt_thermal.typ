// Thermal receipt template (Phase 14, docs/v2/12-documents-pdf-excel.md §5).
// Condensed single-column layout matching src/modules/invoices/components/
// InvoiceThermal.vue (the v1/browser-fallback HTML receipt) content-wise, but
// laid out in Typst so it can be rasterized to PNG and dithered to ESC/POS.
//
// Reads `data.json` (a `DocumentPayload`, same shape as the A4 templates —
// see src-tauri/src/pdf/payload.rs) and `opts.json` (paper width + font size
// only matter here; the receipt ignores column/footer options meant for A4).
#import "lib.typ": font-family

#let data = json("data.json")
#let opts = json("opts.json")

// `lib.typ::page-typst-size` returns `(width: .., height: auto)` for 80mm/58mm
// as a *conceptual* paper size, but Typst's `page(paper: ..)` parameter only
// accepts named paper strings (a0/a4/us-letter/...), not a width/height dict
// — that dict is meant for `page(width:, height:)` directly. Rather than
// editing shared `lib.typ` (out of this phase's scope), this template reads
// `opts.paper`/`opts.printableWidthMm` itself and sets `width`/`height`
// explicitly. The page is sized to the printer's *printable* width (set by
// `print::render::build_world`), not the nominal paper width — see
// `print::payload::ReceiptWidth`'s doc comment for why (576/384 dots is a
// printable-area convention, not the full 80mm/58mm roll width).
#let paper = opts.at("paper", default: "80mm")
#let receipt-width = opts.at("printableWidthMm", default: if paper == "58mm" { 48 } else { 72 }) * 1mm

#set document(title: data.document.number, date: datetime(year: 2026, month: 1, day: 1))
// `receipt-width` is already the printer's *printable* area (see above), so
// the page margin here is purely a small visual inset within that area, not
// an additional cut into the paper — keep it tiny so line wrapping matches
// what the print head can actually lay dots on.
#set page(width: receipt-width, height: auto, margin: (x: 1mm, y: 2mm))
#set text(font: font-family(opts), lang: "ar", region: "sa", size: 8.5pt, dir: rtl)
#set par(justify: false, leading: 0.55em)

#let hr = line(length: 100%, stroke: 0.6pt + gray)

// ---- Header: logo/name, address, phone, VAT ----
#align(center)[
  #if data.company.logo != none {
    image("logo.png", height: 1.4cm)
    v(2pt)
  }
  #text(size: 10pt, weight: "bold")[#data.company.name]
  #if data.company.address != none [
    #linebreak()
    #text(size: 8pt)[#data.company.address]
  ]
  #if data.company.phone != none [
    #linebreak()
    #text(size: 8pt)[#data.company.phone]
  ]
  #if data.company.vatNumber != none [
    #linebreak()
    #text(size: 8pt)[الرقم الضريبي: #data.company.vatNumber]
  ]
]

#v(4pt)
#hr
#v(3pt)
#align(center)[#text(weight: "bold")[#data.document.titleAr]]
#v(2pt)

// ---- Document meta ----
#table(
  columns: (auto, 1fr),
  stroke: none,
  inset: (y: 1pt, x: 0pt),
  text(size: 8pt)[رقم الفاتورة], align(right)[#text(size: 8pt)[#data.document.number]],
  text(size: 8pt)[التاريخ], align(right)[#text(size: 8pt)[#data.document.date]],
  ..if data.party != none {
    (text(size: 8pt)[العميل], align(right)[#text(size: 8pt)[#data.party.name]])
  } else { () },
)

#v(4pt)
#hr
#v(3pt)

// ---- Lines: name on its own row, qty x price ... total on the next ----
#for line in data.lines [
  #text(size: 8.5pt)[#line.name]
  #table(
    columns: (1fr, auto),
    stroke: none,
    inset: (y: 1pt, x: 0pt),
    text(size: 8pt, fill: gray)[#line.qty × #line.price], align(left)[#text(size: 8.5pt)[#line.net]],
  )
  #v(2pt)
]

#v(2pt)
#hr
#v(3pt)

// ---- Totals ----
#table(
  columns: (1fr, auto),
  stroke: none,
  inset: (y: 1pt, x: 0pt),
  text(size: 8.5pt)[المجموع], align(left)[#text(size: 8.5pt)[#data.totals.subtotal]],
  ..if data.totals.discount != none and data.totals.discount != "0.00" {
    (text(size: 8.5pt)[الخصم], align(left)[#text(size: 8.5pt)[−#data.totals.discount]])
  } else { () },
  text(size: 8.5pt)[ضريبة القيمة المضافة], align(left)[#text(size: 8.5pt)[#data.totals.vat]],
)
#v(2pt)
#line(length: 100%, stroke: 0.9pt + black)
#v(2pt)
#table(
  columns: (1fr, auto),
  stroke: none,
  inset: (y: 1pt, x: 0pt),
  text(size: 11pt, weight: "bold")[الإجمالي], align(left)[#text(size: 11pt, weight: "bold")[#data.totals.grand]],
)
#if data.totals.paid != none [
  #table(
    columns: (1fr, auto),
    stroke: none,
    inset: (y: 1pt, x: 0pt),
    text(size: 8.5pt)[المدفوع], align(left)[#text(size: 8.5pt)[#data.totals.paid]],
    text(size: 8.5pt)[المتبقي], align(left)[#text(size: 8.5pt)[#data.totals.remaining]],
  )
]

// ---- QR ----
#if data.qr != none [
  #v(6pt)
  #align(center)[#image("qr.svg", width: 2.8cm)]
]

// ---- Footer ----
#if opts.at("receiptFooter", default: none) != none [
  #v(4pt)
  #align(center)[#text(size: 8pt)[#opts.receiptFooter]]
]
#if data.at("sample", default: false) [
  #v(4pt)
  #align(center)[#text(size: 8.5pt, weight: "bold")[— اختبار طباعة —]]
]
#v(4pt)
