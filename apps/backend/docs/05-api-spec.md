# 05 — API spec

- Base URL: `SERVER_BASE` + `/api` (`https://api.farook.app/api`).
- Responses use the reference `ApiResponse` shape: `{ status, message, data?, ...pagination }`. Errors add
  `{ errors?, code?, action? }`, where `code` is one of the stable error codes below.
- List endpoints accept the reference `ApiFeatures` query: `page`, `limit`, `sort`, `fields`, `search`, `searchBy`, filters.

## Admin (`/api/admin`, AdminRequiredAuth unless noted)

| Method | Path | Role | Purpose |
|---|---|---|---|
| POST | `/auth/login` (public) | | email + password → `{ totpRequired, challengeId }` |
| POST | `/auth/totp` (public) | | challengeId + code → access token, refresh cookie |
| POST | `/auth/refresh` (cookie) · `/auth/logout` · `/auth/logout-all` · GET `/auth/me` | any | session |
| GET | `/organizations` · `/organizations/:id` | any | list and detail (subscription, devices, payments, credits) |
| PATCH | `/organizations/:id` | owner, support | notes, suspend/unsuspend |
| GET/POST | `/plans` · `/plans/:id/versions` | owner | list, create a draft version |
| PUT | `/plans/versions/:id` | owner | edit a draft |
| POST | `/plans/versions/:id/publish` · `/retire` | owner | lifecycle |
| GET | `/payments?status=pending` | owner, finance | approval queue |
| POST | `/payments/:id/approve` (Idempotency-Key) · `/payments/:id/reject` | owner, finance | review |
| GET | `/payments/:id/receipt` | owner, finance | a 5-minute signed R2 URL (audited) |
| GET/POST | `/releases` · PATCH `/releases/:id` (rollout, pause, mandatory) | owner | releases |
| POST | `/releases/upload-url` | owner | a presigned R2 PUT for the installer |
| GET | `/telemetry/errors` · `/telemetry/errors/:id` · `/telemetry/export` | any | error groups, ingest export |
| POST | `/diagnostics` `{deviceId \| orgId}` · GET `/diagnostics` · GET `/diagnostics/:id/download` | owner, support | pull diagnostics |
| GET/PATCH | `/feedback` · `/feedback/:id` | owner, support | inbox |
| GET | `/analytics/overview` · `/analytics/credits` · `/analytics/versions` | owner | metrics |
| GET/POST/PATCH | `/admins` · GET `/activity` | owner | staff and audit |

## Portal (`/api/portal`, PortalRequiredAuth unless noted, scoped to `req.orgId`)

| Method | Path | Purpose |
|---|---|---|
| GET | `/plans` (public) | Current published versions for the pricing page |
| POST | `/auth/signup` · `/auth/verify-otp` · `/auth/login` · `/auth/forgot` · `/auth/reset` (public, rate-limited) | account |
| POST | `/auth/refresh` · `/auth/logout` · `/auth/logout-all` · GET `/auth/me` | session |
| GET/PUT | `/organization` | business profile |
| GET/POST/DELETE | `/users` (owner) | portal users |
| GET | `/subscription` | current subscription, plan, entitlements, period, events |
| POST | `/subscription/checkout` `{planVersionId, interval}` | start or upgrade → invoice |
| POST | `/subscription/cancel` · `/subscription/resume` | at period end |
| GET | `/invoices` · `/payments` | billing history |
| POST | `/payments` (multipart: invoiceId, method, reference, receipt) | manual payment → pending |
| GET | `/payment-instructions` | InstaPay, wallet and bank details (from config) |
| GET | `/devices` · PATCH `/devices/:id` (rename) · POST `/devices/:id/deactivate` | devices |
| POST | `/activation/approve` `{challenge, state, terminalId, deviceName}` | → `{code, userCode, redirect}` |
| GET | `/credits` | usage this month |

## Device (`/api/device`)

Auth: `Authorization: Device <deviceId>.<secret>` unless noted.

| Method | Path | Purpose |
|---|---|---|
| POST | `/register` (public, rate-limited per IP) | `{terminalId, appVersion, os, role}` → `{deviceId, secret}` (the secret is returned once) |
| POST | `/activate` (device + Idempotency-Key) | `{code \| userCode, verifier}` → `{license, org: {id, name}}` · errors `invalid_activation_code`, `activation_expired`, `device_limit` |
| POST | `/heartbeat` | `{appVersion, os, telemetryEnabled, diagnosticsAllowed}` → `{license?, freePolicy, diagnosticsRequests: [{id}], serverTime}` |
| GET | `/credits` | `{period, used, limit, resetsAt}` |
| POST | `/credits/consume` (Idempotency-Key) | `{feature}` → `{grant, remaining}` · errors `account_required`, `credits_exhausted`, `not_creditable` |
| GET | `/releases/latest?current=&channel=` | Tauri updater JSON + `mandatory`, or 204 |
| POST | `/telemetry/errors` | `[{fingerprint, code, source, message, appVersion, count, firstSeen, lastSeen}]` (≤ 200 per batch) |
| PUT | `/diagnostics/:id` (multipart zip ≤ 20 MB, or JSON `{declined: true}`) | fulfil a request |
| POST | `/feedback` (multipart) | message, screenshot?, bundle? |
| POST | `/deactivate` | unbind this device |

### License token

`base64url(payloadJson) + "." + base64url(ed25519(payloadBytes))`. This is not a JWT on purpose: there is no
algorithm field and no shared secret in the app.

```jsonc
{
  "v": 1, "kid": "2026-10", "lid": "<license id>",
  "org": "<org id | null>", "dev": "<terminalId>", "plan": "pro", "pv": "<plan_version id>",
  "ent": { "limits": { "maxProducts": null, "maxBranches": 1, "maxTerminals": 3, "maxUsers": 5, "actionCreditsPerMonth": null },
           "features": ["report.profitLeakage", "priceLists"] },
  "iat": 1790000000,
  "paidUntil": 1792600000,       // null for free
  "refreshAfter": 1790604800,    // iat + 7 days
  "graceUntil": 1792592000       // iat + 30 days; after this, offline → Free
}
```

- The **Free policy** has the same format, with `plan: "free"`, `org: null` and `dev: "*"`.
- A **credit grant** is signed the same way: `{ "v": 1, "kid", "type": "grant", "feature", "grantId", "dev", "exp" }`.

### Error codes (stable, used by the desktop and the dashboard)

`validation_failed`, `unauthorized`, `forbidden`, `token_reused`, `not_found`, `conflict`, `invalid_transition`,
`payment_already_reviewed`, `amount_mismatch`, `idempotency_conflict`, `rate_limited`, `account_required`,
`credits_exhausted`, `not_creditable`, `device_limit`, `invalid_activation_code`, `activation_expired`,
`license_revoked`, `org_suspended`.
