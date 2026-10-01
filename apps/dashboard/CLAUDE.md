# CLAUDE.md — Equal dashboard (`apps/dashboard`)

These rules are **mandatory**. The dashboard must feel like the desktop app ("same soul"): Arabic, RTL-first,
calm, consistent, light and dark, and keyboard-friendly.

## Read first

1. [docs/03-architecture.md](docs/03-architecture.md): where every file goes.
2. [docs/04-screens.md](docs/04-screens.md): the page you're building (route name, data, actions).
3. [docs/05-ui-rules.md](docs/05-ui-rules.md): the visual rules.
4. `apps/backend/docs/05-api-spec.md`: the endpoint you call. Never invent an endpoint.

## Module rules

- Every feature lives in `src/modules/<module>/` with this layout:
  - `pages/` (split into `pages/admin/` and `pages/portal/` when the module serves both)
  - `components/`, `composables/`, `helpers/`, `types/`
  - `services/`: the **only** place that calls `shared/api/http`
  - `schemas/`: Zod
  - `routes.ts`
  - `stores/` only when state is shared across pages
- A page calls its module's `services/` and `composables/` only.
- A module may import another module's `services/`, `schemas/` or `types/`, **never its `pages/` or `components/`**.
  Anything used by two modules moves to `src/shared/`.
- Every form and every API response has a Zod schema in `schemas/`. Types come from `z.infer`, not hand-written copies.
- Pages stay under ~250 lines. Split page pieces into `components/`.
- No new top-level folders under `src/`.

## Navigation

- Every navigation target is a **named route object** `{ name, params?, query? }`. Never a path string.
- Every route has `meta.title`, `meta.area` (`admin` | `portal` | `public`) and, for admin, `meta.roles` when restricted.
- Sidebar items come only from `shared/config/navigation.ts`.

## UI

- Build from `shared/components/app/*` (App wrappers, DataTable, FormField, PageHeader, MoneyText, StatusBadge,
  EmptyState, ConfirmDialog), then shadcn primitives in `shared/components/ui/*`. Never inline a reusable piece in a page.
- Colors, radius and fonts come only from tokens. No hex colors, no `text-[Npx]`, no `rounded-[…]`.
- RTL logical utilities only (`ms/me`, `ps/pe`, `start/end`, `text-start/end`). Numbers, phones and codes stay `dir="ltr"`.
- Money comes from the API in piasters and is displayed only through `MoneyText`. Dates only through `shared/helpers/format.ts`.
- One primary action per view. Destructive actions go through `ConfirmDialog`.
- The shared Arabic copy (empty, loading, error, unsaved, confirm delete) lives in `shared/config/copy.ts`.

## Auth

- The access token lives in memory only (never localStorage). Refresh goes through the httpOnly cookie via the
  single-flight interceptor in `shared/api/http.ts`.
- Guards read `meta.area` and `meta.roles`. The UI hides what a role can't do, but the backend is the authority.

## Definition of done

```bash
bun run build     # vue-tsc + vite build
bun run check     # module-boundary, named-route and token guards
```

- Checked in light + dark, at 1280 px and 1920 px, in RTL, and by keyboard.
- New list pages: server pagination/search through `DataTable`, plus an empty state.

## Don't

- Don't call axios or fetch from a page or component.
- Don't store tokens in localStorage or sessionStorage.
- Don't copy-paste a table, form section or filter bar. Extract it to `shared/`.
- Don't hard-code the API URL, prices, plan names or limits. They come from the API or `VITE_API_URL`.
