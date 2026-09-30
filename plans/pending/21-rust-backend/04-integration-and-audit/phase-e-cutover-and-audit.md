# 04 · Phase E — The single cutover, the Rust-mode boot path, and the final accounting audit

> **Status:** pending. Wave 3. Lane **E** (Opus) does E-1 to E-5 and E-7. A separate **audit** agent (Opus,
> read-only) does E-6. The manager runs E-8. Decision P4-1 (all domains flip together) is why this
> phase is one small code change surrounded by checks.
>
> **Status (2026-09-30, lane E): E-2, E-3, E-4, E-5 done; E-1, E-6, E-7, E-8 open.** Everything added is
> a no-op while `RUST_DOMAINS` is empty (every new branch is behind `usesRust(...)`/`usesRustEverywhere()`,
> both false outside Tauri). Gates run: `bun run build` green, `bun run check` green, `bun run contract` +
> `contract:check` green (A-2/A-4: 0 violations), `bun run parity --mock-only` green (174 cases, every lane),
> `cargo-safe check --workspace --all-targets` 0 errors, `domain_import::` green on the newest test exe.
> Ready for E-1: `ALL_BACKEND_DOMAINS` (with its compile-time completeness check) and `usesRustEverywhere()`
> already exist in `backend.ts` (E-3 needed them); E-1 is left with `RUST_DOMAINS = new Set(ALL_BACKEND_DOMAINS)`,
> the `'mock'` dev override + the mixed-mode `log.debug` warning, and the file header. Note for E-1: with the
> dev override `*` inside Tauri, `usesRustEverywhere()` is already true today, so E-3's gates apply there.

## Preconditions (all must hold before E-1 is merged)

