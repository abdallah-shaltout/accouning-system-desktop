# 03 — Architecture

## 1. Layout (mirrors `references/accouning-system/POS-Fares-server`)

The reference has three layers, and we keep all three:

- **Shared domains** (`src/domains/<d>/`): the model plus a generic CRUD surface.
- **Per-app surfaces** (`src/apps/<app>/`): each app has its own `core/` base classes, `routes/` with auth applied
  once, app-specific overrides and `services/auth`.
- **Cross-cutting** (`src/shared/`, `src/config/`).

The reference's `admin` + `store` apps become **`admin`** (owner/staff), **`portal`** (customers) and **`device`**
(the desktop app).

```
apps/backend/
  plopfile.mjs  .dev/modules/*.hbs     ← `bun run plop` module generator (copied, templates → Drizzle + Zod)
  drizzle.config.ts  build.ts  nodemon.json  tsconfig.json  vitest.config.ts  .env.example
  src/
    index.ts                   ← validateEnv → db/redis connect → listen(PORT) → graceful shutdown
    app.ts                     ← AppUse(app) → /health → /api routes → routeNotFoundHandler → globalErrorHandler
    config/
      appUse.ts  bootstrap.ts  commonConfig.ts  corsConfig.ts  securityConfig.ts  rateLimiterConfig.ts
      loggerConfig.ts  validateEnv.ts
      database/  client.ts (Drizzle on pg.Pool(DB_URL))  redisConfig.ts  migrations/ (drizzle-kit)
      seeders/   01.seedPlans.ts  02.seedSuperAdmin.ts
      routes/index.ts          ← router.use('/admin'), ('/portal'), ('/device'); mounted at /api
    apps/
      admin/   core/ admin.controller.core.ts admin.service.core.ts
               routes/ index.ts (auth, metrics, then domains)  domains.ts (router.use(AdminRequiredAuth) once)
               services/auth/ (auth.route|controller|service|validation|validation.rules.ts, token/)
               domains/<d>/ admin.<d>.route|controller|service.ts   (only where admin needs more than the shared domain)
      portal/  core/ portal.controller.core.ts portal.service.core.ts   (tenant-scoped to req.orgId)
               routes/ index.ts (auth, public plans)  domains.ts (PortalRequiredAuth + tenant rate limit)
               services/auth/ (signup, otp, login, refresh, logout, reset)
               domains/<d>/ portal.<d>.route|controller|service.ts
      device/  core/ device.controller.core.ts   (scoped to req.device, + req.orgId when linked)
               routes/ index.ts (register, activate: public + rate-limited)  domains.ts (DeviceRequiredAuth)
               domains/<d>/ device.<d>.route|controller|service.ts
    domains/<d>/
      <d>.schema.ts  <d>.service.ts  <d>.controller.ts  <d>.route.ts  <d>.validation.ts  <d>.validation.rules.ts
      <d>.types.ts?  <d>.events.ts?  schema/  extend/<feature>/  __tests__/{unit,integration,mocks}
    shared/
      core/        controller.core.ts  service.core.ts  response.core.ts
      middleware/  auth/ (admin.protacted.ts, portal.protacted.ts, device.protacted.ts)
                   validator/ (validation.core.ts, validatorErrorHandling.ts, commonValidator.ts: Zod)
                   error/ (apiError.ts, catchError.ts, globalErrorHandling.ts)
                   idempotency/  rateLimit/tenantRateLimit.ts
      libs/        query.ts (ApiFeatures → Drizzle)  money.ts  date.ts  phone.ts  index.ts
      utils/       Token.ts  redis-cache.ts  logger.ts  uuid.ts  sanitize.ts  whatsapp/ (GOWA)  nodemailer/
      logger/      (copied)
      security/    password.ts (argon2id)  licenseSigner.ts (Ed25519)  hash.ts  totp.ts
      events/      bus.ts
      cron/        subscriptionLifecycle/  creditPeriods/  diagnosticsExpiry/  (+ Redis lock)
      storage/     r2.ts (S3 client on R2)
      types/       Express.type.d.ts (req.admin, req.user, req.orgId, req.device)  api.type.d.ts  env.d.ts
```

