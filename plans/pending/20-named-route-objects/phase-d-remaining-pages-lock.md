# 20.D — Remaining pages, then lock the rule in

**Status: pending.** Depends on 20.C.

Goal: the last string targets go, and then the rule is enforced by types **and** by
`bun run check`, so it can't come back.

## Tasks

### D1 — products / inventory
- [ ] `ProductListPage` [3] (`product-new`, `adjustment-new` + `query.type`), `ProductDetailPage` [5]
  (`labels` + `query.productId`, `adjustment-new` + `query: { type: 'STOCK_IN', product }`, `product-edit`),
  `ProductFormPage` [5], `LabelBuilderPage` [1], `ExpiryReportPage` [1].
- [ ] `StockAdjustmentListPage` [4], `StockAdjustmentFormPage` [2], `StockAdjustmentDetailPage` [4],
  `StockCountListPage` [2], `StockCountNewPage` [2], `StockCountDetailPage` [2].

### D2 — accounting
- [ ] `JournalListPage` [5]: draft → `{ name: 'journal-new', query: { draft: row.id } }`, else `journal-entry`;
  day-book print at line 307 → `{ name: 'report-day-book', query }`.
- [ ] `JournalDetailPage` [7] (`journal-new` + `query.duplicate`, reversal links, `report-ledger` + `query.account`),
  `JournalEntryFormPage` [3], `JournalTemplatesPage` [1], `ChartOfAccountsPage` [1], `FiscalYearsPage` [1], `VatSettlementPage` [1].

### D3 — reports
- [ ] `ReportShell` [1] (`back` → `reports`), `StatementSection` [1], `TrialBalancePage` [1]
  (`{ name: 'report-ledger', query: { account, from, to } }`), `DayBookPage` [1], `AgingReportPage` [1],
  `OverdueReportPage` [1], `PurchasesReportPage` [2], `SalesReportPage` [1], `ShiftsReportPage` [1],
  `StockHealthReportPage` [2], `StocktakeVariancesPage` [1], `TransfersReportPage` [1], `InventoryReportPage` [1].

### D4 — core (dashboards and shell)
- [ ] `AccountantHome` [10], `CashierHome` [6], `StorekeeperHome` [11] (`{ name: 'products', query: { stock: 'low' } }`, …),
  `DashboardPage` [6], `AppTopbar` [1], `NavUser` [4] (`settings-backup`, `settings-appearance`,
  `dev-diagnostics`), `ForbiddenPage` [1], `NotFoundPage` [1].
- [ ] Noted, not fixed here: `AccountantHome.vue:46,49` and `DashboardPage.vue:130` link to hard-coded
  account ids (`acc-1110`, `acc-1120`). Only the link *form* changes in this plan. Resolving
  system-role accounts is accounting logic (CLAUDE.md "Accounting safety").

### D5 — settings, templates, users, setup
- [ ] `ProductsSettingsPage` [1], `TemplateListPage` [3], `TemplateDesignerPage` [5] (`settings-template-designer` + id, `settings-templates`, `settings-general`).
- [ ] `UserListPage` [2] (`/users/new` → `{ name: 'user-editor', params: { id: 'new' } }`, Decision 10), `UserEditorPage` [3], `WelcomePage` [2] (`login`, `setup-wizard`).
- [ ] `SetupWizardPage` [1] (`login`), `SetupChecklistCard` [1] (`setup-wizard`).

### D6 — Lock
- [ ] `node scripts/check-routes.js` → **0** findings in the whole app (only `route-ok` lines left:
  `LoginPage` redirect).
- [ ] Narrow shared props to `AppRoute` (Decision 6): `AppButton.to`, `KpiCard.to`, `PageHeader.back`.
  `vue-tsc` must stay green, which proves no string target is left in those props (including
  variables the guard can't see).
- [ ] `package.json` `check`: add `node scripts/check-routes.js` after `check-rtl.js`.
- [ ] `CLAUDE.md` rule 25: change "(it joins `bun run check` in plan 20.D)" to "(part of `bun run check`)".
  In the Definition of done comment for `bun run check`, add "route-object guard".
- [ ] `docs/design_system.md` (shared components section): note that `AppButton :to`, `KpiCard :to`
  and `PageHeader :back` take an `AppRoute` object, never a path.

### D7 — Finish the plan
- [ ] Every phase status `done`, `git mv plans/pending/20-named-route-objects plans/completed/20-named-route-objects`,
  fix the link in `docs/v2/README.md`, `bun run memory`.
- [ ] If the real `bun run desktop` check could not be done, leave the folder in `pending/` and write
  what is missing at the top of the README (CLAUDE.md "Plans").

## Gate
- [ ] `bun run check` includes `check-routes.js` and is green. Adding `router.push('/invoices')`
  anywhere fails it. Revert.
- [ ] Passing `to="/pos"` to `AppButton` fails `vue-tsc`. Revert.
- [ ] Full click-through: every sidebar item, quick action, dashboard card and button for each role
  (admin, manager, accountant, cashier, storekeeper), back buttons on every detail page, report drill-downs
  (trial balance → ledger, stock health → product, aging → party).
- [ ] `bun run build`, `bun run check`, `bun run verify:mocks`, `bun run memory:check`, full e2e suite,
  no console errors, light + dark at 1280/1920 px, RTL, keyboard.