- [ ] The phase A, B, B2 (all 5 lanes) and C gates are green. The phase D static gate is green.
- [ ] The attachments domain (m0017) has landed, and its L1 parity cases are green.
- [ ] The manager's `cargo test --test all` is fully green (including `TestDb::finish()`), and `bun run bindings:check` is green.
- [ ] In `docs/diagnostics/ISSUES.md`, every `ACC-` issue is `verified` (ACC-0003 to ACC-0008 via B2's task-0 cases, P4-7).

## Tasks

- [ ] **E-1 The flip** (`src/modules/core/services/backend.ts`):
      - Add `export const ALL_BACKEND_DOMAINS = [...] as const satisfies readonly BackendDomain[]`, listing
        every member of the union. Add a compile-time check that the list and the union agree
        (an `Exclude<BackendDomain, (typeof ALL_BACKEND_DOMAINS)[number]>` that must be `never`).
      - `RUST_DOMAINS = new Set(ALL_BACKEND_DOMAINS)`. `usesRust` still returns `false` outside Tauri, so the
        browser build and the browser e2e suite stay on the mock (P4-8).
      - Dev override precedence (dev builds only, with the `import.meta.env.DEV` gate unchanged): `'mock'` means
        every domain on the mock (UI work without a DB); a domain list means only those on Rust, for debugging
        (log one `log.debug('backend', …)` warning that mixed mode shows inconsistent books, P4-1). Production
        ignores the override.
      - Add `export function usesRustEverywhere(): boolean` (every domain uses Rust). E-3 uses it.
      - Update the file header ("`RUST_DOMAINS` is empty in this wave …") to describe the flipped state.
- [x] **E-2 Walk through the Rust-mode boot path in code** (read it, fix gaps; `bun run desktop` is not part of this phase):
      (1) not configured → `device-setup` (the `src/router/index.ts:66-69` guard, `ensureDeviceSetupState`);
      (2) configured but the DB is down → `ServerFailureScreen` (`App.vue:37`, `useBackendHealth`; confirm its
      `start()` gate now fires, because `usesRust('settings')` is true);
      (3) connected with no users → `welcome`, through `authService.isFreshInstall`'s Rust branch
      (`isFreshInstallCached`, which the guard fills first). On a Main PC, `LegacyImportCard` appears when
      `readPersistedSnapshot()` finds data; otherwise "start company" (the setup wizard). The demo card goes
      through `setup_import_snapshot` `mode: 'demo'`, which **release builds refuse** (00-import step 1,
      `cfg!(debug_assertions)`). Confirm the welcome page hides or disables it in release, so a real company
      can never be wiped;
      (4) login. Record the walk-through (the file:line of each step) in this file's status note.
      **Walk-through (lane E, 2026-09-30):**
      (0) `src/main.ts:51` skips `bootMockDb()` (E-3); `App.vue:22` starts `useBackendHealth` (`useBackendHealth.ts:49`
      gate = `usesRust('settings')`, true after E-1, so the poller and `ServerFailureScreen` (`App.vue:49`) are live).
      (1) `src/router/index.ts:66-72`: `deviceStateForNavigation` (`deviceService.ts:49`, over `ensureDeviceSetupState`)
      → `configured: false` → `device-setup`. `DeviceSetupPage` → main: `provisionMainDevice` + `refreshDeviceSetupState`
      → `/welcome`; terminal: `TerminalPairingForm` → `/login`.
      (2) `index.ts:76` `mustHoldForDatabase` (`deviceService.ts:73`): configured, `hasUsers: false` and
      `core_backend_status.connected == false` → the navigation is held (`return false`), `ServerFailureScreen` covers the
      app, and `App.vue:26-33` reloads the window on the first poll that reports the DB connected.
      (3) `index.ts:79` (`/welcome` only while the company has no users) + `:83-85` (`isFreshInstall` →
      `isFreshInstallCached`, `authService.ts:85`) → `WelcomePage`: `LegacyImportCard` (`WelcomePage.vue:81`, shows only
      on the Main PC with an unimported snapshot that holds a company, `legacyImportService.ts:44-51`), "start company"
      (`SetupWizardPage`, Rust seeds the shell in `setup_get_onboarding_progress`), demo card only when
      `canLoadDemoData()` (`devToolsService.ts:27`, `WelcomePage.vue:116`): always on the mock, dev builds only on Rust;
      `setup_import_snapshot` now also refuses `mode: 'demo'` in release (`import/commands.rs:70`,
      `refuse_demo_in_release` + unit test) — before this, only `replaceExisting` was refused, so a release demo
      import would have put demo books into a real company's empty DB for good.
      (4) `LoginPage` → `authService.login` → `users_login` (`authService.ts:11`); `index.ts:88` `auth.restore()`
      (`users_restore_session` returns `null` after a restart: Rust sessions are in memory) → `:98` settings load.
      **Gaps found and fixed:** (a) a stale cached `hasUsers: false` bounced `/login` back to `/welcome` after the
      wizard (its shell seed creates `admin`), a legacy import or the demo import — `deviceService`'s comment claimed
      the import/demo services refreshed the cache, they did not. Fixed once, in the guard: `deviceStateForNavigation`
      re-reads a cached "no users" whenever it would decide the navigation (kept out of the services so the parity
      host, where `setup_get_device_setup_state` is unavailable on `MockRuntime`, is unaffected). (b) DB down on a
      configured device reported `hasUsers: false`, so a real company landed on `/welcome` behind the failure screen and
      stayed there after reconnecting — now held + reloaded (above). (c) `/welcome` stayed reachable with users, offering
      "start company"/demo on a live company — now redirected to `/login` in Rust mode. (d) The legacy card lived only on
      `DeviceSetupPage` right after provisioning, so a Main PC restarted before importing never saw it again — it moved
      to `/welcome` (the one screen a connected, empty Main PC always reaches); `DeviceSetupPage` now always continues to
      `/welcome`. (e) `initAutoBackup()` (`main.ts`) runs before a Main PC's server is connected; its first
      `settings_run_auto_backup_if_due` threw, which rejected `initAutoBackup` before the 60 s timer and the close-time
      backup hook were installed — for the whole session. `backupService.runRustAutoBackup` now skips until
      `core_backend_status.connected` and logs a real failure (`log.error`) instead of throwing. (f) Release demo import
      (above).
- [x] **E-3 Mock boot off in Rust mode:** `src/main.ts:49` calls `bootMockDb()` only when `!usesRustEverywhere()`.
      In `src/mocks/persist.ts`, `mutate()` never writes the snapshot while `usesRustEverywhere()`, and the
      dev "reset data" `clearSnapshot()` refuses (it would delete legacy data that has not been imported
      yet, which breaks zero data loss). `readPersistedSnapshot()` is unchanged. Why: in Rust mode the mock must
      neither seed nor overwrite the user's legacy IndexedDB data (P4-9). `bootMockDb` would also load a stale
      copy of the books into memory for nothing.
      **Done (lane E):** `usesRustEverywhere()` = `ALL_BACKEND_DOMAINS.every(usesRust)` (`backend.ts`). `main.ts:51`
      gates `bootMockDb()`. `persist.ts`: `mutate()` schedules nothing, `writeSnapshotNow()` (so `flushSnapshot()`)
      writes nothing, `clearSnapshot()` throws `FORBIDDEN` (the dev "reset data" shows it as an error toast);
      `readPersistedSnapshot()` unchanged. Under the parity host transport `usesRustEverywhere()` is true too; no
      Rust pass reads the snapshot, and the mock pass is unchanged (`--mock-only` green).
- [x] **E-4 Legacy-import marker (P4-9):** after a successful legacy import, `legacyImportService.ts` writes
      `importedAt` under its own key in the `mock-db`/`snapshot` store. It never touches the `current` record.
      `LegacyImportCard.vue` hides when the marker exists. The importer's own empty-target refusal
      (00-import step 1) remains the real guard. Terminals never show the card (00-import D-3; confirm). Add one
      `text-caption` line to the card saying the old data stays on this PC.
      **Done (lane E):** `persist.ts` `writeLegacyImportMarker`/`readLegacyImportMarker` (key `legacyImportedAt`,
      value `{ importedAt }`, never touches `current`); `importLegacySnapshot` writes it after a successful import (a
      failed marker write is logged, not thrown); `hasLegacySnapshot` hides the card when it exists. It replaces the
      earlier `localStorage['equal.legacyImportedAt']` key (only ever written by a Rust-mode import, which no
      production build has run). **Terminals: not true before — `hasLegacySnapshot` had no role check**; it now
      returns `false` unless `DeviceSetupState.role === 'main'` (the importer's own D-3 gate stays the authority).
      The card also needs the snapshot to hold a company (at least one user). New `text-caption` line added.
- [x] **E-5 Rust-side regression rule (P4-11):** where recording has no Rust equivalent, the `/dev/diagnostics`
      "المحاسبة" tab's repro-recording buttons show 16 D-5's refusal. Confirm the tab's invariants, drift and
      posting-trace panels call the ported Rust commands. The parity case `diagnostics/*` (L1) covers them.
      **Done (lane E):** `accountingDebugService.reproRecordingRefusal()` (one copy of the D-5 text, also used by
      `startReproRecording`'s refusal); `DevAccountingTab` disables "بدء/إيقاف تسجيل إعادة الإنتاج" and "تصدير حالة
      لإعادة الإنتاج" on Rust and shows the refusal plus where a Rust regression case goes (`scripts/parity/cases/` +
      `tests/domain_*.rs`). Confirmed every other panel's service has its `usesRust('diagnostics')` switch line:
      documents `diagnostics_list_recent_documents`, trace `diagnostics_get_posting_trace`, lines
      `diagnostics_get_journal_entry_raw`, balances `diagnostics_get_balances_around`, invariants
      `diagnostics_get_invariant_results`, drift `diagnostics_get_drift_report`, explain
      `diagnostics_explain_account_balance`. Parity coverage (`scripts/parity/cases/diagnostics/`): invariants,
      drift (clean DB), balances-around, audit filter/entities. **For E-6:** no parity case covers
      `getPostingTrace`, `getJournalEntryRaw`, `listRecentDocuments` or `explainAccountBalance` (the trace ring is
      in-memory and its line shape is a documented `contract-ok` narrowing).
- [ ] **E-6 Final accounting audit** (audit agent, read-only; findings go to the owning lane, and the audit re-runs):
      1. **Posting coverage matrix.** Derive every journal source type from the code (`grep -rn "postJournal("
         src/mocks/backend` and `grep -rn "ledger::post" src-tauri/src/domains`). Map each to at least one green parity
         case whose trial balance, party and stock diffs are clean. Any uncovered source type becomes a new case in its lane.
      2. **Allowlist review.** List every global and per-case entry with its cited id. Reject any entry that
         covers a money, qty, cost or balance field without an explicit decision. No wildcards over arrays of documents.
      3. **Invariants.** Green after every step of every case, and on `import-demo-sa`/`import-demo-eg`/`import-edge`.
      4. **Rounding and floats.** `architecture_rules` has no `f32`/`f64` and passes `check_decimal_serde` (A-3); parity shows 0 cent
         diffs; `verify:mocks`'s `rounding` area is green.
      5. **Authority.** Every `#[tauri::command]` under `src-tauri/src/domains/**/commands*.rs` that writes
         calls `cx.require`/`require_any` before its service call. The only exceptions are the public
         set: login, device setup/pairing, the importer on an empty DB and `core_backend_status`, each with its reason.
         Every write goes through `with_tx` (`check_transactions_confined_to_tx_rs`).
      6. **Ledger and contract.** Every `ACC-` is `verified`. No `// G-25`/`// G-31` "kept as-is" markers left
         (`grep -rn "G-25\|G-31" src-tauri/src src/mocks`; comments that cite the fix's `ACC-` id are fine).
         `bun run memory` shows 0 contract gaps. The `contract:check` A-2/A-4 rules pass.
      Write the results as a table in this file's status note (item, evidence, result).
- [ ] **E-7 Text updates** (the manager merges CLAUDE.md):
      - `00-MASTER-PLAN.md`: in §1 and §5, replace "no big-bang switch / cutover is per domain" with P4-1 and its
        reason. Move §9 D5 to **Answered** (P4-8). Set the §6 row 04 status. Update "Next step" to the final testing plan.
      - `README.md` row 04; `03-DOMAINS-IMPLEMENTATION.md` status note ("flipped in Part 04 E-1").
      - `CLAUDE.md`: in **Workflow**, replace "UI-only against the mock backend" with "the Tauri app runs on the
        Rust backend; browser dev and browser e2e run on the mock until D5". In **Accounting safety**, add the P4-11
        regression rule and note that `shared::invariants` is what the app runs (the mock copy serves the browser
        build, and both stay in lockstep through `bun run parity`). Add `bun run parity` to the **Definition of done**
        block. In **Rust ↔ Vue**, add "a new or changed command ships with a parity case".
      - Run `bun run memory`.
- [ ] **E-8 Final gate run (manager):** `bun run parity --bundles --ingest` over every case gives 0 findings.
      Then `bun run diag` and `diag:check`, `build`, `check`, `verify:mocks`, `verify:replay`, `contract:check`,
      `memory:check`, `cargo test --test all`, and `bindings:check`. Tick the Part 04 DoD (entry file §6).

## Gate

- The entry file §6 DoD is fully ticked. `RUST_DOMAINS` = every `BackendDomain`.
- The E-6 audit table has no open item.
- The entry file §8 list is handed to the final testing plan. **Plan 21 stays in `plans/pending/`** until that
  plan has run the real desktop, installer and Tauri e2e checks (CLAUDE.md "Move on completion").
