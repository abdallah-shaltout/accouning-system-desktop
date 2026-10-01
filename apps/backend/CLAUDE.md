# CLAUDE.md — Equal backend (`apps/backend`)

These rules are **mandatory**. They keep the server small, predictable and safe for billing and licensing data.

## Read first

1. [docs/03-architecture.md](docs/03-architecture.md): where every file goes.
2. The doc for the area you touch: [data model](docs/04-data-model.md), [API](docs/05-api-spec.md),
   [security](docs/06-security.md) or [env](docs/07-environment.md).
3. The phase file in `plans/pending/23-subscription-platform/` for the task list. When a task is done, tick it there.

## Copy before you write

The structure and most base code come from `references/accouning-system/POS-Fares-server` (the reference).
Before writing any base class, middleware, util, auth or token code, **find the reference file and copy it**. Change
only:

- the DB layer (Mongoose → Drizzle);
- validation (express-validator → Zod);
- the tenant (`store`/`req.storeId` → `organization`/`req.orgId`);
- anything [docs/03-architecture.md](docs/03-architecture.md) lists as a deliberate deviation.

Keep the reference's file names, class names, `ApiResponse` shape, Arabic messages and `.env` key names.

## Structure rules

- A domain is `src/domains/<d>/` with **exactly** these roles:
  - `<d>.schema.ts` (Drizzle tables + inferred types)
  - `<d>.service.ts` (all business rules)
  - `<d>.controller.ts` (thin, extends the app's base controller)
  - `<d>.route.ts`
  - `<d>.validation.ts` (Zod request schemas → middleware arrays)
  - `<d>.validation.rules.ts`
  - optional: `<d>.types.ts`, `<d>.events.ts`, `schema/`, `extend/<feature>/`, `__tests__/{unit,integration,mocks}`
  
  Create one with `bun run plop` and never by hand.
- App surfaces live in `src/apps/{admin,portal,device}/`: `core/`, `routes/`, `services/auth`, and
  `domains/<d>/<app>.<d>.{route,controller,service}.ts` only when an app needs more than the shared domain.
- **Auth is applied once per router** (`router.use(AdminRequiredAuth)` in `routes/domains.ts`), never per route.
- A domain talks to another domain only through its **service** or a **bus event**. It never imports another
  domain's controller or route.
- Cross-cutting code goes in `src/shared/`. Configuration goes in `src/config/`. No new top-level folders.

## Data rules

- **Money** is a `bigint` in piasters plus `currency`. Never a float, and never computed in the controller.
- Every write that changes a subscription, payment, license, credit or device runs **inside one transaction**
  (`withTx`) and writes its append-only event/audit row in the same transaction.
- `subscription.status` changes only through `subscriptionService.transition()`.
- A published `plan_version` is immutable.
- Bus handlers run **after commit**. A handler failure is logged and retried; it never rolls back the source write.
- Every schema change comes with a migration from `bun run db:generate`, committed together.
- Idempotency: payment approval, credit consumption, activation and any provider webhook require an
  `Idempotency-Key` (reference middleware) or a unique business key.

## Security rules

- Never log secrets, tokens, passwords, OTPs, receipts or license private keys.
- Passwords use argon2id. OTPs and device secrets are stored hashed. Refresh tokens live in Redis (reference helpers).
- License tokens are signed only by `shared/security/licenseSigner.ts`.
- Every admin write is recorded in `admin_activity` by the admin core.
- Validate every request body, query and param with Zod. Never trust client-sent prices, plan ids for amounts, or
  counts.

## Config rules

- Every setting comes from `.env` through `src/config/validateEnv.ts`, which fails fast on a missing key.
  Don't read `process.env` anywhere else.
- No Docker files: Coolify builds with `bun run build` and starts with `bun run start` (see
  [docs/07-environment.md](docs/07-environment.md)).

## Definition of done

```bash
bun run lint          # tsc --noEmit
bun run test          # unit + integration (real Postgres + Redis from TEST_DB_URL / TEST_REDIS_URI)
bunx drizzle-kit check
bun run build && bun run start   # /health returns 200
```

- A new domain ships with service unit tests and route integration tests (happy path, validation errors, auth
  failures, and tenant isolation where the domain is tenant-scoped).
- If the entitlement catalog changed, re-run `bun run catalog:export` and update the desktop snapshot in the same change.

## Don't

- Don't write a base class, middleware or util that already exists in the reference. Copy it.
- Don't add Docker files, and don't hard-code URLs, keys or prices.
- Don't return hidden or blurred real data for locked features. Locked responses carry summaries only.
- Don't let the free tier depend on this server being up. The desktop must keep working offline.
