# Phase B — Billing, subscriptions, devices, licenses, credits

> **Status (2026-09-29): done.** Gate B green: `bun run lint`, `bunx drizzle-kit check`, and the
> full test suite (51 tests / 8 files across phases A+B, run twice for stability) all pass against
> the real `equal_dev`/`equal_test` Postgres databases and local Redis, including a new end-to-end
> `tests/integration/gate-b.scenario.test.ts` that drives the exact Gate B scenario through the real
> service singletons: signup → checkout Pro → manual payment → admin approve → device activation
> with a real Ed25519-verified Pro license → the hourly cron lapsing it through
> past_due→grace→expired → a reissued license correctly downgrading to Free. Production build
> (`bun run build && bun run start`) serves `/health` = 200 with the cron wired into bootstrap.
> Built with 5 parallel agents (org/user · plan+seeder · device/activation/license ·
> subscription/payment/cron · credit) plus manager-written Drizzle schemas/migration for all 7
> tables. See Deviations for the cross-agent integration fixes and one dev-environment note.

## Deviations

- **Entitlements schema bug fixed**: `plan/schema/entitlements.schema.ts`'s `limitValue` used
  `z.number().int().positive()`, which rejects `0` — but Free's `maxTerminals` is legitimately `0`.
  Changed to `.nonnegative()`. Found by the plan-domain agent while seeding (its seeder bypasses the
  parse, so this didn't block seeding, but would have rejected any future admin-drafted version
  reusing a `0` limit, e.g. suspending a plan's capacity to zero).
- **Two integration seams wired up after their agents finished**: the device/activation/license
  agent correctly stubbed `license.service.ts`'s `resolveEffectivePlanForOrg` and
  `activation.service.ts`'s `maxTerminals` check to a safe deny-by-default (Free) fallback, since
  `subscriptionService.getEffectiveEntitlements` didn't exist yet when it ran. Once the
  subscription/payment agent finished and exported that function, the manager wired both call
  sites to the real lookup (removing the `TODO(subscription-integration)` markers). This is the
  intended one-pass "review every agent's output against the docs" step, not a correction of wrong
  work — both stubs were exactly what was asked for given the ordering.
- **R2 receipt upload stays a documented stub**: `payment.service.ts`'s `storeReceipt` throws `503
  storage_not_configured` when a receipt file is attached, since `shared/storage/r2.ts` doesn't
  exist yet and R2 credentials are empty in dev (per the plan's "items that need me" list). This is
  the correct behavior per docs/07-environment.md, not a gap — a payment can still be submitted
  without a receipt file for testing/manual-review-via-reference-number flows.
- **Dev license signing key generated and stored**: `LICENSE_ACTIVE_KID=dev-1` was already in
  `.env`, but `LICENSE_SIGNING_KEY_dev-1` had no matching value — generated one with the same
  Ed25519 code path as `bun run keys:generate` and wrote it directly into `.env` (never printed to
  any tool output or log). Needed for the license-issuing tests and the Gate B scenario to sign
  real tokens.
- **Test flakiness fixed, not a product bug**: `payment.service.test.ts`'s "approves a pending
  payment... transitions the subscription to active" test used a fixed 50ms sleep to wait for the
  fire-and-forget `payment.approved` bus handler before asserting — flaky under full-suite load
  (passed alone, failed ~intermittently in the full run). Changed to a short poll loop (matching
  the pattern already used in the new Gate B scenario test and the subscription lifecycle tests).
  The underlying `payment.approved → transition()` wiring itself was correct throughout.
- **Test-only plan seeding**: the dev-only `bun run seed` (which includes `seedPlans`) never
  touches `equal_test` — each test file that needs a published plan version seeds its own (either
  via the existing `createPublishedPlanVersion` helper in `subscription/__tests__/testHelpers.ts`,
  or, for the new Gate B scenario test, by calling the real idempotent `seedPlans()` directly in
  `beforeAll`, exercising the actual seeder code instead of a parallel fixture).

Every domain is generated with `bun run plop module <name>` and follows `apps/backend/docs/03-architecture.md`. Business rules live in `<d>.service.ts`.
Controllers stay thin.

## B1 — Organizations and users

- [x] `domains/organization` + `domains/user` (portal users belong to an org; the first user is `owner`). The org is
      created at portal signup.
- [x] Portal: `GET/PUT /portal/organization`, and `GET/POST/DELETE /portal/users` (owner only; the org's `maxUsers`
      does not apply here, since these are portal logins, not desktop users).
- [x] Admin: list/search/detail orgs with subscription, devices, payments and credits, plus internal notes and
      suspend/unsuspend.

## B2 — Plans and versions

- [x] `domains/plan`: `plan` + `plan_version`. `entitlements` jsonb is validated by the Zod schema
      `plan/schema/entitlements.schema.ts`, whose keys come from 01 §3 (the **one** catalog; the desktop's
      `licensing/catalog.rs` must match it, checked in phase E).
- [x] Publishing makes a version immutable. Editing means creating a new draft version. `retire` stops new sign-ups
      but keeps existing subscribers on it (loss aversion, 01 §6.10).
- [x] `config/seeders/01.seedPlans.ts`: Free, Pro 29,900/299,000 piasters, Business 69,900/699,000 piasters, with
      the entitlements from 01 §3. Max stays unpublished.
- [x] Public `GET /portal/plans` (no auth) for the portal pricing page.

## B3 — Subscriptions

- [x] `domains/subscription`: `transition(subId, event, ctx)` implements the state machine in `apps/backend/docs/03-architecture.md`. It is the only
      writer of `status` and inserts a `subscription_event` in the same transaction. Illegal transitions throw
      `ApiError 409`.
- [x] `startCheckout(orgId, planVersionId, interval)` → `pending_payment` + a `billing_invoice`.
- [x] `shared/cron/subscriptionLifecycle`: runs hourly and is idempotent. `active` past `currentPeriodEnd` → `past_due` →
      (+7 days) `grace` → `expired`. Emits events so licenses get reissued (the Free policy is applied on expiry).
- [x] `cancelAtPeriodEnd` from the portal, and resume.
- [x] Unit tests cover every legal and illegal transition. An integration test covers the full lapse → Free path.

## B4 — Payments (manual first, D11)

- [x] `domains/payment`: a `PaymentProvider` interface (`createIntent`, `handleWebhook`). The first adapter is
      `ManualProvider`.
- [x] Portal: `POST /portal/payments` (invoiceId, method `instapay`/`vodafone_cash`/`bank`, reference, receipt upload
      to R2, `R2_BUCKET_PRIVATE`) → `pending`. `GET /portal/payments`, `GET /portal/invoices`.
      Note: receipt upload to R2 is stubbed (`shared/storage/r2.ts` doesn't exist yet and R2 env vars
      are unset in dev) — a receipt attempt throws `503 storage_not_configured` per docs/07-environment.md
      rather than faking success. See `payment.service.ts`'s `storeReceipt` TODO(r2-integration).
- [x] Admin: an approval queue. `approve` (idempotent) marks the payment `approved`, marks the invoice paid, and emits
      `payment.approved`, which extends the subscription by one interval from `max(now, currentPeriodEnd)`.
      `reject` requires a reason.
- [x] Amounts are bigint piasters. The invoice number sequence is gap-free, per year.

## B5 — Devices and activation (D9)

- [x] `domains/device`: `POST /device/register` (anonymous) creates a device plus a credential. The secret is
      returned once and stored hashed. Add `shared/middleware/auth/device.protected.ts`.
- [x] `domains/activation`:
  - [x] The portal page (phase D) calls `POST /portal/activation/approve {challenge, state, terminalId, deviceName}`
        and gets back `{code, userCode}` (5 min, single use).
  - [x] Device `POST /device/activate {code | userCode, verifier}`. The server checks
        `sha256(verifier) == challenge`, binds `device.orgId`, and issues a license. It enforces `maxTerminals`
        (terminals count separately from the main device) and returns 409 `device_limit` with the device list.
- [x] Portal: list devices, rename, deactivate (which revokes the license and credential).

## B6 — Licenses (D10)

- [x] `shared/security/licenseSigner.ts`: Ed25519 sign with the active `kid`. Keys come from env. Provide
      `bun run keys:generate` (it prints the public key to paste into the desktop's `licensing/keys.rs`).
- [x] `domains/license`: `issueFor(deviceId)` builds the payload from the org's effective plan version (active or
      grace → its plan; otherwise the Free policy) exactly per `apps/backend/docs/05-api-spec.md`, stores it, and returns the token. The
      `license.reissue` event handler reissues for every device in the org.
- [x] `POST /device/heartbeat` returns a new license when `refreshAfter` has passed or the plan changed, plus the
      signed Free policy.
- [x] A unit test verifies signatures with the public key and rejects a tampered payload.

## B7 — Credits (3 per month on Free, D6)

- [x] `domains/credit`: `consume(orgId, feature, idempotencyKey)` runs in one transaction: `SELECT … FOR UPDATE` on the
      org's period row, checks `used < limit`, inserts a ledger row, and returns a signed grant
      `{feature, grantId, exp: now + 10 min}` (same signer). Replaying the same idempotency key returns the same grant.
- [x] Credits need a linked device (a free account). An anonymous device gets 401 `account_required`, and the desktop
      then opens the activation flow (D9: the account is created at the first "جرّبها" click).
- [x] Only features whose catalog kind is `action` are creditable. Otherwise the response is 400.
- [x] A concurrency test: 10 parallel consumes with limit 3 → exactly 3 grants.

## Gate B

- [x] `bun run lint`, `bun run test` green. `drizzle-kit check` clean.
- [x] Integration scenario: signup → checkout Pro → manual payment → admin approve → a device activates by code →
      the license has Pro entitlements → the cron expires it → heartbeat returns the Free policy.
