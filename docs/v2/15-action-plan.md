# 15 — Action Plan v2 (execute from here)

**Agent instructions:**
- **Constraints:** still **UI-only against the mock backend**. The only native code is the PDF
  engine (Phase 11) and optional thermal printing (Phase 14).
- **Order:** follow the dependency table and waves in *Execution order & parallel waves* below.
  Each phase starts by reading its feature file.
- **Tracking:** tick the boxes here as you go, and add a status note at the top when a phase ends.
- **Decisions:** follow the decisions in [README](README.md). Ask the user only about
  scope-changing or irreversible choices.

**Definition of done (every phase):**
1. **Build:** `bun run build` passes (vue-tsc + vite).
2. **Ledger:** `bun run verify:mocks` passes every invariant in [02 §4](02-accounting-review.md)
   that applies so far.
3. **E2E:** the phase's main flow is added (`scripts/e2e/flows/<area>.py` once Phase 0 splits
   `scripts/e2e_flows.py`) and all flows pass with no console errors.
4. **Screenshots:** new screens in light and dark (`scripts/screenshot.py … --dark`), checked at
   1280 px and 1920 px.
5. **Seam rule:** pages touch only `modules/*/services`; only services import `src/mocks`.
6. **UI conventions:** Arabic copy, full keyboard use, permissions on routes, the sidebar and actions.
7. **Tables and search:** every new table has Excel export; every new entity has a
   command-palette search provider.

---

## Execution order & parallel waves

The phases below are numbered by topic. **Run them in the order of this dependency table**, not
by number. A phase can start when everything in its "Needs" column is merged and green.

| Phase | Needs | Main areas it edits |
|-------|-------|---------------------|
| 0 Foundations | — | `src/mocks/persist*`, `modules/core/*`, settings/appearance, `scripts/`, `src-tauri/` (spike only), all `.vue` (text-token codemod) |
| 1 Accounting core | 0 | `src/mocks/backend/*`, `src/mocks/fixtures/accounts*`, `src/mocks/seed/*`, `modules/accounting` (CoA page) |
| 2 Journal | 1 | `modules/accounting` (journal, fiscal years, VAT settlement) |
| 3 Taxes & payment methods | 1 | `modules/invoices/helpers/totals*`, `modules/settings` (taxes, payment methods), sales posting |
| 4 Parties & allocation | 1 | `modules/parties`, `modules/payments`, `src/mocks/backend/payments*`, balances |
| 6 Products & inventory | 1, 3 | `modules/products`, `src/mocks/backend/inventory*`, users (storekeeper role) |
| 7 Sales | 3, 4, 6 | `modules/invoices` (POS, desk form, quotations, credit notes, shifts) |
| 8 Purchases & expenses | 3, 4, 6 | `modules/purchases`, new `modules/expenses`, vouchers |
| 9 Branches / currencies / cost centers | 7, 8 | cross-cutting: run **alone** |
| 5 Onboarding | 4, 6, 9 | new `modules/setup`, import wizard |
| 10 Home & recommendations | 7, 8 | `modules/core` (dashboard), new `modules/analytics`, `src/mocks/insights` |
| 11a PDF engine + designer + invoice template | 0 | `src-tauri/src/pdf`, `src-tauri/templates`, `modules/core/services/pdfService*`, settings/templates |
| 11b Remaining templates + labels | 6, 7, 8, 11a | `src-tauri/templates`, `modules/products` (labels page) |
| 12 Reports | 9 | `modules/reports` |
| 13a Backup & restore | 0 | `modules/settings` (backup), `src/mocks/persist*` |
| 13b Notifications, approvals, palette providers, shortcuts, final e2e | everything | cross-cutting |
| 14 Thermal printing | 11a | `src-tauri/src/print`, settings/printers |

**Recommended waves** (phases in the same wave run in parallel):

