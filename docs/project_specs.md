# Desktop Accounting & POS UI — Product Spec (Frontend-Only)

## 1. What this is

A **desktop-first, single-store** accounting + POS + invoicing UI for small retail/clothing shops (محلات الملابس والتجزئة), built with **Tauri v2 shell + Vue 3 (Composition API) + TailwindCSS v4**.

This phase builds **UI only**: every screen, list, form, and report renders against **local mock/fixture data and in-memory or localStorage state** — there is no real database, no Rust backend logic, no Tauri SQL plugin wiring, and no server. The goal is a fully clickable, realistic desktop app that looks and behaves like the finished product, so the real data layer can be slotted in later without reworking any screen.

This replaces the earlier plan (see git history / previous `project_specs.md`) which targeted a real Tauri+SQLite backend from day one. That work is being restarted from scratch, keeping the same visual language and module folder convention, but scoped down to match a single shop instead of a multi-tenant platform.

## 2. Why this scope

We have a full reference implementation at `references/accouning-system/` — a real multi-tenant SaaS POS/accounting platform (`POS-Fares-server` + `POS-Fares-webiste`) built for many stores/branches at once, plus `references/books` (Frappe Books, a real desktop accounting app) and `references/vat-invoice-app` (a small ZATCA-QR invoice tool). The multi-tenant reference is **too complex** for our target user — a single clothing/retail shop owner who wants one installed app, not a SaaS subscription with plans, branches, cost centers, and fixed-asset depreciation schedules.

See `docs/domain_model.md` for the full simplified entity list this app is built around, and what was deliberately cut from the reference system (multi-tenancy, plans/subscriptions, cost centers, fixed assets, ZATCA e-invoicing, offline sync, background jobs — all backend/SaaS concerns irrelevant to a frontend-only single-store desktop tool).

## 3. Tech stack

- **Shell:** Tauri v2 (window chrome, native menus, file dialogs for export — no custom Rust commands needed yet)
- **UI:** Vue 3 `<script setup>`, Vue Router, Pinia (client-only state, no persistence layer required beyond optional localStorage for UI prefs)
- **Styling:** TailwindCSS v4 + the existing Linear-inspired design tokens in `src/assets/styles/design-system.css` (light/dark mode already scaffolded)
- **Data:** static/mock TypeScript fixtures (`src/mocks/` or per-module `__fixtures__`), no Tauri SQL plugin, no network calls
- **Language:** Arabic-first UI (RTL), matching the existing `ACCOUNT_ROOT_TYPES`/`ACCOUNT_TYPES` Arabic labels already in the codebase

## 4. Folder structure (unchanged convention)

Keep the existing modular structure — it already matches this plan well:

```text
src/modules/[module-name]/
├── components/     # UI components specific to this module
├── controllers/    # Pinia stores / composables — now backed by mock data, not services
├── design/         # Module-specific styles, if any
├── helpers/        # Formatters, calculators (totals, tax, discounts)
├── pages/          # Route-level Vue views
├── routes/         # Vue Router route definitions for this module
├── services/       # Mock data providers — return Promises resolving to fixture data,
│                   # simulating a future real API/IPC call shape
├── types/          # TypeScript interfaces (see domain_model.md)
└── validators/     # Form validation schemas (Zod)
```

`services/` in this phase are **fake async services** (e.g. `getInvoices(): Promise<Invoice[]>`) backed by in-memory arrays. This keeps the exact same call shape pages will use later, so swapping in real Tauri SQL / IPC calls afterward is a service-layer-only change — no page or component should need to change.

## 5. Modules & screens

Scope, simplified from the reference platform (full detail in `docs/domain_model.md`):

