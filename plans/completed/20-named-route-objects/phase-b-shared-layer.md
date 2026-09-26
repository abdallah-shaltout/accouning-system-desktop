# 20.B — Shared layer: navigation config, palette, services, guards

**Status: done** (2026-09-26). Build green, `bun run check` green. Manual click-through of sidebar
highlighting, settings tabs and the palette not done this session — verify before relying on it.

Goal: every **shared** place that stores or returns a destination holds an `AppRoute`, and its type
says so. Pages that only *pass through* these values (`:to="row.sourceLink"`,
`router.push(n.actionTo)`) need no edits.

Convert one-to-one: same route, same params, same query. Names come from the Routes table in
`AGENT_MEMORY.md` (or the generated map).

## Tasks

### B1 — Sidebar and quick actions (`core/helpers/navigation.ts`, 34 sites)
- [ ] `NavItem.to` and `QuickAction.to` → `AppRoute`. Every entry becomes `{ name: '…' }`
  (`"/"` → `home`, `"/pos/shifts"` → `pos-shifts`, `"/sales/invoices/new"` → `invoice-new`, …).
- [ ] `"/settings"` is a nameless redirect record today. Give it a name in
  `settings/routes/index.ts`: `{ path: '/settings', name: 'settings', redirect: { name: 'settings-general' } }`.
  The sidebar item becomes `{ name: 'settings' }`. That keeps the item's base path `/settings`, so
  every `/settings/*` page still highlights it (a `settings-general` target would only match
  `/settings/general`).
- [ ] `NavMain.vue`: `isActive` compares against `router.resolve(item.to).path`, computed once per
  item (not per render). The exact/prefix logic stays the same. `:key` uses `item.label`
  (unique per group) instead of the path string.
- [ ] `NavQuickActions.vue`: `:key="action.label"`.

### B2 — Settings tabs (`settings/components/SettingsTabs.vue`, 15 sites)
- [ ] Each tab `to: { name: 'settings-…' }`. The active class uses `route.name === t.to.name`
  instead of `route.path === t.to`. `:key="t.to.name"`.

### B3 — Command palette
- [ ] `core/types/commandPalette.ts`: `PaletteResult.to?: AppRoute`.
- [ ] `core/commandPalette/exampleProviders.ts` (6): `buildPageCommands` maps `to: { name: r.name as RouteName }`
  (the filter already requires `r.name`). Static and provider results use names
  (`pos`, `customers`, `settings-general`, `customer`/`supplier`/`product` with `params.id`).
- [ ] `core/controllers/useCommandPalette.ts:193`: barcode hit → `{ name: 'product', params: { id: product.id } }`.
- [ ] Module `commands.ts` files:
  - [ ] `accounting/commands.ts` (4): `journal-entry` + id, `report-ledger` + `query.account`, `journal-new`, `accounts`.
  - [ ] `approvals/commands.ts` (1): `approvals`.
  - [ ] `invoices/commands.ts` (4): `invoice` + id, `invoice-print` + id, `invoice-refund` + id, `invoice-new`.
  - [ ] `purchases/commands.ts` (5): `purchase`, `purchase-print`, `purchase-receive`, `purchase-return` (+ id), `purchase-new`.
  - [ ] `vouchers/commands.ts` (4): `voucher-detail`, `voucher-print` (+ id), `voucher-new`, and
    `'/payments/new?type=RECEIVED'` → `{ name: 'payment-new', query: { type: 'RECEIVED' } }`.
  - [ ] `reports/commands.ts` (28): the list holds `{ title, to: { name: 'report-…' } }`
    (`'/inventory/expiry'` → `expiry`). Command id → `report:${to.name}` (Decision 12).
  - [ ] `settings/commands.ts` (16): same pattern. Id → `settings:${to.name}`.
  - [ ] `diagnostics/commands.ts` (1): `DEV_PAGES` type → `{ title: string; to: AppRoute }[]`, `{ name: 'dev-diagnostics' }`.
- [ ] `reports/pages/ReportsHubPage.vue` (28): card `to` → `{ name: 'report-…' }`, same names as
  `reports/commands.ts`. (The two lists repeat each other. Merging them is a separate change, see Later.)

### B4 — Services and controllers that return destinations
- [ ] `accounting/services/accountingService.ts`: `sourceLink()` returns `AppRoute | undefined`
  (`invoice`, `purchase` + id; `payments` + `query.highlight`; `adjustment` + id). `JournalRow.sourceLink?: AppRoute`.
  Readers (`JournalDetailPage.vue:190`, `JournalListPage.vue:423`) pass it to `:to` unchanged.
- [ ] `products/services/inventoryService.ts`: `refLink()` returns `AppRoute | undefined`. Update the
  return type of `getStockMovements` and `StockMovementRow.refLink`. Readers `ProductDetailPage.vue:134`
  and `StockMovementsPage.vue:78` stay as they are.
- [ ] `core/services/insightTypes.ts`: `actionTo: AppRoute`. Then `insightRules.ts` (22) and
  `insightEngine.ts` (3): `'/purchases/new'` → `{ name: 'purchase-new' }`,
  `` `/products/${id}/edit` `` → `{ name: 'product-edit', params: { id } }`,
  `{ path: '/products', query: { stock: 'low' } }` → `{ name: 'products', query: { stock: 'low' } }`, …
- [ ] `core/controllers/useNotifications.ts` (3 + type): `actionTo: AppRoute`
  (`` `/inventory/transfers?highlight=${t.id}` `` → `{ name: 'transfers', query: { highlight: t.id } }`,
  `approvals`, `settings-backup`). Stored read-state keys are notification ids, not routes, so
  nothing stored changes.
- [ ] `approvals/types/index.ts`: `link?: AppRoute` (both places). No caller sets `link` today
  (checked: `src/mocks/backend/approvals.ts:30` only copies `input.link`), so nothing stored has to move.

### B5 — Router guards and route-record redirects
- [ ] `src/router/index.ts:78`: `return { path: '/' }` → `return { name: 'home' }`.
- [ ] `settings/routes/index.ts:6`: done in B1 (`redirect: { name: 'settings-general' }`).
- [ ] `accounting/routes/index.ts:65` (`day-book`): `redirect: (to) => ({ name: 'report-day-book', query: { ...to.query, print: '1' } })`.
- [ ] `users/pages/LoginPage.vue:38-40`: a `?redirect=` value (a fullPath the guard saved) is replayed
  as-is with `/* route-ok: auth-guard fullPath round-trip */`. With no redirect →
  `auth.role === 'cashier' ? { name: 'pos' } : { name: 'home' }`. Same behavior: the guard never
  saves `redirect` for `/`.

## Gate
- [ ] `node scripts/check-routes.js` shows **0** findings in every file listed above.
- [ ] Sidebar: open a list, a detail (`/invoices/:id`), a `new` and an `edit` page in each group.
  The same item highlights as before. Every `/settings/*` page highlights "الإعدادات".
- [ ] Settings tabs highlight the current tab. The palette opens pages, reports, settings, records
  (customer/product/invoice) and the barcode hit. Recents still list.
- [ ] Bell notifications and dashboard insight actions go to the same screens as before.
- [ ] `bun run build`, `bun run check`, `bun run verify:mocks`, `bun run memory` (the `settings` route
  got a name, so it is a route change), full e2e suite, no console errors.
