# 23 — Subscription platform (freemium licensing, dashboard, updates, telemetry)

> **Status (2026-09-29):** phase 0 (app docs) done. Phases A–G pending; nothing implemented yet. All product decisions below were agreed with the
> user in discussion on 2026-09-29. Everything that was open is resolved (see "Resolved").

**To implement:** paste [EXECUTE-PROMPT.md](EXECUTE-PROMPT.md) into a fresh session.

## What this plan builds

Equal is an offline-first desktop app (Tauri + Rust + bundled MariaDB, LAN main + cashier terminals). This plan
adds everything around it that needs a server:

1. **`apps/backend`**: an independent Node.js + Express + TypeScript + PostgreSQL project. It handles plans, subscriptions,
   payments, device activation, signed licenses, usage credits, app releases (auto-update), telemetry and remote
   diagnostics.
2. **`apps/dashboard`**: an independent Vue 3 + shadcn-vue + Tailwind v4 + Zod project with two areas:
   the **admin** console (the owner runs customers, plans, payments, releases and diagnostics from it) and the
   **customer portal** (sign up, pay, upgrade, manage devices, activate).
3. **Desktop changes (this repo)**: entitlement enforcement in Rust, lock/upgrade UI, one-click activation
   (browser + deep link), usage credits, background auto-update, telemetry and remote diagnostics pull, and a
   privacy toggle.

## Decisions (agreed with the user, 2026-09-29)