### 5.1 Dashboard (core)
Home screen with KPI cards (today's sales, low stock count, unpaid invoices, cash position), recent invoices, recent activity feed. Widget-grid layout, no real analytics computation — mock numbers.

### 5.2 POS / Sales (invoices)
- One POS/sale screen: product grid + cart + checkout panel (walk-in or select customer).
- Invoice list (search, filter by status: paid/unpaid/partially paid/canceled).
- Invoice detail / print preview — **both A4 and thermal (58mm/80mm) receipt layouts**, switchable in settings.
- ZATCA-style QR code on the printable invoice (TLV/base64 generator ported from `references/vat-invoice-app/js/zatca-qr.js` — cosmetic/Phase-1 style only, no real e-invoicing integration).
- Simple sales returns/refunds against an existing invoice.

### 5.3 Products & Inventory
- Product list (with category, SKU/barcode, stock qty, cost/sale price); distinguish **Product** (tracked stock) vs **Service** (no stock tracking).
- Categories, units, and optional price lists (multiple prices per product).
- Stock adjustments: stock-in, loss/write-off, and a stocktake (جرد) screen that shows counted vs. system qty and previews the resulting adjustment.
- Stock movement history (read-only ledger view).

### 5.4 Customers & Suppliers
- Customer list + detail (contact info, running balance, statement/history list).
- Supplier list + detail (same shape, AP side).
- No dedicated per-party GL sub-account UI — balances are shown as simple computed numbers from mock data.

### 5.5 Purchases
- Purchase order list + create screen (supplier, line items, totals).
- Simple purchase return.

### 5.6 Accounting Engine
- Chart of Accounts — hierarchical tree view (add group/account, single `code` field, no internal-ID split, no complexity tiers). Seed with the simplified ~20-25 account default set in `domain_model.md`.
- Manual Journal Entry screen (header + balanced debit/credit lines, must validate debits = credits before "saving" to mock state).
- Read-only view of auto-generated journal entries behind POS sales/purchases/payments (so the UI demonstrates the double-entry link without a real posting engine).
- Fiscal year: a simple settings screen to set the current fiscal year start/end — no real period-locking logic needed yet.

### 5.7 Payments
- Record a payment received (against a customer invoice) or paid (to a supplier), with method (cash/card/bank transfer/other).
- No cross-invoice payment-allocation engine — one payment maps to one invoice/target, kept simple.

### 5.8 Reports
Single Reports hub page linking to individual report views, each rendering from mock aggregate data:
- Trial Balance, Profit & Loss (Income Statement), Balance Sheet
- Account Statement / Ledger (كشف حساب) — generic, works for any account or customer/supplier
- Sales Report, Inventory Report
- VAT/Tax Summary report
- Export actions (PDF, Excel/CSV, Markdown) — can be stubbed as buttons that trigger a client-side export of the currently-rendered mock table.

### 5.9 Users & Permissions
- Login screen (mock auth — hardcoded/local users, no password hashing needed against a real DB).
- User list with a handful of role presets: **Admin, Manager, Accountant, Cashier** (dropped the full 100+ permission-string matrix from the reference; role presets are enough for this UI phase).
- Per-user Max Discount % field and assigned price list, shown as simple form fields on the user editor (enforced at the POS UI level against the "current mock session user").

### 5.10 Settings
- General (store name, logo, currency, tax number, VAT rate default).
- Hardware/Printing — printer type selection (A4 vs thermal), paper width, mock "Test Print" button (opens the print preview, no real device I/O yet).
- Appearance — light/dark theme toggle (already scaffolded).
- Fiscal year settings (see 5.6).

## 6. Explicitly out of scope (this phase, and likely permanently for a single-shop tool)

Per the reference-system analysis, do **not** build:
- Multi-tenancy (`store`/tenant model, plans/subscriptions, usage limits, super-admin app)
- Multi-branch consolidation, cost centers, inter-branch transfer approval workflows
- Fixed asset depreciation subsystem
- Real ZATCA Phase-2 e-invoicing (XML/UBL, cryptographic stamping, reporting API)
- Third-party integrations (WhatsApp, push notifications, webhooks)
- Background jobs/cron, offline-sync queues, real-time sockets
- Payment allocation engine (multi-invoice matching)
- Credit/debit notes (the reference itself never implemented these)
- Chart-of-accounts complexity tiers, `internalCode` vs `code` split

If any of these turn out to be genuinely needed later, they get scoped as a deliberate follow-up — not built speculatively now.

## 7. Design system

Use the existing Linear-inspired tokens already defined in `src/assets/styles/design-system.css` (near-black/near-white surfaces, hairline borders, Inter-family type, compact spacing, single accent color for primary actions). Full reference in `docs/design_system.md`, adapted from `references/vat-invoice-app/design.md`. Keep Arabic (RTL) as the primary layout direction; numbers/currency stay LTR within RTL text per existing `toHindi`/number-formatting conventions seen in the reference invoice app.

## 8. Reference-usage rules

- For **domain shapes / entity fields** (accounts, invoices, products, journal entries): base them on `docs/domain_model.md`, which was distilled from `references/accouning-system` and simplified for single-store use. Don't re-read the raw reference source per screen — the domain doc is the source of truth going forward.
- For **ZATCA QR / tax calculation logic**: port the exact TLV/base64 approach from `references/vat-invoice-app/js/zatca-qr.js` — it's small, self-contained, and already correct for Phase-1 QR needs.
- For **double-entry bookkeeping concepts** (chart of accounts tree, journal header/lines, fiscal year): use `references/books` (Frappe Books) as the conceptual model for a *real desktop* accounting app, simplified further per `domain_model.md`.
- Do not copy multi-tenant, SaaS, or backend infrastructure code from `POS-Fares-server`/`POS-Fares-webiste` — those are architecture references for *what a feature does*, not code to port, since this phase has no backend at all.
