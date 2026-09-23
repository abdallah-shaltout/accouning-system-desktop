# نظام المحاسبة ونقاط البيع — Desktop Accounting & POS (UI phase)

Arabic-first (RTL) accounting + POS + invoicing desktop app for a single retail/clothing shop.
Tauri v2 shell · Vue 3 (`<script setup>`) · Pinia · Vue Router · Tailwind CSS v4 · Zod.

**This phase is UI-only.** Every screen runs against an in-memory mock backend that behaves like a
real one (double-entry postings, stock movements, balances). Reloading the app re-seeds the data.
See `docs/project_specs.md`, `docs/domain_model.md`, `docs/design_system.md`, `docs/action_plan.md`.

## Run

```bash
bun install
bun run dev          # browser at http://localhost:1420
bun run tauri dev    # desktop window
bun run build        # type-check + production build
```

### Demo accounts

| User | Password | Role |
|------|----------|------|
| `admin` | `admin123` | مدير النظام — everything |
| `manager` | `manager123` | مدير المتجر — everything except users |
| `accountant` | `acc123` | محاسب — accounting, reports, payments; sales/purchases read-only |
| `cashier` | `cashier123` | كاشير — POS & sales; lands on the POS after login |
| `cashier2` | `cashier123` | كاشير — sells at the wholesale price list |

In dev builds the user menu has a quick user switcher.

## Architecture

```text
src/
├── mocks/                 # the fake backend (only services/ may import it)
│   ├── db.ts              # in-memory tables + document numbering
│   ├── backend/           # posting rules: sales, refunds, purchases, payments, stock, journal
│   ├── fixtures/          # static seed data (chart of accounts, products, people, settings)
│   └── seed.ts            # replays ~75 days of shop activity through the posting rules
├── modules/<module>/      # core, users, products, parties, accounting, invoices,
│   ├── services/          #   purchases, payments, reports, settings
│   ├── pages/ components/ controllers/ helpers/ routes/ types/ validators/
└── router/                # assembles module routes; role gating via route meta
```

- **Services are the seam.** Pages only call `modules/*/services/*` functions shaped like a real API
  (`Promise<T>`, simulated latency, deep-cloned results, Arabic `ApiError` messages). Replacing the
  mock with Tauri IPC / SQL later is a service-layer-only change.
- **Posting rules** (`src/mocks/backend/`) keep stock, the journal and balances consistent: a sale
  posts Cash/Bank/AR ↔ Sales + VAT output and COGS ↔ Inventory; purchases, refunds, payments and stock
  adjustments post their own balanced entries. Reports are computed from the journal.
- **Roles** are presets (`modules/users/helpers/permissions.ts`), gating routes, the sidebar and actions.
- **Printing:** A4 tax invoice and 58/80 mm thermal receipt, both with a ZATCA Phase‑1 QR code
  (`modules/invoices/helpers/zatcaQr.ts`). Reports export to PDF (print dialog), CSV and Markdown.

## Checks

```bash
bun run verify:mocks                        # ledger balances, AR/AP = party balances, stock history
python scripts/e2e_flows.py <out_dir>       # POS sale, role gating, journal, refund, payment, reports
python scripts/screenshot.py <out_dir> / /pos /reports/trial-balance --dark
```

The two Python scripts need the dev server running and Python Playwright installed.

The previous SQLite-based `src/` is archived in `references/legacy-src/` for reference.