| Wave | Phases |
|------|--------|
| W1 | 0 (the text-token codemod first, alone; then its other items in parallel) |
| W2 | 1 ∥ 11a ∥ 13a |
| W3 | 2 ∥ 3 ∥ 4 |
| W4 | 6 ∥ 14 |
| W5 | 7 ∥ 8 |
| W6 | 9 |
| W7 | 5 ∥ 10 ∥ 11b ∥ 12 |
| W8 | 13b + the final full verification |

**Rules for parallel work:**
- **Isolation:** each parallel phase runs in its own git worktree/branch. The orchestrator merges
  the branches, resolves conflicts, then runs the full gate (the definition of done below) on the
  merged tree before the next wave.
- **The orchestrator alone edits** this file (the ticks and status notes), `package.json`/`bun.lock`
  merges (it re-runs `bun install` after merging), and `src/router/index.ts` conflicts.
- **Shared files are split** in Phase 0, so parallel phases *add files* instead of editing the same
  one:
  - seeding: `src/mocks/seed/<area>.ts`
  - invariants: `scripts/verify/<area>.ts`
  - e2e flows: `scripts/e2e/flows/<area>.py`
  - `src/mocks/db.ts`: tables are added as separate, commented blocks.
- **Dev servers:** each parallel agent uses its own port (1421, 1422…), passing `--base` to the
  Python scripts. Rust builds share one `CARGO_TARGET_DIR`.
- **Missing dependencies:** if a phase needs something from a phase that isn't merged yet, it adds
  the smallest stub with a `// TODO(phase N)` note. The orchestrator checks that such stubs are gone
  by the end.

---

## Phase 0 — Foundations → [14](14-platform.md)

**Status: done** (2026-09-23). Landed as 5 commits on `master`: `5267fa9` (text-token codemod),
`884f995` (persistence, welcome screen, event bus, paged queries, script splitting), `2c13c27`
(attachments, xlsx export, command palette, Arabic search), `fe34679` (appearance settings v2),
`1cc806a` (Typst spike — **passed** every gate criterion, see [12 §1](12-documents-pdf-excel.md)).
Full gate green on the merged tree: `bun run build`, `bun run verify:mocks` (18 ok / 4 todo — all
deferred to their owning phase / 0 failed), `bun run check`, all 5 e2e flows + a new Phase 0 smoke
check (welcome screen, persistence-survives-reload, dev reset, command palette, appearance
settings) in light and dark, zero console errors. No `TODO(phase` stubs remain except the
intentional ones pointing at later phases (journal/purchases/parties/products attachments,
per-module command-palette providers).

- [x] Parallel-work enablers: seed split into `src/mocks/seed/<area>.ts`; `scripts/verify/<area>.ts` + runner; `scripts/e2e/flows/<area>.py` + runner (`python scripts/e2e/run.py <dir> [--only area] [--base URL]`); `--base` option in `screenshot.py`

- [x] IndexedDB persistence of the mock DB (`mutate()` wrapper, debounced snapshot, schema version + migrations)
- [x] Welcome screen: *start your company* / *demo data*; dev menu (reset, load demo, latency switch)
- [x] Event bus: `ledger:changed`, `catalog:changed`, `parties:changed`; Pinia master-data caches use it
- [x] Paged query contract `{ page, pageSize, sort, filters } → { rows, total, totals }` + `DataTable` server mode
- [x] `normalizeArabic()` search helper, used by every search input
- [x] Text tokens with `--font-scale`; codemod the 167 `text-[Npx]` uses; grep check against new ones
- [x] Appearance settings v2: font family (4 bundled), text size, density, accent presets, Hijri, week start, table prefs
- [x] `AttachmentField` + blob storage (IndexedDB) + viewer (lightbox / PDF)
- [x] `exportXlsx()` helper (exceljs, lazy) + "تصدير" menu on `DataTable` (all filtered rows)
- [x] Command palette shell: Ctrl+K, pages + actions, recents, prefixes, permission filtering
- [x] **Typst spike (gate)**: the [12 §1](12-documents-pdf-excel.md) checklist; record pass/fail in [12](12-documents-pdf-excel.md) — **PASS**, 49–63ms steady-state render, PDF/A-3b achieved
- [x] `verify:mocks` v2 harness listing every invariant (unmet ones reported as TODO, not failures, until their phase)

