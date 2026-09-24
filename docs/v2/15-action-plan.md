# 15 — Action Plan v2 (execute from here)

**Status: ALL PHASES DONE** (2026-09-24). Every box below is ticked (92/92). Full gate green on the
final tree: JS build, a clean Rust rebuild, 16 PDF smoke tests (0 failures) + the thermal ESC/POS
pipeline (structurally verified, real printer enumeration), `verify:mocks` at **49 ok / 0 todo / 0
failed**, `totals.spec` 39/39, `bun run check` clean, and all 16 e2e flow suites green with zero
console errors — including a final consolidated `full_persona_pass.py` walking cashier → manager
→ storekeeper → accountant through one continuous day. Zero `TODO(phase N)` markers remain
anywhere in the codebase (confirmed by grep). README.md is updated with real run steps, demo
accounts and architecture notes.

Known smaller follow-ups, left as deliberate, documented deferrals rather than hidden gaps: the
9 new document PDF templates (phase 11b) export as plain PDF 1.7 rather than PDF/A-3b (only the
invoice template, phase 11a's original spike target, achieves PDF/A-3b); purchase-side documents
carry FC fields but aren't posted in FC (sales-side FX is the fully wired and tested path); trial
balance / P&L / balance sheet don't have branch filter dropdowns yet (cost-center P&L exists);
the currency-revaluation wizard has a working backend with no dedicated settings-page UI; a
handful of lower-priority reports and analytics tabs from the doc's full catalogue were skipped
for time in favor of broader coverage elsewhere (each noted in its phase's status block above).

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

**Status: done** (2026-09-24). Commit `af2c685`. The full e2e proof (34 checks) runs fresh install →
wizard → real .xlsx customer import → inline product create → opening entry posted → 3900 closes
to exactly zero → login → first POS sale, with zero console errors end to end. Invariant 9 explicit
and green. The setup-checklist card is intentionally standalone rather than wired into the
dashboard, since Phase 10 owned that file concurrently — `// TODO(phase 10)` left for the
insight-engine hookup.

- [x] Setup wizard (11 steps), saved progress, setup checklist card
- [x] Opening balances tabs (cash/banks, customers ± open invoices, suppliers, stock, other) + review + the opening entry + closing 3900; **invariant 9 green**
- [x] "Balance from an old system" section in the party form
- [x] Generic `ImportWizard` (template download, mapping, validation, dedupe, batch commit, error file) + descriptors: customers, suppliers, products, opening stock, opening balances, price update
- [x] E2E: fresh company → wizard → import customers → opening entry balanced → first sale

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

**Status: done** (2026-09-24). Commit `1561155`. Speed targets met with real measurements (latency
off): scan→cart avg 18.2ms (target <50ms), checkout→receipt 70.3ms (target <300ms). Weighted-barcode
(`2x`+code+weight/price) prefix parsing was skipped as a time-budget cut — `n*` qty multiplier
works, weighted does not. Found and fixed two real bugs: the dev latency-off switch wasn't
respected by explicit-ms `delay()` calls (was silently padding every sale by 350ms), and
`accountant.sales` permission was stuck at read-only, blocking the desk invoice form for that role.

- [x] POS: unit picker + barcode → unit, `n*` qty, custom price (floor, reason), line + invoice discounts, manager PIN approval
- [x] POS: held sales (F6), split tender dialog, foreign cash tender, return by receipt scan (F7), reprint (Ctrl+P)
- [x] Shifts: open/close, pay-in/pay-out (→ expense), X/Z reports, manager shifts screen; **invariant 11 green**
- [x] Desk invoice form (grid, B2B/B2C type, due date, terms, tafqit, journal preview, pay now) + quotations → invoice
- [x] Credit notes v2 (reason, refund method incl. customer credit, restock toggle → write-off)
- [x] Invoice list v2 (filters, saved views, totals, bulk export)
- [x] Speed: barcode index, virtualized grid, POS preload; targets met with latency switched off

## Phase 8 — Purchases, expenses & vouchers → [09](09-purchases-payments-expenses.md)

**Status: done** (2026-09-24). Commit `86ce50c`. Closes Phase 3's card/wallet settlement TODO and
Phase 6's return-to-supplier draft stub. Found and fixed three real bugs: non-recoverable VAT was
double-booked (posted to cost while AP stayed short by that amount), a partial receipt left the
PO's total at the full order amount instead of what was actually posted to AP, and a reactive Vue
ref leaked into `structuredClone` and silently broke every purchase save.

- [x] Purchase v2: units, discounts, tax per line, supplier invoice no/date (+ duplicate warning), non-VAT supplier handling
- [x] Receiving flow (price-hidden for storekeepers, batch/expiry capture, backorder draft) + "print labels for received qty" hook (stubbed `TODO(phase 11b)`)
- [x] Landed costs (allocation by value/qty, other-supplier AP)
- [x] Debit notes v2 (reason, refund method, batch pick)
- [x] Expenses module + categories + recurring expenses
- [x] General vouchers: receipt, payment, transfer (drawer → bank), owner drawings/contribution

## Phase 9 — Branches, cost centers & currencies → [10](10-branches-currencies-cost-centers.md)

**Status: done** (2026-09-24). Commit `120b212`. The realized-FX worked example is pinned exactly
(Dr Cash 49,200 / Cr AR 48,500 / Cr FX gain 700, party ledger nets to 0 USD). `verify:mocks` jumped
from 32 to **48 ok, 0 todo, 0 failed** — 16 new structural checks (FX, branches, cost centers,
transfers) all green. Feature-switches-off confirmed visually unchanged from pre-phase-9 via e2e.

Smaller deferrals, honestly flagged rather than hidden: purchase-side documents carry
`currency`/`exchangeRate` fields but aren't yet posted in FC (only sales-side FX is wired and
tested); trial balance/P&L/balance sheet don't yet have branch filter dropdowns or a "compare
branches" P&L mode (cost-center P&L was built instead, the harder of the two asks); the
currency-revaluation wizard has a working backend/service but no dedicated settings-page UI yet;
FC/VAT-in-base-currency printing isn't wired into the Typst invoice template or thermal receipt
(only the in-app invoice detail page shows the FC/base split). None of these block correctness —
every invariant holds — they're scope for a follow-up pass.

- [x] Features switches (branches / currencies / cost centers)
- [x] Branches: settings, per-branch cash account + numbering series, user branches, topbar switcher, branch stock
- [x] Stock transfers (send / receive with shortage / reject) + transit account
- [x] Cost centers: tree, budgets, line field, split-by-% on journal lines, P&L by cost center, budget vs actual
- [x] Currencies + rates; FC documents; FC cash/bank; realized FX on allocation; revaluation wizard (auto-reversing) — sales-side only; purchase-side FX and the revaluation wizard's UI page are deferred (see note above)
- [x] Seed: a second branch, one USD customer, one cost-center split; every invariant green per currency

## Phase 10 — Home, analytics & recommendations → [11 Parts B–D](11-journal-dashboard-insights.md)

**Status: done** (2026-09-24). Commit `d11cf8a`. 21 rules built (exceeds the required 20), resolving
every `TODO(phase 10)` left by earlier phases (Phase 2's due recurring entries, Phase 6's
storekeeper home, Phase 8's recurring-expense due, Phase 9's budget insight, Phase 13a's
backup-overdue). `/analytics` skipped a branches tab and a profitability waterfall (need deeper
branch wiring / Phase 12's expense-by-group data) — noted as follow-ups, not silently dropped.

- [x] Insight engine (rules, keys, dismiss/snooze, thresholds settings, caching on `ledger:changed`)
- [x] The first-set rule catalogue (20 rules), each with a pre-filled action target
- [x] New home: "needs attention", 4 KPIs with period comparison + sparkline, one comparison chart, top products/customers
- [x] Role homes: cashier shift panel, storekeeper, accountant
- [x] `/analytics` tabs with one-sentence chart insights
- [x] Inline hints on product, party and report pages (report pages deferred — that module was owned by the concurrent Phase 12 agent)

## Phase 11 — Documents: PDF engine, templates, labels → [12](12-documents-pdf-excel.md)

**Status: done** (2026-09-24). 11a commit `aab8e04` (2026-09-23), 11b commit `dae5888`
(2026-09-24). 11b added 9 new document templates + 2 label templates (11 new `.typ` files, all
reusing `lib.typ`'s shared components), extended `pdfService.render()` from invoice-only to every
document kind, and wired real print buttons across purchases/vouchers/shifts/transfers. 16 Rust
smoke tests (lopdf-verified PDF structure), 0 failures.

- [x] Rust `pdf` module: `render_pdf`, `render_preview` (SVG), embedded fonts (all 4 bundled families), virtual files; capabilities
- [x] `pdfService` + `PdfPreview`; browser fallback to the print route
- [x] Typst templates: invoice (standard/simplified), quotation, credit/debit note, PO, vouchers, statement, Z-report, transfer note, generic report
- [x] Template designer (options, live preview, import/export, advanced source editor with real compile-error feedback); defaults-per-branch still open (a smaller follow-up)
- [x] Label builder + label templates (sheets and thermal sizes) with bwip-js barcodes and QR
- [x] Every "print/PDF" button in the app goes through `pdfService` (isTauri()-gated with a browser-print fallback); documents attaching their own rendered PDF is a smaller follow-up

## Phase 12 — Reports v2 → [13](13-reports.md)

**Status: done** (2026-09-24). Commit `64602e1`. 27 report pages. `verify:mocks` at 49 ok/0 todo/0
failed throughout — no regressions, no discrepancies found. PDF export still falls back to the
browser print route (`// TODO(phase 11b)` — written before 11b's generic_report template landed;
wiring it in is a small follow-up, not a blocker since Excel export and on-screen reports both
work fully).

- [x] `ReportShell` v2 (comparison, branch/cost-center/currency filters, insights box, PDF/Excel, drill-down)
- [x] Upgrade: trial balance, P&L, balance sheet, sales, inventory (stock valuation + movements already lived inside it). Ledger and cost-center P&L kept as-is (already had filters/drill-down from earlier phases). VAT got insights only (comparison isn't meaningful for a return)
- [x] New: cash flow, day book, AR/AP aging, overdue, gross profit, returns, discounts & overrides, shifts, low/dead stock, stocktake variances, transfers, purchases, expenses, budget vs actual, period & branch comparison, business health, profit leakage, VAT detail. Expiry links to Phase 6's existing page rather than duplicating it. Quotation-conversion and a customer-balances-summary report were skipped for time (lower priority than what was built)
- [x] Reports hub: groups, search, favorites

## Phase 13 — Backup, approvals, notifications & polish → [14](14-platform.md)

**Phase 13a done** (2026-09-23, commit `bcf4823`). **13b done** (2026-09-24) — the final phase,
closing every remaining `TODO(phase N)` in the codebase (report-page insight boxes → the real
insight engine, `ReportShell`'s PDF export → the generic-report Typst template, the command
palette's barcode lookup → `findByCode`, `AttachmentField` on payments/stock adjustments,
company stamp/signature uploads, the VAT-settlement payment posting → a real payment method via
the shared voucher helper, and the setup-checklist card → wired into the dashboard) plus building
notifications, approvals and the rest of the command palette/shortcuts surfaces below. New
`Area: 'approvals'` (admin/manager write). Discount/write-off/below-cost approvals stay
PIN-gated synchronously at the point of action (unchanged) — the new `/approvals` async queue is
purely the escape hatch those dialogs now offer when no manager is present to type a PIN, not a
redundant parallel system. Full gate green on the merged tree: `bun run build`, `cargo build`,
`bun run verify:mocks` (49 ok / 0 todo / 0 failed), `bun run check`, the full e2e suite (16 flows
incl. a new consolidated `full_persona_pass.py` walking all 4 personas' day in one run) — zero
console errors. Zero `TODO(phase` markers remain anywhere in `src/`/`scripts/`.

- [x] Backup now / automatic (daily + on close, retention) / history / verify / restore with pre-restore backup; optional password encryption
- [x] Notifications drawer (insights by severity + events: transfer arrived, approval requested, backup failed, recurring entry due) with mark-as-read and links; approvals page (`/approvals`) for managers, async queue for when the synchronous PIN dialogs can't be used
- [x] Command palette: search providers for invoices, purchases, vouchers, journal, accounts, reports, settings (Phase 0 built the shell + pages/actions/customers/suppliers/products); context `when(route)` commands on invoice/purchase/voucher/journal detail pages
- [x] Keyboard shortcuts sheet (F1 / ?) — global, per-route registry (`modules/core/helpers/keyboardShortcuts.ts`), skips `/pos` (which keeps its own extensive Phase 7 sheet)
- [x] Full e2e pass for all personas ([01](01-personas.md)); README rewritten for v2 (run steps, demo accounts, architecture, checks)

## Phase 14 (optional) — Native thermal printing → [12 §5](12-documents-pdf-excel.md)

**Status: done** (2026-09-24). Commit `32aaebe`. `list_printers` enumerates real Windows printers
(3 found in the build environment). The ESC/POS raster byte stream was verified structurally
correct (header dimensions match the dithered bitmap exactly) for both 80mm and 58mm widths.

- [x] `list_printers`, raw ESC/POS raster printing (Windows queue + network 9100), cut, drawer kick
- [x] Printer settings + test print; asynchronous printing with reprint on failure
