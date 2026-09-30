# 04 · Phase B — Parity harness (the same TS services, run on the mock and on Rust, diffed)

> **Status:** pending. Two parallel lanes in Wave 0: **H-RS** (Opus, the Rust host) and **H-TS**
> (Opus, the TS runner). **H-TS lands B-6 (`case.ts`) first**, because that file unblocks case
> authoring in all five B2 lanes. Decisions P4-2 to P4-6 in the entry file are why this phase has
> the shape it has.
>
> **H-RS status (2026-09-29): code complete; `check --workspace --all-targets --features accounting-app/parity`
> is clean (only the pre-existing ts-rs `transparent` notes).** Files: `src-tauri/src/bin/parity_host.rs` (new),
> `scripts/parity/build-host.ps1` (new: builds through `cargo-safe.ps1` and copies the exe to
> `.diagnostics/parity/bin/parity_host-<stamp>.exe`), `lib.rs` (`pub fn invoke_handler::<R>()` holds the
> one command list, `run()` calls it; plus `wry_handle`), `Cargo.toml` (`[features] parity`, `[[bin]] parity_host`),
> `core/diag/mod.rs` + `infrastructure/print/commands.rs` (generic `AppHandle<R>`), `domains/settings/commands.rs` +
> `domains/setup/commands.rs` (6 device/LAN commands generic), `infrastructure/import/{idmap,run}.rs` (B-5),
> `domains/settings/service/currency.rs` + its callers (`settings/commands.rs`, `setup/service/country_tax.rs`,
> 3 call sites in `tests/domain_settings.rs`) for B-3. Decisions logged:
> (1) **B-1 deviation:** the 6 settings/setup device/LAN commands take `AppHandle<R>` and get the `Wry` handle back
> through `crate::wry_handle` (a checked `Any` downcast; `FORBIDDEN "هذه الميزة غير متاحة في هذا الوضع"` on any
> other runtime). Their services stay `Wry`-typed: making them generic would have spread into `core/db.rs`,
> `core/poller.rs`, `core/events.rs`, `infrastructure/database/{hosting,lan,payload}.rs` (tray, autostart,
> bundled server), which are outside H-RS and never run in parity. The 6 diag/print commands are fully generic.
> (2) The list keeps its `generate_handler![` text in `lib.rs` (first occurrence in the file), so
> `ipc_manifest_matches_handler` and `scripts/memory/parse/rust.ts` read it unchanged.
> (3) **B-3:** of the plan's sites, `credit.rs:114` and `shared/stock/batches.rs:181-182` are inside `#[cfg(test)]`
> (not business logic), and `products/service/batches.rs:68` is the unreachable fallback of `SELECT UTC_DATE()`
> (which `SET timestamp` already pins). The plan's "use `cx.clock.today()`" there would contradict the documented
> D-I8/Q-I11 (the expiry report deliberately matches the mock's UTC "today"), so it stays. Only
> `settings/service/currency.rs` `create`/`update` moved to `cx.clock.now` (new `cx: &TxCtx` parameter).
> (4) The clock hook is the existing one: `with_tx`/`with_read_ctx` read `UTC_TIMESTAMP(3)`, so the host runs
> `SET timestamp = <epoch.micros>` (or `DEFAULT`) on its one pooled connection before every command and meta DB
> step. No new code hook was needed. (5) `__reset`/`__reset_empty` **rebuild the mock app** with a fresh `AppState`
> (instead of adding `clear()` methods to `ApprovalGrants`/`TraceRing`). This clears the session, grants, traces,
> change cursors, collected events and the static bootstrap-session marker. `__reset` runs `import_snapshot` inside
> `with_tx(require_user: false)` exactly like `setup_import_snapshot` (the debug build wipes first). (6) Pool:
> `max_connections(1)`, `acquire_timeout` 30 s (under the runner's 60 s), so a command that would need a second
> connection fails with a readable error instead of a kill. Panics are caught and returned as a string error.
> `EQUAL_PARITY_LOG` sets the stderr log level (default `info`; `__reset` logs its id-pair count and duration).
> **First host build (2026-09-29):** `scripts/parity/build-host.ps1` built it (exit 0, 362 s) and copied it to
> `.diagnostics/parity/bin/parity_host-20260929-210652.exe`. **The real rust pass is blocked by a migration bug, not
> by the host:** `bun run parity --lane L1 --case "_selftest/*"` against `bun run db:dev` (MariaDB 11.4.13) creates
> `equal_parity_l1`, applies m0001 to m0015, then **`m0016_part03_schema` fails** on its G-22 line
> `ALTER TABLE invoice_lines MODIFY COLUMN product_id CHAR(36) NULL`, with
> `1832: Cannot change column 'product_id': used in a foreign key constraint 'fk_invoice_lines_product_id'`
> (m0015 added that FK). The host exits 2 with that message on stderr, as designed. Every `TestDb::fresh()` runs the same
> `Migrator::up`, so all DB-backed cargo tests will hit it too. **Owner: the manager (migrations).** The half-migrated
> `equal_parity_l1` was dropped. So B-9's Rust timings are still unmeasured, and the B-H `cargo test` step is still open.
>
> **H-TS status (2026-09-29): code complete; mock side green end to end; the Rust side waits for the
> first host build.** Files: `scripts/parity/{case,bases,clock,host-protocol,transport,diff,allow,books,pass,bundles,services,ingest,run}.ts`,
> `diff.selftest.ts`, `cases/_selftest/identity.ts`; the `setParityTransport` hook in
> `src/modules/core/services/backend.ts`; `"parity"` in `package.json`. Verified: `bun scripts/parity/diff.selftest.ts`
> green (35 checks); `bun run parity --mock-only --case "_selftest/*"` green; `bun run parity --mock-only --bundles`
> replays ACC-0001…0008 green; the rust pass was driven end to end against a **protocol stub host** (a TS stand-in
> speaking `host-protocol.ts`): spawn, framing, `__reset` id pairs, `__clock`, the transport hook, `ApiError` code
> conversion, the id bijection and the 60 s timeout/kill path all work. `verify:mocks` (128 ok), `verify:replay`,
> `check` green; a `tsc` pass over `scripts/parity/**` is clean. Not run here (lane rules): `bun run build`,
> `bun run memory` (**stale now**: new scripts and a new `backend.ts` export; the manager regenerates it).
> Decisions logged: (1) `host-protocol.ts` is the H-RS ↔ H-TS contract: `__reset.snapshot` is the snapshot **object**
> (the host re-serializes it), `templates` is the `pdf_templates_v1` JSON string, `idPairs` is serde's default
> `[[mockId, uuid], …]` (the runner also accepts a map). (2) The rust pass blanks the mock `db`, so a service that
> still reads the mock in Rust mode shows up as a diff. (3) A step's top-level `undefined` and a unit command's `null`
> both record `null`. `[]`/absent/`null` inside a DTO stays a diff. (4) `demo-sa` is stamped `settings.country = 'SA'`,
> because the SA settings fixture has no country, and the Rust importer would otherwise use the machine's zone.
> (5) `Math.random` is seeded per case, and `uid()` counters are reset per pass (additive `resetIdCounters()` in
> `src/mocks/utils.ts`), so `--mock-only` really is a determinism check. (6) `--bundles` alone runs only the bundles,
> and with `--lane`/`--case` it runs both. Bundles gate on `ok` and `{code, message}`; value diffs are written to
> the output file only (they teach the id map). (7) `--host` also takes a `.ts` stub, run under Bun, for testing the runner.
> **B-9 timings:** mock `runAllInvariants` on `demo-sa` ≈ 21 ms per call (it runs after every step). Rust
> `__invariants` and `__reset` are **not measured yet**, because there is no host build. The runner prints both
> averages on every run, so record them after the manager's first host round.
> Finding: the plain `seedDatabase(2026-06-30, 'SA')` already fails `allocations-within-total`
> (`pay-135->inv-634 (48500 > 1000)`, which looks like an FX allocation compared in mixed currencies) before any step.
> `verify:mocks` does not run that invariant. The runner subtracts it as a baseline, like `verify:replay` does,
> but it needs an owner (probably L3 payments, or the invariant itself).
>
> **Follow-up (2026-09-29, closing the L1 harness gaps):** three harness-only gaps L1's Wave 1 found and
> routed around are now fixed, all within B-8/B-9's already-checked scope (not new checklist items):
> `scripts/parity/polyfills.ts` (new file) gives the harness an in-memory `localStorage`/`sessionStorage`,
> a real `indexedDB` (`fake-indexeddb`, added as a devDependency), a `window` alias, and a minimal
> `document` stub for `saveFile.ts`'s browser-download path — every install checks the global is
> genuinely absent first, so it can never affect the real app or `bun run dev`/`e2e`. `run.ts` imports it
> before any other module; `pass.ts`'s `prepareFrontend` (now `async`) calls `resetPolyfills()` every
> pass. Also fixed: `books.ts`'s `mockInvariants()` now catches a thrown invariant check (a base whose
> CoA is missing a role `accountFor()` needs, e.g. `edge`) and records one synthetic failing row instead
> of crashing `invariantsNow()` — this unblocked `import/import-edge` (00-import §8(b)) without touching
> the off-limits fixture or `invariants.ts`. Also fixed (L4 finding, same pass): `clock.ts`'s
> `unpinClock()` now always **reassigns** `process.env.TZ` to `ORIGINAL_TZ` instead of `delete`-ing it —
> Bun's ICU timezone cache ignores a `TZ` set after a delete, so a lane run mixing SA and EG cases used
> to silently keep computing every case's dates in whichever zone ran first; `bases.ts`'s `demo()`/custom
> branch now call `resetIdCounters()` (and `resetDb()` for `demo()`) before seeding, so a base's own ids
> no longer depend on which base another case's `loadBase()` call happened to build first; `pass.ts`'s
> `s.login`/`s.logout` step names are now de-duplicated (`login:<user>#2`, …) so logging the same user in
> twice, or calling `logout()` more than once, in one case no longer collides with the "step name used
> twice" guard. Separately, `src/mocks/backend/branches.ts`'s `createBranchCashAccount` used `uid('acc')`
> for a new branch's cash-drawer account — the same bare counter the chart-of-accounts fixture's own ids
> use (`acc-<code>`, e.g. `acc-1`/`acc-2` for the root "1"/"2" groups), so the demo seed's second branch
> (`فرع جدة`) minted an account literally colliding with a root CoA group; renamed to `uid('bacc')`.
> `bun run verify:mocks` (128 ok, 0 failed) and `bun run verify:replay` stayed green after that rename
> (no pinned figure changed — the FX/branches worked-example numbers in `verify:mocks`' output are
> unaffected). `bun run parity --mock-only --lane L1`: 31/31 green. `bun run parity --mock-only` (all 5
> lanes, 172 cases): all green, 0 unexplained diffs. See `phase-b2-parity-cases.md`'s L1 status note for
> the case-level detail (restored `templates-lifecycle` and the 3 backup cases, new `attachments/*`
> cases, new `import/import-edge`).

## How it works

```text
scripts/parity/run.ts  ──(case.run(s) calls real services: invoiceService.createSale(...), ...)──┐
   pass 1 "mock":  usesRust() = false → mock bodies against `db` restored from the base snapshot │
   pass 2 "rust":  usesRust() = true  → backendCall → parity transport ─stdin/stdout JSON lines─►│
                                                   src-tauri/src/bin/parity_host.rs             │
                                                   (tauri::test MockRuntime + the real          │
                                                    invoke_handler + AppState on MariaDB)       │
   after both: diff step results + books (TB, parties, stock) + invariants → report ◄────────────┘
```

The same base snapshot feeds both sides. The mock side does `Object.assign(db, snapshot)`, exactly
like `scripts/verify/replay.ts`. The Rust side runs `import_snapshot` (00-import D-1). The same pinned
clock and the same user login apply to both.

## Lane H-RS — Rust host (owns the files listed in the entry file's ownership table)

- [x] **B-1 One shared invoke handler.** In `src-tauri/src/lib.rs`, move the `generate_handler![…]`
      list, unchanged and still in `lib.rs`, into
      `pub fn invoke_handler<R: tauri::Runtime>() -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static`.
      `run()` then calls `.invoke_handler(invoke_handler())`. For this to compile against
      `MockRuntime`, make the 12 commands that take a `Wry`-bound `AppHandle` generic
      (`<R: tauri::Runtime>(app: tauri::AppHandle<R>, …)`), along with the service functions they call:
      `core/diag/mod.rs` (`diag_append`, `diag_read`, `diag_clear`, `diag_open_folder`, `diag_rotate`),
      `infrastructure/print/commands.rs` (`print_thermal_receipt`), `domains/settings/commands.rs` (3 LAN
      commands) + `service/network.rs` (both `cfg` variants), and `domains/setup/commands.rs` (3 device
      commands) + `service/device.rs`. `core/ipc.rs`'s `ipc_manifest_matches_handler` parses `lib.rs`
      text for `generate_handler!`, so it keeps working. Confirm it does.
      Reason: P4-3. There is one handler list, and no second dispatcher.
- [x] **B-2 Cargo wiring** (a request to the manager, who merges it). In `src-tauri/Cargo.toml`:
      `[features] parity = ["tauri/test"]` and
      `[[bin]] name = "parity_host" path = "src/bin/parity_host.rs" required-features = ["parity"]`.
      Normal builds do not compile the `test` feature.
- [x] **B-3 The clock comes from the transaction.** Replace the direct `chrono::Utc::now()` reads in
      business logic with the caller's `cx.clock` (`TxCtx.clock`, `core/tx.rs:76`), adding a parameter where
      a function has no `TxCtx`:
      `domains/invoices/service/credit.rs:114`, `domains/products/service/batches.rs:68` (it also takes
      the **UTC** date as "today", so near midnight in Cairo/Riyadh it picks the wrong business day:
      use `cx.clock.today()`), `domains/settings/service/currency.rs:79,127`, and
      `shared/stock/batches.rs:181-182`. Keep `shared/ledger/trace.rs:302` (diagnostic time) and the
      importer's `import_base` (import time is the intended meaning). Reason: P4-4. The Main PC's DB
      clock is the one time authority (P2-08).
- [x] **B-4 `src-tauri/src/bin/parity_host.rs`.** `main` is a plain `fn`. `tauri::test::get_ipc_response`
      blocks until an async command resolves on Tauri's own runtime; the meta commands use
      `tauri::async_runtime::block_on`.
      - Args: `--db <name>` (default `equal_parity`). Admin URL from `EQUAL_TEST_DATABASE_URL` (fail loudly
        if unset, like `tests/support/mod.rs`). `CREATE DATABASE IF NOT EXISTS <name>` (utf8mb4_unicode_ci),
        connect with **`max_connections(1)`** (so `SET timestamp` covers every statement), then
        `migration::Migrator::up`.
      - `AppState` is built exactly like `TestDb::fresh()` (`DeviceRole::Main`, `CollectingEventSink`,
        `db_status = Connected`). App: `tauri::test::mock_builder().manage(state).invoke_handler(accounting_app_lib::invoke_handler())`
        `.build(tauri::test::mock_context(tauri::test::noop_assets()))`, plus one webview window `"main"`.
        No plugins: business commands use none.
      - Protocol: one JSON object per line on stdin, one per line on stdout (stdout carries nothing else;
        logs go to stderr). Request `{ "id": n, "cmd": "<command>", "args": <object|null> }`. Reply
        `{ "id": n, "ok": true, "value": … }` or `{ "id": n, "ok": false, "error": <ApiErrorPayload|string> }`.
        Normal commands go through `get_ipc_response` with body `{"args": args}` (or `{}` when args is null),
        exactly what `backendCall` sends. Before each one, run `SET timestamp = <pinned epoch>`
        (or `SET timestamp = DEFAULT` when nothing is pinned).
      - Meta commands (handled in the bin, never IPC-registered): `__reset { snapshot, templates?, templateBranchId? }`
        calls `import_snapshot(conn, …, ImportOpts { replace_existing: true, adopt_terminal: None, .. })`
        (it wipes first, debug build only). It also clears `state.session`, `approval_grants` and `traces`,
        and replies `{ idPairs, counts }`. `__reset_empty` does `wipe_business_rows` only. `__clock { iso }`
        pins the clock. `__invariants` returns `shared::invariants::run_all` as `[{ key, passed, message }]`.
- [x] **B-5 Importer id pairs.** Add `IdMap::pairs() -> Vec<(String, Id)>` (`infrastructure/import/idmap.rs`)
      and `ImportReport.id_pairs` (`infrastructure/import/run.rs:87`). The command DTO is unchanged: only
      the host reads it. Reason: bundle replay (B-10) and the diff's id bijection (B-8) need the base mapping.
- [ ] **B-H gate (H-RS):** `scripts/cargo-safe.ps1 h-rs check --manifest-path src-tauri/Cargo.toml --workspace --all-targets`
      and `… check --manifest-path src-tauri/Cargo.toml --bin parity_host --features parity` are both clean.
      Then the manager builds the host once (entry §5) and runs `cargo test --test all core::` (or the
      lib test `ipc_manifest_matches_handler`).

## Lane H-TS — TS runner (owns `scripts/parity/*.ts`, the `backend.ts` hook, `package.json` `parity`)

- [x] **B-6 First: `scripts/parity/case.ts` (the case API) and `scripts/parity/bases.ts`.**
      ```ts
      export default defineCase({
        name: 'invoices/sale-cash-inclusive',            // <domain>/<id from the 03-domains §8(b) list>
        source: '03-domains/08-invoices.md §8(b)',       // traceability (required)
        base: 'demo-sa',                                 // 'demo-sa' | 'demo-eg' | 'edge' | 'empty' | () => MockDb
        user: 'admin',                                   // logged in on both sides before run()
        clock?: '2026-06-30T09:00:00.000Z',              // default: the base's savedAt
        allow?: [{ path: 'steps.post.value.attachmentIds', reason: '12-accounting #10: [] vs absent' }],
        epsilon?: ['steps.*.value.rows[].qty'],          // only fields named by 13 R-7 / 13b / 14b
        unordered?: ['steps.list.value'],                // only where Q6 / Q-I10 say so
        async run(s) {
          const sale = await s.step('create', () => invoiceService.createSale({ /* … */ }));
          await s.step('detail', () => invoiceService.getInvoice(sale.id));
          await s.expectError('over-refund', () => invoiceService.createRefund(sale.id, { /* … */ }));
        },
      });
      ```
      `s.step` records `{ ok, value | {code, message} }` and returns the value, so later steps chain on
      real ids. `s.expectError` records an error that is expected on both sides. `bases.ts` builds
      `demo-sa`/`demo-eg` with `seedDatabase(new Date('2026-06-30T09:00:00.000Z'), country)` (the same call as
      `scripts/verify/export-snapshot.ts`) and reads `edge` from `src-tauri/tests/fixtures/mock-snapshot-edge.json`.
      `empty` is `__reset_empty` on Rust and the mock's own empty state on the mock (the state the dev
      "reset to empty" action in `devToolsService.ts` produces). Each base maps to a timezone: SA →
      `Asia/Riyadh`, EG → `Africa/Cairo` (cross-cutting §7).
