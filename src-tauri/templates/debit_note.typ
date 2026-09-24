// Debit note (إشعار مدين) — purchase return, references the original purchase order.
// Same shape as credit_note.typ (`document.refNumber`/`refDate`/`reason`), no QR (purchasing
// document, never ZATCA-relevant).
#import "lib.typ": header, party-box, lines-table, totals-block, footer-block, page-margin, page-typst-size, font-family, font-size, accent

#let data = json("data.json")
#let opts = json("opts.json")

#set document(title: data.document.number)
#set page(paper: page-typst-size(opts), margin: page-margin(opts))
#set text(font: font-family(opts), lang: "ar", region: "sa", size: font-size(opts), dir: rtl)
#set par(justify: false)

#header(data, opts)
#party-box(data, opts)

#if data.document.at("refNumber", default: none) != none [
  #rect(width: 100%, inset: 8pt, radius: 3pt, stroke: 0.6pt + accent(opts))[
    #text(size: 9pt)[
      مرجع أمر الشراء الأصلي: #text(weight: "bold")[#data.document.refNumber]
      #if data.document.at("refDate", default: none) != none [ بتاريخ #data.document.refDate ]
      #if data.document.at("reason", default: none) != none [ — السبب: #data.document.reason ]
    ]
  ]
  #v(8pt)
]

#lines-table(data, opts)
#v(12pt)
#totals-block(data, opts)
#v(10pt)
#footer-block(data, opts)
