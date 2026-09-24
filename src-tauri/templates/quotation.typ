// Quotation (عرض سعر) — docs/v2/12-documents-pdf-excel.md §3 document kinds table.
// Same shape as an invoice's DocumentPayload but with no QR/tax-invoice framing; reuses
// lib.typ's header/party-box/lines-table/footer, skips totals' "paid/remaining" (a quotation
// isn't a payable document) and always skips the QR block regardless of data.qr.
#import "lib.typ": header, party-box, lines-table, totals-block, footer-block, page-margin, page-typst-size, font-family, font-size

#let data = json("data.json")
#let opts = json("opts.json")

#set document(title: data.document.number)
#set page(paper: page-typst-size(opts), margin: page-margin(opts))
#set text(font: font-family(opts), lang: "ar", region: "sa", size: font-size(opts), dir: rtl)
#set par(justify: false)

#header(data, opts)
#party-box(data, opts)
#lines-table(data, opts)
#v(12pt)
#totals-block(data, opts)
#v(10pt)
#if data.document.at("validUntil", default: none) != none [
  #text(size: 9pt, fill: gray)[هذا العرض ساري حتى: #data.document.validUntil]
  #v(6pt)
]
#footer-block(data, opts)