- [x] **B-7 Parity hook in `src/modules/core/services/backend.ts`.** Add
      `setParityTransport(t: ((cmd: string, payload?: { args: unknown }) => Promise<unknown>) | null)`.
      It throws if `isTauri()`, so the real app can never use it. While a transport is set, `usesRust()`
      returns `true` and `backendCall` calls the transport instead of `invoke`, with the same `ApiError`
      conversion. Nothing else changes, and `bun run build` stays green. Reason: P4-2 without touching any service.
- [x] **B-8 `scripts/parity/run.ts` + `transport.ts` + `clock.ts` + `diff.ts` + `allow.ts`.**
      - `transport.ts` spawns the host exe (`--host`, default the newest `.diagnostics/parity/bin/parity_host-*.exe`),
        and handles JSON lines with an id → promise map, plus a 60 s per-call timeout (kill, record, next case).
      - `clock.ts`: while a pass runs, `globalThis.Date` is a subclass whose no-argument constructor and
        `Date.now()` return the pinned instant. It also sets `process.env.TZ` to the base's zone (Bun applies
        it at runtime). It sends `__clock` to the host.
      - For each case: **mock pass** (restore the base into `db`, pin, `authService.login(user, pw)`, `run`, books,
        invariants), then **rust pass** (`__reset` with the base as `{ version: SCHEMA_VERSION, savedAt, data }`,
        pin, transport on, login, `run`, books, `__invariants`), then diff.
      - `diff.ts` rules (P4-6): a structural walk. **Ids:** a bijection seeded from `idPairs` and extended when
        a mock id string faces a UUID at the same path. A conflicting pair is a diff. **Instants** compare as epoch ms.
        **Numbers** compare exactly, except the case's `epsilon` paths (`|a−b| ≤ 1e-9`). **Arrays** compare
        in order, except the `unordered` paths (multiset). **Errors** compare `{code, message}` byte for byte.
        A `[]`/absent/`null` mismatch is a diff unless allowlisted.
      - `allow.ts`: global entries (seeded from phase A's A-6), plus each case's `allow`. **An entry whose
        `reason` does not cite a decision or quirk id** (for example `A-D1`, `Q6`, `R-7`, `12-accounting #10`,
        `D-8`) fails the run.
      - CLI: `bun run parity [--lane L1..L5] [--case <glob>] [--mock-only] [--host <exe>] [--db <name>] [--bundles] [--ingest]`.
        `--mock-only` runs the mock pass twice and diffs run 1 against run 2 (a determinism check), with
        invariants. This is what B2 lanes use before the host exists. Default `--db` is `equal_parity_<lane>`.
      - Output: a one-line summary per case, and `.diagnostics/parity/<ts>/<case>.json` with every diff (path,
        mock, rust, step). Exit code is non-zero on any unexplained diff.
- [ ] **B-9 `scripts/parity/books.ts`** *(code done and green on the mock; stays open until the Rust `__invariants`/`__reset` timings are recorded after the first host build)*, run after every case on both sides through the **same TS services**:
      `reportService.getTrialBalance` (full history), `partyService.getCustomers()` / `getSuppliers()`
      (balances), `reportService.getInventoryReport()` (qty/value per product), and the invariants (mock
      `runAllInvariants(db)`, Rust `__invariants`) compared by `key`, `passed` and `message`. Invariants also
      run **after every step** of the Rust pass, and a failing one fails the case (phase C, C-4). Measure the
      cost of `run_all` on `demo-sa` and of one `__reset`, and record both in this file's status note. If a
      reset takes more than 10 s, cases in one domain file share one reset and run in order (write it down,
      and don't guess it ahead of time).
- [x] **B-10 `--bundles`:** replay every `scripts/verify/cases/*.json` `ReproBundle` on both sides through
      `serviceRegistry()` (reuse `replay.ts`'s `importAllServices`). The Rust side remaps argument ids with
      `idPairs` plus ids learned from results. Each step's `ok` must equal the recorded `ok` on both sides,
      and rejections must match `{code, message}`. Today that means `ACC-0001` and `ACC-0002`.
- [x] **B-11** `package.json`: add `"parity": "bun run scripts/parity/run.ts"`. Check that `.diagnostics/` is
      gitignored (the dev log directory already lives there).
- [x] **B-12 `--ingest`** (used by E's gate run only, never in lane loops): write unexplained diffs as
      findings and call `bun run scripts/diagnostics/run.ts --ingest <findings.json>` (the one 18.G mechanism).
      A GL, party, stock or invariant diff becomes `ACC-`, anything else `BUG-`, with `area` = the case's domain.
- [x] **B-13 Self-tests:** `scripts/parity/diff.selftest.ts` (synthetic inputs: an id bijection conflict,
      epsilon in and out, a multiset, an allowlist entry without an id rejected, an error-message mismatch).
      Plus `scripts/parity/cases/_selftest/identity.ts` (base `demo-sa`, login, `userService.getUsers()`,
      `settingsService.getSettings()`).

## Gate

- `bun scripts/parity/diff.selftest.ts` is green. `bun run parity --mock-only --case "_selftest/*"` is green.
- `bun run build`, `bun run check`, `bun run memory` then `memory:check` (a new bin and scripts) are green.
- Cargo: see B-H. The manager's host build then gives `bun run parity --case "_selftest/*"` green against Rust
  (`bun run db:dev` running, `EQUAL_TEST_DATABASE_URL=mysql://root:equal-dev@127.0.0.1:3499`).
- The B-9 timings are recorded in this file's status note.
