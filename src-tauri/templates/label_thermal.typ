// Thermal label (ملصق حراري) — one label per page, for dedicated label printers (40×25, 50×30,
// 58×40 mm — docs/v2/07-products-and-inventory.md §6 "Thermal label printers"). Same `data.labels`
// item shape as label_sheet.typ, but each item gets its own page sized exactly to the label.
#let data = json("data.json")
#let opts = json("opts.json")

#let lbl = opts.at("label", default: (:))
#let cell-w = lbl.at("widthMm", default: 50) * 1mm
#let cell-h = lbl.at("heightMm", default: 30) * 1mm

#let show-store = lbl.at("showStoreName", default: true)
#let show-price = lbl.at("showPrice", default: true)
#let show-sku = lbl.at("showSku", default: false)
#let show-batch = lbl.at("showBatch", default: false)
#let show-barcode = lbl.at("showBarcode", default: true)
#let show-qr = lbl.at("showQr", default: false)

#set document(title: "ملصق")
#set page(width: cell-w, height: cell-h, margin: 2mm)
#set text(font: opts.at("fontFamily", default: "Cairo"), lang: "ar", region: "sa", size: 8pt, dir: rtl)
#set par(justify: false, leading: 0.4em)

#for (i, item) in data.labels.enumerate() [
  #align(center)[
    #if show-store and data.company.at("name", default: none) != none [
      #text(size: 6.5pt, fill: gray)[#data.company.name]
    ]
    #text(size: 9pt, weight: "bold")[#item.name]
    #if show-sku and item.at("sku", default: none) != none [
      #linebreak()
      #text(size: 6.5pt, fill: gray)[#item.sku]
    ]
    #if show-batch and item.at("batchNo", default: none) != none [
      #linebreak()
      #text(size: 6.5pt, fill: gray)[
        #item.batchNo
        #if item.at("expiryText", default: none) != none [ — صلاحية #item.expiryText ]
      ]
    ]
    #if show-price and item.at("priceText", default: none) != none [
      #v(2pt)
      #text(size: 11pt, weight: "bold")[#item.priceText]
    ]
    #if show-barcode and item.at("barcodeSvg", default: none) != none [
      #v(2pt)
      #image(bytes(item.barcodeSvg), format: "svg", width: 85%)
    ]
    #if show-qr and item.at("qrSvg", default: none) != none [
      #v(2pt)
      #image(bytes(item.qrSvg), format: "svg", width: 1.6cm)
    ]
  ]
  #if i < data.labels.len() - 1 { pagebreak() }
]