**Path aliases** (`tsconfig.json`, same scheme as the reference): `@@<domain>/*` → `src/domains/<domain>/*`,
`@@shared/*`, `@@config/*`, `@/*` → `src/apps/*`, `~/*` → `src/*`. The plop generator adds `@@<d>/*` automatically.

## 2. Copied from the reference (D16)

| From the reference | Status |
|---|---|
| `shared/core/{controller,service,response}.core.ts` | `response` as-is. `controller` as-is apart from types. `service` ported to Drizzle (same method names and return shapes) |
| `shared/middleware/error/*` | as-is, plus Postgres error codes 23505 and 23503 → 409 |
| `shared/middleware/auth/*.protacted.ts` | as-is logic (blacklist, `passwordChangeAt`, `clearToken`), plus an audience check |
| `shared/middleware/idempotency/*`, `rateLimit/*` | as-is (Redis). Tenant key `store` → `org` |
| `shared/middleware/validator/*` | same files and roles, reimplemented with Zod |
| `shared/utils/{Token,redis-cache,logger,uuid,sanitize}.ts`, `shared/logger/*`, `utils/whatsapp/*`, `utils/nodemailer/*` | as-is. `Token.ts` gets `aud` |
| `config/{appUse,bootstrap,commonConfig,corsConfig,securityConfig,rateLimiterConfig,loggerConfig,validateEnv}.ts`, `config/database/redisConfig.ts` | as-is, with Mongo/NATS/session parts removed and new env keys added |
| `apps/*/core`, `apps/*/routes`, `apps/*/services/auth` (+ `token/`) | copied as admin/portal. The store core becomes the portal core (`storeId` → `orgId`) |
| `plopfile.mjs` | as-is. Templates emit Drizzle + Zod |

**Deliberate deviations:**

- Mongoose → **Drizzle**, keeping the `<d>.schema.ts` file role.
- express-validator → **Zod**.
- `deleted: boolean` → `deletedAt`.
- bcrypt → **argon2id**.
- `DB_URL` now points at Postgres (the key name is kept).

## 3. Layering

```
route  →  validation (Zod)  →  controller (thin)  →  service (rules, withTx)  →  schema (Drizzle)
                                                          │
                                                          └─ emits bus events (after commit) → other domains' services
```

- A controller never touches Drizzle directly.
- A service may call another domain's **service**, never its controller or route.
- App overrides (`apps/<app>/domains/<d>/`) extend the shared domain's service or controller. They never copy it.

## 4. Domains

`admin`, `adminActivity`, `organization`, `user`, `otp`, `plan` (+ versions), `subscription` (+ events), `payment`
(+ invoices), `device` (+ credentials), `activation`, `license`, `credit`, `release`, `telemetry`, `diagnostics`,
`feedback`. The tables are in [04-data-model.md](04-data-model.md).

## 5. Events (in-process bus, after commit)

| Event | Emitted by | Handled by |
|---|---|---|
| `payment.approved` | payment | subscription (extend/activate) |
| `subscription.changed` (activated, extended, past_due, grace, expired, canceled, plan_changed) | subscription | license (mark the org's devices for reissue), notification (WhatsApp message) |
| `org.suspended` / `org.unsuspended` | organization | license |
| `device.deactivated` | device | license (revoke) |

## 6. Cron (node-cron, each job idempotent, Redis lock `cron:<job>`)

| Job | Schedule | Does |
|---|---|---|
| `subscriptionLifecycle` | hourly | `active` → `past_due` → `grace` → `expired` by date |
| `creditPeriods` | 00:05 on the 1st | Opens the new month's `credit_period` rows lazily (consumption also creates them on demand) |
| `diagnosticsExpiry` | daily | `pending` requests older than 14 days → `expired` |
| `renewalReminders` | daily | WhatsApp reminder 3 days before `currentPeriodEnd` (portal link, no pressure) |
