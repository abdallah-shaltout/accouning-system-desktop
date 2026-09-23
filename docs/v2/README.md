# Plan v2 — From Demo Shop to Real Business Tool

**Status:** planning (2026-09-23). v1 (`docs/action_plan.md`) is finished and stays as history.
**Still UI-only.** Every feature here runs against the mock backend in `src/mocks/`. Pages only
call `modules/*/services/*`, so a real backend can still replace the mock later without touching screens.
**One exception:** PDF rendering runs in Rust inside the Tauri shell, because you asked for real PDFs
(see [12-documents-pdf-excel.md](12-documents-pdf-excel.md)). It's a local desktop feature with no server
involved.

## Why v2

v1 proved the screens, the double-entry mock and the RTL design. Going through the app as a
cashier, storekeeper, accountant and owner showed what it still needs before a real shop can start
using it:

- **Starting up:** no setup wizard, no opening balances, no way to bring over customers and suppliers
  with their existing balances.
- **Structure:** one shop, one currency and one VAT rate. There are no branches, cost centers or
  payment-method accounts.
- **Accountants:** they need a full invoice form, not only the POS. The journal pages need to be much
  stronger, with attachments and properly posted opening entries.
- **Products:** pharmacy-style units (box → strip), tax category and GL accounts per product, expiry
  dates, and printed barcode labels.
- **Documents:** real PDFs, customizable invoice templates with a logo, and Excel export/import
  everywhere.
- **Guidance:** the dashboard should be simpler, and the app should tell each role *what to do next*.
- **Correctness:** an accounting review of v1 found real bugs (see [02](02-accounting-review.md)).

## Files

| # | File | What it covers |
|---|------|----------------|
| 01 | [personas.md](01-personas.md) | Walking through the app as cashier, storekeeper, accountant and owner; what each is missing; role matrix v2 |
| 02 | [accounting-review.md](02-accounting-review.md) | **Review of every v1 posting rule**: bugs, fixes, corrected rules, invariants to verify |
| 03 | [chart-of-accounts.md](03-chart-of-accounts.md) | New CoA tree (headers + postable accounts), system roles, templates |
| 04 | [domain-model.md](04-domain-model.md) | New and changed entities, with field lists |
| 05 | [onboarding.md](05-onboarding.md) | Setup wizard, opening entry, opening balances for parties and stock, Excel import |
| 06 | [sales-and-pos.md](06-sales-and-pos.md) | POS upgrades, full invoice form, pricing/discount/VAT math, returns, shifts |
| 07 | [products-and-inventory.md](07-products-and-inventory.md) | Units, taxes, accounts, batches/expiry, branch stock, transfers, labels |
| 08 | [customers-and-suppliers.md](08-customers-and-suppliers.md) | Full party profile, party page, phone input, opening balance, statements |
| 09 | [purchases-payments-expenses.md](09-purchases-payments-expenses.md) | Purchase invoice v2, payment methods, allocations, expenses, vouchers |
| 10 | [branches-currencies-cost-centers.md](10-branches-currencies-cost-centers.md) | Branches, multiple currencies and cost centers |
| 11 | [journal-dashboard-insights.md](11-journal-dashboard-insights.md) | Journal redesign, simpler home, analytics, recommendations engine |
| 12 | [documents-pdf-excel.md](12-documents-pdf-excel.md) | Typst PDF engine, template designer, labels, thermal receipts, Excel |
| 13 | [reports.md](13-reports.md) | Report catalogue v2 |
| 14 | [platform.md](14-platform.md) | Command palette, appearance settings, backup/restore, attachments, speed |
| 15 | [action-plan.md](15-action-plan.md) | **Phased checklist**: the file to execute from |

Read order for an implementer: 02 → 03 → 04 → 15, then the feature file for the phase in progress.

## Scope changes vs v1

v1's `project_specs.md` §6 cut several features as "not for a single shop". You have now asked for
them, so they come back in scope, each in a deliberately smaller form:

| v1 cut | v2 decision |
|--------|-------------|
| Multi-branch | **In.** Stock per branch, branch on every document, transfers, branch reports. One stock location per branch (no separate warehouse entity). |
| Cost centers | **In, optional.** A dimension on journal lines. Each branch gets one automatically. |
| Payment allocation | **In.** One receipt can settle several invoices; unallocated money stays on the party's account. |
| Credit notes | **In.** Sales returns become إشعار دائن, with a reason and a link to the original invoice. |
| CoA tiers / internalCode | **In, simplified.** Templates are chosen at onboarding; posting rules use *system roles*, not codes. |
| Multi-currency | **New.** Base currency plus foreign-currency documents, exchange rates and realized FX gains/losses. |

Still out of scope: multi-tenancy/subscriptions, ZATCA Phase-2 integration (the data fields are
reserved for it), sending messages through WhatsApp/SMS APIs (links such as `wa.me` are allowed),
server sync, a fixed-asset register (depreciation is done with a recurring journal template), and payroll.

## Decisions made without asking

Per your instruction to pick the recommended option on anything that isn't scope-changing. Any
of these can be reversed; each is explained in the file linked.

1. **The mock DB persists to IndexedDB.** A new company, opening balances and restored backups
   survive a reload. First run offers *start a new company* (runs the wizard) or *explore demo data*
   (today's seed). There's a dev-menu action to reset. → [14](14-platform.md)
2. **Posting rules resolve accounts by system role** (`cash`, `receivable`, `inventory`…), not
   by code, so accountants can renumber and extend the CoA freely. → [03](03-chart-of-accounts.md)
3. **Journal lines carry a party** (customer/supplier) plus branch, cost center and currency.
   Party balances come from the ledger, not from summing open documents. This is what lets opening
   balances and on-account payments work. → [02](02-accounting-review.md)
4. **VAT is per line with a tax category** (standard 15% / zero-rated / exempt / out of scope) and
   rounded per line. **Retail prices include VAT by default** (Saudi consumer-pricing rule), with a
   store setting to change it. → [06](06-sales-and-pos.md)
5. **Discounts always reduce the VAT base**, never the VAT itself. The order is line discount →
   invoice discount (spread across lines in proportion) → VAT. → [06](06-sales-and-pos.md)
6. **Each customer or supplier has one currency.** Their documents are in that currency; the ledger
   is in the base currency and each line also stores the foreign amount. → [10](10-branches-currencies-cost-centers.md)
7. **Weighted average cost, stored to 4 decimals.** Returns re-average the cost, so the inventory
   account always equals Σ qty × average cost. → [02](02-accounting-review.md)
8. **Stock is kept in the smallest unit** (strip/tablet/piece); boxes and cartons are conversions
   with their own barcode and price. → [07](07-products-and-inventory.md)
9. **Opening balances post against "أرصدة افتتاحية" (3900)**, which is closed to capital at the end
   of onboarding. → [05](05-onboarding.md)
10. **New "storekeeper" (أمين مخزن) role preset.** Presets stay; admins can adjust a role's access
    matrix. → [01](01-personas.md)
11. **Libraries:** Typst (Rust) for PDFs, `exceljs` for .xlsx, `libphonenumber-js` for phone numbers,
    `bwip-js` for barcodes, `fflate` for backup archives. Each is lazy-loaded where it's heavy.
12. **The font-size setting scales text through rem tokens.** This requires replacing the 167
    hard-coded `text-[13px]`-style sizes found in 55 files. → [14](14-platform.md)