| # | Decision | Reason |
|---|---|---|
| D1 | **Independent projects in `apps/`, no monorepo tooling**: `apps/backend` and `apps/dashboard`, like the existing `apps/landing`. Each has its own `package.json`, lockfile and `node_modules`, and there are no workspaces. `apps/mobile-app` is an empty placeholder for a future Flutter LAN-sync app (see its README). Their **specs, requirements, goals and rules live in each app's `docs/` and `CLAUDE.md`**, and those are the source of truth ([02-architecture.md](02-architecture.md) is the index) | User 2026-09-29: "في apps folder as backend, frontend(dashboard)" + "مش عاوز تعقيد mono repo" |
| D2 | **Backend structure mirrors `references/accouning-system/POS-Fares-server`** (apps/ + domains/ + shared/ + config/, per-domain file set, base controller/service, router-level auth, path aliases, plop generator). See [02-architecture.md](02-architecture.md) | User: the reference structure is "very good, scalable and organized" |
| D3 | **Frontend is module-first**: every module has `pages/ components/ composables/ services/ schemas/ (Zod) helpers/ types/ routes.ts`, and cross-module code goes in `shared/` | User requirement; the same pattern this repo already uses |
| D4 | **PostgreSQL + Drizzle ORM**, not MongoDB | User: knows Postgres well. Billing data is relational and transactional |
| D5 | **Freemium, ChatGPT/Claude style**: Free forever (a real, complete shop), Pro, Business, and Max later (AI). "Free users are happy but want to go up; paid users feel it's beyond imagination" | User |
| D6 | **Free limits**: 100 products, 1 branch, main device only, 2 users, **3 Pro action credits / month** (online only, counted on the server) | User accepted 100 over the initial 50 (50 is exceeded in a grocery's first week) |
| D7 | **Lapse/over-limit**: nothing is hidden or locked for reading. Everything stays visible, sellable, reportable and exportable. Only *creating beyond* a limit is blocked | User wanted "show only 50 products". Rejected: hidden products break stock valuation, reports and invoices (CLAUDE.md: never wrong numbers). User accepted |
| D8 | **Prices (EGP)**: Pro 299/mo or 2,990/yr · Business 699/mo or 6,990/yr (3 branches) · Max ≈1,299/mo when AI ships. Every limit and price is `plan_version` data, editable from the dashboard without a release | Evidence in [01-pricing-and-tiers.md](01-pricing-and-tiers.md). User: "the recommendation is fine if it's evidence-based" |
| D9 | **Activation = one click**: browser + PKCE + deep link `equal://activate`, with a 6-character code as fallback. No token copy/paste. The account is created at the first "جرّبها" or "رقّي" click; the free tier needs no account | User: "مخلهوش يضيف توكن" |
| D10 | **License = Ed25519-signed token verified in Rust** (never in Vue). Offline grace 30 days, then fall back to Free (never a lockout) | Offline app, and backend authority |
| D11 | **Payments: manual first** (InstaPay / Vodafone Cash / bank, receipt upload, admin approves) behind a `PaymentProvider` interface. Paymob comes later | Realistic for the Egyptian launch, and zero integration risk |
| D12 | **Auto-update**: check and download in the background, install only on app close or when the user clicks "أعد التشغيل الآن". Never interrupts a POS shift. Staged rollout plus `minVersion` from the dashboard. Backup before any migration | User |
| D13 | **Telemetry + remote diagnostics**: error groups by fingerprint, and a "pull diagnostics" button in the admin that the device answers on its next check-in. Opt-out in Settings. The accounting DB is **never** pulled remotely | User. The customer's accounting data stays private |
| D14 | **Separate auth realms**: admin, portal and device. Admin and portal use a 15-minute access JWT (an `aud` claim per realm) plus a rotating refresh token in an httpOnly cookie, **stored in Redis with the reference's `redis-cache.ts` helpers**, and we add family reuse detection. Devices use a per-device credential instead of a user JWT | User asked for access + refresh tokens and provides Redis. Devices are not users |
| D15 | **Deploy with Coolify on the user's VPS. No Docker files in this plan.** The user handles deployment; we handle the code only. Every setting comes from `.env` (the user provides `DB_URL` for Postgres and `REDIS_URI`). The server exposes `GET /health`, and `build`/`start` scripts that Coolify's Nixpacks can run | User, 2026-09-29 |
| D16 | **Reuse the reference code, don't rewrite it.** Copy `shared/core`, `shared/middleware/*` (error, auth, validator, idempotency, rateLimit), `shared/utils` (`Token.ts`, `redis-cache.ts`, `logger`, `whatsapp/`, `nodemailer/`), `config/*` (`appUse`, `corsConfig`, `securityConfig`, `rateLimiterConfig`, `validateEnv`, `redisConfig`), `apps/*/core`, `apps/*/routes`, the admin/store `services/auth` (+ `token/`) and `plopfile.mjs`. Adapt only the DB layer (Mongoose → Drizzle) and validation (express-validator → Zod). **Keep the reference's `.env` key names** | User: "most of the backend code I already gave you" |
| D17 | **Domain `farook.app` (temporary)**: `api.farook.app` and `app.farook.app` (portal + admin). Always read from env (`SERVER_BASE`, `PORTAL_URL`, `VITE_API_URL`). The desktop keeps the server URL in **one** place (`licensing/config.rs`), so a later rename is a config change | User |

## Rules for every phase

- Backend code follows the reference structure exactly (D2). Frontend code follows D3. A file in the wrong layer is a
  review failure. The details are in [02-architecture.md](02-architecture.md).
- Desktop changes follow this repo's `CLAUDE.md`: seam rule, `backendCall` from services only, new `#[tauri::command]`
  in `generate_handler!` + `ipc_sig!`, ts-rs bindings, named routes, RTL, tokens, `bun run memory` after structural changes.
- **Cargo**: only through `scripts/cargo-safe.ps1` (a machine-wide mutex, below-normal priority). Implementers may
  run `cargo check`. Test builds and runs are batched at phase G. Never call cargo directly.
- Money is stored as **integer piasters** (`bigint`) with a `currency` column. Never float.
- Every state change to a subscription, payment, license or credit goes through its domain service inside a Postgres
  transaction, and writes an append-only event row.

## Phases

| File | What | Size | Status |
|---|---|---|---|
| [phase-0-app-docs.md](phase-0-app-docs.md) | Goals, requirements, specs and rules for `apps/backend` and `apps/dashboard`, plus the `apps/mobile-app` placeholder | S | done |
| [01-pricing-and-tiers.md](01-pricing-and-tiers.md) | Competitor prices, scientific basis, tiers, the entitlement matrix mapped to real desktop routes | S | done (decided) |
| [02-architecture.md](02-architecture.md) | Index of the app specs + the cross-project contracts (catalog, license token, device API, deep link) | S | done |
| [phase-a-server-foundation.md](phase-a-server-foundation.md) | Scaffold `apps/backend` on the reference structure: config, shared core, middleware, Drizzle, plop, admin and portal auth | M | done |
| [phase-b-server-billing-licensing.md](phase-b-server-billing-licensing.md) | Organizations, plans and versions, subscriptions and the state machine, manual payments, devices, activation, licenses, credits | L | done |
| [phase-c-server-ops.md](phase-c-server-ops.md) | Releases and the update endpoint, heartbeat, error telemetry, diagnostics requests, feedback, admin metrics | M | done |
| [phase-d-dashboard.md](phase-d-dashboard.md) | Scaffold `apps/dashboard`, shared kit, auth, admin modules, portal modules and the activation page | L | D1 done, D2/D3 pending |
| [phase-e-desktop-licensing.md](phase-e-desktop-licensing.md) | Rust `licensing` domain, capacity and feature guards, credits, deep-link activation, lock and upgrade UI, subscription settings page | L | pending |
| [phase-f-desktop-updates-telemetry.md](phase-f-desktop-updates-telemetry.md) | Updater plugin, backup before migration, LAN version check, heartbeat, error upload, diagnostics pull, feedback, privacy card | M | pending |
| [phase-g-launch.md](phase-g-launch.md) | Van Westendorp price validation, landing copy, signing keys, the Coolify handoff (env list + scripts), the final cargo gate, smoke test | S | pending |

Order: 0 (done) → A → B → C (server), D can start after A and B, E after B, F after C, then G last.

## Definition of done

- `apps/backend`: `bun run lint` (tsc), `bun run test` (vitest + supertest against a real Postgres and Redis from
  `TEST_DB_URL` / `TEST_REDIS_URI` in `.env`), and `drizzle-kit check` all green. Every domain has unit tests for its service and integration tests for its routes.
- `apps/dashboard`: `vue-tsc` + `vite build` green, RTL/light/dark checked at 1280 and 1920, keyboard-navigable.
- Desktop: the full CLAUDE.md gate (`build`, `check`, `verify:mocks`, `memory`, `diag:check`, full e2e), plus the
  single throttled `cargo build` + `cargo test` + `bindings:check` at phase G, plus a real `bun run desktop` check of
  activation, lock, credits, update and diagnostics pull against a staging server.
- End-to-end scenario passes on staging: install → free use → hit the product limit → try a Pro report with a credit →
  sign up → pay manually → admin approves → the app activates by deep link → Pro unlocked → subscription lapses →
  back to Free with all data visible.

## Resolved (2026-09-29)

| # | Question | Answer |
|---|---|---|
| O1 | OTP provider | WhatsApp through the reference's existing GOWA integration (`shared/utils/whatsapp`, `GOWA_*` env keys). A console adapter is used in dev |
| O2 | Hosting and storage | Coolify on the user's VPS (D15). The user supplies the Postgres and Redis URLs. Files go to **Cloudflare R2** (`R2_*` env keys, see `apps/backend/docs/07-environment.md`) |
| O3 | Domain | `farook.app` for now (D17) |

There are no open decisions left.

## Later (not in this plan's tasks)

- Paymob online payments (cards and wallets) as a second `PaymentProvider` adapter.
- Max tier and AI-powered reports: `ai_reports` becomes an action feature metered through the same credit ledger,
  and the server proxies the model call so the key never ships in the app.
- Reverse trial (14 days of Pro at signup), if conversion data asks for it.
- The `apps/mobile-app` Flutter companion: LAN sync with the Main PC on the same Wi-Fi, paired by scanning a QR code
  the desktop shows. It needs its own plan. Its open questions are in `apps/mobile-app/README.md`.
