// Label sheet (ملصقات — A4 sheet grid) — docs/v2/07-products-and-inventory.md §6 and
// docs/v2/12-documents-pdf-excel.md §3/§4. Renders `data.labels` (one entry per physical label,
// already expanded to "one entry per copy" in TS — see labelService.ts's `buildLabelPayload`) in
// a grid sized by `opts.label` (cell width/height, columns, rows, starting cell, gutters), on an
// A4 page, so a half-used adhesive sheet can be reused by skipping the first `startCell - 1` cells.
//
// Each `data.labels[i]` item: { name, priceText?, sku?, unit?, batchNo?, expiryText?,
//   barcodeSvg? (raw <svg> markup from bwip-js), qrSvg? (raw <svg> markup, uqr — same convention
//   as data.qr elsewhere: pre-rendered SVG markup, not raw bytes) }.
// `opts.label`: { widthMm, heightMm, cols, rows, marginTopMm, marginLeftMm, gutterXMm, gutterYMm,
//   startCell (1-based, row-major), showStoreName, showPrice, showSku, showBatch, showBarcode, showQr }.

#let data = json("data.json")
#let opts = json("opts.json")

#let lbl = opts.at("label", default: (:))
#let cell-w = lbl.at("widthMm", default: 70) * 1mm
#let cell-h = lbl.at("heightMm", default: 37) * 1mm
#let cols = lbl.at("cols", default: 3)
#let rows = lbl.at("rows", default: 8)
#let margin-top = lbl.at("marginTopMm", default: 10) * 1mm
#let margin-left = lbl.at("marginLeftMm", default: 8) * 1mm
#let gutter-x = lbl.at("gutterXMm", default: 2) * 1mm
#let gutter-y = lbl.at("gutterYMm", default: 0) * 1mm
#let start-cell = lbl.at("startCell", default: 1)

#let show-store = lbl.at("showStoreName", default: true)
#let show-price = lbl.at("showPrice", default: true)
#let show-sku = lbl.at("showSku", default: false)
#let show-batch = lbl.at("showBatch", default: false)
#let show-barcode = lbl.at("showBarcode", default: true)
#let show-qr = lbl.at("showQr", default: false)

#set document(title: "ملصقات")
#set page(paper: "a4", margin: (top: margin-top, left: margin-left, right: margin-left, bottom: margin-top))
#set text(font: opts.at("fontFamily", default: "Cairo"), lang: "ar", region: "sa", size: 7pt, dir: rtl)
#set par(justify: false, leading: 0.4em)

#let one-label(item) = box(width: cell-w, height: cell-h, inset: 3pt, stroke: 0.3pt + gray)[
  #if item == none {
    // Empty placeholder cell (used to skip already-used cells on a half sheet).
  } else {
    align(center)[
      #if show-store and data.company.at("name", default: none) != none [
        #text(size: 6pt, fill: gray)[#data.company.name]
      ]
      #text(size: 7.5pt, weight: "bold")[#item.name]
      #if show-sku and item.at("sku", default: none) != none [
        #linebreak()
        #text(size: 6pt, fill: gray)[#item.sku]
      ]
      #if show-batch and item.at("batchNo", default: none) != none [
        #linebreak()
        #text(size: 6pt, fill: gray)[
          #item.batchNo
          #if item.at("expiryText", default: none) != none [ — صلاحية #item.expiryText ]
        ]
      ]
      #if show-price and item.at("priceText", default: none) != none [
        #v(1pt)
        #text(size: 9pt, weight: "bold")[#item.priceText]
      ]
      #if show-barcode and item.at("barcodeSvg", default: none) != none [
        #v(1pt)
        #image(bytes(item.barcodeSvg), format: "svg", width: 90%)
      ]
      #if show-qr and item.at("qrSvg", default: none) != none [
        #v(1pt)
        #image(bytes(item.qrSvg), format: "svg", width: 1.4cm)
      ]
    ]
  }
]

// Build a flat cell list: `start-cell - 1` empty placeholders, then the real labels, padded out
// to a full number of rows so the grid always renders complete rows.
#let placeholders = range(0, calc.max(0, start-cell - 1)).map(_ => none)
#let cells = placeholders + data.labels
#let per-page = cols * rows
#let total-pages = calc.ceil(cells.len() / per-page)
#let padded = cells + range(0, calc.max(0, total-pages * per-page - cells.len())).map(_ => none)

#for page-idx in range(0, calc.max(1, total-pages)) [
  #let page-cells = padded.slice(page-idx * per-page, calc.min((page-idx + 1) * per-page, padded.len()))
  #grid(
    columns: (cell-w,) * cols,
    rows: (cell-h,) * rows,
    column-gutter: gutter-x,
    row-gutter: gutter-y,
    ..page-cells.map(one-label),
  )
  #if page-idx < total-pages - 1 { pagebreak() }
]
