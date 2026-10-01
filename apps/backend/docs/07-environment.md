# 07 — Environment and deployment (Coolify)

## `.env` keys

The reference's key names are kept. `src/config/validateEnv.ts` validates everything at startup and exits on a
missing required key. Nothing else reads `process.env`.

| Key | Required | Example / note |
|---|---|---|
| `NODE_ENV` | yes | `DEV` / `PROD` (reference values) |
| `PORT` | yes | Coolify sets it |
| `SERVER_BASE` | yes | `https://api.farook.app` |
| `DB_URL` | yes | PostgreSQL URL (provided by the owner) |
| `REDIS_URI`, `REDIS_TLS` | yes / no | Redis URL (provided by the owner) |
| `JWT_SECRET_KEY`, `JWT_EXPIRE_TIME` | yes | `15m` |
| `JWT_REFRESH_SECRET_KEY`, `JWT_REFRESH_EXPIRE_TIME` | yes / no | different secret, `30d` |
| `COOKIE_PARSER_SECRET_KEY` | yes | |
| `ALWED_WEBSITE` | yes | `https://app.farook.app` (comma-separated CORS origins, name kept as in the reference) |
| `IDEMPOTENCY_TTL_SECONDS`, `IDEMPOTENCY_PENDING_TTL_SECONDS`, `IDEMPOTENCY_ENABLED` | no | reference defaults |
| `GOWA_BASE_URL`, `GOWA_BASIC_AUTH_USER`, `GOWA_BASIC_AUTH_PASSWORD` | yes in PROD | WhatsApp OTP and reminders. In DEV the console adapter is used |
| `PORTAL_URL` | yes | `https://app.farook.app` |
| `LICENSE_ACTIVE_KID`, `LICENSE_SIGNING_KEY_<kid>` | yes | base64 Ed25519 private key(s) |
| `DATA_ENCRYPTION_KEY` | yes | 32-byte base64 key (TOTP secrets) |
| `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` | yes | Cloudflare R2 (S3 API endpoint `https://<account>.r2.cloudflarestorage.com`) |
| `R2_BUCKET_RELEASES`, `R2_PUBLIC_RELEASES_URL` | yes | public-read bucket for installers + its public/custom domain |
| `R2_BUCKET_PRIVATE` | yes | receipts, diagnostics bundles, feedback attachments |
| `PAYMENT_INSTRUCTIONS_JSON` | yes | InstaPay handle, wallet number, bank details shown in the portal |
| `SUPER_ADMIN_EMAIL`, `SUPER_ADMIN_PASSWORD` | first run | used by `bun run seed` |
| `TEST_DB_URL`, `TEST_REDIS_URI` | tests | separate database for vitest |

## Coolify settings (the owner configures, the code supports)

| Setting | Value |
|---|---|
| Build pack | Nixpacks (Node, bun detected from `bun.lock`) |
| Build command | `bun install --frozen-lockfile && bun run build` |
| Start command | `bun run start` |
| Pre-deploy command | `bun run db:migrate` |
| Health check | `GET /health` (200 when the DB and Redis are reachable) |
| Port | from `PORT` |
| Domain | `api.farook.app` |

No Dockerfile or compose file lives in this project.

## Dev setup (current)

The real values live only in **`apps/backend/.env`** (gitignored, already created). Never copy them into docs,
commits, logs or tool output.

| What | Dev value |
|---|---|
| PostgreSQL | The owner's Coolify Postgres (public port). `DB_URL` → database **`equal_dev`**, `TEST_DB_URL` → database **`equal_test`**. Phase A creates both with `CREATE DATABASE` through the admin connection noted in `.env` |
| Redis | Local: `REDIS_URI=redis://localhost:6379`, `REDIS_TLS=false`. Tests use `TEST_REDIS_URI=redis://localhost:6379/1` |
| Server | `PORT=5050`, `SERVER_BASE=http://localhost:5050` |
| Dashboard | `ALWED_WEBSITE` / `PORTAL_URL` = `http://localhost:5173`. The dashboard's `.env` has `VITE_API_URL=http://localhost:5050/api` |
| JWT / cookie / data-encryption secrets | Random, generated for dev |
| License key | Empty until `bun run keys:generate` (phase B) |
| R2, GOWA | Empty until the owner provides them. DEV uses the console OTP adapter, and uploads are skipped behind a clear `storage_not_configured` error |

**Test safety:** the test setup refuses to run unless `TEST_DB_URL`'s database name ends in `_test` and differs from
`DB_URL`. Tests truncate tables only in that database.

**Before production (owner):**
- Create a dedicated, non-superuser role and a production database `equal` instead of using `postgres`.
- Restrict the public Postgres port (5433) with the VPS firewall to the Coolify app and your own IP.
- Rotate the `postgres` password, since the current one was shared in a chat.
