// Generic report template (title + filter line + repeating-header table + totals + page x of y) —
// docs/v2/12-documents-pdf-excel.md §3 "Reports: a generic report template". Superseded for the
// reports module by `report.typ` (the official layout: letterhead, meta strip, sectioned tables,
// signatures), which every report page now prints through; kept for `pdfService.renderGenericReport`.
//
// Expected `data` shape (a DocumentPayload — see src-tauri/src/pdf/payload.rs):
//   document: { kind: "report", number: "", date, titleAr, titleEn, filterLine? }
//   company:  { name, ... } (same as other documents, for the header)
//   party:    null (reports have no single counterparty)
//   lines:    an array of row objects; opts.columns picks which keys show, same mechanism as
//             lines-table's `key`/`label`/`visible` — a report just supplies its own column list
//             via opts.columns (arbitrary keys, not the invoice-only key set).
//   totals:   { grand: "..." } — shown as a closing totals line when present.
#import "lib.typ": header, footer-block, page-margin, page-typst-size, font-family, font-size, accent

#let data = json("data.json")
#let opts = json("opts.json")

#set document(title: data.document.at("titleAr", default: "تقرير"))
#set page(paper: page-typst-size(opts), margin: page-margin(opts))
#set text(font: font-family(opts), lang: "ar", region: "sa", size: font-size(opts), dir: rtl)
#set par(justify: false)

#header(data, opts)

#if data.document.at("filterLine", default: none) != none [
  #text(size: 8.5pt, fill: gray)[#data.document.filterLine]
  #v(10pt)
]

// Report rows are arbitrary key/value objects (not the fixed invoice column set) — opts.columns
// supplies { key, label, visible } same as lines-table, but cell values are read straight off the
// row by key rather than through lines-table's invoice-specific `cell-for`.
#let cols = opts.at("columns", default: ()).filter(c => c.at("visible", default: true))
#let cols = if cols.len() > 0 { cols } else {
  if data.lines.len() > 0 {
    data.lines.at(0).keys().map(k => (key: k, label: k))
  } else { () }
}

#if cols.len() > 0 [
  #table(
    columns: cols.map(_ => 1fr),
    align: center,
    stroke: 0.5pt + gray,
    inset: 6pt,
    table.header(
      repeat: true,
      ..cols.map(c => text(weight: "bold", size: 8.5pt)[#c.label]),
    ),
    ..data.lines.map(row => cols.map(c => text(size: 9pt)[#row.at(c.key, default: "")])).flatten(),
  )
]
#v(10pt)
#if data.totals.at("grand", default: none) != none [
  #align(left)[
    #rect(inset: 8pt, radius: 3pt, stroke: 0.6pt + accent(opts))[
      #text(size: 9pt)[الإجمالي: #text(weight: "bold", size: 11pt)[#data.totals.grand]]
    ]
  ]
  #v(10pt)
]
#footer-block(data, opts)
