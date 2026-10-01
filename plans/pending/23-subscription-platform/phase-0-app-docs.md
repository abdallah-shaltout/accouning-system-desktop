# Phase 0 — App docs (goals, requirements, specs, rules)

> **Status (2026-09-29): done.** Every doc below is written. Phases A–G implement against them. When an
> implementation choice changes a spec, update the spec in the same change.

## 0.1 — `apps/backend`

- [x] `README.md`: what it is, the stack, the doc index, commands.
- [x] `CLAUDE.md`: mandatory rules (copy from the reference first, structure, data, security, config, definition of done, don'ts).
- [x] `docs/01-goals.md`: goals G1–G6, non-goals, success metrics.
- [x] `docs/02-requirements.md`: FR-AUTH/ORG/PLAN/SUB/PAY/DEV/LIC/CRED/REL/OPS and NFR-SEC/DATA/REL/PERF/OBS/I18N/OPS.
- [x] `docs/03-architecture.md`: the layout mirroring `references/accouning-system/POS-Fares-server`, the copy list,
      the deviations, layering, domains, events, cron.
- [x] `docs/04-data-model.md`: all tables and constraints, the subscription state machine.
- [x] `docs/05-api-spec.md`: admin, portal and device endpoints, the license token / Free policy / grant format,
      error codes.
- [x] `docs/06-security.md`: realms, refresh rotation with reuse detection, rate limits, license keys and rotation,
      data protection.
- [x] `docs/07-environment.md`: `.env` keys (the reference names plus the new ones, R2) and the Coolify settings.

## 0.2 — `apps/dashboard`

- [x] `README.md`, `CLAUDE.md` (module-first rules, navigation, UI, auth, definition of done, don'ts).
- [x] `docs/01-goals.md`: users, goals G1–G5, non-goals.
- [x] `docs/02-requirements.md`: FR-UI/ADM/POR and NFR-UI/SEC/A11Y.
- [x] `docs/03-architecture.md`: the module layout, module list, HTTP/auth flow, routing, the Coolify static deploy.
- [x] `docs/04-screens.md`: every page with route name, path, roles, data, primary action.
- [x] `docs/05-ui-rules.md`: tokens, RTL, forms, tables, copy, portal on phones.

## 0.3 — `apps/mobile-app`

- [x] `README.md`: a placeholder for the future Flutter companion (LAN sync over the same Wi-Fi, QR pairing shown by the
      desktop). There is no code and no tasks until it has its own plan.

## 0.4 — Plan wiring

- [x] Plan 23 `README.md` D1 updated (the projects live in `apps/`, no workspaces). O2 resolved to Cloudflare R2.
- [x] [02-architecture.md](02-architecture.md) turned into an index pointing at the app docs, plus the cross-project contracts.
- [x] Phase files point at `apps/backend` / `apps/dashboard` and the app docs.

## Gate 0

- [x] Every link in the app docs and the plan resolves.
- [x] `bun run memory` regenerated (the new plan files are indexed; `apps/` is outside the scanner's roots).
