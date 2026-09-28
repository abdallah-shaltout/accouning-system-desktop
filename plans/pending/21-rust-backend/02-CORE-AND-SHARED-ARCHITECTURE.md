# 21 · Part 02 — Core and shared architecture (the Rust foundation)

> **🛑 BUILD RULE (user decision 2026-09-27, supersedes the per-wave gates below):** cargo builds
> froze the user's whole PC. **During all remaining phase work nobody runs `cargo` (build/check/
> test/run), `bun run db:dev`, or any test/gate command — agents only write code.** After every
> phase is written, the manager runs ONE sequential build at below-normal priority, then ONE test
> run (`bun run db:dev` + full `cargo test` + the fast `bun run` gates), then ONE review/fix pass.
> Permanent limits are in the repo: `src-tauri/.cargo/config.toml` `[build] jobs = 4`;
> `Cargo.toml` `[profile.dev] debug = "line-tables-only"`, deps `debug = false`.
>
> **⏸ CODE COMPLETE — FINAL TEST PASS PAUSED (2026-09-28, user request: "testing takes too long —
> stop it, continue development, we'll come back to it").** All Part-02 phases (A, A2, A3, B, C, D,
> E, F) are written. Where the single build/test/review pass stopped:
> - **Green:** `cargo check --workspace --all-targets` (0 errors, 0 warnings); full `cargo test
>   --no-run` build; lib unit tests **137/137**; `architecture_rules` **8/8**; `db_foundation`
>   **7/7** against the real bundled MariaDB (`bun run db:dev`, port 3499; migrations m0001–m0015
>   apply cleanly); `db_dev_server` provisions, stops and reattaches.
> - **Real bugs found and fixed in the pass** (no test had ever run before): provisioning left the
>   server running and `ensure_running` started a second `mariadbd` (adoption compared `@@datadir`
>   with mixed `\`/`/`); "ibdata1 must be writable" was classified as `data-corrupt` (now
>   `datadir-locked`); the fixed dev credential store generated a random root secret; `log_bin=OFF`
>   *enabled* binlog (now `skip-log-bin`); `db_dev_server` used 3406 not 3499
>   (`provision_main_on_port`); unbounded `child.wait()` after a failed stop (now
>   `process::wait_or_kill`); `DATE_FORMAT()` in generated `_key` columns (error 1901 → `CAST`);
>   `NULL`/`NOT NULL` on generated columns (1064); currency FK collation mismatch (`utf8mb4_bin`);
>   reserved column `by` unquoted in FKs; FKs added one table-copy at a time (now grouped + in place
>   on empty tables); `change_versions`/`document_counters` were `BIGINT UNSIGNED` but read as
>   `i64`; **`mysql_errno` read the SQLSTATE instead of the errno** (deadlock retry and
>   duplicate→`CONFLICT` never worked); `TraceRing` id index never evicted (leak); `PostJournal`/
>   `ReverseRequest` dropped the `DocDate` instant; f64 in `InvariantDiff`; bare `find()` on soft-
>   delete tables in invariants (now `find_including_deleted()`); `session_end::start` never
>   called; migrations used `.uuid()` = `binary(16)` instead of the native `UUID` column
>   (P2-03) — **fixed in code after the last run, not yet re-tested** (it caused most of the
>   remaining failures: "Data too long for column 'id'").
> - **Not yet re-run / still failing at pause:** the other DB suites (`db_entities_*`,
>   `db_deadlock_retry`, `shared_*`) — last run: 147 ok / 25 failed, mostly the UUID issue above;
>   also `entities_match_schema` (entities omit the generated `_key` columns — decide: add them to
>   the entities or exclude generated columns in the test) and `db_deadlock_retry` ("expected at
>   least one retry"). The server suites (`db_server_lifecycle`, `db_server_lan`) hung in parallel
>   runs; now serialised with a per-file lock + fixture `Drop` that stops its server — not re-run.
>   Fast `bun run` gates (build, check, verify:mocks, contract:check, memory, diag:check,
>   bindings:check) — not run yet.
> - **Resume:** `node scripts/fetch-mariadb.js` (payload is cached), build once, copy
>   `src-tauri/target/debug/db_dev_server.exe` elsewhere and run the copy (a running exe blocks the
>   next link), then run **one test binary at a time** with `EQUAL_TEST_DATABASE_URL=
>   mysql://root:equal-dev@127.0.0.1:3499` and `RUST_TEST_THREADS=4` via a below-normal-priority
>   cargo. Each fresh test DB takes ~70 s to migrate on this machine (Defender scanning the new
>   `.ibd` files is a likely cause — exclude `src-tauri	arget` in Windows Security). Nothing is
>   committed.
>
> **Status (2026-09-27):** Phase A implemented (DB gate pending — see its status note). Phases
> A2/A3 (bundled MariaDB, D11) added 2026-09-27 and come next, before Phase B. Written after Part
> 01's gate went green ([`01-FRONTEND-ANALYSIS.md`](01-FRONTEND-ANALYSIS.md) §6). It cites Part 01's
> findings and does not re-derive them. Implement one phase at a time (A → A2 → A3 → B → F). Each
> phase file lists what to read, what to create, a checklist and its own gate. Per master plan §8,
> the full e2e suite does **not** run in these gates. It runs once, at the end of plan 21.

## 1. Goal and scope

Build everything that every Part 03 domain stands on, and nothing domain-specific:

- **In scope:** the Cargo workspace and crate layout (master plan §4), the MariaDB pool and
  migrations, `AppError`, `AppState` (session, terminal identity, device settings), the one
  transaction helper (`with_tx`), the bundled MariaDB server for the Main PC (D11): build-time
  payload, installer integration, provisioning, supervision, version upgrades, uninstall safety,
  the dev/test DB, and LAN hosting for terminals, SeaORM entities and migrations for all 46
  `MockDb` tables plus `print_templates` (D9), `shared::ledger` (+ numbering + currency),
  `shared::stock`, `shared::invariants` (the Rust port of the 14 invariants) and the read-only
  `shared::balances`, `shared::activity` (+ the undo-by-compensation registry), and the IPC
  bridge: typed TS bindings, the frontend switch point, and cross-terminal change events.
- **Out of scope (Part 03):** every `domains/<d>/` command and service, the per-domain undo
  compensators, the D10 snapshot importer (see §9), and all backup/restore logic.
- **No domain is flipped to Rust in Part 02.** `RUST_DOMAINS` stays empty. Browser dev and e2e
  behave exactly as today.

## 2. Read first (in this order)

1. [`../../../CLAUDE.md`](../../../CLAUDE.md): rules, the "Accounting safety" section, and
   "ARCHITECTURAL AUTONOMY".
2. [`00-MASTER-PLAN.md`](00-MASTER-PLAN.md): §3 rules 1–10, §4 layout, §9 decisions.
3. [`01-frontend-analysis/cross-cutting.md`](01-frontend-analysis/cross-cutting.md), all of it.
   This is the primary input (auth/session, terminal id, settings split, `AppError`, events,
   paging/search, dates, the 46-table map, the D10 spec).
4. [`../../../docs/v2/02-accounting-review.md`](../../../docs/v2/02-accounting-review.md): §3
   posting rules and §4 invariants.
5. The mock engine being ported: `src/mocks/backend/core.ts`, `journal.ts`, `accounts.ts`,
   `inventory.ts`, `currency.ts`, `balances.ts`, `posting-trace.ts`, `invariants.ts`,
   `src/mocks/db.ts`, `src/mocks/utils.ts`.

## 3. Decisions made in this part (architectural autonomy; each is logged here)

| # | Decision | Grounded in |
|---|---|---|
| P2-01 | **ts-rs** (stable) for typed bindings, not tauri-specta. We add our own `ipc_sig!` manifest. It generates `IpcCommands`, and a Rust test cross-checks it against `generate_handler!` in `lib.rs`. | Master rule 1 needs types checked by `vue-tsc`. ts-rs is a stable release line (tauri-specta v2 has lived on RCs). It exports through `cargo test` with no app run, and it honours serde attributes plus `#[ts(optional)]`, which exact `Equals` checks need. It leaves `generate_handler!`, the memory scanner's handler parser and CLAUDE.md's "registered in `generate_handler!`" rule unchanged. Phase F. |
| P2-02 | Cargo workspace: app crate `.` + `migration` crate. Entities live in the app crate (`src/entities/`). Migrations use sea-query schema builders, not entities. | Master §4. This avoids a crate cycle, because the app runs the migrator on boot. |
| P2-03 | MariaDB **≥ 10.11 LTS** (target 11.4 LTS), checked at boot. A DB test proves UUIDv7 values sort in time order in the native `UUID` column. | Master rule 8 (native `UUID`, ≥ 10.7) + D2 (time-ordered inserts). The test checks real behaviour instead of trusting a version string. |
| P2-04 | `utils::id::Id` newtype over `uuid::Uuid` (v7). It binds as hyphenated text and decodes from either a 36-char text or 16 raw bytes. | Native `UUID` columns reach the driver as text. The newtype stays correct whichever form the server returns. The Phase A round-trip test pins it. |
| P2-05 | Pool built with `sea_orm::sqlx` MySQL options (utf8mb4, session `time_zone '+00:00'`, strict `sql_mode`) and wrapped with `SqlxMySqlConnector::from_sqlx_mysql_pool`. | Instants are stored in UTC (cross-cutting §7). Strict mode stops silent truncation (zero data loss). |
| P2-06 | Command transactions run at **READ COMMITTED** with explicit `FOR UPDATE` / `LOCK IN SHARE MODE` locks and unique constraints. Multi-statement reads (reports, invariants) run at **REPEATABLE READ, READ ONLY**. | D8 plus every 01.B §5 table: correctness comes from explicit row locks. RC avoids stale snapshot reads in check-then-write code, and RR gives one consistent snapshot for the 14 invariants. |
| P2-07 | Deadlock (errno 1213) → the whole closure is retried, 3 attempts max. Lock-wait timeout (1205) → `CONFLICT` "قيد التعديل من جهاز آخر". | D8 multi-terminal locking. The closure runs in one transaction, so a retry is always safe. |
| P2-08 | **Branch clock = the MariaDB server clock.** `SELECT UTC_TIMESTAMP(3)` runs once per transaction. Business "today" = that instant converted through a new `settings.timezone` (IANA). When the timezone is NULL, the process's OS timezone is used. | Cross-cutting §7 (Main PC authoritative, recommended `timezone` field). Fixes terminal clock skew for both instants and business days. |
| P2-09 | **`DocDate`**: every document `date`-like field that the mock fills with *either* `YYYY-MM-DD` *or* an ISO instant is stored as `x_day DATE` + `x_instant DATETIME(3) NULL` + a generated `x_key` (the mock's exact string). | The mock writes instants into `date` (`sales.ts:241`, `core.ts:648`, `inventory.ts:275,371,412`, `journal.ts:294`, `payments.ts:350`), and some code compares `date.slice(0,10)` (`invariants.ts:281`, `core.ts:495`). Storing only `DATE` would lose data and break parity. See conflict C-09. |
| P2-10 | `with_tx` collects **post-commit effects**. It bumps `change_versions` at the very end of the transaction (fixed order, so it is always the last lock taken), emits the Tauri event `backend:changed` after COMMIT, and pushes posting traces after COMMIT. | Cross-cutting §5 ("same transaction as the write"). Last-lock ordering prevents deadlocks, and a rolled-back command emits nothing. |
| P2-11 | The cross-terminal poll runs **in Rust** (a tokio task every 2 s) and pushes `backend:changed`. There is no `core_get_change_versions` command. | Same table and categories as cross-cutting §5. No JS timer and no IPC chatter. See C-06. |
| P2-12 | `ledger::post` always touches `ledger`, and also touches `parties` when any line carries a party. `stock::apply_change` touches `catalog`. | `postJournal` already emits `ledger:changed` on every post (`core.ts:159`). Stock figures are catalog-visible (products.md §8). Deriving events from the data closes the cross-cutting §5 event gaps structurally. See C-07. |
| P2-13 | `assert_open_period` **always** takes shared locks on the settings row and the covering fiscal-year row, even when `allow_closed_period` is set. `close_year` takes an exclusive lock on the year. | accounting.md §5 (sharpest race). Shared locks for posters give the same guarantee as `FOR UPDATE` without serialising posters against each other. See C-05. |
| P2-14 | Two journal tables, `journal_entries` and `journal_drafts`, each with its own lines table. | accounting.md §2 (firm recommendation) and `checkDraftsIsolated`. |
| P2-15 | One **`parties`** table (`kind ENUM('customer','supplier')`) replaces the mock's `customers`/`suppliers` arrays. `journal_lines(party_id, party_kind)` gets a composite FK to `parties(id, kind)`. | parties.md §9 left the choice to 02-B. A single table is the only shape that gives real FK integrity to party-tagged ledger lines and payment targets. DTOs stay `Customer`/`Supplier`. |
| P2-16 | Master-data deletes become **soft deletes** (`deleted_at`). A generated "live" column keeps uniqueness per live row, and reads go through `find_live()`. Drafts, held sales and worklists are still hard-deleted. Posted documents can never be deleted: the architecture test forbids it. | Master rules 7 and 8. Freeing a deleted name for reuse keeps DTO parity with the mock's hard delete. |
| P2-17 | Column collation follows each mock comparison rule. Exact/trim-only uniqueness → `utf8mb4_bin`. Case-insensitive rules (username, SKU, branch/cost-center codes) → `utf8mb4_unicode_ci`. Table default is `utf8mb4_unicode_ci`. | products.md §5 (don't case-fold names), users.md §5, settings.md §3. |
| P2-18 | Value objects go in JSON columns: addresses, route links, template options, settings sub-policies, attachment-id lists, held-sale carts, audit diffs. Every line, allocation or tender list that posting or reports read is a child table. | cross-cutting §8, templates.md §2, setup.md §2, invoices.md §2. |
| P2-19 | **Quantities round with `round2`** and are stored as `DECIMAL(19,4)`. Cost is `round4` in `DECIMAL(19,4)`, money `round2` in `DECIMAL(19,2)`, rates `DECIMAL(19,6)`, percentages `DECIMAL(9,4)`. | products.md §9 asked for this decision before Part 02 (recommendation adopted). The mock rounds quantities to 2dp everywhere (`core.ts:279`, `inventory.ts:120-130`, `sales.ts:81`). See C-03. |
| P2-20 | `settings.default_branch_id` replaces the mock's `'branch-main'` constant. | `DEFAULT_BRANCH_ID` (`core.ts:96`) cannot survive UUIDs. |
| P2-21 | Numbering: a per-kind `document_counters` row, incremented and read back under its own row lock. For `MAX+1` codes (party codes), `numbering::lock(..)` rows serialise the computation and a live-unique `(kind, code)` backs it up. | D8 row, parties.md §5 (`nextCode`), products.md §5. Keeps the mock's exact `MAX+1` semantics. |
| P2-22 | Every unique constraint is named `uq_<table>_<cols>`, so domains can map a violation back to the mock's exact Arabic `CONFLICT` message (`AppError::map_unique`). | products.md §5, users.md §5, settings.md §5 ("same message as the pre-check"). |
| P2-23 | `AppError` adds a sixth wire code, **`INTERNAL`**, for infrastructure failures only (DB unreachable, unexpected DB error). | cross-cutting §4 has no code for these. See C-02 (the user may veto this). |
| P2-24 | Undo metadata lives on `audit` (`action_type`, `payload`, `is_undoable`, `undo_of`, `undone_by`, plus `terminal_id`). Domains register compensators in a trait-object registry. **No IPC command** for undo until D4's UI plan exists. | Master rule 7, §2 (audit shape + new fields), D4 recommendation. |
| P2-25 | D4 and D7 are **adopted as recommended**: undo is a backend capability only, and undo in a closed period is refused unless the user is admin (compensators pass `allow_closed_period = actor is admin`). | Master §9 recommendations. This matches `reverseJournal`'s `allowClosedPeriod: isAdmin` (`journal.ts:152`). See C-04. |
| P2-26 | New read-only `shared::balances`: ports of `balances.ts` plus `allocatedTotal`/`unallocatedAmount`/`unallocatedCreditFor` from `payments.ts`. Used by the invariants now and by the parties/payments/reports domains later, so there is one copy. | `invariants.ts:21-22` imports them, and CLAUDE.md says "no second copy". |
| P2-27 | `shared::invariants` is built in Phase D, together with stock. | Invariant 4 is the GL↔stock tie, so D's tests use the full suite. |
| P2-28 | Migrations run **only on the Main PC** (`device.role = main`). A terminal checks that the applied migrations equal its own build's list; otherwise it refuses the DB with an Arabic message. | D1 (one Main PC per branch) and zero data loss: a newer terminal must not change the schema under other terminals. |
| P2-29 | The DB password is kept in the OS keyring (`keyring` crate, service = the Tauri identifier). There is no plaintext fallback. | cross-cutting §3 (keyring is the recommended default). Strictest option. |
| P2-30 | `device-settings.json` gains `role: "main" \| "terminal"`. | settings.md D-S1/D-S2 need a Main-PC flag to gate backups. |
| P2-31 | A Rust **architecture-rules test** (`tests/architecture_rules.rs`) text-scans `src/`. It fails the build when rules 3, 4 or 5 are broken, when a soft-deletable entity is read with `find()`, when a posted document is deleted, or when a `RouteRef` uses an unknown route name. | Master rules 3, 4 and 5 ("a CI grep fails…", "review failure"). |
| P2-32 | Generated bindings go in `src/modules/<module>/types/gen/` (committed, never hand-edited). Checks go in `src/modules/<module>/types/contract.check.ts` using an exact `Equals<>` type. | CLAUDE.md "no new top-level folders" + master rule 1. |
| P2-33 | `Decimal` crosses IPC as a **JSON number**, converted through its shortest round-trip decimal text (`utils::money::serde_number`). Never a string, never serde_json `arbitrary_precision`. | TS types are `number`. The global `arbitrary_precision` feature would change serde_json behaviour inside Tauri. |
| P2-34 | Frontend switch `src/modules/core/services/backend.ts`: `usesRust(domain)` + a typed `backendCall(cmd, args)`. There is **no silent fallback** to the mock when Rust fails. | Master §5. A fallback would put writes into the browser store while the user believes they are in the DB. |
| P2-35 | Posting traces stay in memory: a 500-entry ring in `AppState`, never in the DB. | `posting-trace.ts:8-13` (never in a backup), diagnostics.md §2. |
| P2-36 | `ledger::post` refuses group accounts (strict addition). It does **not** refuse inactive explicit ids (the mock allows them). | Rule 6 "account rules before any insert". A group posting can only come from a bug, so parity with valid data is unchanged. |
| P2-37 | The Rust module is named `core` as in master §4. Convention: always write `crate::core::…` and never `use core::…` in `lib.rs`. | Avoids ambiguity with the built-in `core` crate. |
| P2-38 | `search_normalized` columns (U+001F-joined `normalize_arabic` fields) only where a list's search haystack is entirely the row's own fields: `products`, `parties`, `journal_entries`, `vouchers`. | cross-cutting §6. Joined-name haystacks are handed to Part 03 (§9). |
| P2-39 | CHECK constraints: `journal_lines` one-sided and non-negative, `journal_entries` total debit = total credit. | Rule 6 defence in depth, invariant §4.1. |
| P2-40 | Every table gets `id UUID` PK **except** the two infrastructure tables `document_counters(kind)` and `change_versions(category)`. | D2's reasons (sync, index order) don't apply to per-branch mechanics that are never synced. cross-cutting §5 already defines `change_versions` with `category` as PK. See C-11. |
| P2-41 | Payload = the official **MariaDB Community Server 11.4 LTS `winx64` ZIP** (no-install package), pinned (`11.4.13`) with its SHA-256. Fetched at build time by `scripts/fetch-mariadb.js` into `src-tauri/mariadb-11.4.13-winx64/` (gitignored), pruned by a denylist, checked against a required-files list, and shipped through `bundle.resources` → `$INSTDIR\mariadb\`. | The WebView2 `fixedRuntime` precedent (`tauri.conf.json`, `scripts/fetch-webview2.js`: pinned, offline, nothing for the customer to install). P2-03 targets 11.4 LTS (supported to 2029, longer than 11.8 LTS). MSI rejected: it needs admin, registers a system-wide service and collides with other MySQL/MariaDB installs on the same PC. |
| P2-42 | **One universal installer.** Every install carries the payload. Only a Main PC ever copies or runs it. | The installer already ships ~668 MB of WebView2 to every PC (NSIS 286 MB today). MariaDB adds an estimated 60–90 MB, measured at A2's gate. Two installers would force a "which one?" choice on a non-technical owner, and a terminal promoted to Main PC would need a reinstall. |
| P2-43 | Windows bundle target = **NSIS only**. `installMode` stays `currentUser` (no admin at install). Uninstall hooks live in `src-tauri/windows/installer-hooks.nsh`. Windows-only bundle keys go in a new `src-tauri/tauri.windows.conf.json`. | Tauri NSIS `installerHooks` (PRE/POSTINSTALL, PRE/POSTUNINSTALL). WiX would need a parallel custom-action set. The platform file keeps non-Windows builds free of the Windows payload. See C-21. |
| P2-44 | Business data root = **`%ProgramData%\com.abdallah.accounting-app\database\`** (layout in A2 §Layout). Never under `$INSTDIR`, `$APPDATA\<id>` or `$LOCALAPPDATA\<id>`. | Tauri's uninstaller "delete app data" checkbox runs `RmDir /r "$APPDATA\${BUNDLEID}"` and `"$LOCALAPPDATA\${BUNDLEID}"` (tauri-bundler `installer.nsi`), so a DB there could be wiped by one checkbox. Zero-data-loss rule. The folder is named after the identifier because CLAUDE.md says never change the identifier. Machine-level, not roaming, always an ASCII path. |
| P2-45 | The server runs from a stable copy of the payload, **`database\server-bin\current\`**, made on the Main PC only. It is swapped (staging → current) only while the server is stopped. | The installer can then replace `$INSTDIR\mariadb` with no locked files. The firewall rule gets a stable program path. The path is ASCII even when the Windows user name is Arabic. Version swaps are atomic. |
| P2-46 | **Process model: `mariadbd.exe` is a child process supervised by the app. No Windows service.** It starts only when `device.role == main` and the managed server is ready. The app adopts an instance that is already running (pid file + image path + `@@datadir`), restarts it after an unexpected exit (2 s/5 s/15 s, at most 3 in 10 min), and shuts it down cleanly (SQL `SHUTDOWN`, wait 60 s, then terminate) on app exit and at Windows session end (a hidden window handles `WM_QUERYENDSESSION`/`WM_ENDSESSION`). | A service needs admin to register, plus a UAC prompt at role choice and service-account ACLs. Crash-safety comes from InnoDB (`innodb_flush_log_at_trx_commit=1`). A later service mode can reuse the same layout (§10). |
| P2-47 | TCP only. The Main PC app connects to **`127.0.0.1`**. Default port **3406**, with fallbacks 3407–3415 allowed **only while LAN sharing is off**. The server binds `127.0.0.1` until A3 turns LAN sharing on. | sqlx has no Windows named-pipe transport. 3306/3307 are commonly taken by XAMPP/Laragon/other POS installs (this dev PC has Laragon MySQL). A loopback bind means no firewall prompt and no network exposure on single-PC shops. |
| P2-48 | `database\my.ini` is **generated** from `server.json` on every start (never hand-edited). Forward-slash paths. Server-wide `character-set-server=utf8mb4`, `collation-server=utf8mb4_unicode_ci`, `default-time-zone='+00:00'`, the P2-05 `sql_mode`, `innodb_lock_wait_timeout=10`, `innodb_flush_log_at_trx_commit=1`, `innodb_doublewrite=1`, `innodb_buffer_pool_size=256M`, `lower_case_table_names=1`, `skip-name-resolve`, `local_infile=0`. | P2-05/P2-07 made server-wide, so every terminal session gets them even if a client forgets (see C-19). MariaDB option files treat `\` as an escape, so paths must use `/`. |
| P2-49 | Accounts: `root@'127.0.0.1'` for server admin only. `equal_app@'127.0.0.1'` for the Main PC app (DML + DDL on `equal.*`). `equal_lan@'%'` for terminals (DML only, created by A3). Passwords are random Crockford-base32 strings (20 chars for root/app, 16 for LAN), generated in Rust and stored only in the OS keyring. | P2-29 (keyring, no plaintext). Least privilege. Terminals lacking DDL enforces P2-28 at the privilege level too. |
| P2-50 | **Credential self-heal:** if the keyring secrets for a ready data dir are lost, the app restarts the server loopback-only with `--skip-grant-tables`, resets the passwords, then restarts normally. The LAN password rotates, so terminals must re-pair. | Otherwise a Windows profile reset or cleared Credential Manager would lock the owner out of intact data. Anyone who could do this already has file access to the data dir. |
| P2-51 | Version lifecycle: pinned to series **11.4**. Patch upgrades happen in place: clean stop → cold copy of `data\` into `pre-upgrade\` (only with free space ≥ 2 × data + 512 MB, otherwise stay on the old binary) → swap `server-bin` → start → `mariadb-upgrade`. Downgrade and series change are refused. A test guards the series constant against drift between script and Rust. | MariaDB supports in-place upgrade within a series. Ties into `core::db::check_version` (≥ 10.11 still passes). |
| P2-52 | Backup/restore (Part 03 `infrastructure/backup/`) reads and writes **through SQL** inside one `with_read` snapshot, using the existing archive format (`settings/helpers/backupArchive.ts` manifest/encryption). **`mariadb-dump` is not bundled.** The payload keeps `mariadb.exe`, `mariadb-upgrade.exe` and `mariadb-check.exe`, which `mariadb-upgrade` needs. | settings.md §1 (`backupNow` builds the archive of real tables; `previewRestore` compares against the schema version). No credentials passed to subprocesses. Works the same against the dev/test DB. |
| P2-53 | **Dev/test DB = the same payload:** `bun run db:dev` (bin `db_dev_server`, data `src-tauri/target/mariadb-dev/`, port 3499, root password `equal-dev`, loopback only) gives `EQUAL_TEST_DATABASE_URL=mysql://root:equal-dev@127.0.0.1:3499`. | Phase A's DB gate is unverified for lack of a server (its status note). Tests then run against the exact binary customers get. |
| P2-54 | A2/A3 add **no IPC command** (Phase F's "only `core_backend_status`" stays true). Part 03 `setup`/`settings` add the commands that call `infrastructure::database`. Verification goes through the `db_server_smoke` bin, following the `pdf_smoke`/`thermal_smoke` precedent. | CLAUDE.md "Rust ↔ Vue" (commands only with a service caller, 0 gaps). Phase F F-2/F-6 counts. |
| P2-55 | Pool session settings are applied **per connection** through `sea_orm::sqlx::mysql::MySqlPoolOptions::after_connect`, wrapped with `SqlxMySqlConnector::from_sqlx_mysql_pool`. The debug `EQUAL_DB_URL` path goes through the same function. | P2-05's own wording. Fixes C-19. |
| P2-56 | LAN sharing (A3): one elevated `netsh advfirewall` rule (program + port scoped, `remoteip=localsubnet`, `profile=any`), added at enable time, before the bind switches to `0.0.0.0`. The uninstaller does not remove it (see C-26). | The Windows "Allow access" prompt needs admin, and cancelling it silently creates *block* rules. `profile=any` because shop networks are often classed Public. `localsubnet` keeps exposure to the shop LAN. |
| P2-57 | Terminal pairing (A3): the Main PC shows its computer name, LAN IPv4 addresses, port and a 16-char pairing code (`XXXX-XXXX-XXXX-XXXX` = the `equal_lan` password). A terminal stores `host` = computer name plus `lastKnownAddress`, and falls back to the address when the name doesn't resolve. | D8 (connection string, no intermediary server). A DHCP address change must not strand terminals. |
| P2-58 | Continuity (A3): when LAN sharing is on, the Main PC app starts at login in the background (`tauri-plugin-autostart`, `--background`), closing the window hides it to a tray icon, and exiting from the tray asks for confirmation when terminals are connected. **Single instance** (`tauri-plugin-single-instance`, registered first) is mandatory from A2 on. | Terminals depend on the Main PC's DB (D1). Two app instances would mean two supervisors on one data dir. |
| P2-59 | **Reattach after reinstall:** if `device-settings.json` has no connection but `server.json` is `ready`, boot restores `role: main` + the connection. If `terminal.json` is missing, it is restored from `server.json.hostTerminalId`. | Zero data loss and cross-cutting §2 (the terminal id is never regenerated): a reinstall, even with "delete app data" ticked, lands back on the same data and the same till id. |

## 4. Conflicts and corrections found (surfaced, not silently resolved)

| # | Found | Where | Handled |
|---|---|---|---|
| C-01 | Rule 6 names `AppError::Unbalanced`/`PeriodLocked`, but the wire enum is the closed code set and the mock throws `VALIDATION` / `FORBIDDEN` for these. | master §3 rule 6 vs cross-cutting §4, `core.ts:85,110,114` | Named constructors (`AppError::unbalanced`, `period_locked_*`) produce `VALIDATION`/`FORBIDDEN` with the mock's exact text. Task C-9 rewords rule 6. |
| C-02 | cross-cutting §4 calls the five codes "closed". Infrastructure failures (Main PC unreachable) have no truthful code. | cross-cutting §4, F5 | Adds `INTERNAL` (P2-23). The mock never produces it, so parity is unaffected. **The user may veto.** The alternative is to map infrastructure failures to `CONFLICT`, which we judge misleading. |
| C-03 | Rule 5 says "qty = round4", but the mock rounds quantities to 2dp. | master rule 5 vs products.md §8/§9, invoices.md §2, purchases.md | P2-19 (`round_qty = round2`). Task C-9 corrects the rule text. |
| C-04 | The task brief says "D1–D10 all answered", but master §9 still lists D4–D7 as **Open** (D4 and D7 block 02-E). | master §9 | D4/D7 adopted as recommended (P2-25). Task E-1 records them as answered. D5/D6 don't block Part 02. |
| C-05 | accounting.md §5 prescribes `FOR UPDATE` on the fiscal-year row for **every** posting. | accounting.md §5 | Refined to shared locks for posters and an exclusive lock for close (P2-13). Same guarantee, less serialisation. |
| C-06 | cross-cutting §5 names a `core_get_change_versions` command. | cross-cutting §5 | Replaced by a Rust poller plus a pushed event (P2-11). Same table and semantics. |
| C-07 | cross-cutting §5's event-gap table (and invoices.md §6) say that sale/purchase posting doesn't emit `ledger:changed`. It does: `postJournal` emits on every post (`core.ts:159`). | cross-cutting §5, invoices.md §6, purchases.md | Factual correction. P2-12 also closes the `catalog`/`parties` halves structurally. Part 03 no longer needs to add those emits by hand. |
| C-08 | Name clash for the per-product price-list override child table: `price_list_values(…)` in cross-cutting §8 vs `product_prices(…)` in products.md §2. | cross-cutting §8 vs products.md §2 | Uses `product_prices` (the owning module's, more detailed design). |
| C-09 | cross-cutting §7 says business dates are stored as SQL `DATE`. Mock document dates often carry ISO instants, and some code relies on the UTC `slice(0,10)`. | cross-cutting §7, `invariants.ts:281`, `core.ts:495` | `DocDate` triple (P2-09). `DATE` is still the business-day column. |
| C-10 | Master §4's domain list has no home for diagnostics' 3 production `port` functions (`getAuditEntries`, `getAuditEntities`, `exportSupportBundle`). | master §4 vs diagnostics.md §1 | `diagnostics` added to `BackendDomain` (Phase F). Task F-12 adds it to the §4 list, following the "Vue module names 1:1" rule. |
| C-11 | D2 says UUIDv7 for "every primary key". | master §9 D2 | Documented exception for the 2 infrastructure tables (P2-40). |
| C-12 | `list_printers` is invoked straight from a **page** (`PrintingSettingsPage.vue:62-63`). | master rule 10, CLAUDE.md "Rust ↔ Vue" | Fixed in Phase A (A-2): routed through `printService.listPrinters()`. |
| C-13 | Master §6 still shows Part 01 as "in progress (01.C, 01.D remain)", and README.md is also stale. | `00-MASTER-PLAN.md` §6, `README.md` | Corrected together with this plan's status edit. |
| C-14 | `StoreSettings.theme` is required in the TS type, but no reader reads it (only seeds write it: `db.ts:194`, `seed/index.ts:117`, `fixtures/settings.ts:41`). | cross-cutting §3 asked for confirmation | Confirmed dead. No column is created. Handoff §9: Part 03 `settings` removes it from the TS type when it ports `getSettings`. |
| C-15 | MariaDB has no partial indexes (accounting.md suggests a partial unique on `system_role`, invoices.md one on the open shift per terminal). | accounting.md §2, invoices.md §5 | A generated-column unique for the open shift and the default template. **No** uniqueness on `system_role`, because the mock doesn't enforce it and the importer must accept any valid snapshot. |
| C-16 | Attachment blobs live in the browser's IndexedDB (`src/mocks/attachments.ts`) and are in neither the 46-table map nor any 01.B review. Under D8, a file attached on one terminal is invisible on the others. | cross-cutting §8 (absent) | Not Part 02's to design. **Open question for the user.** Recorded in §9. Part 02 stores `attachment_ids` as an opaque JSON string list. |
| C-17 | D1 (one MariaDB per branch) sits beside the mock's multi-branch-in-one-company model (`branches`, inter-branch transfers). | master §9 D1 vs `db.ts` | Part 02 models the mock exactly (`branches` table, transfers). Cross-Main-PC branches are D6 (sync). Recorded as an observation, not changed. |
| C-18 | D1 says the Main PC "hosts the branch's MariaDB" and D8 says terminals connect to it, but nothing says how MariaDB gets onto a non-technical customer's PC. | master §9 D1/D8 | New D11 (user instruction) + phases A2/A3. |
| C-19 | `core::db::connect` runs `SET SESSION sql_mode…`/`innodb_lock_wait_timeout` through `db.execute_unprepared` on a **pool**, so only one pooled connection gets them. The debug `EQUAL_DB_URL` branch of `connect_and_migrate` skips them entirely. P2-05 asked for `after_connect`. | `src-tauri/src/core/db.rs` `connect`, `connect_and_migrate` | Fixed in A2-10 (P2-55). The managed server also sets them server-wide (P2-48). |
| C-20 | Phase A's 8 DB-backed tests have never run (no MariaDB on the dev machine, only a stopped Laragon MySQL 8.4). | phase-a-foundation.md status note | A2's `bun run db:dev` (P2-53). Phase A's DB gate is closed inside A2's gate. **Order changes to A → A2 → A3 → B.** |
| C-21 | `tauri.conf.json` `bundle.targets: "all"` builds both MSI (381 MB) and NSIS (286 MB). A2's hooks exist only for NSIS. | `src-tauri/tauri.conf.json` | Windows targets → `["nsis"]` (P2-43). The MSI artifact goes away. |
| C-22 | Phase F: `BackendStatus` has no server field, and `DbStatus` has no server states. | phase-f-ipc-bridge.md F-2, `core/state.rs` | A2 adds `DbStatus::ServerStarting`/`ServerFailed`. F-2's TS type gains `server?` (amendment noted in A2's own §Cross-phase edits). The IPC command count is unchanged (P2-54). |
| C-23 | D8: "Main PC = `localhost`". | master §9 D8 | Refined to `127.0.0.1`: `localhost` can resolve to `::1` first against an IPv4 bind, and `skip-name-resolve` matches accounts by IP. `device.rs`'s `default_role` already treats `127.0.0.1` as loopback, so no code conflict. |
| C-24 | Part 02 §5 lists only `infrastructure/{pdf,print}` and says Part 02 creates no other infrastructure module. Master §4 has no DB-hosting module. | §5, master §4 | `infrastructure/database/` added (A2). Backup/restore logic stays out of Part 02; P2-52 only constrains its mechanism. |
| C-25 | Tauri's uninstaller "delete app data" checkbox also removes `$LOCALAPPDATA\<id>`, which holds the WebView2 profile with the IndexedDB `mock-db` snapshot that D10 job 1 imports. | cross-cutting §9, `installer.nsi` | The business DB is out of reach (P2-44), and device/terminal identity is restored (P2-59). The D10-source risk predates this plan → handoff §9: the importer runs before the user is ever asked to reinstall, and its UI says so. |
| C-26 | The bundled-DB brief asks the uninstaller to fully remove the server. The uninstaller does stop the server and delete `server-bin\`, but **leaves the firewall allow rule**. | A2-2, P2-56 | Deliberate: the rule is program-scoped to a deleted exe and does nothing, and a reinstall/upgrade (whose "uninstall before installing" runs these hooks) then needs no second UAC. |
| C-27 | settings.md §9 D-S2 (who owns the auto-backup timer) assumed a Main PC window may not be open. | settings.md §9 | Observation: A3's tray-resident Main PC keeps its webview alive, so D-S2's frontend-timer recommendation is workable. The decision stays with Part 03. |
| C-28 | CLAUDE.md's DoD says DB tests need "a real MariaDB ≥ 10.11" with no way to get one. A fresh clone's `cargo build` needs the payload present (tauri-build validates resources), just as it needs the WebView2 folder. | CLAUDE.md "Definition of done" | A2-12 updates the bullet (`bun run db:dev`) and the fresh-clone note (`node scripts/fetch-mariadb.js`). |
| C-29 | New bundling constraint → reconsider embedded SQLite instead of MariaDB? | D1/D2/D3/D8 | Not reopened. D8 needs LAN multi-terminal access to one DB. SQLite over a network share is unsafe, and the lock design (P2-06/07/13/21) presumes a server. Bundling is solved for MariaDB by A2, so the constraint gives no reason to switch. |

## 5. Target layout after Part 02

```text
src-tauri/
  Cargo.toml            [workspace] members = [".", "migration"]
  .cargo/config.toml    TS_RS_EXPORT_DIR (Phase F)
  tauri.windows.conf.json   bundle.resources (MariaDB payload) + nsis {installMode, installerHooks} (A2)
  windows/installer-hooks.nsh   uninstall: stop server, delete server-bin, keep data, Arabic notice (A2)
  mariadb-11.4.13-winx64/   build-time payload (gitignored; scripts/fetch-mariadb.js)
  migration/            sea-orm-migration crate: m0001 infra … m0013 foreign keys
  src/
    lib.rs              plugins, AppState, boot task, generate_handler!
    core/               error.rs state.rs tx.rs lock.rs db.rs auth.rs terminal.rs device.rs
                        settings.rs events.rs ipc.rs  diag/ (moved)
    entities/           one file per table (75) + mod.rs + soft_delete.rs
    shared/             ledger/{accounts,period,post,reverse,trace}.rs numbering.rs currency.rs
                        stock/{mod,cost,batches}.rs balances.rs invariants/ activity/{record,diff,undo}.rs
    infrastructure/     database/ (bundled MariaDB — A2: mod paths state_file payload config errors
                        credentials admin process supervisor provision recovery upgrade session_end
                        cli; A3: lan pairing hosting), pdf/ (moved), print/ (moved)
    utils/              money.rs id.rs dates.rs text.rs route.rs
    bin/                db_dev_server.rs, db_server_smoke.rs (A2)
  tests/                support/, architecture_rules.rs, db_*.rs, db_server_lifecycle.rs (A2),
                        db_server_lan.rs (A3)
scripts/fetch-mariadb.js
src/modules/core/services/backend.ts          the one switch point (Phase F)
src/modules/<m>/types/gen/*.ts                ts-rs output (Phase F)
src/modules/<m>/types/contract.check.ts       drift checks (Phase F)
```

`domains/`, `infrastructure/backup/` and `infrastructure/sync/` are created by Part 03 or later,
not here.

## 6. Phases

| File | What | Size | Status |
|---|---|---|---|
| [`02-core-and-shared/phase-a-foundation.md`](02-core-and-shared/phase-a-foundation.md) | Workspace, deps, module moves, `utils`, `AppError`, `AppState` (terminal/device/session), pool + version gate, infra migration, `with_tx` + locks, test harness | L | implemented, DB-gate unverified (no local MariaDB) |
| [`02-core-and-shared/phase-a2-bundled-database.md`](02-core-and-shared/phase-a2-bundled-database.md) | Bundled MariaDB (D11): build-time payload, NSIS integration, ProgramData layout, provisioning, supervisor (adopt/restart/clean stop/session end), credentials + self-heal, patch upgrades, reattach, uninstall safety, `db_pool` fix (C-19), `bun run db:dev`, smoke bin | L | pending — **next** |
| [`02-core-and-shared/phase-a3-main-pc-hosting.md`](02-core-and-shared/phase-a3-main-pc-hosting.md) | LAN sharing: firewall rule (one UAC), bind switch, `equal_lan` + pairing code, terminal host fallback, autostart/background/tray/close-to-tray | M | pending |
| [`02-core-and-shared/phase-b-entities.md`](02-core-and-shared/phase-b-entities.md) | Conventions, 75 tables for the 46 `MockDb` keys + `print_templates`, migrations m0002–m0013, entities, soft delete, settings accessor, `TxCtx::require` | L | pending |
| [`02-core-and-shared/phase-c-ledger.md`](02-core-and-shared/phase-c-ledger.md) | `shared::ledger` (resolve_account, assert_open_period, post, reverse, posting trace), `shared::numbering`, `shared::currency` | M | pending |
| [`02-core-and-shared/phase-d-stock.md`](02-core-and-shared/phase-d-stock.md) | `shared::stock` (lock, apply_change, weighted-average cost-out, receive_batch, FEFO), `shared::balances`, `shared::invariants` (all 14) | M | pending |
| [`02-core-and-shared/phase-e-activity.md`](02-core-and-shared/phase-e-activity.md) | `shared::activity` (record/log, entity-from-link, diff_fields) + undo registry, `undo_of`/`undone_by` | M | pending |
| [`02-core-and-shared/phase-f-ipc-bridge.md`](02-core-and-shared/phase-f-ipc-bridge.md) | ts-rs bindings + `ipc_sig!` manifest, `contract.check.ts`, `core_backend_status`, `backend.ts` switch + `backendCall`, change poller + event bridge, scanner updates | M | pending |

**Order:** A → A2 → A3 → B → C → D → E → F. A2 comes first because every DB-backed test (Phase
A's included) needs the server it provides. A3 depends only on A2 and may slip to after F, but
not past Part 02's DoD. C needs B's tables. D's and E's tests post through C. F's poller needs
A's effects and B's `change_versions`.

## 7. Rust test database (every gate needs it)

DB-backed tests need `EQUAL_TEST_DATABASE_URL=mysql://<admin>:<pw>@127.0.0.1:3306` pointing at
MariaDB ≥ 10.11 (a local install or a container on the dev machine). Each test creates
`equal_test_<uuid>`, migrates it and drops it afterwards. **If the variable is missing, the tests
fail with a clear message. They never pass by skipping.** Get one with `bun run db:dev` (P2-53):
`EQUAL_TEST_DATABASE_URL=mysql://root:equal-dev@127.0.0.1:3499`.

## 8. Definition of done (Part 02)

- [ ] Phases A–F done: every box ticked, each phase's gate green, and a status note at the top of each phase file.
- [ ] Phases A2 and A3 gates green (including Phase A's 8 DB tests run against `bun run db:dev`).
- [ ] `cargo build --manifest-path src-tauri/Cargo.toml` and `cargo test --manifest-path src-tauri/Cargo.toml` (with `EQUAL_TEST_DATABASE_URL`) green. That includes `architecture_rules`, the rounding pins, and the ledger/stock/activity/invariants/tx suites.
- [ ] `bun run build`, `bun run check`, `bun run verify:mocks` (still 128 ok / 0 failed: Part 02 changes no posting code in the mock), `bun run contract` + `contract:check`, `bun run memory` + `memory:check` (IPC contract gaps are now **fatal**, and there are 0), `bun run diag:check`, `bun run bindings:check` all green.
- [ ] `AGENT_MEMORY.md` IPC table: `greet` and `render_pdf_spike` gone, `core_backend_status` present, 0 gaps.
- [ ] `bun tauri build` produces one NSIS installer; its size is recorded in phase A2's status note.
- [ ] A real `bun run desktop` check on Windows: (1) boots with no DB configured and the app runs on the mock with no console errors; (2) boots as Main PC against the **managed** MariaDB (provisioned by `db_server_smoke provision-main`): migrations applied, `core_backend_status` reports `connected` and `server.state = running`; (3) an external `UPDATE change_versions …` reaches the mock event bus within ~2 s (dev override on).
- [ ] On a clean Windows PC (no VC++ redistributable, no dev tools): install → provision as Main PC → app `connected` → uninstall keeps `%ProgramData%\com.abdallah.accounting-app\database\data` and shows the Arabic notice → reinstall reattaches with the same data and terminal id. A second PC pairs as a terminal (A3).
- [ ] Master-plan and CLAUDE.md text updates listed in the phases are applied (rule 5, rule 6, §4 domain list, §9 D4/D7, the CLAUDE.md DoD and "Rust ↔ Vue" bullets).
- [ ] `RUST_DOMAINS` is empty, so the e2e suite is unaffected. Per master §8 it runs once at the end of plan 21.
- [ ] Part 02 status set to `done` in master §6. The plan folder stays in `plans/pending/` (plan 21 as a whole is not finished).

## 9. Handoff to Part 03 (write Part 03 only after §8 is green)

- **D10 importer first.** It needs every Part 02 piece (entities, `Id`, `DocDate`, argon2,
  `shared::invariants`), and demo data in desktop dev (D10 job 2) is what lets each domain be
  exercised. Recommend making it Part 03's step 00 (`infrastructure/import/`), following
  cross-cutting §9. Its mappings: mock id → `Id` (generated in array order so `ORDER BY created_at, id`
  keeps mock order), `'branch-main'` → `settings.default_branch_id`, free-text invoice lines
  (`freetext-N`) → `product_id NULL`, legacy `terminalId` strings → one new `Id` each, and an empty
  `settings.currency` → the default country's currency (`currency.ts:15`).
- Each domain's `commands.rs`: one `args` struct per command, an `ipc_sig!` line, a
  `require(area, access)` call (the area per command is decided against the pages that call it),
  `with_tx`/`with_read`, and DTOs that derive `TS` with a `contract.check.ts` entry.
- Register undo compensators for the five undoable actions listed in
  [`phase-e-activity.md`](02-core-and-shared/phase-e-activity.md) §E-5.
- Search haystacks that include joined names (`expenses.categoryName`, `invoices.customerName`,
  `payments.partyName`, `purchases.supplierName`) need a per-endpoint decision: store the
  normalized joined name at write time, or filter in Rust after SQL narrows the rows.
- Settings: remove the dead `StoreSettings.theme` (C-14), and set `settings.timezone` from the
  country profile in setup.
- Before the first domain flips: teach `scripts/contract` that `backendCall('<cmd>')` is an IPC
  call (its disposition heuristic only recognizes `invoke` today, `scripts/contract/extract.ts:153`).
- Open question for the user (C-16): where attachment blobs live under D8.
- Part 03 `setup` adds the first-run role step ("هذا الجهاز هو الجهاز الرئيسي" / "جهاز كاشير")
  calling `infrastructure::database::provision_main` and the terminal pairing form. Part 03
  `settings` adds the LAN-sharing toggle, pairing-code display and the server-failure screen
  driven by `BackendStatus.server`. Part 03 `diagnostics` `exportSupportBundle` includes
  `database::diagnostics_snapshot()` (error-log tail + `server.json`, no secrets). The backup
  domain follows P2-52 and triggers an automatic backup before pending migrations. The D10
  importer handles C-25.

## 10. Later (not in Part 02)

A maintenance-mode flag for multi-terminal restore (settings.md §9), `cargo clippy -D warnings`
as a gate, a keyset-paging fast path, summary tables (master rule 9: only on measurement), the
sync worker (D6), and the undo UI (D4). Windows-service mode for the same layout (if field data
shows terminals often hitting a closed Main PC); LAN auto-discovery for pairing (UDP broadcast);
a series upgrade to the next LTS (dump/restore plan); a "move the Main PC to another computer" flow.
