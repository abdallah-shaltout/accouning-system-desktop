// Standard (B2B) tax invoice — docs/v2/12-documents-pdf-excel.md §3.
// Reads `data.json` (DocumentPayload) and `opts.json` (TemplateOptions),
// both written as virtual files by the Rust World before compiling.
#import "lib.typ": invoice-document

#let data = json("data.json")
#let opts = json("opts.json")

#invoice-document(data, opts, simplified: false)
