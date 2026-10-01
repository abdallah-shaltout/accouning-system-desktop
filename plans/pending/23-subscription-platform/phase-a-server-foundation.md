# Phase A — `apps/backend` foundation

> **Status (2026-09-29): done.** Gate A green: `bun run lint`, `bunx drizzle-kit check`, and 10/10
> integration tests (admin login → TOTP → refresh rotation → reuse-detection cascade → logout;
> portal signup → OTP → org+user creation → login → refresh) all pass against the real
> `equal_dev`/`equal_test` Postgres databases (created this phase) and local Redis. Production
> build (`bun run build && bun run start`) serves `/health` = 200 with DB and Redis both reachable.
> `bun run plop module <name>` verified live (generated a compiling module, then removed it). One
> fix from the plan as written: `argon2` had to be added to `build.ts`'s esbuild `external` list
> (native addon, breaks when bundled) — noted under Deviations below.

## Deviations

- **`build.ts` externals**: added `argon2` to the esbuild `external` array (alongside `pg-native`,
  `pino*`, `prom-client`). Bundling a native addon breaks its prebuild resolution at runtime
  ("No native build was found..."); keeping it external and letting Node's own `require` resolve
  it from `node_modules` at start time fixed it. Same category of fix the reference already applied
  to `bcrypt`.
- **Refresh cookie `secure` flag**: both `admin` and `portal` auth controllers set the cookie
  `secure: NODE_ENV === "PROD"` rather than always `true`. A hard `true` silently drops the cookie
  over plain HTTP (local dev / tests), which is indistinguishable from a real bug until the refresh
  flow is exercised end-to-end. PROD still always gets `Secure` per docs/06-security.md.
- **`idempotency.metrics.ts`**: registers its three `prom-client` counters against the default
  global registry — there is no `src/shared/metrics/` module yet in this project. Migrate to a
  shared registry if/when one is added.
- **`nodemailer.templates.ts`**: uses inline string template builders instead of the reference's
  on-disk Handlebars files (no `templates/mail/*.hbs` convention exists yet here, and no current
  caller needs it — pre-built for a later phase).
- **Test infrastructure** (`tests/setup/setup.ts`, `vitest.config.ts`): not explicitly in the phase
  task list but required to satisfy Gate A's "integration tests... green" line. Notable subtleties
  future phases should know about: (1) `NODE_ENV` is forced to `DEV` for the test run (vitest's own
  default of `"test"` fails `validateEnv`'s strict `DEV`/`PROD` enum); (2) `TEST_DISABLE_RATE_LIMIT`
  is set so `express-rate-limit`-based limiters (`authRateLimit` et al.) skip during tests — the
  Redis-backed tenant limiters (`tenantRateLimit.ts`) are NOT yet wired to this flag since Phase A
  has no portal/admin CRUD routes to test against it; Phase B/C tests hitting those must add the
  same skip; (3) with `pool: "forks"` + `isolate: false`, every test file in a worker shares the
  same `db`/`redisClient` module instance, so the shared setup must not close the pool/Redis in its
  own `afterAll` (that would break sibling test files still running in the same worker).

**Goal:** a server that follows the reference structure (`apps/backend/docs/03-architecture.md`), built mostly by **copying the reference code**
(D16). Admin and portal auth work end to end, and there is a module generator, so phases B and C only add domains.
There are no Docker files: Coolify deploys it (D15).

**Source:** `references/accouning-system/POS-Fares-server/`.

## A1 — Scaffold

- [x] Scaffold `apps/backend` inside this repo, independent like `apps/landing`: own `package.json` and `bun.lock`, no
      workspaces, and `.env` gitignored. Copy the reference's tooling files as-is: `.editorconfig`,
      `.prettierrc`, `.prettierignore`, `.eslintrc.js`, `.gitignore`, `nodemon.json`, `build.ts`, `tsconfig.json`
      (reset `paths` to the aliases in `apps/backend/docs/03-architecture.md`), `vitest.config.ts`. No husky here: git hooks belong to the repo root.
- [x] `package.json`: start from the reference's scripts (`dev`, `build`, `start`, `test`, `lint`, `format`, `plop`).
      Keep only the reference dependencies this project uses:
      `express`, `helmet`, `cors`, `compression`, `cookie-parser`, `hpp`, `express-rate-limit`, `rate-limiter-flexible`,
      `express-async-handler`, `ioredis`, `jsonwebtoken`, `pino`/`pino-http`, `winston` (if the copied logger needs it),
      `multer`, `@aws-sdk/client-s3` (R2), `axios`, `nodemailer`, `uuid`, `phone`.
      Add: `zod`, `drizzle-orm`, `pg`, `argon2`, `@noble/ed25519`, `node-cron`, `otpauth` (admin TOTP).
      Dev: `drizzle-kit` plus the reference dev dependencies.
      Drop the Mongo-only packages (`mongoose`, `express-mongo-sanitize`, `ts-cache-mongoose`).
- [x] Coolify-ready (D15): `build` produces `dist/`, `start` runs `node dist/index.js`, the server listens on `PORT`,
      `GET /health` checks the DB and Redis, and SIGTERM closes the pool and Redis.
      Migrations run with `bun run db:migrate`, which Coolify runs as a pre-deploy command.
- [x] `.env.example` holds every key in `apps/backend/docs/07-environment.md` with a comment and **no values**. The real
      dev `.env` already exists (gitignored). Keep it; add any new key to both files.

## A2 — Copy the shared core

Copy these, then adapt only what's noted:

- [x] `src/app.ts`, `src/index.ts`, `config/appUse.ts`, `config/bootstrap.ts`, `config/commonConfig.ts`,
      `config/corsConfig.ts`, `config/securityConfig.ts`, `config/rateLimiterConfig.ts`, `config/loggerConfig.ts`,
      `config/database/redisConfig.ts`.
  - Remove the Mongo connect, session and NATS pieces that aren't used.
  - Add the `/health` route.
- [x] `config/validateEnv.ts`: the reference list with `DB_URL` described as Postgres, plus the new keys from `apps/backend/docs/07-environment.md`.
- [x] `shared/core/response.core.ts` (as-is).
- [x] `shared/middleware/error/*` (as-is), plus a mapping from Postgres `23505` → 409 and `23503` → 409.
- [x] `shared/middleware/idempotency/*` and `shared/middleware/rateLimit/tenantRateLimit.ts`, as-is on Redis. Rename the
      tenant keys `store` → `org`.
- [x] `shared/utils/{Token.ts, redis-cache.ts, logger.ts, uuid.ts, sanitize.ts}` and `shared/logger/*`, as-is.
      `Token.ts` gains an `aud` parameter and verification.
- [x] `shared/utils/whatsapp/*` (GOWA) and `shared/utils/nodemailer/*`, as-is.
- [x] `shared/core/service.core.ts`: port the reference `BaseService` to Drizzle with the same method names and return
      shapes (`createDocument`, `readDocument` with pagination/search/sort, `readDocumentById`, `updateDocument`,
      `deleteDocument` soft delete via `deletedAt`, `countDocuments`, `isExists`), plus `withTx(fn)`. Port
      `shared/libs/query.ts` (`ApiFeatures`) to build Drizzle `where`/`orderBy`/`limit`.
- [x] `shared/core/controller.core.ts`: the reference `BaseController`, unchanged apart from the service type.
- [x] `apps/{admin,portal,device}/core/*.core.ts`, from the reference `admin`/`store` cores. The portal core is the
      reference `StoreBaseController/Service` with `store`/`req.storeId` → `orgId`/`req.orgId`. The admin core writes
      `admin_activity` on every write.
- [x] `shared/middleware/validator/`: keep the file names (`validation.core.ts`, `validatorErrorHandling.ts`,
      `commonValidator.ts`) and re-implement them with Zod: `validate({ body?, query?, params? })` plus
      `BaseValidationRules` (text, phone EG, money piasters, id), with the same Arabic messages.
- [x] `shared/events/bus.ts` (new): typed `emit/on`. Handlers run after the transaction commits.

## A3 — Database

- [x] `config/database/client.ts`: Drizzle on `pg.Pool(DB_URL)`, `drizzle.config.ts`, and the `db:generate`,
      `db:migrate` and `db:studio` scripts. It uses the Postgres URL from `.env` (the user provides it). No local Docker.
- [x] Create the databases `equal_dev` and `equal_test` on the owner's Coolify Postgres (`CREATE DATABASE`, using the
      admin connection noted in `apps/backend/.env`). Never touch any other database on that server.
