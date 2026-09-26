# 20 — Named route objects everywhere (no path strings)

**Status: planned** (2026-09-26). Nothing below is started yet.

**Before starting:** the working tree holds ~45 uncommitted files from plan 18 (18.B diagnostics:
`SettingsTabs.vue`, `PosPage.vue`, `ExpenseListPage.vue`, `navigation.ts`, `src/mocks/backend/*`…).
This plan edits many of the same files. Commit (or finish) that work first so each 20.x commit
only holds route changes.

Triggered by a user decision: every Vue navigation target must be a **route object**
(`{ name, params, query, hash }`), never a path string like `` `/invoices/${id}` `` and never
`{ path: '/…' }`. A survey of the code (2026-09-26) found **~400 string targets in ~100 files** and
**zero** named-object navigations outside the auth guard:

| Kind of site | Count | Example |
|---|---|---|
| `router.push/replace('…')` / `` (`…`) `` / ternaries | ~75 | `router.push(`/invoices/${r.id}`)` (`InvoiceListPage.vue:280`) |
| `{ path: '/…' }` objects (push, `:to`, `actionTo`) | 16 | `{ path: '/payments/new', query: { type: 'PAID' } }` (`PaymentListPage.vue:77`) |
| Template `to="/…"` / `` :to="`…`" `` (`RouterLink`, `AppButton`, `KpiCard`) | ~100 | `<AppButton to="/pos">` (`AppTopbar.vue:23`) |
| `PageHeader back="/…"` | 28 | `back="/invoices"` (`InvoiceDetailPage.vue:56`) |
| Config `to: '/…'` (sidebar, quick actions, settings tabs, reports hub, palette commands) | ~170 | `navigation.ts` (34), `reports/commands.ts` (28), `ReportsHubPage.vue` (28), `SettingsTabs.vue` (15) |
| Link helpers returning paths (services + pages) | 13 | `accountingService.sourceLink`, `inventoryService.refLink`, `PartyDetailPage.docLink` |
| `actionTo` in the insight engine / notifications | ~25 | `insightRules.ts` (22) |
| Route-record `redirect` strings / guard `{ path: '/' }` | 3 | `settings/routes/index.ts:6`, `router/index.ts:78` |
| Path inside a query value (`?back=/pos`) | 3 | `PosPage.vue:505` |

**Why it matters (found in the code, not guessed):**
- A path string is not checked by anything. A typo or a renamed route fails only at run time, as a
  silent `not-found` page.
