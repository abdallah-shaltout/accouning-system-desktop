# 02 — Architecture (index)

The specs live **with the code they describe**, and those docs are the source of truth. This file only points to them
and records the few cross-project contracts.

| Topic | Source of truth |
|---|---|
| Backend folder structure (mirrors `references/accouning-system/POS-Fares-server`), what's copied, layering, events, cron | [`apps/backend/docs/03-architecture.md`](../../../apps/backend/docs/03-architecture.md) |
| Tables, constraints, subscription state machine | [`apps/backend/docs/04-data-model.md`](../../../apps/backend/docs/04-data-model.md) |
| Every endpoint (admin / portal / device), **license token format**, error codes | [`apps/backend/docs/05-api-spec.md`](../../../apps/backend/docs/05-api-spec.md) |
| Auth realms, refresh rotation, device credentials, license keys, rate limits | [`apps/backend/docs/06-security.md`](../../../apps/backend/docs/06-security.md) |
| `.env` keys, Coolify settings | [`apps/backend/docs/07-environment.md`](../../../apps/backend/docs/07-environment.md) |
| Backend rules for agents | [`apps/backend/CLAUDE.md`](../../../apps/backend/CLAUDE.md) |
| Dashboard module-first structure, HTTP/auth flow, routing, deploy | [`apps/dashboard/docs/03-architecture.md`](../../../apps/dashboard/docs/03-architecture.md) |
| Every dashboard page | [`apps/dashboard/docs/04-screens.md`](../../../apps/dashboard/docs/04-screens.md) |
| Dashboard UI rules | [`apps/dashboard/docs/05-ui-rules.md`](../../../apps/dashboard/docs/05-ui-rules.md) |
| Dashboard rules for agents | [`apps/dashboard/CLAUDE.md`](../../../apps/dashboard/CLAUDE.md) |

## Cross-project contracts

1. **The entitlement catalog** (keys and kinds from [01 §3](01-pricing-and-tiers.md)) is defined once in
   `apps/backend/src/domains/plan/schema/entitlements.schema.ts`.
   - `bun run catalog:export` writes `src-tauri/src/domains/licensing/catalog.snapshot.json`.
   - A desktop Rust test fails if `licensing/catalog.rs` drifts from that snapshot.
2. **The license token / Free policy / credit grant** format is in `apps/backend/docs/05-api-spec.md#license-token`.
   The desktop verifies them in `src-tauri/src/domains/licensing/service/token.rs` with the public keys in `keys.rs`.
3. **The device API** (`/api/device/*`) is the only surface the desktop calls. Its URL comes only from
   `src-tauri/src/domains/licensing/config.rs` (D17).
4. **Error telemetry export** matches `IngestFinding` in `scripts/diagnostics/run.ts`.
5. **The activation deep link** is `equal://activate?code=&state=` (desktop `tauri-plugin-deep-link`), produced by
   the dashboard `/activate` page.