## Phase 1 — Accounting core v2 → [02](02-accounting-review.md), [03](03-chart-of-accounts.md)

**Status: done** (2026-09-23). Commit `658a579`. Rebuilt the CoA on the standard (~60-account)
template with SA + pharmacy add-ons, `accountFor(role, ctx)` replacing every hard-coded code,
party balances derived from the ledger, `stockValue`-based inventory (GL now matches
Σ stockValue exactly), control-account + fiscal-lock enforcement. `verify:mocks`: **23 ok, 1 todo
(VAT report, phase 3), 0 failed** — invariants 1–4, 7, 8 all green. Full e2e suite green, zero
console errors. Branches/currencies/cost-centers fields exist but stay inert until phase 9; the
fiscal-year closing wizard UI is phase 2 (this phase ships the lock-date check only).

- [x] Account v2 fields (tree, `isGroup`, subtype, role, currency, branch, flags); CoA templates basic/standard/detailed + country/business add-ons
- [x] `accountFor(role, ctx)`; remove every hard-coded code from `src/mocks/backend/*`
- [x] Journal line dimensions: party, branch, cost center, currency/amountFc/rate (default branch + base currency for now)
- [x] Party balances and statements computed from the ledger (C2)
- [x] Control-account rules (B1); fiscal-period check + lock date (B2)
- [x] `stockValue` per product, re-averaging on returns, 4-decimal average cost, zero-stock remainder (A1/A2)
- [x] Stock-in reasons (A3); variance 5110 / write-off 5120 (A4); count snapshot (A5)
- [x] Purchase return refund method (E1); line purchase accounts (E2)
- [x] Chart of accounts page: header/leaf rules, path in pickers, drag re-parent, balances with a period filter
- [x] Seed rebuilt on the standard template; **invariants 1–4, 7, 8 green**

## Phase 2 — Journal redesign & period tools → [11 Part A](11-journal-dashboard-insights.md)

**Status: done** (2026-09-24). Commit `bb93223`. Found and fixed a real latent bug along the way:
`uid()`'s in-memory id counters never resynced after `loadSnapshot()` restored a persisted DB,
which could silently collide new record ids with existing ones after a reload.

- [x] Journal list v2: day grouping, inline line preview, full filters, saved views, totals, day-book PDF (print route until Phase 11)
- [x] Journal form v2: spreadsheet grid, "=" balancing, party/cost-center columns, paste from Excel, attachments, drafts
- [x] Templates + recurring entries (due → insight placeholder)
- [x] Detail v2: audit trail, related entries, attachments viewer; reversal dialog with date + reason (B3)
- [x] Fiscal-year close wizard + reopen; lock-date setting
- [x] VAT settlement screen + payment to the authority (minimal cash/bank posting; routes through a shared payment-method voucher helper in phase 8)

## Phase 3 — Taxes, pricing & payment methods → [06 §3](06-sales-and-pos.md), [09 §2](09-purchases-payments-expenses.md)

**Status: done** (2026-09-24). Commit `b5534a9`. `totals.spec.ts` (39 assertions) matches the
worked example exactly: net 108.53, VAT 12.97, gross 121.50.

- [x] Tax entity v2 (category S/Z/E/O, direction, account, exemption reason) + settings screen; country presets
- [x] `totals.ts` rewrite: per-line VAT, inclusive/exclusive, proportional invoice discount; `totals.spec` with the worked example and edge cases
- [x] Payment methods entity + settings (type, account, per-branch override, fee %, POS/payment visibility)
- [x] Tenders (split payment) in the sale posting; card/wallet clearing; card settlement voucher deferred to phase 8 (`TODO(phase 8)`)
- [x] VAT report v2 (country boxes) + VAT detail; **invariant 5 and 10 green**

## Phase 4 — Customers & suppliers v2 → [08](08-customers-and-suppliers.md), [09 §3](09-purchases-payments-expenses.md)

