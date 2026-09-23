// Typst PDF spike template (Phase 0 gate, docs/v2/12-documents-pdf-excel.md §1).
// Fixed template with hardcoded/sample data — NOT the production templating system
// from §2 (that is Phase 11's job). Demonstrates, in one document:
//   - Cairo (Latin/UI) + Noto Naskh Arabic (Arabic text) fonts, embedded.
//   - Mixed Arabic, English and digits in table cells.
//   - A long product name that wraps across multiple lines in its cell.
//   - A table header that repeats when the table spans page 2 (15-20+ rows).
//   - Amount-in-words (tafqit) hardcoded sample text.
//   - A QR code image embedded on the page.

#set document(title: "Tax Invoice / فاتورة ضريبية — Typst spike", date: datetime(year: 2026, month: 9, day: 23))
#set page(paper: "a4", margin: (x: 1.8cm, y: 1.8cm))
#set text(font: "Cairo", lang: "ar", region: "sa", size: 10pt, dir: rtl)
#set par(justify: false)

#let data = json("data.json")

// ---- Header -----------------------------------------------------------
#align(center)[
  #text(size: 18pt, weight: "bold")[فاتورة ضريبية / Tax Invoice]
  #v(2pt)
  #text(size: 10pt, font: "Noto Naskh Arabic")[شركة الفاتورة النموذجية للتجارة]
]

#v(8pt)
#line(length: 100%, stroke: 0.5pt + gray)
#v(8pt)

// ---- Party box ----------------------------------------------------------
#grid(
  columns: (1fr, 1fr),
  gutter: 12pt,
  [
    #text(weight: "bold")[البائع / Seller]
    #v(2pt)
    #text(font: "Noto Naskh Arabic")[متجر الفاتورة النموذجي]
    #linebreak()
    الرقم الضريبي: ٣١١١١١١١١١١١١١٣
  ],
  [
    #text(weight: "bold")[المشتري / Buyer]
    #v(2pt)
    #text(font: "Noto Naskh Arabic")[#data.party.name]
    #linebreak()
    SKU account: #data.party.code
  ],
)

#v(10pt)

// ---- Line items table (spans 2 pages; header repeats via `table.header`)
#table(
  columns: (2.2fr, 1.6fr, 1fr, 1fr, 1fr, 1.2fr),
  align: (right, left, center, center, center, left),
  stroke: 0.5pt + gray,
  inset: 6pt,
  table.header(
    repeat: true,
    [*الصنف / Item*], [*SKU*], [*الكمية / Qty*], [*السعر / Price*], [*الضريبة / VAT*], [*الإجمالي / Total*],
  ),
  ..data.lines.map(line => (
    text(font: "Noto Naskh Arabic")[#line.name_ar],
    line.sku,
    line.qty,
    line.price,
    line.vat,
    line.total,
  )).flatten()
)

#v(14pt)

// ---- Totals + amount in words (tafqit) ---------------------------------
#grid(
  columns: (1fr, 1fr),
  gutter: 12pt,
  [
    #text(weight: "bold")[المبلغ كتابة / Amount in words]
    #v(4pt)
    #text(font: "Noto Naskh Arabic", size: 9.5pt)[
      فقط لا غير: ثلاثة آلاف وأربعمائة وخمسة وعشرون ريالاً سعودياً وخمسة وسبعون هللة، مع رمز المنتج ABC-1234 وكمية ١٥ قطعة.
    ]
  ],
  [
    #align(left)[
      #table(
        columns: (auto, auto),
        stroke: none,
        inset: 4pt,
        [الإجمالي قبل الضريبة], [#data.totals.subtotal],
        [ضريبة القيمة المضافة], [#data.totals.vat],
        [*الإجمالي*], [*#data.totals.grand*],
      )
    ]
  ],
)

#v(10pt)

// ---- QR code -------------------------------------------------------------
#align(center)[
  #image("qr.png", width: 3cm)
  #v(2pt)
  #text(size: 8pt, fill: gray)[QR — ZATCA placeholder, spike only]
]

#v(1fr)
#align(center)[#text(size: 8pt, fill: gray)[صفحة #context counter(page).display() من #context counter(page).final().at(0)]]