- Paths are duplicated: `/reports/…` exists in `reports/routes`, `reports/commands.ts` **and**
  `ReportsHubPage.vue`. The route record should be the only owner of a URL (CLAUDE.md "Centralize
  configuration").
- vue-router **5.3.1** (installed) supports typed routes (`TypesConfig.RouteNamedMap`). With a
  generated map, `vue-tsc` rejects an unknown route name or a missing `:id`. That only works for
  **named object** targets.

## Decisions

| # | Decision | Why |
|---|---|---|
| 1 | The only allowed target form is a **named location**: `{ name, params?, query?, hash? }`. No strings, no `{ path }`. Same-page query/hash updates may leave out `name` (`router.replace({ query: { ...route.query, x } })`, 8 existing sites such as `useReportRange.ts:26`), because they already are objects that keep the current route. | User decision; makes every target type-checkable. |
| 2 | **Generate** a typed `RouteNamedMap` into `src/router/route-map.gen.d.ts` from the route records, as a second output of `bun run memory` (reusing `scripts/memory/parse/routes.ts`). `bun run memory:check` also fails when it is stale. | The route records stay the single owner. CLAUDE.md already requires `bun run memory` after any route change, so no new habit is needed. A hand-written map would drift. |
| 3 | No `ROUTES = { … }` constants file. Route names are string literals checked by the generated map (with editor autocomplete). | A constants layer would duplicate the route records (rule "Centralize configuration"). |
| 4 | One project type, `AppRoute`, in `src/modules/core/types/route.ts` = vue-router's `RouteLocationAsRelative<N>` with `name` **required**. Every prop or field that holds a destination uses it: `AppButton.to`, `KpiCard.to`, `PageHeader.back`, `NavItem.to`, `QuickAction.to`, `PaletteResult.to`, notification/insight `actionTo`, `JournalRow.sourceLink`, `StockMovementRow.refLink`, approvals `link`. | vue-router's own `RouteLocationRaw` still accepts strings; our narrower type makes `vue-tsc` catch string targets that a regex guard can't see (variables). |
| 5 | Turning on the typed map changes `route.params` typing. Pages that read params switch to `useRoute('<route-name>')` (26 files, listed in Phase A) **in the same phase**, so the build stays green. | Measured: `useRoute()` becomes a union of all routes, where `.params.id` does not exist on routes without `:id`. |
| 6 | Narrowing `AppButton.to` / `KpiCard.to` / `PageHeader.back` from `RouteLocationRaw` to `AppRoute` is an **intentional API change** to shared components (CLAUDE.md UI rule 4 says keep `App*` APIs stable). It is done **last** (Phase D), after every call site is migrated, so no page breaks in between. | Rule 4 exists to stop surprise breakage; here the break is the point, and it is staged. |
| 7 | New guard `scripts/check-routes.js`, joins `bun run check` in Phase D (once the count is 0). Escape hatch: `/* route-ok: <reason> */` on the line, same style as `rtl-ok`. | Types can't stop `router.push('/x')` (vue-router accepts strings), so a lint guard is needed for literals. |
| 8 | **Stays a string, on purpose:** route-record `path:` definitions; `to.fullPath` saved in `?redirect=` by the auth guard and read back by `LoginPage` (a URL round-trip); e2e URLs (URLs do not change, so `scripts/e2e` needs no edits); `ActivityEntry.link` / `AuditEntry.link` in the mock backend (stored data, only parsed by `entityFromLink` in `src/mocks/backend/core.ts`, never navigated by the UI, and present in old backup files). | These are URLs as *data*, not navigation targets in Vue code. |
| 9 | The print page's `?back=<path>` query becomes `?from=pos`. `InvoiceFormPage` and `PrintingSettingsPage` pass exactly the default target already (`/invoices/:id`, `/settings/printing`), so only POS needs a non-default back. | Removes the only path-in-a-query. Behavior is the same. |
| 10 | `/users/new` becomes `{ name: 'user-editor', params: { id: 'new' } }`. No new `user-new` route. | Same behavior: `UserEditorPage.vue:31` already treats `id === 'new'` as "create". A separate route is out of scope (see Later). |
| 11 | Sidebar active state: `NavMain` resolves each item once with `router.resolve(item.to).path` and keeps today's prefix match. Settings tabs compare `route.name`. | Routes are flat, so `RouterLink`'s built-in `isActive` would not highlight "الفواتير" on `/invoices/:id`. Today's behavior must not change. |
| 12 | Palette command ids built from a path (`settings:/settings/general`, `report:/reports/…`) become name-based (`settings:settings-general`). | Ids must not depend on URLs. Stored recents with old ids are dropped silently (`useCommandPalette` filters unknown ids), so a user loses at most a few recent entries once. |

## Phases

| Phase | File | What | Size | Status |
|---|---|---|---|---|
| A | [phase-a-typed-map.md](phase-a-typed-map.md) | CLAUDE.md rule, generated `RouteNamedMap`, `AppRoute` type, typed `useRoute('<name>')` in 26 files, `check-routes.js` (report mode) | M | pending |
| B | [phase-b-shared-layer.md](phase-b-shared-layer.md) | Shared config and services: sidebar, quick actions, settings tabs, palette commands, reports hub, insight/notification `actionTo`, service link helpers, guards and redirects | M | pending |
| C | [phase-c-document-pages.md](phase-c-document-pages.md) | Pages: invoices/POS, purchases, payments, vouchers, expenses, parties (plus print `?back` → `?from`) | M | pending |
| D | [phase-d-remaining-pages-lock.md](phase-d-remaining-pages-lock.md) | Pages: products, accounting, reports, core dashboards/shell, settings, templates, users, setup. Then **lock**: narrow `App*` prop types, `check-routes.js` into `bun run check` | M | pending |

**Why this order.** A makes route names type-checked before anything uses them, so every later
edit is checked by `vue-tsc` as it is written. B migrates the shared layer that many pages read
(prop and field types), so C and D are mechanical page edits. D ends with the lock, so the rule is
enforced from then on.

**Agent instructions** (same rules as [15](../../../docs/v2/15-action-plan.md) and
[17](../../../docs/v2/17-ui-system-rtl-themes.md)):
- UI-only against the mock backend. **URLs do not change.** Only how Vue code *names* them changes.
- Keep behavior exactly the same: same destination, same query, same back target. This is a refactor.
- Tick boxes as you go. Set each phase's Status to `done` and add a status note at the top of that
  phase file when finished. One commit per phase.
- Run the **whole** e2e suite at the end of each phase. Navigation is used by every flow.
- When D is green, move this folder to `plans/completed/`, set every status to `done`, update the
  row in [`docs/v2/README.md`](../../../docs/v2/README.md), and run `bun run memory`.

**Definition of done (every phase):** `bun run build`, `bun run check`, `bun run verify:mocks`
(no money logic changes, it must stay all OK), `bun run memory` (A adds a generated file and a
script; route changes regenerate the map), the full e2e suite with no console errors, and a manual
click-through: sidebar highlight on list **and** detail pages, back buttons, palette navigation,
light + dark at 1280/1920 px, RTL, keyboard.

## Risks

| Risk | Mitigation |
|---|---|
| The route parser misses a record (for example the dev-only spread in `core/routes/index.ts`), so a valid name fails `vue-tsc` | Phase A gate: the number of names in the generated map equals the number of named routes in `router.getRoutes()` in a dev build. A miss fails loudly at build time, never silently. |
| Typed `useRoute()` breaks pages mounted by several routes (`PartyFormPage`: `customer-new`/`customer-edit`/`supplier-new`/`supplier-edit`) | Use `useRoute<'customer-edit' \| 'supplier-edit' \| …>()` and read `'id' in route.params ? route.params.id : undefined`. Listed per file in Phase A. |
| Sidebar or settings-tab highlight changes | Decision 11 keeps the same matching logic. Phase B gate checks list, detail, `new` and `edit` pages in every group. |
| A behavior change hides in a "mechanical" edit (lost query key, wrong back target) | Each phase lists every site with its current string. Convert one-to-one. e2e covers create → detail → print → back in sales, purchases, payments and vouchers. |
| Conflicts with plan 18 (in progress, same files) | Commit 18's working tree first (top of this file). 18.C–G do not touch navigation targets. New code they add follows rule 25 from Phase A on. |

## Later (not in this plan)

- A real `user-new` route instead of `user-editor` with `id: 'new'`.
- `reports/commands.ts` and `ReportsHubPage.vue` list the same 28 reports twice (titles and
  targets). One shared list in `reports/helpers` would remove the copy. It is a separate change:
  the hub also has descriptions, icons and `requires` flags.
- The mock backend stores `link: '/…'` strings in `db.activity` / `db.audit` and guesses the entity
  from the path (`entityFromLink`, `src/mocks/backend/core.ts:285`). A real backend should store
  `{ entity, entityId }` and let the UI map it to an `AppRoute`. That changes persisted data and
  backup files, so it belongs with the real-backend work, not here.
