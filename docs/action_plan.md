# Execution Plan — Frontend-Only Rebuild

**Agent instructions:** This restarts the app from scratch as a **pure UI build** — no Tauri SQL plugin, no Rust backend logic, no real persistence beyond optional `localStorage` for UI preferences (theme, last-selected filters). Every screen reads from and writes to **mock data services** (in-memory arrays with fake async delays), matching the shapes in `docs/domain_model.md`. Follow `docs/project_specs.md` and `docs/design_system.md` for scope and look. Proceed phase by phase; don't skip ahead.

Before starting: confirm with the user whether to delete the existing `src/modules/*` content (real SQLite-wired services, existing pages) or keep specific pieces — this plan assumes a clean restart of `src/` per the earlier decision to start fresh, keeping only `src/assets/styles/design-system.css`, `scripts/scaffold.js`, and the Tauri shell config (`src-tauri/`) as-is.

> **Next:** the v2 plan lives in [`docs/v2/`](v2/README.md), with its checklist in [`docs/v2/15-action-plan.md`](v2/15-action-plan.md).
>
> **Status (2026-09-23): all phases implemented.** The previous `src/` was archived (not deleted) to `references/legacy-src/`. Mock backend lives in `src/mocks/` (in-memory DB, posting rules in `src/mocks/backend/`, deterministic ~75-day seed in `src/mocks/seed.ts`). Checks: `bun run verify:mocks` (ledger/balance consistency), `python scripts/e2e_flows.py <dir>` (main flows, needs the dev server), `python scripts/screenshot.py <dir> <routes…> [--dark]`. Only two `src-tauri/` edits: the `fs:allow-write-text-file` capability (report export) and a desktop-sized window.

## Phase 0: Cleanup & Foundation

- [x] Remove old `src/modules/*` real-service code that assumed a SQLite backend (`db.ts`, `*ServiceAPI.ts`, bcrypt-based auth, etc.) — keep the folder-structure convention, not the implementation.
- [x] Set up a `src/mocks/` (or per-module `services/__fixtures__/`) convention for fixture data + a small `delay()` helper so mock services feel async and realistic.
- [x] Confirm design tokens in `design-system.css` cover status colors (add a `--color-warning` token for "partially paid" if missing).
- [x] Set up Vue Router with route-level `meta.role` gating (mock auth only — a Pinia store holding "current mock user" with a role switcher in dev, no real login validation).
- [x] Global layout shell: sidebar + topbar (`default` layout) and a chrome-free `blank` layout for POS/print screens.
- [x] Global toast/notification system (already scaffolded — verify it doesn't depend on removed services).
- [x] RTL verified end-to-end (`<html dir="rtl">`, Cairo font loaded, number formatting checked).

## Phase 1: Auth & Users (mock)

- [x] Login screen — mock credential check against fixture users; store "current user" in Pinia (no hashing/DB needed).
- [x] User list + editor page: role (admin/manager/accountant/cashier), max discount %, price list assignment — per `domain_model.md` §10.
- [x] Role-based nav gating: sidebar items hide/show per current mock user's role.

## Phase 2: Dashboard (Home)

- [x] KPI cards: today's sales, unpaid invoices count, low-stock count, cash position — all computed from mock fixture arrays.
- [x] Recent invoices widget, recent activity feed (mock log entries).
- [x] Widget-grid layout, responsive down to a reasonable minimum desktop width.

## Phase 3: Products & Inventory

- [x] Product list (search, filter by category/type), product create/edit form (product vs. service toggle, stock qty, cost/sale price, optional price-list values).
- [x] Categories & units management (simple CRUD against mock arrays).
- [x] Price lists management screen.
- [x] Stock adjustment screens: Stock-In, Loss/Write-off, Stocktake (جرد) — each produces a `StockAdjustment` record and updates mock `Product.stockQty`.
- [x] Stock movement history (read-only table, filterable by product).

## Phase 4: Customers & Suppliers

- [x] Customer list + detail page (info, computed balance, transaction history list from mock invoices/payments).
- [x] Supplier list + detail page (mirrors customer, AP side).

## Phase 5: Accounting Engine (mock double-entry)

- [x] Chart of Accounts tree view — seed with the ~23-account default set from `domain_model.md` §1; add/edit group or account (single `code` field).
- [x] Manual Journal Entry form: header + dynamic debit/credit line rows; client-side validation that totals balance before "saving" to mock state.
- [x] Read-only Journal Entries list showing both `SYSTEM` (auto, linked to an invoice/PO/payment) and `MANUAL` entries — this demonstrates the double-entry link even though no real posting engine runs behind it.
- [x] Fiscal Year settings screen (start/end date, is-closed flag) — used only as the default date range for report screens.

## Phase 6: Invoicing / POS

- [x] POS screen (chrome-free layout): product grid with category filter/search, cart panel, walk-in vs. customer toggle, discount input capped by current mock user's `maxDiscount`.
- [x] Checkout: payment method selection, tender/change (for cash), generates an `Invoice` + a mock `SYSTEM` journal entry preview.
- [x] Invoice list (filter by status/payment status, search by number/customer).
- [x] Invoice detail / print preview with **both A4 and thermal (58/80mm) layouts** (see `design_system.md`), toggle driven by `StoreSettings.printer.mode`.
- [x] Port the ZATCA-style TLV/base64 QR generator from `references/vat-invoice-app/js/zatca-qr.js` into `src/modules/invoices/helpers/` and render it on the printable invoice.
- [x] Simple refund/return flow against an existing invoice.

## Phase 7: Purchases

- [x] Purchase order list + create screen (supplier picker, line items, totals).
- [x] Simple purchase return.

## Phase 8: Payments

- [x] Record payment received (against a customer/invoice) and payment paid (to a supplier/PO) — simple one-payment-to-one-target model, no allocation engine.
- [x] Payment list, filterable by type/method/date.

## Phase 9: Reports

- [x] Reports hub page linking to each report.
- [x] Trial Balance, Profit & Loss, Balance Sheet — computed client-side from mock Journal Entries.
- [x] Account Statement / Ledger (كشف حساب) — generic view reusable for any account or a customer/supplier.
- [x] Sales Report, Inventory Report, VAT/Tax Summary.
- [x] Export actions: PDF (print-to-PDF via the browser/Tauri dialog), CSV, and a Markdown export (AI-agent-friendly plain text table) for the currently rendered report table.

## Phase 10: Settings & Hardware (UI only)

- [x] General settings: store name, logo upload (local file, stored as data URL in mock state), currency, VAT number, default tax rate.
- [x] Hardware/Printing settings: printer mode (A4/thermal), thermal paper width, "Test Print" button that opens the print preview (no real device I/O in this phase).
- [x] Appearance: light/dark theme toggle (already scaffolded — verify still works after Phase 0 cleanup).

## Phase 11: Polish

- [x] Empty states, loading states (skeleton or spinner during mock `delay()`), and error boundaries for every list/detail screen.
- [x] Keyboard shortcuts for the POS screen (e.g. F9 new sale, F12 pay) — desktop-first ergonomics.
- [x] Full RTL + light/dark visual QA pass across every screen built above.
- [x] Confirm every `services/` function has a real-API-shaped signature (`Promise<T>`, same params a future Tauri IPC call would take) so a later backend integration only touches the service layer.
