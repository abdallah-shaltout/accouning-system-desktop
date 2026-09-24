# نظام المحاسبة ونقاط البيع — Desktop Accounting & POS

Arabic-first (RTL) accounting + POS + invoicing desktop app for a single retail/clothing shop —
multi-branch, multi-currency capable, with real PDF documents. Tauri v2 shell · Vue 3
(`<script setup>`) · Pinia · Vue Router · Tailwind CSS v4 · Zod.

**Still UI-only against a mock backend.** Every screen calls `modules/*/services/*`, which talk to
an in-memory "database" (`src/mocks/`) that behaves like a real one: double-entry postings, stock
movements, party balances, everything derived from the ledger. The mock DB persists to IndexedDB
(500ms after every write), so a company you set up survives a reload — it isn't re-seeded each
time. The one piece of real native code is PDF/print rendering, which runs in Rust inside the
Tauri shell (`src-tauri/src/pdf`, `src-tauri/src/print`) — everything else is UI-only by design, so
a real backend can replace the mock later without touching a single page.

See `docs/v2/README.md` for the full plan and `docs/v2/15-action-plan.md` for what shipped in each
phase. `docs/action_plan.md` is v1's history and is no longer current.

## Run

```bash
bun install
bun run dev          # browser at http://localhost:1420
bun run tauri dev    # desktop window (needed for real PDF export, thermal printing, native dialogs)
bun run build        # type-check (vue-tsc) + production build (vite)
```

On first run (no persisted data yet) you land on the **welcome screen**, with two ways to start:

- **ابدأ شركتك** (start your company) — an empty shell, then the 11-step onboarding wizard (chart
  of accounts template, company info, taxes, branches, opening balances, a first Excel import for
  customers/suppliers/products, users). Everything you enter is saved and survives a reload.
- **استكشف ببيانات تجريبية** (explore with demo data) — seeds ~75 days of realistic shop activity
  (products, customers, invoices, purchases, journal entries) so you can look around immediately.

In dev builds, the user menu / dev menu has a quick user switcher, a "reset data" action (wipes the
persisted snapshot back to a fresh install) and a latency switch (0 / realistic / slow), useful for
testing loading states.

### Demo accounts (seeded by "استكشف ببيانات تجريبية")

| User | Password | Role |
|------|----------|------|
| `admin` | `admin123` | مدير النظام — everything, including user management and backup restore |
| `manager` | `manager123` | مدير المتجر — everything except user management |
| `accountant` | `acc123` | محاسب — accounting, reports, payments, sales (write); purchases/inventory/parties read-only |
| `cashier` | `cashier123` | كاشير — POS & sales only; lands on the POS after login |
| `cashier2` | `cashier123` | كاشير — sells at the wholesale price list |
| `storekeeper` | `store123` | أمين مخزن — inventory, receiving, transfers, labels; no accounting or POS |

## Architecture

```text
src/
├── mocks/                 # the fake backend (only modules/*/services/* may import it)
│   ├── db.ts              # in-memory tables + document numbering
│   ├── persist.ts         # debounced IndexedDB snapshot save/load + schema migrations
│   ├── attachments.ts     # separate IndexedDB object store for attachment blobs
│   ├── events.ts          # ledger:changed / catalog:changed / parties:changed event bus
│   ├── backend/           # posting rules: sales, refunds, purchases, payments, stock, journal,
│   │                         vouchers, approvals, backup…
│   ├── fixtures/          # static seed data (chart of accounts templates, products, people)
│   └── seed/<area>.ts     # replays realistic shop activity through the posting rules, per area
├── modules/<module>/      # core, users, products, parties, accounting, invoices, purchases,
│   │                         payments, expenses, vouchers, reports, settings, setup, analytics,
│   │                         approvals, templates…
│   ├── services/          # the seam every page calls through
│   ├── pages/ components/ controllers/ helpers/ routes/ types/ commands.ts (command palette)
└── router/                # assembles every module's routes; role gating via route meta

src-tauri/
├── src/pdf/                # Typst-based PDF rendering (render_pdf, render_preview) + templates
├── src/print/               # native ESC/POS thermal printing (Windows queue + network)
└── templates/*.typ          # one Typst template per document kind (invoice, PO, voucher, labels…)

scripts/
├── verify/<area>.ts + run.ts     # ledger/stock invariant checks against a freshly seeded mock DB
├── e2e/flows/<area>.py + run.py  # Playwright flows per area, run in dependency order
├── screenshot.py                  # ad-hoc route screenshots (light/dark)
└── codemod-text-tokens.js         # the one-time px→token migration this codebase already ran
```

