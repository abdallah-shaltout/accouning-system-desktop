// Transfer note (إذن تحويل) — docs/v2/07-products-and-inventory.md §4, Phase 9 stock-transfer
// data. No QR/party-box (party is a branch-to-branch move, shown as a from/to grid instead).
#import "lib.typ": header, lines-table, footer-block, page-margin, page-typst-size, font-family, font-size

#let data = json("data.json")
#let opts = json("opts.json")

#set document(title: data.document.number)
#set page(paper: page-typst-size(opts), margin: page-margin(opts))
#set text(font: font-family(opts), lang: "ar", region: "sa", size: font-size(opts), dir: rtl)
#set par(justify: false)

#header(data, opts)

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [
    #text(size: 8.5pt, fill: gray)[من فرع]
    #v(2pt)
    #text(size: 10pt, weight: "bold")[#data.document.at("fromBranch", default: "—")]
  ],
  [
    #text(size: 8.5pt, fill: gray)[إلى فرع]
    #v(2pt)
    #text(size: 10pt, weight: "bold")[#data.document.at("toBranch", default: "—")]
  ],
)
#v(8pt)
#grid(
  columns: (auto, auto, 1fr, auto, auto),
  gutter: 6pt,
  text(size: 8.5pt, fill: gray)[رقم الإذن], text(size: 9pt, weight: "bold")[#data.document.number], [],
  text(size: 8.5pt, fill: gray)[التاريخ], text(size: 9pt)[#data.document.date],
)
#v(10pt)

#lines-table(data, opts)
#v(10pt)
#if data.document.at("note", default: none) != none [
  #text(size: 8.5pt, fill: gray)[ملاحظات: #data.document.note]
  #v(6pt)
]
#footer-block(data, opts)
