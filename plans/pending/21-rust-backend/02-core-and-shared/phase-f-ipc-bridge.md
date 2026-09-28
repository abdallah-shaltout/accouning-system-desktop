# 21 · 02.F — IPC bridge: typed bindings, the frontend switch, change events

> **Status (2026-09-27, implementer F, wave 2):** code complete, not yet compiled/tested (build
> rule 2026-09-27) — this session finishes what wave 1 deferred: F-2's `core_backend_status`
> command (`src-tauri/src/core/status.rs`, new) built from `AppState` + (on Windows, Main PC only)
> `AppState.server`'s live `ServerRuntime`/`server.json`; its `ipc_sig!` line in `core/ipc.rs`
> (manifest is no longer empty); F-3's `getBackendStatus()` in `backend.ts`; and F-5's change
> poller (`src-tauri/src/core/poller.rs`, new — a tokio task spawned from `state::boot`, 2s tick,
> pure diff function unit-tested without a DB). `src/modules/core/types/gen/ipc.gen.ts` was
> hand-updated to include the one registered command (ts-rs 12 emits an `import type` line per
> referenced DTO plus one interface member — matched exactly by hand since `cargo test` could not
> be run this session either; same reason as wave 1, other agents' files were still mid-edit at
> various points). `contract.check.ts` gained a `_BackendServerStatus` identity check (was missing
> in wave 1, the nested DTO F-2 also lists). **Left for the next session / the manager:** the full
> gate chain below — none of it has run this session (write-only per the task brief): `cargo build`
> + `cargo test` (in particular `export_bindings`, `ipc_manifest_matches_handler`, and this file's
> two new `#[cfg(test)]` modules in `status.rs`/`poller.rs`), `bun run bindings` (must produce a
> byte-for-byte-or-better `ipc.gen.ts`/`BackendStatus.ts`/etc. — compare against what's hand-written
> here), `bun run bindings:check`, `bun run build`, `bun run check`, `bun run verify:mocks`,
> `bun run memory` (+ `memory:check`), `bun run diag:check`, and the drift-check proof. A real
> `bun run desktop` check is deferred to the final testing plan per project convention (no exercise
> of `core_backend_status`/the poller against a live app this session either).

**Goal:** (1) Every Rust DTO and command signature is generated as TS and **type-checked against
the existing TS type** by `vue-tsc` (master rule 1). (2) One switch point in
`src/modules/core/services/` decides mock or Rust per domain (master §5), with no page edits.
(3) Cross-terminal refresh reaches the existing `onLedgerChanged`/`onCatalogChanged` subscribers
unchanged (cross-cutting §5).

