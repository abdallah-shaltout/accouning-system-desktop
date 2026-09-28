# 22 — Invoice templates: 10 × A4, 10 × mobile image, thermal unchanged

> **Status (2026-09-26):** phases A–D implemented; build, check, verify:mocks, memory, diag:check and the
> new `invoice_templates` e2e flow are green. **Left in `pending/` because** the desktop build was not run
> this session: "حفظ كصورة" in `bun run desktop` (native Save dialog writing a `.png` via `plugin-fs`) and
> "نسخ الصورة" in the Tauri WebView clipboard are unverified. Move to `completed/` once checked there.
> Requested by the user on 2026-09-26: "at
> least 10 A4 templates, a mobile *image* invoice type with 10 templates, thermal stays as it is —
> each template completely different, not just colours; one mobile template in the InstaPay receipt
> style". Inspiration: `references/accouning-system/POS-Fares-webiste/app/components/pages/invoice/templates/*`
> (desktop-first there — ours are built for their real medium: A4 paper, and a ~400 px phone screen).

## Why

`/print/invoices/:id` (`InvoicePrintPage.vue`) renders exactly one A4 layout (`InvoiceA4.vue`) and one
thermal layout (`InvoiceThermal.vue`). Shops share invoices on WhatsApp as a picture far more often than
they print A4, and there is no way to produce one.

## Decisions

1. **Three layouts on the print page:** `a4`, `thermal` (unchanged, 58/80 mm) and `image` (new). The
   stored default *print* mode (`StoreSettings.printer.mode`) keeps meaning "what POS auto-prints", so it
   stays `'a4' | 'thermal'`; `image` is a print-page-only layout (`PrintLayout` type).
2. **Templates are Vue components, not Typst.** The print page is the browser/WebView document the user
   actually sees and prints; the Typst designer (`modules/templates`, PDF export) is a separate
   pipeline and is not touched.
3. **One view model, many looks.** `controllers/useInvoiceDoc.ts` derives every printable value
   (lines, totals, QR, titles, amount in words, payment/status labels) once. Templates only lay it out —
   no template re-implements money math (UI rule 9). Line VAT/total read the posted `line.net`/`line.vat`
   snapshot when present, falling back to the legacy formula `InvoiceA4.vue` uses.
4. **`InvoiceA4.vue` stays the "standard" template untouched** (ZATCA/EG-verified by e2e).
5. **Registry = single owner** (`helpers/invoiceTemplates.ts` for ids/labels/fonts,
   `components/templates/registry.ts` for the async components). Adding a template = one file + one
   registry row (scale by adding).
6. **Colours are fixed document palettes** (`--color-doc-*` tokens in `design-system.css`, next to the
   existing `--color-print-*`): a printed/shared invoice must look identical from light or dark mode, so
   it cannot follow the app theme. Big figures use a new `--text-display` token.
7. **Fonts per template** from the already-installed fontsource packages (Cairo, IBM Plex Sans Arabic,
   Tajawal, Noto Naskh Arabic), loaded on demand by the registry.
8. **Image export** uses `modern-screenshot` (renders through the browser engine via SVG
   `foreignObject`, so Arabic shaping and ligatures are exact — `html2canvas`, used by the reference,
   breaks joined Arabic letters). Saved through the shared `saveFile` helper (new `image` kind, native
   Save dialog — rule 21) or copied to the clipboard for pasting into WhatsApp.
9. **Defaults per device:** `StoreSettings.printer.a4Template` / `imageTemplate` (optional, lazy like
   `thermal?`). The print page's gallery has "تعيين كافتراضي".

## Templates

| A4 (`a4`) | Idea | Font |
|---|---|---|
| `standard` | Existing ZATCA tax invoice (`InvoiceA4.vue`) | Cairo |
| `corporate` | Government-form grid: boxed meta cells, full-bordered table, stamp + signature boxes | Naskh |
| `sidebar` | Coloured start-side column holds brand, meta and the total; lines in the main column | Plex |
| `banner` | Dark header band carrying the grand total; airy line rows | Cairo |
| `swiss` | Typographic: huge invoice number, asymmetric grid, hairlines only, one red mark | Plex |
| `bento` | Info in tiles (customer / payment / dates), rounded lines panel, emphasised total tile | Tajawal |
| `luxury` | Double frame, monogram medallion, centred serif, bronze rules | Naskh |
| `compact` | Dense, zebra rows, one-line header, totals strip — for long invoices | Plex |
| `letter` | Letterhead + salutation paragraph + table + amount in words + signature | Naskh |
| `geometric` | Angled header/footer shapes, numbered line badges | Cairo |

| Image (`image`) | Idea | Font |
|---|---|---|
| `instapay` | InstaPay-style transfer receipt: purple gradient, success seal, big amount, detail rows | Cairo |
| `ticket` | Boarding-pass: notched halves, perforation, stub with QR/number | Plex |
| `wallet` | Bank-card hero with total, items below | Plex |
| `chat` | Conversation bubbles on a wallpaper | Tajawal |
| `paper` | Torn-edge paper receipt on a coloured desk | Plex |
| `noir` | Dark premium, gold figures | Naskh |
| `spotlight` | Minimal white, one huge total, coral accent | Tajawal |
| `grouped` | iOS-settings grouped lists | Cairo |
| `poster` | Bold colour block, oversized type | Cairo |
| `glass` | Gradient mesh with frosted card | Tajawal |

## Definition of done

`bun run build`, `bun run check`, `bun run verify:mocks`, `bun run memory`, `bun run diag:check`, full
`python scripts/e2e/run.py` (new flow `invoice_templates.py` renders every template with no console
errors and exports an image), light + dark app theme, 1280/1920 px, keyboard.

## Phases

| File | What | Size | Status |
|---|---|---|---|
| this README · A | Tokens, view model, registry, `saveFile` image kind, settings fields | S | done |
| this README · B | 9 new A4 templates | L | done |
| this README · C | 10 image templates + export (save / copy) | L | done (desktop save/copy unverified) |
| this README · D | Print page gallery + defaults, settings preview, e2e flow, docs | M | done |

### Tasks
- [x] A: `--color-doc-*` + `--text-display` tokens; `useInvoiceDoc`; registry; `saveFile` `image` kind; `printer.a4Template/imageTemplate`
- [x] B: 9 A4 templates
- [x] C: 10 image templates; `invoiceImageService` (save PNG / copy)
- [x] D: `TemplateGallery`, `InvoiceDocument`, print page layout switch, settings preview uses the default template, e2e flow, `docs/design_system.md`
- [x] Gates green; `bun run memory`

Later (not in scope): per-template accent override from the store's brand colour; sharing the image
straight to WhatsApp (needs a Tauri share plugin).

- [ ] Desktop check: `bun run desktop` → print preview → صورة للموبايل → حفظ كصورة writes a PNG; نسخ الصورة pastes into a chat app
