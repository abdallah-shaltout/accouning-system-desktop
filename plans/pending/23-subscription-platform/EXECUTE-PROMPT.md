# Execute prompt (paste into a fresh session)

> Copy everything below the line into a new session. It assumes the working directory is the repo root
> `desktop-app/`. It's written for an Opus-class manager session.

---

Implement **plan 23 (subscription platform)** exactly as specified in `plans/pending/23-subscription-platform/`.
All product decisions are final (D1–D17 in the plan README). Don't reopen them, and don't ask me questions. Where
there's a genuine gap, pick the option that best preserves backend authority, data integrity and the reference
structure, record it under a "Deviations" heading in that phase file, and continue.

## Read first, in this order (no code before you finish)

1. `CLAUDE.md` (repo root). The desktop rules apply to phases E and F.
2. `plans/pending/23-subscription-platform/README.md`: decisions, rules, phase table, definition of done.
3. `plans/pending/23-subscription-platform/01-pricing-and-tiers.md` §3 (the entitlement catalog) and `02-architecture.md` (the
   index plus the cross-project contracts).
4. `apps/backend/CLAUDE.md` and `apps/backend/docs/01…07`. These are the source of truth for the server.
5. `apps/dashboard/CLAUDE.md` and `apps/dashboard/docs/01…05`. These are the source of truth for the dashboard.
6. The reference server `references/accouning-system/POS-Fares-server/`: read `src/shared/core`,
   `src/shared/middleware`, `src/shared/utils/{Token,redis-cache}.ts`, `src/config`, `src/apps/*/{core,routes,services/auth}`,
   `src/domains/plan` and `plopfile.mjs` before writing any backend file. **Copy it, don't reinvent it** (D16).
7. `AGENT_MEMORY.md`, before touching desktop code (phases E and F).

## What you're building

- `apps/backend`: Express + TS + PostgreSQL (Drizzle) + Redis + R2, mirroring the reference structure.
- `apps/dashboard`: Vue 3 + shadcn-vue + Tailwind v4 + Zod, module-first, with the admin console and customer portal.
- Desktop changes in this repo:
  - the Rust `licensing` domain (entitlements, guards, credits, deep-link activation) and the Vue `licensing` module;
  - the auto-updater, heartbeat, error telemetry, remote diagnostics pull, feedback and the privacy card.

`apps/mobile-app` is out of scope. Don't touch it.

## Environment (already set up)

- `apps/backend/.env` exists (gitignored) with the dev values. Postgres is on the owner's Coolify server, Redis is local
  `redis://localhost:6379`, and the dev JWT/cookie/encryption secrets are generated. `apps/dashboard/.env` has
  `VITE_API_URL`.
- **Never** print, log, commit or copy any `.env` value into docs, commits or tool output. Never commit `.env`.
- Phase A creates the databases `equal_dev` and `equal_test`. Touch no other database on that server. Tests run only
  against `equal_test`, behind the guard described in phase A.
- If local Redis isn't running, say so once in the final report, and skip only the tests that need it (mark them).
  Keep going with the rest.

## How to run it: you're the manager

- You (the main session) are the **manager**. You own the shared files (`package.json`s, `tsconfig.json` aliases,
  Drizzle schema index + migrations, `src/config/routes/index.ts`, `apps/*/routes/domains.ts`,
  `src-tauri/Cargo.toml`, `lib.rs`, `core/ipc.rs`, the plan files, and `AGENT_MEMORY.md` regeneration). You define the
  interfaces and schemas first, then hand out disjoint file sets.
- The implementers are parallel `general-purpose` agents on `model: sonnet`, 3–5 at a time. Each owns its own
  files, and no two agents edit the same file. No git worktrees (CLAUDE.md).
- Give each agent:
  - the exact files it owns;
  - the docs sections it implements;
  - the reference files to copy from;
  - its gate commands.
- Review every agent's output against the docs before accepting it.
- Advance automatically from wave to wave. **Don't report back to me until everything below is done** (or blocked
  only by items that need me).