**Status: done** (2026-09-24). Commit `d78b2a3`. Found and fixed two real bugs in the verify
script's own invariant-6 logic (purchase-return AP reduction and sales-refund AR reduction were
checked against the wrong settlement amounts).

- [x] Country data + `AppPhoneInput` (E.164, libphonenumber-js/min) replacing every phone field
- [x] Party model + form v2 (sections, national address, VAT/CR/IBAN validation, groups, terms, credit limit, contacts)
- [x] Party page with tabs (overview, documents, payments, statement, aging, attachments, history)
- [x] Payments with allocation grid, unallocated credit, allocate-later from the party page; **invariant 6 green**
- [x] Credit-limit enforcement + override permission; due dates from terms

## Phase 5 — Onboarding, opening balances & Excel import → [05](05-onboarding.md)

- [ ] Setup wizard (11 steps), saved progress, setup checklist card
- [ ] Opening balances tabs (cash/banks, customers ± open invoices, suppliers, stock, other) + review + the opening entry + closing 3900; **invariant 9 green**
- [ ] "Balance from an old system" section in the party form
- [ ] Generic `ImportWizard` (template download, mapping, validation, dedupe, batch commit, error file) + descriptors: customers, suppliers, products, opening stock, opening balances, price update
- [ ] E2E: fresh company → wizard → import customers → opening entry balanced → first sale

## Phase 6 — Products & inventory v2 → [07](07-products-and-inventory.md)

**Status: done** (2026-09-24). Commit `8efbc1b`. Branch stock/transfers (§4) intentionally deferred
to phase 9. `verify:mocks` gained 2 new batch invariants, both green: **31 ok, 0 todo, 0 failed**.

- [x] Product form v2 tabs; account resolution product → category → settings
- [x] Multiple units with factors and unit barcodes; price matrix (price list × unit); min price; unit presets
- [x] Batches & expiry (FEFO, alerts, expiry report actions)
- [x] Stock-in / write-off with approval threshold; stocktake v2 (scope, snapshot, blind count, scan counting, review)
- [x] Storekeeper role preset + home; role matrix editor (Settings → Users & roles)
- [x] Custom fields (Settings → Products); product images gallery

## Phase 7 — Sales v2 → [06](06-sales-and-pos.md)

- [ ] POS: unit picker + barcode → unit, `n*` qty, custom price (floor, reason), line + invoice discounts, manager PIN approval
- [ ] POS: held sales (F6), split tender dialog, foreign cash tender, return by receipt scan (F7), reprint (Ctrl+P)
- [ ] Shifts: open/close, pay-in/pay-out (→ expense), X/Z reports, manager shifts screen; **invariant 11 green**
- [ ] Desk invoice form (grid, B2B/B2C type, due date, terms, tafqit, journal preview, pay now) + quotations → invoice
- [ ] Credit notes v2 (reason, refund method incl. customer credit, restock toggle → write-off)
- [ ] Invoice list v2 (filters, saved views, totals, bulk export)
- [ ] Speed: barcode index, virtualized grid, POS preload; targets met with latency switched off

## Phase 8 — Purchases, expenses & vouchers → [09](09-purchases-payments-expenses.md)

- [ ] Purchase v2: units, discounts, tax per line, supplier invoice no/date (+ duplicate warning), non-VAT supplier handling
- [ ] Receiving flow (price-hidden for storekeepers, batch/expiry capture, backorder draft) + "print labels for received qty" hook
- [ ] Landed costs (allocation by value/qty, other-supplier AP)
- [ ] Debit notes v2 (reason, refund method, batch pick)
- [ ] Expenses module + categories + recurring expenses
- [ ] General vouchers: receipt, payment, transfer (drawer → bank), owner drawings/contribution

## Phase 9 — Branches, cost centers & currencies → [10](10-branches-currencies-cost-centers.md)

