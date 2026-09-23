// Shared layout building blocks for invoice templates (Phase 11a).
// docs/v2/12-documents-pdf-excel.md §2/§3: "a shared lib.typ for the header,
// party box, lines table, totals, QR and footer".
//
// Every function reads from the `data` dict shaped like `DocumentPayload`
// (see src-tauri/src/pdf/payload.rs) and the `opts` dict shaped like
// `TemplateOptions` (see src-tauri/src/pdf/mod.rs). Both are passed in
// explicitly rather than read from a global so the same functions work from
// custom (advanced-tab) templates too.

#let accent(opts) = rgb(opts.at("accentColor", default: "#4f46e5"))

#let logo-align(opts) = {
  let pos = opts.at("logoPosition", default: "start")
  if pos == "center" { center } else if pos == "end" { end } else { start }
}

#let logo-size(opts) = {
  let s = opts.at("logoSize", default: "m")
  if s == "s" { 1.4cm } else if s == "l" { 2.6cm } else { 2cm }
}

// ---- Header --------------------------------------------------------------
#let header(data, opts) = {
  let h = opts.at("header", default: (:))
  let showName = h.at("showCompanyName", default: true)
  let showAddr = h.at("showAddress", default: true)
  let showVat = h.at("showVatNumber", default: true)
  let showCr = h.at("showCommercialRegister", default: true)
  let showPhone = h.at("showPhone", default: true)
  let showEmail = h.at("showEmail", default: false)
  let showWebsite = h.at("showWebsite", default: false)
  let title = h.at("title", default: data.document.titleAr)
  let titleEn = h.at("titleEn", default: data.document.titleEn)

  grid(
    columns: (auto, 1fr),
    align: (logo-align(opts), end),
    gutter: 10pt,
    {
      if data.company.logo != none {
        image("logo.png", height: logo-size(opts))
      } else if showName {
        text(size: 14pt, weight: "bold", fill: accent(opts))[#data.company.name]
      }
    },
    [
      #text(size: 16pt, weight: "bold", fill: accent(opts))[#title]
      #linebreak()
      #text(size: 9pt, fill: gray)[#titleEn]
    ],
  )

  v(6pt)
  if showName {
    text(size: 11pt, weight: "bold")[#data.company.name]
    linebreak()
  }
  if showAddr and data.company.address != none {
    text(size: 8.5pt, fill: gray)[#data.company.address]
    linebreak()
  }
  text(size: 8.5pt, fill: gray)[
    #if showVat and data.company.vatNumber != none [الرقم الضريبي: #data.company.vatNumber   ]
    #if showCr and data.company.commercialRegister != none [س.ت: #data.company.commercialRegister   ]
    #if showPhone and data.company.phone != none [هاتف: #data.company.phone   ]
    #if showEmail and data.company.email != none [#data.company.email   ]
    #if showWebsite and data.company.website != none [#data.company.website]
  ]

  v(6pt)
  line(length: 100%, stroke: 1.4pt + accent(opts))
  v(8pt)
}

// ---- Party box -------------------------------------------------------------
#let party-box(data, opts) = {
  let doc-no = data.document.number
  let doc-date = data.document.date
  grid(
    columns: (1fr, 1fr),
    gutter: 14pt,
    [
      #text(weight: "bold")[البائع]
      #v(2pt)
      #text(size: 9pt)[#data.company.name]
      #if data.company.vatNumber != none [
        #linebreak()
        #text(size: 8.5pt, fill: gray)[الرقم الضريبي: #data.company.vatNumber]
      ]
    ],
    [
      #text(weight: "bold")[المشتري]
      #v(2pt)
      #if data.party != none {
        text(size: 9pt)[#data.party.name]
        if data.party.at("vatNumber", default: none) != none {
          linebreak()
          text(size: 8.5pt, fill: gray)[الرقم الضريبي: #data.party.vatNumber]
        }
        if data.party.at("address", default: none) != none {
          linebreak()
          text(size: 8.5pt, fill: gray)[#data.party.address]
        }
      } else {
        text(size: 9pt)[عميل نقدي]
      }
    ],
  )
  v(8pt)
  grid(
    columns: (auto, auto, 1fr, auto, auto),
    gutter: 6pt,
    text(size: 8.5pt, fill: gray)[رقم المستند], text(size: 9pt, weight: "bold")[#doc-no], [],
    text(size: 8.5pt, fill: gray)[التاريخ], text(size: 9pt)[#doc-date],
  )
  v(10pt)
}

// ---- Line items table -------------------------------------------------------
// `opts.columns`: ordered list of { key, label, visible }.
#let default-columns = (
  (key: "index", label: "#"),
  (key: "name", label: "الصنف"),
  (key: "qty", label: "الكمية"),
  (key: "price", label: "السعر"),
  (key: "discount", label: "الخصم"),
  (key: "net", label: "صافي"),
  (key: "vatRate", label: "الضريبة %"),
  (key: "vat", label: "الضريبة"),
  (key: "total", label: "الإجمالي"),
)

#let cell-for(line, key, idx) = {
  if key == "index" { str(idx + 1) }
  else if key == "name" { line.name }
  else if key == "sku" { line.at("sku", default: "") }
  else if key == "barcode" { line.at("barcode", default: "") }
  else if key == "unit" { line.at("unit", default: "") }
  else if key == "qty" { str(line.qty) }
  else if key == "price" { line.price }
  else if key == "discount" { line.at("discount", default: "0.00") }
  else if key == "net" { line.net }
  else if key == "vatRate" { str(line.vatRate) + "%" }
  else if key == "vat" { line.vat }
  else if key == "total" { line.total }
  else { "" }
}

#let lines-table(data, opts) = {
  let cols = opts.at("columns", default: default-columns)
  cols = cols.filter(c => c.at("visible", default: true))
  if cols.len() == 0 { cols = default-columns }

  let widths = cols.map(c => {
    if c.key == "name" { 2.4fr }
    else if c.key == "index" { 0.5fr }
    else { 1fr }
  })

  table(
    columns: widths,
    align: (x, y) => if cols.at(x).key == "name" { right } else { center },
    stroke: 0.5pt + gray,
    inset: 6pt,
    table.header(
      repeat: true,
      ..cols.map(c => text(weight: "bold", size: 8.5pt)[#c.label]),
    ),
    ..data.lines.enumerate().map(((idx, line)) =>
      cols.map(c => text(size: 9pt)[#cell-for(line, c.key, idx)])
    ).flatten(),
  )
}

// ---- Totals + amount in words ------------------------------------------------
#let totals-block(data, opts) = {
  let t = opts.at("totals", default: (:))
  let showWords = t.at("showAmountInWords", default: true)
  let showBalance = t.at("showBalance", default: false)

  grid(
    columns: (1fr, 1fr),
    gutter: 14pt,
    [
      #if showWords and data.totals.amountInWords != none [
        #text(weight: "bold", size: 9pt)[المبلغ كتابة]
        #v(4pt)
        #text(size: 8.5pt)[#data.totals.amountInWords]
      ]
    ],
    align(left)[
      #table(
        columns: (auto, auto),
        stroke: none,
        inset: 4pt,
        text(size: 9pt)[الإجمالي قبل الضريبة], text(size: 9pt)[#data.totals.subtotal],
        ..if data.totals.discount != none and data.totals.discount != "0.00" {
          (text(size: 9pt)[الخصم], text(size: 9pt)[#data.totals.discount])
        } else { () },
        text(size: 9pt)[ضريبة القيمة المضافة], text(size: 9pt)[#data.totals.vat],
        text(size: 10pt, weight: "bold")[الإجمالي], text(size: 10pt, weight: "bold")[#data.totals.grand],
        ..if data.totals.paid != none {
          (text(size: 9pt)[المدفوع], text(size: 9pt)[#data.totals.paid],
           text(size: 9pt)[المتبقي], text(size: 9pt)[#data.totals.remaining])
        } else { () },
        ..if showBalance and data.totals.previousBalance != none {
          (text(size: 9pt)[الرصيد السابق], text(size: 9pt)[#data.totals.previousBalance],
           text(size: 9pt, weight: "bold")[الرصيد الحالي], text(size: 9pt, weight: "bold")[#data.totals.currentBalance])
        } else { () },
      )
    ],
  )
}

// Parses a simple "3cm" / "30mm" / "1.5in" string (as sent by
// `pdfService.ts`'s `TemplateOptions.qr.size`, e.g. from the designer's plain
// text input) into a real Typst length. JSON has no length type, so this is
// the QR-size equivalent of `font-size`'s `* 1pt` conversion above, just for
// a value that can be one of several units.
#let parse-length(s, fallback: 3cm) = {
  let m = s.trim()
  let num-str = m.find(regex("^[0-9.]+"))
  if num-str == none { return fallback }
  let n = float(num-str)
  if m.ends-with("cm") { n * 1cm }
  else if m.ends-with("mm") { n * 1mm }
  else if m.ends-with("in") { n * 1in }
  else if m.ends-with("pt") { n * 1pt }
  else { n * 1cm }
}

// ---- QR --------------------------------------------------------------------
#let qr-block(data, opts) = {
  let q = opts.at("qr", default: (:))
  let pos = q.at("position", default: "center")
  let size = parse-length(q.at("size", default: "3cm"))
  let al = if pos == "start" { start } else if pos == "end" { end } else { center }
  align(al)[
    #image("qr.svg", width: size)
  ]
}

// ---- Footer ------------------------------------------------------------------
#let footer-block(data, opts) = {
  let f = opts.at("footer", default: (:))
  let terms = f.at("terms", default: none)
  let bank = f.at("bankDetails", default: none)
  let showSignatures = f.at("showSignatureLines", default: false)
  let thankYou = f.at("thankYouLine", default: none)
  let showPageNumbers = f.at("showPageNumbers", default: true)

  if terms != none {
    text(size: 8pt, fill: gray)[الشروط والأحكام: #terms]
    v(4pt)
  }
  if bank != none {
    text(size: 8pt, fill: gray)[البيانات البنكية: #bank]
    v(4pt)
  }
  if showSignatures {
    v(16pt)
    grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 10pt,
      align(center)[#line(length: 70%) #v(2pt) #text(size: 8pt)[المستلم]],
      align(center)[#line(length: 70%) #v(2pt) #text(size: 8pt)[المحاسب]],
      align(center)[#line(length: 70%) #v(2pt) #text(size: 8pt)[المدير]],
    )
  }
  if thankYou != none {
    v(8pt)
    align(center)[#text(size: 9pt, fill: accent(opts))[#thankYou]]
  }
  if showPageNumbers {
    v(1fr)
    align(center)[#text(size: 8pt, fill: gray)[صفحة #context counter(page).display() من #context counter(page).final().at(0)]]
  }
}

// ---- Page setup --------------------------------------------------------------
#let paper-size(opts) = opts.at("paper", default: "a4")

#let page-margin(opts) = {
  let p = paper-size(opts)
  if p == "80mm" or p == "58mm" { (x: 3mm, y: 4mm) } else { (x: 1.8cm, y: 1.8cm) }
}

#let page-typst-size(opts) = {
  let p = paper-size(opts)
  if p == "80mm" { (width: 80mm, height: auto) }
  else if p == "58mm" { (width: 58mm, height: auto) }
  else if p == "a5" { "a5" }
  else if p == "letter" { "us-letter" }
  else { "a4" }
}

#let font-family(opts) = opts.at("fontFamily", default: "Cairo")
// `opts.fontSize` arrives from JSON as a plain number (e.g. `10`), not a
// Typst length — JSON has no length type, so `pdfService.ts` sends a raw
// point count and this converts it to a real length (`10 * 1pt`).
#let font-size(opts) = opts.at("fontSize", default: 10) * 1pt

// ---- Full invoice document (used by invoice_standard.typ / invoice_simplified.typ) ----
#let invoice-document(data, opts, simplified: false) = {
  set document(title: data.document.number, date: datetime(year: 2026, month: 1, day: 1))
  set page(paper: page-typst-size(opts), margin: page-margin(opts))
  set text(font: font-family(opts), lang: "ar", region: "sa", size: font-size(opts), dir: rtl)
  set par(justify: false)

  header(data, opts)
  party-box(data, opts)
  lines-table(data, opts)
  v(12pt)
  totals-block(data, opts)
  v(10pt)
  if data.qr != none {
    qr-block(data, opts)
    v(8pt)
  }
  footer-block(data, opts)
}
