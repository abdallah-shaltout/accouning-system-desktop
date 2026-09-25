// Official report template — the native-PDF twin of `src/modules/reports/print/renderHtml.ts`.
// Both lay out the same `ReportDocument` model (src/modules/reports/print/types.ts), passed here as
// `data.report`: letterhead, title row + badge, meta strip, then blocks (kpis / heading / table /
// columns / banner / note / boxes) and signatures; every page gets a running footer
// (company — report · system note · page x of y). Layout ported from the reference accounting system's report partials
// (grayscale, ruled tables, section / subtotal / total / grand rows).

#let data = json("data.json")
#let r = data.report
#let c = r.company

#let black = rgb("#262626")
#let ink = rgb("#333333")
#let mid = rgb("#555555")
#let muted = rgb("#808080")
#let rule-c = rgb("#c6c6c6")
#let hair = rgb("#e2e2e2")
#let stripe = rgb("#f4f4f4")
#let band = rgb("#eceff4")
#let warn-c = rgb("#9b1c1c")

#let landscape = r.at("orientation", default: "portrait") == "landscape"

#let issued-date = {
  let iso = r.at("issuedIso", default: none)
  if iso == none { auto } else {
    let p = iso.slice(0, 10).split("-")
    datetime(year: int(p.at(0)), month: int(p.at(1)), day: int(p.at(2)))
  }
}