- [ ] Features switches (branches / currencies / cost centers)
- [ ] Branches: settings, per-branch cash account + numbering series, user branches, topbar switcher, branch stock
- [ ] Stock transfers (send / receive with shortage / reject) + transit account
- [ ] Cost centers: tree, budgets, line field, split-by-% on journal lines, P&L by cost center, budget vs actual
- [ ] Currencies + rates; FC documents; FC cash/bank; realized FX on allocation; revaluation wizard (auto-reversing)
- [ ] Seed: a second branch, one USD customer, one cost-center split; every invariant green per currency

## Phase 10 — Home, analytics & recommendations → [11 Parts B–D](11-journal-dashboard-insights.md)

- [ ] Insight engine (rules, keys, dismiss/snooze, thresholds settings, caching on `ledger:changed`)
- [ ] The first-set rule catalogue (20 rules), each with a pre-filled action target
- [ ] New home: "needs attention", 4 KPIs with period comparison + sparkline, one comparison chart, top products/customers
- [ ] Role homes: cashier shift panel, storekeeper, accountant
- [ ] `/analytics` tabs with one-sentence chart insights
- [ ] Inline hints on product, party and report pages

## Phase 11 — Documents: PDF engine, templates, labels → [12](12-documents-pdf-excel.md)

**Phase 11a done** (2026-09-23, commit `aab8e04`), built on the Typst spike's PASS. 11b (remaining
templates + labels) waits on phases 6/7/8 per the wave table.

- [x] Rust `pdf` module: `render_pdf`, `render_preview` (SVG), embedded fonts (all 4 bundled families), virtual files; capabilities
- [x] `pdfService` + `PdfPreview`; browser fallback to the print route
- [x] Typst templates: invoice (standard/simplified) — 11a scope. Quotation, credit/debit note, PO, vouchers, statement, Z-report, transfer note, generic report → **11b**
- [x] Template designer (options, live preview, import/export, advanced source editor with real compile-error feedback) — 11a proved this against the invoice template; defaults-per-branch waits on phase 9
- [ ] Label builder + label templates (sheets and thermal sizes) with bwip-js barcodes and QR → **11b**
- [ ] Every "print/PDF" button in the app goes through `pdfService`; documents can attach their PDF → invoice's print button does this now; the rest land with their own document kind in 11b

## Phase 12 — Reports v2 → [13](13-reports.md)

- [ ] `ReportShell` v2 (comparison, branch/cost-center/currency filters, insights box, PDF/Excel, drill-down)
- [ ] Upgrade: trial balance, P&L, balance sheet, ledger, sales, stock valuation, movements, VAT
- [ ] New: cash flow, day book, AR/AP aging, overdue, gross profit, returns, discounts & overrides, shifts, low/dead stock, expiry, stocktake variances, transfers, purchases, expenses, budget vs actual, period & branch comparison, business health, profit leakage
- [ ] Reports hub: groups, search, favorites

## Phase 13 — Backup, approvals, notifications & polish → [14](14-platform.md)

**Phase 13a done** (2026-09-23, commit `bcf4823`). 13b (notifications, approvals, remaining palette
providers, shortcuts, final e2e) waits on everything, per the wave table.

- [x] Backup now / automatic (daily + on close, retention) / history / verify / restore with pre-restore backup; optional password encryption
- [ ] Notifications drawer (insights + events); approvals page → **13b**
- [ ] Command palette: every module's search provider + context commands → **13b** (Phase 0 built the shell + 3 example providers)
- [ ] Keyboard shortcuts sheet (F1 / ?) per page → **13b**
- [ ] Full e2e pass for all personas ([01](01-personas.md)); README and docs updated → **13b**

## Phase 14 (optional) — Native thermal printing → [12 §5](12-documents-pdf-excel.md)

**Status: done** (2026-09-24). Commit `32aaebe`. `list_printers` enumerates real Windows printers
(3 found in the build environment). The ESC/POS raster byte stream was verified structurally
correct (header dimensions match the dithered bitmap exactly) for both 80mm and 58mm widths.

- [x] `list_printers`, raw ESC/POS raster printing (Windows queue + network 9100), cut, drawer kick
- [x] Printer settings + test print; asynchronous printing with reprint on failure