- [x] Test safety guard in `vitest` global setup: abort unless `TEST_DB_URL`'s database ends in `_test` and ≠ `DB_URL`.
      Migrate the test DB on setup and truncate between suites.
- [x] Local Redis at `redis://localhost:6379` (dev) and DB index 1 (tests). `/health` reports it.
- [x] Tables: `admin`, `admin_activity`. Migrations are checked in.

## A4 — Auth (D14)

- [x] `apps/admin/services/auth/*` and `apps/store/services/auth/token/*` → copy as `apps/admin/services/auth` and
      `apps/portal/services/auth`. Keep `tokenService.refreshToken/revokeToken` on Redis. Add the `familyId` claim
      and the `refreshUsed:<tokenId>` reuse marker (`apps/backend/docs/06-security.md`).
- [x] `shared/middleware/auth/{admin.protacted.ts, protacted.ts}`, copied, become `admin.protacted.ts` and
      `portal.protacted.ts` (keeping the blacklist check, the `passwordChangeAt` check and the `action: 'clearToken'`
      behavior). Add an audience check and `requireAdminRole(...roles)`.
- [x] Admin login: email + password (argon2id) → TOTP step (`otpauth`) → tokens. Also refresh (cookie), logout,
      logout-all and me.
- [x] Portal: signup (phone + password + org name) → WhatsApp OTP through the copied GOWA client (a console adapter
      when `NODE_ENV=DEV`) → verify → login. Also refresh, logout and me. Add `domains/otp` (hash, attempts ≤ 5,
      5-minute expiry).
- [x] `config/seeders/02.seedSuperAdmin.ts` reads `SUPER_ADMIN_*` from env and runs once.

## A5 — Generator and docs

- [x] `plopfile.mjs`: copy the reference and change `.dev/modules/*.hbs` to emit Drizzle `<d>.schema.ts` and Zod
      `<d>.validation.ts` / `<d>.validation.rules.ts`. It keeps the same tsconfig alias transform.
- [x] Keep `apps/backend/README.md` and `CLAUDE.md` (written in phase 0) accurate for anything that changed during scaffolding.

## Gate A

- [x] `bun run lint`, `bun run test` green (against `TEST_DB_URL` / `TEST_REDIS_URI`).
- [x] Integration tests: admin login → access → refresh rotation → **reuse of an old refresh token revokes every
      session** → logout. Portal signup → OTP (console adapter) → login. A token with the wrong `aud` is rejected.
- [x] `bun run build && bun run start` serves `/health` = 200 with the DB and Redis reachable.
- [x] `bun run plop module sample` produces a compiling module (then deleted).