#set document(title: r.title, date: issued-date)
#set page(
  paper: "a4",
  flipped: landscape,
  margin: (x: 12mm, top: 11mm, bottom: 16mm),
  footer: context {
    set text(size: 6.3pt, fill: muted)
    grid(
      columns: (1fr, auto, 1fr),
      align(start)[#c.name — #r.title],
      text(fill: rgb("#999999"))[#r.footerNote],
      align(end)[صفحة #counter(page).display() من #counter(page).final().first()],
    )
  },
)
#set text(font: ("IBM Plex Sans Arabic", "Noto Naskh Arabic"), lang: "ar", region: "sa", size: 7.5pt, dir: rtl, fill: ink)
#set par(justify: false, leading: 0.55em)

// Numbers stay LTR inside RTL text so minus signs / separators never flip.
#let num(s) = box(text(dir: ltr)[#s])

// ---- Letterhead -------------------------------------------------------------------------------
#{
  let facts = (
    if c.at("vatNumber", default: none) != none { [الرقم الضريبي: #c.vatNumber] },
    if c.at("commercialRegister", default: none) != none { [السجل التجاري: #c.commercialRegister] },
    if c.at("address", default: none) != none { [#c.address] },
    if c.at("phone", default: none) != none { [هاتف: #num(c.phone)] },
  ).filter(x => x != none)
  grid(
    columns: (1fr, auto),
    align: (start + top, end + top),
    [
      #text(size: 12pt, weight: "bold", fill: black)[#c.name]
      #if facts.len() > 0 [
        #v(1pt)
        #text(size: 6.8pt, fill: mid)[#facts.join(text(fill: rule-c)[#h(4pt)·#h(4pt)])]
      ]
    ],
    if data.at("logo", default: none) != none {
      box(stroke: 0.6pt + rule-c, fill: stripe, inset: (x: 8pt, y: 4pt), radius: 2pt, image("logo.png", height: 1.1cm))
    } else { [] },
  )
}
#v(5pt, weak: true)
#line(length: 100%, stroke: 2.2pt + black)
#v(6pt, weak: true)

// ---- Title row ----------------------------------------------------------------------------------
#grid(
  columns: (1fr, auto),
  align: (start + bottom, end + bottom),
  [
    #text(size: 11.3pt, weight: "bold", fill: black)[#r.title]
    #if r.at("subtitle", default: none) != none [
      #linebreak()
      #text(size: 6.8pt, fill: muted)[#r.subtitle]
    ]
  ],
  box(stroke: 0.6pt + rule-c, inset: (x: 7pt, y: 3pt), text(size: 6.4pt, weight: "bold", fill: mid)[#r.badge]),
)
#v(4pt, weak: true)
#line(length: 100%, stroke: 0.6pt + rule-c)
#v(6pt, weak: true)

// ---- Meta strip ---------------------------------------------------------------------------------
// One row when it fits (6 portrait / 8 landscape), else balanced rows (8 cells → 4 + 4) — same
// rule as `metaColumns()` in src/modules/reports/print/build.ts.
#if r.meta.len() > 0 {
  let count = r.meta.len()
  let max = if landscape { 8 } else { 6 }
  let n = if count <= max { count } else { calc.ceil(count / calc.ceil(count / max)) }
  let pad = calc.rem(n - calc.rem(count, n), n)
  grid(
    columns: range(n).map(_ => 1fr),
    stroke: 0.6pt + rule-c,
    inset: (x: 7pt, y: 4pt),
    ..r.meta.map(m => [
      #text(size: 6pt, weight: "bold", fill: muted)[#m.label]
      #linebreak()
      #text(size: 7.5pt, weight: "bold")[#m.value]
    ]),
    ..range(pad).map(_ => []),
  )
  v(8pt, weak: true)
}

// ---- Blocks -------------------------------------------------------------------------------------
#let cell-align(col) = {
  let a = col.at("align", default: if col.at("numeric", default: false) { "end" } else { "start" })
  if a == "center" { center } else if a == "end" { end } else { start }
}

// Same rule as the HTML renderer: when any column has a relative width, the others weigh 1.
#let col-widths(cols) = {
  let any-width = cols.any(x => x.at("width", default: none) != none)
  let first-text = cols.position(x => not x.at("numeric", default: false))
  cols.enumerate().map(((i, x)) => {
    let w = x.at("width", default: none)
    if any-width { (if w == none { 1 } else { w }) * 1fr } else if i == first-text { 1fr } else { auto }
  })
}

#let row-stroke(kind) = {
  if kind == "subtotal" { (top: 0.6pt + rule-c, bottom: 1.2pt + rule-c) }
  else if kind == "total" { (top: 1.1pt + black, bottom: 1.1pt + black) }
  else if kind == "grand" { (top: 1.5pt + black, bottom: 2.2pt + black) }
  else { (bottom: 0.5pt + hair) }
}

#let row-fill(kind, i) = {
  if kind == "total" { band }
  else if kind == "subtotal" { rgb("#fafafa") }
  else if kind == "opening" { stripe }
  else if kind == "normal" and calc.odd(i) { stripe }
  else { none }
}

#let report-table(b) = {
  let cols = b.columns
  let n = cols.len()
  let rows = b.rows
  if rows.len() == 0 {
    let msg = b.at("emptyText", default: none)
    if msg != none {
      block(width: 100%, inset: 12pt, stroke: (paint: rule-c, thickness: 0.6pt, dash: "dashed"), align(center, text(fill: muted)[#msg]))
    }
  } else {
    let header = if b.at("hideHeader", default: false) { () } else {
      (table.header(
        repeat: true,
        ..cols.map(col => table.cell(
          fill: band,
          align: cell-align(col),
          stroke: (top: 0.8pt + black, bottom: 0.8pt + black),
          text(size: 6.8pt, weight: "bold", fill: black)[#col.label],
        )),
      ),)
    }
    let body = rows.enumerate().map(((i, rw)) => {
      let kind = rw.at("kind", default: "normal")
      if kind == "section" or kind == "subhead" {
        let label = rw.cells.find(x => x != "")
        (table.cell(
          colspan: n,
          fill: if kind == "section" { band } else { stripe },
          stroke: (bottom: 0.6pt + rule-c),
          text(weight: "bold", size: if kind == "section" { 7.6pt } else { 7.1pt }, fill: if kind == "section" { black } else { mid })[#if label == none { "" } else { label }],
        ),)
      } else {
        let bold = kind in ("subtotal", "total", "grand")
        cols.enumerate().map(((j, col)) => {
          let v = rw.cells.at(j, default: "")
          let content = if col.at("numeric", default: false) and v != "" { num(v) } else { v }
          table.cell(
            fill: row-fill(kind, i),
            align: cell-align(col),
            stroke: row-stroke(kind),
            text(
              size: if kind == "grand" { 8.2pt } else if col.at("dim", default: false) { 6.4pt } else { 7.1pt },
              weight: if bold { "bold" } else { "regular" },
              style: if kind == "opening" { "italic" } else { "normal" },
              fill: if col.at("dim", default: false) and not bold { muted } else if kind == "opening" { mid } else if bold { black } else { ink },
            )[#content],
          )
        })
      }
    }).flatten()
    table(
      columns: col-widths(cols),
      stroke: none,
      inset: (x: 5pt, y: 3.4pt),
      ..header,
      ..body,
    )
  }
  v(7pt, weak: true)
}

#let render-block(b) = {
  let t = b.type
  if t == "kpis" {
    grid(
      columns: range(b.items.len()).map(_ => 1fr),
      stroke: 0.6pt + rule-c,
      inset: (x: 6pt, y: 4.5pt),
      fill: (x, y) => if b.items.at(x).at("emphasis", default: false) { band } else { none },
      align: center,
      ..b.items.map(k => [
        #text(size: 6pt, fill: muted)[#k.label]
        #linebreak()
        #text(size: 9.8pt, weight: "bold", fill: black)[#num(k.value)]
      ]),
    )
    v(8pt, weak: true)
  } else if t == "heading" {
    v(4pt, weak: true)
    block(stroke: (right: 2.2pt + black), inset: (right: 5pt, y: 1pt), text(size: 7.5pt, weight: "bold", fill: black)[#b.text])
    v(4pt, weak: true)
  } else if t == "table" {
    report-table(b)
  } else if t == "columns" {
    grid(
      columns: range(b.columns.len()).map(_ => 1fr),
      gutter: 8pt,
      ..b.columns.map(stack => stack.map(render-block).join()),
    )
  } else if t == "banner" {
    block(width: 100%, fill: stripe, stroke: 0.6pt + rule-c, inset: (x: 9pt, y: 5.5pt), grid(
      columns: (1fr, auto),
      align: (start + horizon, end + horizon),
      [
        #text(size: 9pt, weight: "bold", fill: black)[#b.title]
        #if b.at("subtitle", default: none) != none [
          #linebreak()
          #text(size: 6.8pt, fill: muted)[#b.subtitle]
        ]
      ],
      if b.at("tag", default: none) != none {
        box(fill: white, stroke: 0.6pt + rule-c, inset: (x: 6pt, y: 2pt), text(size: 6.4pt, fill: mid)[#b.tag])
      } else { [] },
    ))
    v(8pt, weak: true)
  } else if t == "note" {
    let warn = b.at("tone", default: "info") == "warn"
    block(
      width: 100%,
      stroke: (rest: 0.6pt + rule-c, right: 2.2pt + if warn { warn-c } else { black }),
      inset: (x: 8pt, y: 4.5pt),
      text(size: 7.1pt, weight: "bold", fill: if warn { warn-c } else { ink })[#b.text],
    )
    v(8pt, weak: true)
  } else if t == "boxes" {
    grid(
      columns: range(b.items.len()).map(_ => 1fr),
      gutter: 6pt,
      ..b.items.map(x => block(
        width: 100%,
        fill: if x.at("emphasis", default: false) { band } else { none },
        stroke: 0.6pt + if x.at("emphasis", default: false) { black } else { rule-c },
        inset: (x: 9pt, y: 7pt),
        [
          #text(size: 6.4pt, weight: "bold", fill: muted)[#x.title]
          #v(-2pt)
          #line(length: 100%, stroke: 0.5pt + rule-c)
          #v(-2pt)
          #text(size: 12.8pt, weight: "bold", fill: black)[#num(x.value)]
          #if x.at("sub", default: none) != none [
            #linebreak()
            #text(size: 6.4pt, fill: muted)[#x.sub]
          ]
        ],
      )),
    )
    v(9pt, weak: true)
  }
}

#for b in r.blocks { render-block(b) }

// ---- Signatures ---------------------------------------------------------------------------------
#if r.signatures.len() > 0 {
  v(16pt)
  block(breakable: false, grid(
    columns: range(r.signatures.len()).map(_ => 1fr),
    align: center,
    ..r.signatures.map(s => [
      #text(size: 6.8pt, weight: "bold", fill: mid)[#s]
      #v(20pt)
      #line(length: 70%, stroke: 0.6pt + ink)
      #v(-3pt)
      #text(size: 6pt, fill: muted)[الاسم / التوقيع]
    ]),
  ))
}
