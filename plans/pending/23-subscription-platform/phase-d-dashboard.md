# Phase D — `apps/dashboard` (admin console + customer portal)

> **Status (2026-09-30): D1 done, D2/D3 pending.** `apps/dashboard` scaffolded: Vite+Vue3+TS strict,
> Pinia, Vue Router (history), Tailwind v4 + real shadcn-vue CLI-installed primitives, Zod +
> vee-validate, TanStack Table, axios. `bun run build` and `bun run check` both green. Two real
> (not placeholder) pages — admin login+TOTP and portal login — verified against the live
> `apps/backend` (real HTTP calls, real error-shape handling; a genuine validation-error-shape bug
> was found and fixed by testing live rather than assuming). D2 (admin modules) and D3 (portal
> modules) are separate, later phases — see Deviations.

## Deviations

- **Login pages verified against the live backend, not assumed.** The build agent read the actual
  `apiError.ts`/`globalErrorHandling.ts`/`response.core.ts` source and a live server's real
  responses rather than guessing the envelope shape, and caught a real bug this way: validation
  errors arrive as a raw Zod-issues array (`[{path, message}]`), not a field-keyed map — fixed in
  `shared/api/errors.ts`'s `fieldErrorsFromIssues`. Re-confirmed by the phase manager afterward with
  a direct `curl` against `/api/admin/auth/login` with a wrong password — response shape matches.
- **No browser tool was available in either the scaffolding agent's or the manager's session** (wmux
  browser reported "workspace identity unknown" both times). Verified instead via: `vue-tsc`/`vite
  build` clean, the `check` guard script clean, `curl` against the dev server confirming
  `dir="rtl" lang="ar"` and the Arabic `<title>`, and a live round-trip against the real backend's
  auth endpoints. The TOTP step's actual rendered UI was not seen by a human or a browser tool this
  session — flag this if a visual regression shows up once someone opens it in a real browser.
