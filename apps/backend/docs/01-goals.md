# 01 — Goals

## Why this server exists

The Equal desktop app is free forever for a small shop and works fully offline. This server exists to:

1. **Turn free users into paying users, gently**, the ChatGPT/Claude way: Free is a real product, and paid tiers
   give more capacity and smarter features. See `plans/pending/23-subscription-platform/01-pricing-and-tiers.md`.
2. **Prove what a device is allowed to do** with a signed, offline-verifiable license, so paid features can't be
   unlocked by editing the app and the free tier never depends on this server.
3. **Let the owner run the business** from one admin console: customers, plans, prices, payments, releases,
   errors and diagnostics.
4. **Keep every customer on a healthy version** with staged, signed auto-updates.
5. **Fix problems before customers report them**, using grouped error telemetry and on-demand diagnostics pulls,
   while never touching the customer's accounting data.

## Goals

| # | Goal | Measured by |
|---|---|---|
| G1 | Upgrades are frictionless: the path from «رقّي» in the app to Pro unlocked is under 3 minutes (manual payment excluded) | E2E timing on staging |
| G2 | Licensing is correct: no paid entitlement without an approved payment, and no paying customer ever locked out | Integration tests, plus zero support tickets of this kind |
| G3 | Prices and limits can change without a desktop release | Changing a `plan_version` reflects in the app within one heartbeat |
| G4 | Updates are safe: staged rollout, pause, mandatory flag, signed files | Release tests, and the adoption table |
| G5 | Problems are visible: every desktop error group is visible with counts per version | The telemetry dashboard |
| G6 | Small and cheap to run: one Node process + Postgres + Redis on the owner's VPS | Coolify, 1 vCPU / 2 GB for the first 5,000 devices |

## Non-goals

- Hosting or syncing the customer's accounting data. That stays on the shop's own machines.
- Online POS or a web version of the app.
- Card payments at launch. Manual payments come first, and Paymob later as a second provider.
- AI features. The Max tier comes later and reuses the credit ledger.
- The mobile app (`apps/mobile-app`). It syncs over the local network and doesn't use this server.

## Success metrics (after launch)

- Free → paid conversion (target 2–5%, the typical freemium range), shown in the admin analytics.
- Which Pro feature drives upgrades (credit usage per feature → upgrade within 30 days).
- Monthly churn, MRR, and active devices at 7 and 30 days.
- Share of devices on the latest version 14 days after a release.