**Tool decision (P2-01): ts-rs.** tauri-specta generates types and command wrappers, but its v2
line has lived on release candidates, it replaces `generate_handler!` with its own builder (which
would break `scripts/memory/parse/rust.ts` and CLAUDE.md's registration rule), and it needs the
app to run to export. ts-rs is stable, respects serde attributes, has `#[ts(optional)]` for exact
`field?: T` output, and exports through `cargo test`. The one thing it doesn't do (a command map)
is covered by a small `ipc_sig!` manifest, and a Rust test checks that manifest against
`generate_handler!`.

**Read first:** [`../01-frontend-analysis/cross-cutting.md`](../01-frontend-analysis/cross-cutting.md)
§4–§6 · `src/modules/core/types/{paging,route,index}.ts`, `src/modules/diagnostics/types/index.ts`,
`src/modules/diagnostics/services/defineService.ts`, `src/mocks/{events,utils}.ts`, `src/main.ts`,
`scripts/memory/parse/{ipc,rust}.ts`, `scripts/contract/{config,extract}.ts`.

## Tasks

### F-1 — ts-rs plumbing (Rust)
- [x] Add `ts-rs` (latest stable; features `uuid-impl`, `chrono-impl`; **not** a decimal impl, so
      a `Decimal` without `#[ts(type = "number")]` fails to compile). Add
      `src-tauri/.cargo/config.toml` `[env] TS_RS_EXPORT_DIR = { value = "../src/modules", relative = true }`.
      *(ts-rs 12.0.1 was already pinned in `Cargo.toml` by another agent before this wave started —
      confirmed its `[env] TS_RS_EXPORT_DIR` semantics and `#[ts(export_to/optional/type)]` behavior
      directly against the vendored crate source, see "Deviations" below.)*
- [x] DTO conventions (enforced by the checks in F-4): `#[serde(rename_all = "camelCase")]`;
      `#[serde_with::skip_serializing_none]` + `#[ts(optional)]` on `Option` fields (absent, never
      `null`, to match TS `field?: T`); `Decimal` fields use `#[serde(with = "utils::money::serde_number")]`
      + `#[ts(type = "number")]`; counts use `i32`/`u32` (ts-rs maps `i64` to `bigint`); `Id`
      exports as `string`; `DocDate` and `iso_ms` instants export as `string`; links use
      `#[ts(type = "import('@/modules/core/types/route').AppRoute")]` (no shim file); `unknown`
      values use `#[ts(type = "unknown")]`. Rust DTO names equal the TS type names; arguments are
      `<FnPascal>Args` structs (one `args` param per command, matching 01.B's "Request DTO"
      column — no commands registered this wave, so no `Args` structs exist yet either). Output
      goes to `src/modules/<vue-module>/types/gen/` (P2-32).
- [x] `core/ipc.rs`: `IpcSig { name, args, returns }` built by `ipc_sig!(cmd, ArgsType | (), ReturnType)`,
      and `all_signatures()`. A `#[test] export_bindings` exports every registered DTO and writes
      `src/modules/core/types/gen/ipc.gen.ts`
      (`export interface IpcCommands { core_backend_status: { args: undefined; returns: BackendStatus } }`
      with relative type imports taken from ts-rs names and paths). *(Wave 2: `all_signatures()` now
      has its first real entry, `ipc_sig!(core_backend_status, (), crate::core::dto::BackendStatus)`
      — the emitted `IpcCommands` interface has exactly one member, hand-matched in `ipc.gen.ts`
      since `cargo test` could not run this session either.)*
- [x] `#[test] ipc_manifest_matches_handler`: parse `src/lib.rs`'s `generate_handler![…]`, take the
      last path segments, and compare them with `all_signatures()` names in both directions. The
      10 untyped legacy commands (`render_pdf, render_preview, list_printers,
      print_thermal_receipt, print_test_receipt, diag_append, diag_read, diag_clear,
      diag_open_folder, diag_rotate`) are an explicit `LEGACY_UNTYPED` list.

### F-2 — Cross-cutting DTOs (the pilot for the whole mechanism)
- [x] New TS types first (rule 1: the frontend defines them): `src/modules/core/types/backend.ts`
      with `ApiErrorCode = 'NOT_FOUND'|'VALIDATION'|'CONFLICT'|'FORBIDDEN'|'UNAUTHORIZED'|'INTERNAL'`,
      `ApiErrorPayload { code; message }`, `ChangeCategory = 'ledger'|'catalog'|'parties'`,
      `BackendChangedPayload { categories }`, and `BackendStatus { connected: boolean; role:
      'main'|'terminal'; terminalId: string; serverVersion?: string; schema: 'ok'|'behind'|'ahead'|'unknown'; error?: string;
      server?: { state: 'provisioning'|'starting'|'upgrading'|'running'|'stopped'|'failed'; version: string; port: number; lanSharing: boolean; failure?: { code: string; message: string } } }`
      (`server` is present only on a Main PC with a managed server — added by phase A2, C-22; built
      from `AppState.server.snapshot()`. The IPC command count is unchanged.).
- [x] Rust DTOs with exported bindings: `ApiErrorPayload` (the serialised shape of `AppError`),
      `ChangeCategory`, `BackendChangedPayload`, `BackendStatus`, `PageSort`, `PagedQuery<F>`,
      `PagedResult<R>` (`total: u32`; `totals` as `Record<string, number>`, core.md §2),
      `ActivityKind`, `ActivityEntry`, `AuditAction`, `AuditFieldDiff`, `AuditEntry`.
- [x] The **only new IPC command in Part 02**: `core_backend_status` (no args) → `BackendStatus`,
      built from `AppState` (`db_status`, device role, terminal id, server version, schema check).
      Registered in `generate_handler!` (by the manager, wave 1) with an `ipc_sig!` line
      (`core/ipc.rs::all_signatures()`, wave 2). `src-tauri/src/core/status.rs` (new): a plain
      `async fn core_backend_status(state: State<'_, AppState>) -> Result<BackendStatus,
      ApiErrorPayload>` — read-only, never actually errors (even `NotConfigured` is a normal
      `BackendStatus`), kept as a `Result` only so a future failure mode has somewhere to go without
      changing the signature. On Windows + `DeviceRole::Main`, `server` is built from
      `state.server.state` (`tokio::sync::RwLock<ServerRuntime>`, read with `.read().await` — no std
      lock held across it) plus `server.json`'s `lan_sharing` (not on `ServerRuntime` itself);
      `None` on a terminal, on non-Windows, or when no managed server was ever provisioned.

### F-3 — The switch point (`src/modules/core/services/backend.ts`)
- [x] `BackendDomain` = the Rust domains in master §4 **plus `diagnostics`** (C-10): `accounting,
      analytics, approvals, core, dashboard, diagnostics, expenses, invoices, parties, payments,
      products, purchases, reports, settings, setup, templates, users, vouchers`.
- [x] `RUST_DOMAINS: ReadonlySet<BackendDomain>` = **empty** (Part 04 flips domains one at a time).
      Dev builds only: `localStorage['equal.backend']` (a comma list, or `*`) adds domains, the
      same pattern as `equal.debug`.
- [x] `usesRust(domain)` = `isTauri() && (RUST_DOMAINS has domain || dev override)`. Browser/e2e
      always use the mock (master §5). **No fallback** to the mock after a Rust failure (P2-34).
- [x] `backendCall<K extends keyof IpcCommands>(command: K, ...args)` returns
      `Promise<IpcCommands[K]['returns']>`. It calls `invoke(command, { args })` and converts any
      rejection into `ApiError(message, code)` (a `{code, message}` object → same code; anything
      else → `INTERNAL` with the generic Arabic message). Add `'INTERNAL'` to `ApiError`'s code
      union in `src/mocks/utils.ts:55` (C-02). `wrap()`'s logging and the `E-XXXX` toast then work
      unchanged (cross-cutting §4).
- [x] `getBackendStatus = wrap('core.getBackendStatus', …)` uses a literal `invoke<BackendStatus>('core_backend_status')`.
      Implemented in `backend.ts` as `backendCall('core_backend_status')` (no args) — not gated by
      `usesRust()`, since there is no mock equivalent to switch away from.
- [x] `initBackendBridge()`: when `isTauri()` and at least one domain uses Rust, `listen('backend:changed')`
      and call `emit(\`${category}:changed\`)` from `src/mocks/events.ts` for each category. Called
      from `src/main.ts` next to `initDiagnostics()`. (Moving `ApiError` and the event bus out of
      `src/mocks` is a D5 prerequisite; noted in the entry file §10, not done here.)
- [x] The service pattern Part 03 applies (written as the module's doc comment):
      `if (usesRust('products')) return backendCall('products_get_product', { id });` at the top of
      the existing wrapped function, with the mock body unchanged below it. Pages never change.
      *(Documented in `backend.ts`'s own doc comment; no existing service was modified this wave —
      `RUST_DOMAINS` is empty, so Part 03/04 do the actual per-service edits.)*

### F-4 — Drift checks that fail `vue-tsc`
- [x] `src/modules/core/types/contract.ts`: `Equals<A,B>` (the exact-identity conditional-type
      form) and `Expect<T extends true>`.
- [x] `src/modules/core/types/contract.check.ts` and `src/modules/diagnostics/types/contract.check.ts`:
      one `export type _X = Expect<Equals<Gen.X, X>>` per F-2 DTO (generics checked at a concrete
      instance — `PagedQuery<Record<string, unknown>>` / `PagedResult<string>`, the same concrete
      type argument the Rust `export_bindings` test uses). Mutual assignability is allowed only
      with a comment explaining why identity can't be expressed — not needed; every DTO in this
      wave achieves exact identity.
- [x] `IpcCommands` makes an unknown command name a `vue-tsc` error, and makes a wrong return type
      fail at the service's declared return type. *(True by construction — `backendCall`'s generic
      signature is typed against `IpcCommands`; not yet exercised by a real call site since no
      command is registered this wave.)*
- [x] Added `_BackendServerStatus` to `contract.check.ts` (F-2's nested DTO, missing from wave 1's
      list). `src/modules/core/types/gen/*.ts` already existed on disk from wave 1 (hand-written,
      matching ts-rs 12's format); this wave only hand-updated `ipc.gen.ts` to add the one
      registered command's entry (`core_backend_status: { args: undefined; returns: BackendStatus }`
      plus its `import type { BackendStatus } from "./BackendStatus"`).
- [ ] ⏳ runs in the single final build/test pass — `bun run build` type-checking `contract.check.ts`
      (including the new `_BackendServerStatus`) against the real `cargo test`-produced `gen/*.ts`
      has not been run this session.

### F-5 — Change poller (Rust) — P2-11
- [x] `src-tauri/src/core/poller.rs` (new): `spawn(app: AppHandle)` called once from
      `state::boot` (`src-tauri/src/core/state.rs`, after the existing `connect_and_migrate` spawn —
      A2's boot logic and the `traces`/`undo` fields are untouched). A `tokio::time::interval` tick
      every 2s reads `SELECT category, version FROM change_versions`, diffs it against
      `AppState.change_seen` via the pure `diff_changed_categories(seen, current)` function (unit
      tested, no DB), raises `change_seen` to the DB's current values, and emits `backend:changed`
      (`state.events.emit_changed`) for exactly the categories whose version rose beyond what this
      process last saw — own commits never re-emit because `with_tx` (`core/tx.rs`) already raises
      `change_seen` to the post-commit value at commit time, so by the poller's next tick this
      process's own cursor is already caught up. Never holds `AppState.change_seen`'s std `Mutex`
      guard (or `state.db`'s `RwLock` read guard) across an `.await` — both are cloned/copied out
      and dropped before the next await point, same discipline as `core::tx::current_connection`.
      Skips quietly when `state.db` is `None` (no connection yet — A-6: the mock keeps working);
      logs a DB error via `log::warn!(target: "core::poller", …)` only on the transition into
      failure (an internal `LastPoll` enum tracks the last outcome), never once per tick. Stops
      automatically when the app exits (`tauri::async_runtime::spawn`'s future is simply dropped,
      like `state.rs`'s other boot-time spawn — no explicit cancellation token needed).

### F-6 — Tooling and docs
- [x] `package.json`: `bindings` = `cargo test --manifest-path src-tauri/Cargo.toml export_bindings`.
      `bindings:check` = `bun run bindings && git diff --exit-code -- "src/modules/*/types/gen"`.
      *(Already present in `package.json` before this wave — another agent/an earlier pass added
      it; verified it matches the spec exactly and made no changes.)*
- [x] `scripts/memory`: the IPC parser (`parse/ipc.ts`) takes its callee names from a new
      `config.ts` list `['invoke', 'backendCall']`. `memory:check` exits 1 when
      `invokedNotRegistered` or `registeredNotInvoked` is non-empty (master §8, "no contract gaps") —
      added this gate to `scripts/memory/run.ts`'s `--check` branch (it didn't check this before).
- [x] `scripts/contract/config.ts`: exclude `**/types/gen/**` and `**/contract.check.ts` from the
      type inventory — added `excludeFromTypes` and wired it into `scripts/contract/extract.ts`'s
      type-collection call site.
- [x] CLAUDE.md "Rust ↔ Vue"/"Definition of done" edit — done by manager 2026-09-27.
- [x] F-12: `00-MASTER-PLAN.md` §4/§6 edits — manager.

## Tests / verification
- [ ] ⏳ runs in the single final build/test pass. Rust: `export_bindings` is deterministic (two
      runs, no diff); `ipc_manifest_matches_handler` (now has one real manifest entry —
      `core_backend_status` — to cross-check against `lib.rs`'s `generate_handler!`, which the
      manager already registered it in). `status.rs`'s 3 unit tests (`schema_status_maps_known_db_statuses`,
      `db_status_error_is_none_only_for_healthy_or_transitional_states`, `backend_role_maps_device_role`)
      and `poller.rs`'s 7 unit tests (the pure `diff_changed_categories` function: no-change,
      single-category rise, first-tick-nonzero, first-tick-zero, dropped-version, multi-category,
      and the `category_from_db_str` lookup) — all written this session, none run yet (write-only
      per the task brief).
- [ ] ⏳ The poller's two documented sub-items (deterministic diff; own-commits-don't-re-emit) are
      covered by the unit tests above for the pure-diff half; the "own commits don't re-emit"
      claim itself rests on `with_tx` already raising `change_seen` at commit time (existing A-7
      code, unmodified) — not independently re-verified against a live DB this session.
- [ ] ⏳ Drift proof — planned procedure unchanged from wave 1: flip `BackendStatus.connected`'s type
      in `core/dto.rs` from `bool` to `String` → `bun run bindings` → `bun run build` must fail
      inside `contract.check.ts`'s `_BackendStatus` line; revert; then rename `ApiErrorPayload`'s
      `code` field → `vue-tsc` must fail the same way. Not run this session.

## Gate
- [ ] ⏳ `cargo build` + `cargo test` — not run this session (write-only). `bun run bindings:check`
      — not run (depends on `cargo test`); the hand-written `ipc.gen.ts` should match what
      `export_bindings` produces, but this has not been confirmed by an actual `cargo test` run.
- [ ] ⏳ `bun run build`, `check`, `verify:mocks`, `contract` + `contract:check`, `memory` +
      `memory:check` (now 1 IPC command registered with a matching `ipc_sig!` entry — 0 gaps
      expected), `diag:check` — none run this session.
- [ ] ⏭ `bun run desktop` on Windows — deferred to the final testing plan per project convention.
- [x] Status note at the top of this file.
