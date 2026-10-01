# Equal backend (`apps/backend`)

The subscription, licensing and operations server for the Equal desktop app. It handles plans, subscriptions,
payments, device activation, signed licenses, usage credits, app releases (auto-update), telemetry and remote
diagnostics.

- **Stack:** Node.js ≥ 22 · Express 4 · TypeScript strict · PostgreSQL (Drizzle ORM) · Redis (ioredis) · Zod ·
  Cloudflare R2 (S3 API) · bun.
- **An independent project** inside this repo's `apps/` (like `apps/landing`): its own `package.json`, lockfile and
  `node_modules`. No workspaces.
- **Structure:** mirrors `references/accouning-system/POS-Fares-server`, and most of its base code is copied from
  there (see [CLAUDE.md](CLAUDE.md)).
- **Deploy:** Coolify on the owner's VPS. This repo contains code only: no Docker files. See
  [docs/07-environment.md](docs/07-environment.md).

## Docs (read in order)

| Doc | What |
|---|---|
| [CLAUDE.md](CLAUDE.md) | Mandatory rules for every change |
| [docs/01-goals.md](docs/01-goals.md) | Goals, non-goals, success metrics |
| [docs/02-requirements.md](docs/02-requirements.md) | Functional (FR-*) and non-functional (NFR-*) requirements |
| [docs/03-architecture.md](docs/03-architecture.md) | Folder structure, layers, what's copied from the reference, events, cron |
| [docs/04-data-model.md](docs/04-data-model.md) | Tables, constraints, the subscription state machine |
| [docs/05-api-spec.md](docs/05-api-spec.md) | Every endpoint (admin, portal, device), the license token, error codes |
| [docs/06-security.md](docs/06-security.md) | Auth realms, token rotation, device credentials, license signing, keys |
| [docs/07-environment.md](docs/07-environment.md) | `.env` keys and the Coolify settings |

Implementation tasks and status live in
[`plans/pending/23-subscription-platform/`](../../plans/pending/23-subscription-platform/README.md).

## Commands (once scaffolded in phase A)

```bash
bun install
bun run dev            # nodemon
bun run build          # dist/
bun run start          # node dist/index.js
bun run lint           # tsc --noEmit
bun run test           # vitest (+ supertest), needs TEST_DB_URL / TEST_REDIS_URI
bun run db:generate    # drizzle-kit generate (after a <d>.schema.ts change)
bun run db:migrate     # apply migrations (Coolify pre-deploy command)
bun run seed           # plans + super admin (first run)
bun run plop           # new domain module
bun run keys:generate  # Ed25519 license key pair
bun run catalog:export # entitlement catalog snapshot for the desktop
```
