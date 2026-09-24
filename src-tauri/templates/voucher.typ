// Receipt / payment / journal voucher (سند قبض / سند صرف / سند قيد) — docs/v2/09 §5.
// Compact A5-friendly layout (own header, not lib.typ's invoice header, since a voucher has no
// party box or line-items table) — a boxed amount, amount-in-words, description, and two
// signature lines. `data.document.kind` selects the Arabic title when opts.header.title isn't set.
#import "lib.typ": accent, page-margin, page-typst-size, font-family, font-size, paper-size

#let data = json("data.json")
#let opts = json("opts.json")

#set document(title: data.document.number)
#set page(paper: page-typst-size(opts), margin: page-margin(opts))
#set text(font: font-family(opts), lang: "ar", region: "sa", size: font-size(opts), dir: rtl)
#set par(justify: false)

#let h = opts.at("header", default: (:))
#let title = h.at("title", default: data.document.titleAr)

// ---- Header (own, not lib.typ's invoice header — no logo/party-box needed) ----
#grid(
  columns: (1fr, auto),
  align: (start, end),
  [
    #text(size: 11pt, weight: "bold")[#data.company.name]
    #if data.company.at("address", default: none) != none [
      #linebreak()
      #text(size: 8.5pt, fill: gray)[#data.company.address]
    ]
  ],
  [
    #text(size: 15pt, weight: "bold", fill: accent(opts))[#title]
    #linebreak()
    #text(size: 9pt, fill: gray)[#data.document.number — #data.document.date]
  ],
)
#v(10pt)
#line(length: 100%, stroke: 1.2pt + accent(opts))
#v(14pt)

// ---- Party (who the money is to/from) ----
#if data.party != none [
  #grid(
    columns: (auto, 1fr),
    gutter: 8pt,
    text(size: 9pt, fill: gray)[الطرف], text(size: 10pt, weight: "bold")[#data.party.name],
  )
  #v(10pt)
]

// ---- Amount box ----
#align(center)[
  #rect(width: 80%, inset: 14pt, radius: 4pt, stroke: 1pt + accent(opts))[
    #text(size: 22pt, weight: "bold")[#data.totals.grand]
    #if data.totals.at("amountInWords", default: none) != none [
      #v(6pt)
      #text(size: 9pt, fill: gray)[#data.totals.amountInWords]
    ]
  ]
]
#v(16pt)

// ---- Description / accounts ----
#if data.document.at("description", default: none) != none [
  #text(size: 10pt)[الوصف: #data.document.description]
  #v(6pt)
]
#if data.document.at("accountsLine", default: none) != none [
  #text(size: 9pt, fill: gray)[#data.document.accountsLine]
  #v(6pt)
]
#if data.document.at("note", default: none) != none [
  #text(size: 8.5pt, fill: gray)[ملاحظات: #data.document.note]
]

#v(1fr)

// ---- Signatures ----
#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 10pt,
  align(center)[#line(length: 70%) #v(2pt) #text(size: 8pt)[المستلم/المحرر]],
  align(center)[#line(length: 70%) #v(2pt) #text(size: 8pt)[المحاسب]],
  align(center)[#line(length: 70%) #v(2pt) #text(size: 8pt)[المدير]],
)