- **Services are the seam.** Pages only call `modules/*/services/*` functions shaped like a real
  API (`Promise<T>`, simulated latency, deep-cloned results, Arabic `ApiError` messages). Swapping
  the mock for a real backend later is a service-layer-only change; nothing in `pages/`,
  `components/` or `controllers/` needs to know.
- **Posting rules** (`src/mocks/backend/`) keep stock, the journal and every balance consistent:
  a sale posts cash/bank/AR ↔ sales + VAT output and COGS ↔ inventory; purchases, refunds,
  payments, stock adjustments, expenses and vouchers each post their own balanced entry. Accounts
  are resolved by **system role** (`accountFor('cash', …)`), never a hard-coded code, so an
  accountant can renumber the chart of accounts freely. Reports and party balances are computed
  from the ledger, not from summing open documents.
- **Event bus** (`src/mocks/events.ts`): `ledger:changed` / `catalog:changed` / `parties:changed`
  fire after a mutating call. Pinia master-data caches (products + barcode index, parties,
  accounts, the insight engine's cache) subscribe instead of refetching on every page visit.
- **Persistence** (`src/mocks/persist.ts`): every table is snapshotted to IndexedDB 500ms after a
  write. Attachment blobs live in a separate object store (`src/mocks/attachments.ts`) so they
  don't ride along with every snapshot write.
- **Insight engine** (`modules/core/services/insightEngine.ts`): a rule catalogue (low stock,
  overdue customers, VAT deadline, backup overdue, recurring entries due, …) evaluated against the
  live mock data, cached and invalidated on the events above. Surfaces on the home page ("يحتاج
  انتباهك"), the notifications drawer, report insight boxes and inline hints.
- **Roles** are presets (`modules/users/helpers/permissions.ts`: admin, manager, accountant,
  cashier, storekeeper), gating routes, the sidebar and actions; an admin can adjust a preset's
  access per area from Settings → Users & roles.
- **Command palette** (Ctrl+K, `modules/core/controllers/useCommandPalette.ts`): every module
  registers its own `commands.ts` (search providers + context-aware commands), assembled once in
  `main.ts`.
- **Approvals:** a discount over the cashier's limit, a stock write-off above the configured
  threshold, or a price below cost normally goes through a synchronous manager-PIN dialog right at
  the point of action. When no manager is physically present, each of those dialogs offers an
  async escape hatch that queues the request instead — a manager decides it later from `/approvals`
  or the notifications bell.
- **Printing:** real PDFs (invoice, quotation, credit/debit note, PO, vouchers, statements,
  Z-reports, transfer notes, labels, generic reports) render through Rust/Typst in Tauri, with a
  browser print-dialog fallback in dev mode. Thermal receipts print natively via ESC/POS (Windows
  queue or network) when a thermal printer is configured; otherwise the A4 route is used.

## Checks

```bash
bun run build                                # type-check + production build
bun run check                                 # grep guard against new hard-coded text-[Npx] sizes
bun run verify:mocks                          # ledger/stock/party-balance invariants (bun run scripts/verify/run.ts)
cargo build --manifest-path src-tauri/Cargo.toml   # Rust PDF/print modules

python scripts/e2e/run.py [<out_dir>] [--only <area>] [--base URL]   # every e2e flow, in dependency order
python scripts/screenshot.py <out_dir> / /pos /reports/trial-balance --dark
```

`scripts/e2e/run.py` discovers every `scripts/e2e/flows/<area>.py` (cashier POS, desk invoice, role
gating, accountant journal, refunds, products, purchases, expenses, branches/currencies, reports,
labels/templates, home & insights, onboarding, and a final consolidated `full_persona_pass` that
walks a cashier → manager → storekeeper → accountant day in one run) and runs them in a fixed order
so later flows can rely on state earlier ones left behind. Both Python scripts need the dev server
running and Python Playwright installed (`PYTHONIOENCODING=utf-8` on Windows/Git Bash if you see
encoding errors on the Arabic output).

`scripts/e2e_flows.py` (flat, single-file) is v1-era and superseded by `scripts/e2e/flows/` — use
the latter.
