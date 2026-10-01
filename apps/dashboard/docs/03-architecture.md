# 03 — Architecture (module-first)

```
apps/dashboard/
  index.html  vite.config.ts  tsconfig.json  components.json (shadcn-vue)  .env.example (VITE_API_URL)
  scripts/check.ts             ← guards: no cross-module pages/components imports, no path-string navigation, no hex/px tokens
  src/
    main.ts  App.vue
    router/index.ts            ← collects every module's routes.ts, installs shared/auth guards
    modules/
      <module>/
        pages/                 ← route pages only (pages/admin/*, pages/portal/* when both)
        components/            ← module-only components
        composables/           ← use<Thing>.ts: page state + calls into services
        services/              ← <module>Service.ts: the ONLY caller of shared/api/http, parses responses with schemas/
        schemas/               ← <entity>.schema.ts: Zod (forms + API responses)
        helpers/               ← pure functions (mappers, formatters specific to the module)
        types/                 ← z.infer re-exports + module enums
        routes.ts              ← route records { path, name, component, meta: { title, area, roles? } }
        stores/                ← Pinia, only when state is shared across pages
    shared/
      components/ui/           ← shadcn-vue primitives (generated, owned in repo)
      components/app/          ← AppButton, FormField, DataTable, PageHeader, MoneyText, StatusBadge, EmptyState, ConfirmDialog, FileDrop
      layouts/                 ← AdminLayout.vue, PortalLayout.vue, AuthLayout.vue
      api/                     ← http.ts (axios), errors.ts (ApiError + code → Arabic message)
      auth/                    ← useSession (per realm), guards.ts, useCan.ts
      helpers/                 ← format.ts (piasters → «299 ج.م», dates), numbers.ts
      config/                  ← brand.ts, navigation.ts, copy.ts (shared Arabic copy), constants.ts
      styles/                  ← tokens.css (light/dark), base.css (dir=rtl, fonts)
      types/                   ← api.ts (ApiResponse, Paginated<T>), route.ts (AppRoute)
```

## Modules

| Module | Area | Pages (see [04-screens.md](04-screens.md)) |
|---|---|---|
| `auth` | admin + portal | admin login + TOTP, portal signup/OTP/login/forgot |
| `organizations` | admin | list, detail (tabs) |
| `plans` | admin + public | plans list, version editor, public pricing |
| `subscriptions` | portal | home, my subscription |
| `payments` | admin + portal | admin queue/history, portal checkout, invoices and payments |
| `devices` | portal (+ admin tab component) | devices |
| `activation` | portal | `/activate` |
| `credits` | admin tab + portal card | (components only) |
| `releases` | admin | releases list, release editor |
| `telemetry` | admin | error groups, group detail |
| `diagnostics` | admin | requests |
| `feedback` | admin | inbox |
| `analytics` | admin | overview |
| `admins` | admin | staff, activity log |
| `account` | portal | profile, users |

An admin org-detail tab that shows another module's data (for example the devices tab) calls that module's
**service** from a component inside `organizations/components/`. It never imports the other module's page or components.

## HTTP and auth flow

- `shared/api/http.ts`: an axios instance with `baseURL: import.meta.env.VITE_API_URL` and `withCredentials: true`.
  - A request interceptor adds `Authorization: Bearer <access>` from the realm's session (memory only).
  - A response interceptor, on 401 (not from `/auth/*`), runs **one shared** `POST /<realm>/auth/refresh` promise,
    retries once, and on failure clears the session and routes to login with `redirect`.
  - Errors become `ApiError { code, message, fieldErrors }`.
- The realm is decided by the route's `meta.area`: admin routes use the admin session, portal routes the portal session.
- On app start, each area tries a silent refresh before the first guarded navigation.

## Routing

- History mode. Areas: `/admin/*` (AdminLayout), `/portal/*` (PortalLayout), public `/pricing`, `/activate`
  (PortalLayout without auth until confirm), and `/login` pages (AuthLayout).
- Named routes only (`AppRoute = { name, params?, query? }`).
- Admin `meta.roles` restricts pages (for example `plans` and `releases`: `owner`).

## Deploy

Coolify static site: build `bun install --frozen-lockfile && bun run build`, publish directory `dist/`, SPA
fallback to `index.html`, domain `app.farook.app`, env `VITE_API_URL=https://api.farook.app/api` (a build-time
variable). No Docker files.
