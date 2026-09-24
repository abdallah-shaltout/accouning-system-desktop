// Party statement (كشف حساب) — docs/v2/08-customers-and-suppliers.md §3, data shaped from
// `PartyStatementRow[]` (docs/v2/12 §3 "statement"). Reuses lib.typ's header, but has its own
// running-balance table (debit/credit/balance columns) instead of `lines-table` (which is
// invoice-shaped — qty/price/vat — and doesn't fit a ledger).
#import "lib.typ": header, footer-block, page-margin, page-typst-size, font-family, font-size, accent

#let data = json("data.json")
#let opts = json("opts.json")

#set document(title: data.document.number)
#set page(paper: page-typst-size(opts), margin: page-margin(opts))
#set text(font: font-family(opts), lang: "ar", region: "sa", size: font-size(opts), dir: rtl)
#set par(justify: false)

#header(data, opts)

// ---- Party + period ----
#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [
    #text(weight: "bold")[كشف حساب: #data.party.name]
    #if data.party.at("code", default: none) != none [
      #linebreak()
      #text(size: 8.5pt, fill: gray)[الرمز: #data.party.code]
    ]
  ],
  align(left)[
    #if data.document.at("periodFrom", default: none) != none [
      #text(size: 9pt)[من #data.document.periodFrom إلى #data.document.at("periodTo", default: "اليوم")]
    ]
  ],
)
#v(10pt)

// ---- Ledger table ----
#table(
  columns: (auto, 1fr, auto, auto, auto, auto),
  align: (x, y) => if x == 1 { right } else { center },
  stroke: 0.5pt + gray,
  inset: 6pt,
  table.header(
    repeat: true,
    text(weight: "bold", size: 8.5pt)[التاريخ],
    text(weight: "bold", size: 8.5pt)[البيان],
    text(weight: "bold", size: 8.5pt)[المرجع],
    text(weight: "bold", size: 8.5pt)[مدين],
    text(weight: "bold", size: 8.5pt)[دائن],
    text(weight: "bold", size: 8.5pt)[الرصيد],
  ),
  ..data.lines.map(l => (
    text(size: 9pt)[#l.date],
    text(size: 9pt)[#l.description],
    text(size: 9pt)[#l.number],
    text(size: 9pt)[#l.debit],
    text(size: 9pt)[#l.credit],
    text(size: 9pt, weight: "bold")[#l.balance],
  )).flatten(),
)
#v(12pt)

// ---- Closing balance ----
#align(left)[
  #rect(inset: 8pt, radius: 3pt, stroke: 0.6pt + accent(opts))[
    #text(size: 9pt)[الرصيد الختامي: #text(weight: "bold", size: 11pt)[#data.totals.grand]]
  ]
]
#v(10pt)
#footer-block(data, opts)
