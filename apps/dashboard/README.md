# Equal dashboard (`apps/dashboard`)

One web app with two areas:

- **Admin console** (`/admin/*`): the owner and staff run customers, plans and prices, payments, releases,
  telemetry, diagnostics and feedback.
- **Customer portal** (`/portal/*`, plus the public `/pricing` and `/activate` pages): shop owners sign up, pay,
  upgrade, manage devices and activate the desktop app with one click.

- **Stack:** Vue 3 `<script setup>` · TypeScript strict · Vite · Pinia · Vue Router (history) · Tailwind v4 ·
  shadcn-vue (reka-ui) · Zod + vee-validate · TanStack Table · axios · bun.
- **An independent project** inside `apps/` (like `apps/landing`): own `package.json`, no workspaces.
- **Talks only to** `apps/backend` through `VITE_API_URL` (dev: `http://localhost:5050/api`, in the gitignored `.env`).
- **Deploy:** Coolify, as a static site (`dist/`) with SPA fallback. See [docs/03-architecture.md](docs/03-architecture.md#deploy).

## Docs (read in order)

| Doc | What |
|---|---|
| [CLAUDE.md](CLAUDE.md) | Mandatory rules for every change |
| [docs/01-goals.md](docs/01-goals.md) | Goals, non-goals, users |
| [docs/02-requirements.md](docs/02-requirements.md) | Functional (FR-*) and non-functional (NFR-*) requirements |
| [docs/03-architecture.md](docs/03-architecture.md) | Module-first structure, shared kit, HTTP/auth flow, routing |
| [docs/04-screens.md](docs/04-screens.md) | Every page: route name, path, area, purpose, data, actions |
| [docs/05-ui-rules.md](docs/05-ui-rules.md) | Design, RTL, tokens, forms, tables, copy |

Tasks and status: [`plans/pending/23-subscription-platform/phase-d-dashboard.md`](../../plans/pending/23-subscription-platform/phase-d-dashboard.md).

## Commands (once scaffolded)

```bash
bun install
bun run dev        # vite
bun run build      # vue-tsc + vite build → dist/
bun run check      # structure guards (module boundaries, named routes, no raw colors)
bun run preview
```
