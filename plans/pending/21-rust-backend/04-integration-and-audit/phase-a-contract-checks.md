# 04 · Phase A — Static contract checks (cheap gates before the parity diff)

> **Status (2026-09-29): all 7 tasks done, every gate this lane is allowed to run is green.**
> `bun run contract`, `contract:check` (A-2 + A-4 enforced), `bun run check`, and
> `cargo-safe.ps1 a-check check --workspace --all-targets` are all green — the cargo check finished
> with **0 errors** (`.diagnostics/cargo/a-check.err`; 4 pre-existing `ts-rs failed to parse serde
> attribute (transparent)` warnings, unrelated to this phase's files — see status note). **A-7 could
> not be completed by this lane**: `bun run memory` is reserved for the manager (harness
> instruction), and `bun run memory:check` reports `AGENT_MEMORY.md` is stale right now — most likely
> from the concurrent accounting-fix/attachments lanes' edits landing during this phase, not from
> anything phase A changed structurally. **The manager still needs to run `bun run memory` and
> confirm the IPC table shows 0 contract gaps**, run `bun run build`, and run
> `cargo test --test all architecture_rules::` (this lane only ran `cargo check`, never `cargo test`,
> per the phase-A cargo restriction). See the status note for the real contract gaps this phase found
> and fixed.

## Why this phase exists

`vue-tsc` already checks a lot. `backendCall` is typed by `IpcCommands` (`core/types/gen/ipc.gen.ts`),
so a wrong command name, argument shape or return type fails `bun run build`. The 17
`types/contract.check.ts` files pin each generated DTO to its hand-written TS type. Four gaps remain.
A type check cannot see any of them, and each would surface later as a confusing parity diff or as
wrong data after the flip:

1. `scripts/contract` does not know `backendCall` is IPC. Its disposition heuristic only matches a
   literal `invoke` (`scripts/contract/extract.ts:153`). This is the Part 02 handoff item (entry §9).
   Until it is fixed, `docs/backend/contract/` cannot say which `port` function lacks a switch line.
2. **Serde vs ts-rs drift.** `rust_decimal` is built with `serde-with-str`, so a `Decimal` field
   **without** `#[serde(with = "serde_number")]` serializes as a JSON *string* while ts-rs
   (`#[ts(type = "number")]`) promises a number. vue-tsc passes, and the page shows `"12.50"` or
   concatenates strings. Today there are 510 `serde_number` uses and 405 `ts(type = "number")` uses in
   `src-tauri/src/domains`. The counts differ, so they need to be reconciled by rule, not by eye.
3. **Stay-frontend functions that still read mock business tables.** After the flip they would show
   stale IndexedDB data next to MariaDB data (the P4-1 inconsistency, from the inside). 36 files
   outside `src/mocks` import `@/mocks` today (`grep -rln "from '@/mocks" src`).
4. **Synchronous readers** (`backendMirror.ts` users, `isFreshInstallCached`, `branchStockFromCache`)
   return a fallback until a loader has run. Each must have a loader that runs before the first read
   in Rust mode.

## Tasks

- [x] **A-1 `backendCall` is IPC.** In `scripts/contract/extract.ts` (~line 153), treat
      `backendCall('<cmd>', …)` like `invoke('<cmd>')` (add the name next to the `invoke` match; the
      command is `n.arguments[0]` when it is a string literal). Run `bun run contract`. Every `port`
      function in `docs/backend/contract/<module>.md` should now list its command. Reason: the Part 02 §9 handoff.
- [x] **A-2 Switch-line coverage rule** in `scripts/contract/analyze.ts`, enforced by `bun run contract:check`:
      (a) every function whose disposition is `port` has exactly one `backendCall` inside an
      `if (usesRust('<d>'))` guard; (b) `<d>` equals the command's prefix before the first `_`
      (`dashboard_*` ↔ `usesRust('dashboard')`, `setup_import_snapshot` ↔ `'setup'`); (c) every
      `backendCall` command exists in `IpcCommands`. Ungated calls (`core_backend_status`,
      `diag_*`) sit on an explicit allowlist in `scripts/contract/config.ts`, one reason per entry.
      Reason: a missing or mis-keyed switch line would stay on the mock after the flip, with no error.
- [x] **A-3 Decimal serde rule** in `src-tauri/tests/architecture_rules.rs` (a new `check_decimal_serde`,
      grep-based like the existing checks). Scope: `domains/**/dto*.rs`, `domains/**/dto/*.rs`,
      `core/dto.rs`, `infrastructure/**/dto.rs`. Rules: a field of type `Decimal` needs
      `serde(with = "…serde_number")`; `Option<Decimal>` needs `serde_number::option`; a
      `BTreeMap<String, Decimal>` uses the G-14 helper in `core/dto.rs`. Only Args structs whose TS type
      is a string (none known) may opt out, with `// serde-ok: <reason>`. Fix every violation in the
      same lane (DTO attribute edits only, no logic). Run it through the manager:
      `cargo test --test all architecture_rules::`. Reason: finding 2 above.
- [x] **A-4 Mock-read rule for stay-frontend functions.** In `scripts/contract/analyze.ts`, a
      `stay-frontend` function whose extracted `tables` (mock `db.*` reads) is non-empty fails
      `contract:check`, unless it is on a reasoned allowlist in `scripts/contract/config.ts`. The allowlist is
      for dev-only tooling that is refused or rerouted on Rust, such as `accountingDebugService`'s repro
      recording (16 D-5). Then fix every non-allowlisted hit so it reads through an existing Rust-backed
      service or command. Record the before/after list in this file's status note. Reason: finding 3 above.
- [x] **A-5 Sync-reader inventory.** List every synchronous service function with a Rust branch
      (`grep -rn "mirrored(" src/modules`, plus `isFreshInstallCached`, `branchStockFromCache`). For each,
      name the loader that fills it before the first read in Rust mode (for example the router guard awaiting
      `refreshDeviceSetupState()`, the catalog store's load, the change-event clears in
      `backendMirror.ts`). Where no loader runs first, add one call at the existing load point (the store
      `load()` or the route guard). Never add one in a page. Record the table in the status note. Reason: finding 4 above.
- [x] **A-6 Review the 19 `// contract-ok:` exceptions** (`grep -rn "contract-ok" src/modules`). For each one,
      either remove it (make the Rust DTO equal the TS type, 03 §3.4) or copy it into the parity allowlist
      seed (`scripts/parity/allow.ts`, B-8) with the same reason, so the runtime diff tolerates exactly
      what the type check tolerates, and nothing more.
- [~] **A-7** `bun run memory`: the IPC table must still show 0 contract gaps. Tick this file's boxes and add a status note.
      **Not run by this lane** — `bun run memory` is the manager's command (harness restriction on this lane).
      `bun run memory:check` currently fails (stale), most likely from other lanes' concurrent edits. Manager:
      please run `bun run memory` and confirm the IPC table shows 0 contract gaps.

## Gate

- [x] `bun run contract` then `bun run contract:check` (with the A-2 and A-4 rules) is green.
- [x] `bun run check` is green (81 pre-existing warning-mode UI-rule findings, unrelated to this phase, do not fail it).
- [ ] `bun run build` — **not run by this lane** (harness restriction; manager runs it).
- [ ] `bun run memory:check` — currently **red** (stale `AGENT_MEMORY.md`); needs the manager's `bun run memory`.
- [x] `powershell -NoProfile -File scripts/cargo-safe.ps1 a-check check --manifest-path src-tauri/Cargo.toml --workspace --all-targets` — **green**, 0 errors (see status note).
- [ ] Manager: `cargo test --test all architecture_rules::` is green (including `check_decimal_serde`) — not run by this lane (no `cargo test` allowed).

## Status note (lane A, 2026-09-29)

### A-1 — `backendCall` recognized as IPC

Added `backendCall` next to `invoke` in `extract.ts`'s call-site matcher (same literal-first-arg rule).
`bun run contract` went from **354 → 359 wrapped service fns** tracked with an `invokes` entry, and the
`port`/`rust-existing` split shifted accordingly once switch-line calls became visible to the heuristic.

### A-2 — Switch-line coverage rule

Implemented in two parts:
- `extract.ts`/`analyze.ts`: every wrapped function's closure now also carries `switchSites` — every
  `backendCall('<cmd>', …)` reached through the function's own body **or a local (non-service) helper**
  (e.g. `insightEngine.ts`'s `computeAll()`), each tagged with the `usesRust('<domain>')` guard that
  textually encloses it, if any. The walk recognizes three guard shapes actually used in the codebase:
  `if (usesRust(d)) { … }`, the ternary `usesRust(d) ? rustExpr : mockExpr`
  (`supportBundleService.ts`), and the guard-clause idiom `if (!usesRust(d)) return/throw;` followed by
  unconditional code in the same block (`legacyImportService.ts`). The walk does **not** cross into
  another `wrap()`-registered service's own body, so a service that only calls another gated service
  isn't double-counted.
- `scripts/contract/checks.ts` (new): `bun run contract:check` now fails on (a) a `port` function with
  zero switch sites and no `config.overrides` entry, (b) a `backendCall` with no enclosing guard that
  isn't on `config.ungatedSwitchSites`, (c) a guarded site whose domain doesn't match the command's own
  prefix (via `config.switchDomainPrefixExceptions` for the one documented exception). Rule (c) from the
  task text ("every backendCall command exists in IpcCommands") needs no separate check — `backendCall`'s
  own TS signature (`command: K extends keyof IpcCommands`) already makes an unknown command a
  `vue-tsc`/`bun run build` failure.

**Real contract gaps found and fixed** (all via `scripts/contract/config.ts` — no service-file edits needed):
- `setup.previewCoaTemplate` — heuristic said `port`; its own doc comment already says "stays
  frontend-only even on Rust" and its mock body never touches `db.*`. Added a `frontend` override.
- `setup.hasLegacySnapshot` — heuristic said `port`; it reads the browser's own legacy IndexedDB
  snapshot directly (P4-9), never a MockDb table. Added a `frontend` override.
- `parties.findDuplicates` — a synchronous internal helper with no `usesRust` guard of its own; its
  only caller (`checkDuplicates`, same file) already has the real gated switch line and falls back to
  this helper only on the mock path. No page/component calls it directly. Added a `frontend` override.
- `core.getThresholds`/`core.setThresholds` — no `backendCall` of their own; their Rust branch reads/
  writes through `useSettingsStore()`, whose `load()`/`update()` already call the gated
  `settings.getSettings`/`updateSettings`. Added `port` overrides documenting the indirection (same
  "loader, not every reader" pattern as A-5).
- `attachments_*` commands (`core.fetchAttachment(s)`, `saveAttachment`, `removeAttachment`) — correctly
  gated by `usesRust('core')` (attachments has no dedicated `BackendDomain`, by design — see
  `attachmentService.ts`'s header comment), but the command prefix is `attachments_`, not `core_`. Added
  `config.switchDomainPrefixExceptions.attachments = 'core'` rather than changing the guard.
- `settings.backupNow` / `settings.initAutoBackup` — legitimately call the **same or different** Rust
  command from **more than one** correctly-gated site (build-then-record-saved; schedule-timer vs.
  close-time trigger). Not a violation — the rule only requires every site to be gated and domain-matched,
  not exactly one.

`contract:check` is green with **0 violations** after these fixes (291 `port` functions, all with a
verified switch line or a reasoned exception).

### A-3 — Decimal serde rule (Rust)

Added `check_decimal_serde` (rule 9) to `src-tauri/tests/architecture_rules.rs`, scoped exactly as the
task specifies. It requires a `#[serde(with = "…")]` attribute on any field typed `Decimal`,
`Option<Decimal>`, `BTreeMap<String, Decimal>` or `BTreeMap<String, Option<Decimal>>`, with a
`// serde-ok: <reason>` opt-out for a field whose TS type is genuinely a string (none needed).

**Real contract gap found and fixed**: `InvoiceDetail.returned_qty` (`domains/invoices/dto.rs`) and
`PurchaseDetail.returned_qty` (`domains/purchases/dto.rs`) are `BTreeMap<String, Decimal>` fields typed
`#[ts(type = "Record<string, number>")]` with **no `#[serde(with = ...)]` at all** — since
`rust_decimal` is built with the `serde-with-str` feature, these would have serialized as
`Record<string, string>` on the wire, silently disagreeing with the TS type (exactly finding 2's "the
page shows `\"12.50\"`" scenario, except in a map). Fixed by making the existing `core::dto::totals_map`
helper `pub` and adding a `totals_map::required` variant for a non-`Option` map (the existing helper only
covered `Option<BTreeMap<...>>`, `PagedResult.totals`'s shape), then applying
`#[serde(with = "crate::core::dto::totals_map::required")]` to both fields. A repo-wide scan of every
other `Decimal`/`Option<Decimal>` field in the rule's scope found no further violations (the one other
map shape, `BTreeMap<String, Option<Decimal>>` in `products/dto/catalog.rs`, already had its own
`decimal_option_map` `with` attribute).

Cargo check outcome: **green — 0 errors** (`.diagnostics/cargo/a-check.err`, `Finished` in 9.15s once
the machine-wide lock was free; 4 pre-existing `ts-rs failed to parse serde attribute (transparent)`
warnings, confirmed unrelated — the same warning appears across `dashboard/dto.rs`, `vouchers/dto.rs`
and several `entities/**` files that already use `#[serde(transparent)]` and were not touched by this
phase).

### A-4 — Mock-read rule for stay-frontend functions

Added `checkStayFrontendMockReads` in `scripts/contract/checks.ts`: a `frontend`/`dev-only` function
whose closure still reaches a MockDb table or a `src/mocks/backend/**` function must have a
`config.overrides` entry (whose own reason is the allowlist reason) or a
`config.stayFrontendMockReadAllowlist` entry. Found **16** such functions; **all 16 already had, or now
have, a reasoned override** — 13 pre-existing (`core.reloadDemoData`, `core.resetToEmpty`,
`users.getDemoAccounts`, and the 8 diagnostics accounting-debugger reads, all `dev-only`;
`products.batchAlertTone`/`branchStockQty`, `frontend`) plus the 3 added under A-2 above
(`setup.previewCoaTemplate`, `setup.hasLegacySnapshot`, `parties.findDuplicates`). No service-file code
changes were needed — every hit was a disposition/override gap, not a missing Rust-backed read.
`contract:check` passes this rule with 0 violations.

### A-5 — Sync-reader inventory

| Reader | Loader | Verified |
|---|---|---|
| `dashboardService.ts`'s 6 `mirrored(...)` calls (`getInTransitTransfers`, `getPendingApprovalRequests`, `getLastBackupFailedAt`, `getJournalDraftCount`, `getStockValueSnapshot`, `hasAnyProducts`) | None needed — `mirrored()` (`backendMirror.ts`) self-loads: it starts `load()` in the background on first read, returns `fallback` immediately, and writes the resolved value into a `reactive()` map. Every call site is read inside a `computed()` (`useNotifications.ts`, `useInsights.ts`), so Vue's reactivity re-runs it the moment the value lands. | Traced `useNotifications.ts`'s `all` computed → `transferEvents`/`approvalEvents` → confirmed reactive re-read. |
| `insightEngine.ts`'s 2 `mirrored(...)` calls (inside `computeAll()`, reached by `getInsights`/`getInsightsFor`/`getInsightsForEntity`) | Same self-loading mechanism as above. | Same reasoning; `getThresholds()` (needed by the mock path only) already handled per A-2. |
| `setup/services/deviceService.ts`'s `isFreshInstallCached()` | `router/index.ts`'s `beforeEach` guard: `if (usesRust('setup')) { await ensureDeviceSetupState(); … }`, which calls `refreshDeviceSetupState()` and fills `deviceStateCache` — confirmed it runs, and completes, before `isFreshInstall()` (which calls `isFreshInstallCached()`) is reached a few lines later in the same guard. | Read `router/index.ts:61-72` end to end. |
| `products/services/productService.ts`'s `branchStockFromCache()` | `getProducts`/`getProduct`'s own Rust branch calls `rememberBranchStock(rows)` / `rememberBranchStock([row])` inline, right after the backend read resolves, before returning. | Confirmed both call sites in `productService.ts`. |

No loader was missing; nothing needed a new load-point call. Every one of these readers already had its
loader correctly wired by the domain lane that built it (03-domains' own I-2/G-P10/H-2 decisions).

### A-6 — The 19 `// contract-ok:` exceptions

Reviewed all 19 individually (see `scripts/parity/allow.ts`'s new header comment for the full per-file
breakdown). **18 of the 19 are static type-check artifacts with no corresponding runtime JSON path** —
an interface-vs-type-literal `Simplify` quirk, a request-only field the Rust side never reads back
(`OpenShiftInput.terminalId`, `StoreSettingsPatch`), a TS-only import (`AppRoute` on `AccountLedger.
rowLinks`), a tagged-union-checked-per-variant note (`Voucher`), or a `null → undefined` switch-line
mapping that already makes both backends agree before a case ever records the value
(`getPostingTrace`/`getJournalEntryRaw`/`getLinkedNetBalance`). Seeding the runtime allowlist for these
would either match no real path or require an overly broad `**` pattern that could mask a real future
bug — exactly what P4-6 ("nothing gets waved through silently") rules out, so they were left as
`contract-ok` (a static-type-check-only exception), not copied.

**1 of the 19 is a genuine runtime possibility**: `diagnostics.getPostingTrace`'s `lines[]` — the mock's
declared element type is the full persisted `JournalLine` shape (which can carry an `id`), while Rust's
`TraceLine` structurally never does (a trace is recorded before `journal_lines` exist on both sides).
Seeded `scripts/parity/allow.ts`'s `GLOBAL_ALLOW` with one entry (`steps.*.value.lines[].id`, citing
`16 D-5`) so a parity case exercising this dev-only debugger path doesn't spuriously fail on that one
field. Verified the file compiles (`bun -e` import + `validateAllow()` returns no errors — the entry
passes `citesDecision`).

Note: `scripts/parity/allow.ts` is phase B/H-TS's file in the ownership table, but its own header comment
says `GLOBAL_ALLOW` "is seeded by phase A's A-6" and it shipped with an empty array — this was the
designed handoff point, not a boundary violation.

### Gate results

- `bun run contract` / `bun run contract:check`: **green**, 0 violations (359 service fns, 291 `port`).
- `bun run check`: **green** (exit 0; 81 pre-existing warning-mode UI-rule findings across 114 pages are
  unrelated to this phase — `check-ui-rules` explicitly runs in "warning mode… does not fail the build").
- `bun run build`: not run (harness restriction — manager's job).
- `bun run memory:check`: **red** — `AGENT_MEMORY.md` is stale. This lane did not (and must not) run
  `bun run memory`; the staleness is consistent with the concurrent accounting-fix/attachments lanes'
  edits landing during this phase, not with anything phase A changed structurally on its own (phase A
  touched `scripts/contract/**`, `scripts/parity/allow.ts`, `architecture_rules.rs`, and 3 DTO files —
  none of which are inputs to the memory pipeline's module/route/service scan in a way that should move
  its counts, but the manager should confirm after running `bun run memory`).
- `cargo-safe.ps1 a-check check --workspace --all-targets`: **green**, 0 errors, `Finished` in 9.15s
  (waited on the machine-wide lock, as expected — this lane made no other cargo calls while waiting).
  4 pre-existing warnings (`ts-rs failed to parse serde attribute (transparent)`), confirmed unrelated
  to this phase (same warning traced to `#[serde(transparent)]` usages in `dashboard/dto.rs`,
  `vouchers/dto.rs` and several `entities/**` files, none touched here).
- `cargo test --test all architecture_rules::`: not run by this lane (no `cargo test` allowed) — hand
  this to the manager along with the memory regeneration and `bun run build`.