### Waves

1. **Phase A** (mostly you, plus 1–2 agents for the copied `shared/*` and `config/*`). Gate A must be green before wave 2.
2. **Phase B.** First you write every Drizzle schema + migration + the entitlement catalog schema. Then parallel
   agents take: organization + user + otp · plan + versions + seed · subscription + payment + invoices + cron ·
   device + activation + license + keys script · credit. Gate B.
3. **Phase C** (release · telemetry + ingest export · diagnostics + feedback · analytics) **in parallel with Phase D1**
   (dashboard scaffold + shared kit + http/auth). Gates C and D1.
4. **Phase D2/D3** (admin modules · portal modules + `/activate`) **in parallel with Phase E**.
   - Rust `licensing` domain: agents own separate files under `src-tauri/src/domains/licensing/`.
   - You do the `lib.rs`, `ipc.rs` and `Cargo.toml` wiring.
   - Also: the mock `licensing.ts`, the Vue `licensing` module and the `settings-subscription` page.
   
   Gates D and E.
5. **Phase F**, then the parts of **Phase G** that need no owner input:
   - G2: the landing copy;
   - G3: generate the dev license key into `.env` via `keys:generate` (the updater key procedure is documented only);
   - G4: `DEPLOY.md` for both apps;
   - G5: the final gate.

## Rules

- Tick every `- [ ]` in the phase files as you complete it. When a phase's gate passes, add a one-line status note at
  the top of that phase file. Keep the README phase table current.
- Backend: `bun run plop` for every new domain. Money is bigint piasters. Every billing/licensing write happens in
  `withTx` with its event row. `subscription.status` changes only through `transition()`. Validation is Zod
  everywhere. Config comes only through `validateEnv.ts`. No Docker files (Coolify, D15).
- Dashboard: module-first, pages call only their module's `services/`, named routes only, token-only styling, RTL
  logical utilities, and the access token in memory only.
- Desktop: the seam rule; `backendCall` from services only; new commands in `generate_handler!` + `ipc_sig!`;
  `bun run bindings`; `bun run memory` after structural changes; guards live in Rust, never in Vue. D7: reads,
  sales, edits, backup and export are **never** blocked.
- **Cargo only through `scripts/cargo-safe.ps1`** (it's a mutex). `cargo check` is fine while implementing. Batch
  test runs at the end. Never call cargo directly, and never run two at once.
- Commit at the end of each phase with a scoped message, e.g. `feat(backend): phase A — foundation`,
  `feat(dashboard): phase D — admin + portal`, `feat(licensing): phase E — entitlements + activation`.
  Stage only that phase's files. Never stage `.env`.
- Skills: `/frontend-design:frontend-design` judgment for dashboard polish (the UI rules in
  `apps/dashboard/docs/05-ui-rules.md` always win), and `/code-review` on each phase's diff before its commit.

## Items that need me: stop at these and list them, don't fake them

- The R2 credentials and bucket names, and the GOWA WhatsApp credentials. Until then, keep them empty and use the dev
  adapters.
- Running the Van Westendorp interviews (G1). Just prepare the sheet template.
- Storing the Tauri updater signing key on the build machine. Configuring Coolify and deploying.
- The real `bun run desktop` check against staging (G5), which needs a deployed server. Run everything else in G5.

## Definition of done

- Phases A–F: every box ticked and every gate green.
  - Backend: `lint`, `test`, `drizzle-kit check`, and `build` + `start` → `/health` 200.
  - Dashboard: `build` + `check`.
  - Desktop: `build`, `check`, `verify:mocks`, `memory`, `diag:check`, full e2e, plus cargo build/test and
    `bindings:check` through `cargo-safe.ps1`.
- Phase G: everything not in "Items that need me" is done.
- The plan README has a "Waiting on the owner" section listing exactly what's left. The plan stays in `pending/`
  until those are done (CLAUDE.md plan lifecycle).
- A final report to me: what was built per phase, test counts, deviations, and the waiting-on-owner list.

Start with Phase A now.
