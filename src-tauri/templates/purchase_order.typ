// Purchase order (أمر شراء) — docs/v2/09-purchases-payments-expenses.md, printed via "إرسال
// للمورد". party-box's "المشتري"/"البائع" labels read fine here (we are the buyer; the supplier
// in `data.party` is who receives the PO), no QR.
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
#footer-block(data, opts)