- **A lightweight `src/modules/core/` module** (not in `docs/03-architecture.md`'s module table)
  holds the two login pages and placeholder home/404 pages, mirroring the desktop app's own
  "shell-level, no clear owner-module" convention — `docs/03-architecture.md`'s module table doesn't
  list an owner for a bare `/admin` or `/portal` landing page, and "no new top-level folders under
  `src/`" ruled out anything outside `modules/`. D2/D3 should fold these into their real modules
  (`auth`, `organizations`, etc.) as those get built, rather than leaving `core` as a permanent
  catch-all.
- **`vue-router`'s query type doesn't allow `boolean`** — `AppRoute.query` is typed
  `Record<string, string | number | (string|number)[] | null | undefined>`, narrower than what the
  D1 task text implied, because `vue-router`'s real `LocationQueryRaw` rejects `boolean` and the
  build only passed once matched to it.

The structure is exactly `apps/dashboard/docs/03-architecture.md`. The design language is the same "same soul" as the desktop app: Arabic, RTL-first,
light and dark, calm, one primary action per view, and the design tokens from `desktop-app/src/assets/styles/design-system.css`
copied as the starting token set (not imported, since these are separate projects, D1).

## D1 — Scaffold and shared

- [x] `apps/dashboard`: Vite + Vue 3 `<script setup>` + TS strict + Pinia + Vue Router (history mode; this is a
      website, not a Tauri app) + Tailwind v4 + shadcn-vue (reka-ui) + Zod + `vee-validate` with the Zod adapter +
      `@tanstack/vue-table`. Independent like `apps/landing` (own `package.json`, no workspaces).
- [x] `shared/styles`: tokens (light/dark), fonts, `dir="rtl"` root, and reka `ConfigProvider dir="rtl"`.
- [x] `shared/components/ui`: add the shadcn primitives (button, input, select, dialog, sheet, dropdown, table,
      badge, card, tabs, toast, form, skeleton).
- [x] `shared/components/app`: `AppButton`, `FormField`, `DataTable` (server pagination/sort/filter matching the
      server's `ApiFeatures` query), `PageHeader`, `MoneyText` (piasters → «299 ج.م»), `StatusBadge`, `EmptyState`,
      `ConfirmDialog`.
- [x] `shared/api/http.ts`: an axios instance on `import.meta.env.VITE_API_URL` (the only env var, D17), with
      `withCredentials` for the refresh cookie. The access token stays in memory only. On 401 there is one
      single-flight `POST /<realm>/auth/refresh` (the cookie), then one retry. On refresh failure, the session is
      cleared and the user goes to login. Errors map to Arabic toasts.
- [x] `shared/auth`: a session store per realm, guards (`meta.area` admin/portal, `meta.permission`), and `useCan()`.
- [x] `shared/layouts`: `AdminLayout` (sidebar with collapsible groups), `PortalLayout` (simple top bar), `AuthLayout`.
- [x] `shared/config`: `brand.ts` (Equal names, same values as `desktop-app/src/modules/core/helpers/brand.ts`),
      `navigation.ts`.

## D2 — Admin modules (`/admin/*`)

Each module has its own `pages/ components/ composables/ services/ schemas/ helpers/ types/ routes.ts`.

- [ ] `auth`: login plus a TOTP step.
- [ ] `organizations`: list (search, plan, status, last seen) and detail tabs: overview · subscription & events ·
      devices · payments & invoices · credits · diagnostics · feedback · notes.
- [ ] `plans`: list plans and their versions, edit a draft version (entitlement editor generated from the catalog
      schema: capacity numbers, feature toggles grouped by action/mode), publish, and retire.
- [ ] `payments`: approval queue (receipt preview, approve/reject with reason) and history.
- [ ] `releases`: upload a release, notes, rollout slider, pause, mandatory flag, and the adoption chart.
- [ ] `telemetry`: error groups, group detail, and "تصدير للـ ledger".
- [ ] `diagnostics`: "اسحب التشخيص" (per device or org), request list, and download.
- [ ] `feedback`: inbox.
- [ ] `analytics`: KPI cards (active devices, conversion, MRR, churn), credit usage per feature, version adoption.
- [ ] `admins`: staff accounts, roles, the activity log.

## D3 — Portal modules (`/portal/*` plus public pages)

- [ ] `auth`: signup (phone, password, org name, governorate/area) → OTP → login, and forgot password (OTP).
- [ ] `plans` (public `/pricing`): the three tiers from 01 §2 with Pro in the middle marked «الأنسب», a monthly/yearly
      toggle (yearly shows «شهرين هدية»), and the copy «أقل من 10 جنيه في اليوم».
- [ ] `subscriptions`: my plan, renewal date, upgrade/downgrade (effective at period end), cancel at period end, resume.
- [ ] `payments`: checkout → manual payment instructions (InstaPay / Vodafone Cash / bank details from server config)
      → reference number and receipt upload → «قيد المراجعة» status. Also invoices and the payments list.
- [ ] `devices`: list (name, role, version, last seen), rename, deactivate. Hitting the device limit shows which device to free.
- [ ] `activation` (`/activate`): reads `challenge`, `state`, `terminalId` and `name` from the query. If not logged in,
      it goes to login/signup and returns. It shows «ربط هذا الجهاز (اسم الجهاز) بحساب (اسم النشاط)؟ [تأكيد]». On
      confirm it calls `POST /portal/activation/approve` and redirects to `equal://activate?code=…&state=…`. The same
      screen shows the 6-character `userCode` as the fallback («لو البرنامج مفتحش تلقائي، اكتب الكود ده فيه»). If the
      user came from «جرّبها» with no plan intent, it links a free account without asking for payment.
- [ ] `account`: profile, password, portal users.

## Gate D

- [ ] `vue-tsc` + `vite build` green. A lint rule or script blocks cross-module `pages/` imports and path-string navigation.
- [ ] Manual check: light + dark, 1280 and 1920, RTL, keyboard only, for the pricing, checkout, activation, admin
      org detail and plans editor pages.
- [ ] Playwright smoke against a local `apps/backend`: signup → checkout → admin approve → activation page returns
      a deep link with a valid code.
