// Z-report (تقرير Z, shift close) — docs/v2/06-sales-and-pos.md §5. Data shaped from Phase 7's
// `Shift`/`ShiftRow` (opening float, movements broken into sales/refunds/pay-in/pay-out/bank-drop,
// counted vs expected cash, over/short). Own layout — no party box, a two-column key/value grid
// instead of an invoice-shaped table.
#import "lib.typ": header, footer-block, page-margin, page-typst-size, font-family, font-size, accent

#let data = json("data.json")
#let opts = json("opts.json")

#set document(title: data.document.number)
#set page(paper: page-typst-size(opts), margin: page-margin(opts))
#set text(font: font-family(opts), lang: "ar", region: "sa", size: font-size(opts), dir: rtl)
#set par(justify: false)

#header(data, opts)

#grid(
  columns: (auto, auto, 1fr, auto, auto),
  gutter: 6pt,
  text(size: 8.5pt, fill: gray)[رقم الوردية], text(size: 9pt, weight: "bold")[#data.document.number], [],
  text(size: 8.5pt, fill: gray)[الكاشير], text(size: 9pt)[#data.document.at("cashierName", default: "—")],
)
#v(4pt)
#text(size: 8.5pt, fill: gray)[فُتحت: #data.document.at("openedAt", default: "—")   أُغلقت: #data.document.at("closedAt", default: "—")]
#v(12pt)

#let row(label, value, bold: false) = (
  text(size: 9pt, fill: gray)[#label],
  if bold { text(size: 10pt, weight: "bold")[#value] } else { text(size: 9pt)[#value] },
)

#table(
  columns: (1fr, auto),
  stroke: none,
  inset: 5pt,
  ..row("الرصيد الافتتاحي", data.totals.at("openingFloat", default: "0.00")),
  ..row("مبيعات نقدية", data.totals.at("cashSales", default: "0.00")),
  ..row("مرتجعات نقدية", data.totals.at("cashRefunds", default: "0.00")),
  ..row("إيداعات (Pay-in)", data.totals.at("payIns", default: "0.00")),
  ..row("سحوبات (Pay-out)", data.totals.at("payOuts", default: "0.00")),
  ..row("إنزال للبنك (Bank drop)", data.totals.at("bankDrops", default: "0.00")),
  ..row("النقدية المتوقعة", data.totals.at("expectedCash", default: "0.00"), bold: true),
  ..row("النقدية المعدودة", data.totals.at("countedCash", default: "0.00"), bold: true),
  ..row("الفرق (زيادة/عجز)", data.totals.grand, bold: true),
)
#v(10pt)

#if data.lines.len() > 0 [
  #text(weight: "bold", size: 9.5pt)[تفاصيل الحركات]
  #v(6pt)
  #table(
    columns: (auto, 1fr, auto, auto),
    align: (x, y) => if x == 1 { right } else { center },
    stroke: 0.5pt + gray,
    inset: 5pt,
    table.header(
      repeat: true,
      text(weight: "bold", size: 8.5pt)[الوقت],
      text(weight: "bold", size: 8.5pt)[النوع],
      text(weight: "bold", size: 8.5pt)[المرجع],
      text(weight: "bold", size: 8.5pt)[المبلغ],
    ),
    ..data.lines.map(l => (
      text(size: 8.5pt)[#l.at("time", default: "")],
      text(size: 8.5pt)[#l.at("kind", default: "")],
      text(size: 8.5pt)[#l.at("ref", default: "")],
      text(size: 8.5pt)[#l.at("amount", default: "")],
    )).flatten(),
  )
]
#v(10pt)
#footer-block(data, opts)
