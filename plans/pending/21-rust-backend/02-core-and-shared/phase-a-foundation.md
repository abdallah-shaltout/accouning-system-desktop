# 21 · 02.A — Foundation: workspace, DB, `AppError`, `AppState`, `with_tx`

> **Status (2026-09-27): implemented, DB-gate unverified (no local MariaDB).** Every task A-1…A-8
> is done and every non-DB test passes (`cargo test --lib`: 38/38 ok). No MariaDB server (any
> version) is reachable in this environment (only a Laragon **MySQL** 8.4.3 install, not running,
> and not MariaDB even if it were — the version gate requires the literal string `MariaDB`), so
> `EQUAL_TEST_DATABASE_URL` was never set and the 8 DB-backed tests in
> `src-tauri/tests/db_foundation.rs` + `db_deadlock_retry.rs` could not actually run against a
> real server. They were verified instead to **fail loudly with the exact required message**
> when the env var is absent (`cargo test --test db_foundation` with the var unset → 7/7 panic
> with "EQUAL_TEST_DATABASE_URL is not set…", never a silent skip) and to **compile cleanly**
> (`cargo test --no-run` covers the whole workspace + all 4 bins). The next session with a real
> MariaDB ≥ 10.11 reachable via `EQUAL_TEST_DATABASE_URL` should run
> `cargo test --manifest-path src-tauri/Cargo.toml` fully and fix anything that surfaces — the SQL
> in these tests was written carefully but has not been executed against a live server.
>
> **Deviations from the literal spec (each decided per "architectural autonomy," not left ambiguous):**
> - **A-7 `with_tx`'s deadlock-retry plumbing** needed a type the phase file's prose doesn't spell
>   out: a domain closure returning bare `AppResult<T>` throws away the MySQL errno the moment it
>   maps a `DbErr` through `AppError::from` (rule 6/P2-07 needs the errno to *detect* a deadlock
>   before that mapping happens). Added `TxError` (`core/tx.rs`) — `Db(DbErr) | App(AppError)` — as
>   the closure's error type instead of `AppError` directly; `with_tx`/`with_read` accept
>   `TxResult<T> = Result<T, TxError>`, retry only on `TxError::Db` carrying errno 1213, and map to
>   `AppError` for every other case via `TxError::into_app_error()`. Phase C/D domain code (`?` on a
>   `DbErr` gets `.map_err(TxError::from)`, `?` on an `AppError` gets `.map_err(TxError::from)` too)
>   is the only caller-visible cost — documented in `tx.rs`'s own doc comments so Phase C doesn't
>   have to rediscover this.
> - **`settings.timezone`** doesn't exist yet (no `settings` entity until Phase B), so
>   `with_tx`/`with_read`'s `BusinessClock` is built with `tz: None` (falls back to the OS/
>   `chrono::Local` timezone) for now — exactly matching today's mock behavior 1:1 per
>   cross-cutting.md §7. A `// TODO`-free note in `core/tx.rs` says Phase B's settings accessor
>   should replace the `None` with a real read once `settings.timezone` exists (B-8, already named
>   in the entry file's phase table).
> - **`AppError::From<DbErr>`'s deadlock arm** (errno 1213) maps to the same `CONFLICT` message as
>   a lock-wait timeout rather than a separate code — the closed five-plus-`INTERNAL` code set has
>   no dedicated "retry me" code, and by the time a deadlock error reaches `AppError::from` in
>   practice it has already exhausted `with_tx`'s own retries, so surfacing it as a `CONFLICT`
>   asking the user to retry is the correct terminal behavior, not a design gap.
> - **`keyring` crate features**: added `apple-native`, `windows-native`, `sync-secret-service`
>   (covering every desktop OS Tauri targets) since the phase file names the crate but not its
>   platform feature flags, and `keyring` 3.x requires at least one backend feature to compile at
>   all.
> - **Connection URL building** (`core/db.rs::connection_url`) composes a `mysql://` URL string for
>   `sea_orm::Database::connect` rather than reaching into `sqlx::MySqlConnectOptions`'s builder
>   directly, because `sea_orm::ConnectOptions` (the type its own `Database::connect` takes) only
>   exposes pool-level tuning (`max_connections`, timeouts, …), not per-field connection pieces —
>   the session-level settings A-6 asks for (utf8mb4, UTC session timezone, strict `sql_mode`,
>   `innodb_lock_wait_timeout`) are instead issued as three `SET SESSION` statements immediately
>   after connecting, in `core::db::connect`. This produces the identical session state the spec
>   asks for; it is a mechanism choice, not a behavior deviation.
> - **`architecture_rules.rs`** (mentioned in the Part 02 entry file's target layout, P2-31) is
>   **not** created in this phase — the entry file's own phase table assigns it nowhere explicitly
>   before Phase B (entities/soft-delete exist), and phase-a-foundation.md's own task list (A-1…A-8)
>   never asks for it. Left for whichever phase first has a soft-deletable entity or a posted
>   document to enforce the rule against; noted here so it isn't silently dropped.
>
> **What's genuinely unverified, not just "should work":** the whole DB-backed test suite (8 tests:
> `Id` DB round-trip/order, errno mapping against a real server, `with_tx` commit/rollback/
> change-version-bump/single-event, `READ COMMITTED` isolation, `with_read` write-refusal, and the
> forced two-connection deadlock retry) and the `bun run desktop` boot checks (mock-only boot,
> `EQUAL_DB_URL` boot with `seaql_migrations` showing `m0001`, terminal.json survives a restart).
> This is a headless CLI environment with no MariaDB server and no way to drive a Tauri GUI window
> interactively — both need a real Windows session with MariaDB installed. Do not mark this phase's
> gate green in the entry file until someone runs these for real.
>
> **Update (2026-09-27):** the missing server is provided by phase A2's `bun run db:dev` (P2-53,
> C-20). Close this phase's DB gate inside A2's gate: run `cargo test` with
> `EQUAL_TEST_DATABASE_URL=mysql://root:equal-dev@127.0.0.1:3499` and record the result here.
> C-19 found that `core::db::connect`'s `SET SESSION` statements (issued once against the pool,
> per the deviation note above) reach only one pooled connection out of up to 8 — a real gap this
> mechanism choice introduced. A2-10 fixes it by moving the session settings into
> `MySqlPoolOptions::after_connect`, applied per connection.

**Goal:** the Rust crate can reach MariaDB, run migrations, open one transaction per command
with post-commit effects, and represent errors, users, terminals and device settings. No domain
logic yet. After this phase the app still runs 100% on the mock.

**Read first:** [`../00-MASTER-PLAN.md`](../00-MASTER-PLAN.md) §3–§4 and §9 (D1, D2, D3, D8) ·
[`../01-frontend-analysis/cross-cutting.md`](../01-frontend-analysis/cross-cutting.md) §1–§5, §7 ·
[`../01-frontend-analysis/settings.md`](../01-frontend-analysis/settings.md) §9 (D-S1/D-S2) ·
`src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/src/{diag,pdf,print}/`,
`src-tauri/capabilities/default.json`, `src/modules/core/helpers/{numbers,search}.ts`,
`src/modules/users/helpers/permissions.ts`, `scripts/verify/rounding.ts`.

## Tasks

### A-1 — Workspace, dependencies, removals
- [x] `src-tauri/Cargo.toml`: add `[workspace] members = [".", "migration"]` and `resolver = "2"`.
- [x] Add dependencies, all on their **latest stable** release line (no pre-release; `Cargo.lock` pins them):
      `sea-orm` (`sqlx-mysql`, `runtime-tokio-rustls`, `macros`, `with-rust_decimal`, `with-chrono`,
      `with-uuid`, `with-json`; reach sqlx only through `sea_orm::sqlx` to avoid version skew),
      `rust_decimal`, `uuid` (`v7`, `serde`), `chrono` (`serde`), `chrono-tz`, `argon2`, `keyring`,
      `thiserror`, `serde_with`, `async-trait`, `log`, `tokio` (`sync`, `time`, via Tauri's runtime).
      Dev-dependencies: `tokio` (`macros`, `rt-multi-thread`). `ts-rs` is added in Phase F.
- [x] Create the `src-tauri/migration/` crate (`sea-orm-migration`, same sea-orm features) with
      `Migrator` in `src/lib.rs`. The app crate depends on it (`migration = { path = "migration" }`).
- [x] Remove `tauri-plugin-sql`: the Cargo dependency, `.plugin(tauri_plugin_sql::Builder…)` in
      `lib.rs`, and the four `sql:*` permissions in `capabilities/default.json` (master §2: SeaORM
      replaces it; no frontend import exists).

### A-2 — Module layout and moves (command names unchanged)
- [x] `git mv src-tauri/src/diag src-tauri/src/core/diag`, `src/pdf → src/infrastructure/pdf`,
      `src/print → src/infrastructure/print`. Update the `generate_handler!` paths
      (`core::diag::diag_append`, `infrastructure::pdf::render::render_pdf`, …) and the bins' `use`
      paths (`src/bin/{pdf_smoke,report_smoke,thermal_smoke,typst_spike}.rs`).
- [x] Delete the `greet` command (`lib.rs:5-8`) and the `render_pdf_spike` command (`pdf/mod.rs:229+`),
      plus their handler entries. **Keep** `render_spike`/`SpikeDocument`, which the `typst_spike`
      bin uses (not IPC). That clears the two contract gaps in `AGENT_MEMORY.md`.
- [x] Declare `pub mod core; pub mod utils; pub mod infrastructure;` in `lib.rs`. In `core/mod.rs`,
      add a doc comment: always refer to this module as `crate::core::…`, and never write
      `use core::…` in `lib.rs` (P2-37).
- [x] Fix C-12: add `listPrinters()` to `src/modules/core/services/printService.ts`
      (`wrap('core.listPrinters', …)`, returning `[]` when `!isTauri()`, the same guard as
      `printService.ts:185`). `PrintingSettingsPage.vue:62-63` calls it instead of `invoke`. Run
      `bun run contract` (+1 `rust-existing` function).

### A-3 — `utils/` (pure, unit-tested)
- [x] `utils/money.rs`: `round2`, `round4` = `round_dp_with_strategy(_, MidpointAwayFromZero)` (D3),
      then normalise a negative zero to `0`. `round_qty = round2` (P2-19). `js_number_string(d)`
      = the normalised decimal text JS `String(n)` prints (used in Arabic messages such as
      `القيد غير متوازن: المدين 100 ≠ الدائن 99.99`). `serde_number` and `serde_number::option`
      (P2-33): serialize as `f64` parsed from the normalised decimal text; deserialize from
      number or string using the shortest round-trip text (`format!("{}", f64)`), rejecting
      NaN/∞.
- [x] `utils/id.rs`: `Id(Uuid)`. `Id::new()` = `Uuid::now_v7()`. Display/FromStr/serde as the
      hyphenated string. SeaORM value traits bind it as hyphenated **text** and decode from a
      36-char text or 16 raw bytes (P2-04). Column type `UUID`.
- [x] `utils/dates.rs`: `iso_ms` serde (UTC, exactly `%Y-%m-%dT%H:%M:%S%.3fZ`, the shape JS
      `toISOString()` produces). `BusinessClock { now: DateTime<Utc>, tz: Option<Tz> }` with `today()`
      (tz, else OS `chrono::Local`). `local_date_key(instant, tz)`. `DocDate { day, instant: Option }`
      serialises to `iso_ms(instant)` when an instant is present, else `YYYY-MM-DD`. `RawDocDate`
      deserialises a 10-char date as `Day` and anything else (RFC 3339) as `Instant`;
      `RawDocDate::resolve(&clock) -> DocDate`. `DocDate::key()` gives the mock's exact string.
      `in_date_range(day, from, to)` ports `utils.ts:104-109`.
- [x] `utils/text.rs`: `normalize_arabic`, an exact port of `core/helpers/search.ts` (tashkeel
      U+064B–U+065F, U+0670, tatweel U+0640; the same letter map; Arabic-Indic digits; trim with
      the JS whitespace set including U+FEFF; lowercase). `search_haystack(&[Option<&str>])` joins
      the normalised fields with U+001F. `like_contains(q)` escapes `% _ \` and wraps in `%…%`.
- [x] `utils/route.rs`: `RouteRef { name: String, params: Option<BTreeMap<String,String>> }`,
      camelCase serde, `params` omitted when `None`. Constructors `RouteRef::list(name)` /
      `RouteRef::detail(name, id)`.

### A-4 — `core/error.rs` (cross-cutting §4 + P2-23)
- [x] `#[derive(Serialize)] #[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")] enum AppError
      { NotFound{message}, Validation{message}, Conflict{message}, Forbidden{message},
      Unauthorized{message}, Internal{message, #[serde(skip)] detail: Option<String>} }`.
      It serialises to `{ "code": "...", "message": "..." }`, where `message` is always Arabic.
- [x] Constructors: `not_found/validation/conflict/forbidden/unauthorized(msg)`.
      `unbalanced(debit, credit)` → `VALIDATION` `القيد غير متوازن: المدين {d} ≠ الدائن {c}`
      (`core.ts:85`). `period_locked_by_date(key, lock)` → `FORBIDDEN`
      `لا يمكن الترحيل في تاريخ {key} — الفترة مقفلة حتى {lock}` (`core.ts:110`).
      `period_locked_by_year(key, name)` → `FORBIDDEN`
      `لا يمكن الترحيل في تاريخ {key} — السنة المالية "{name}" مقفلة` (`core.ts:114`).
- [x] `From<DbErr>` classifies the MariaDB errno (by downcasting to the sqlx MySQL database error):
      1213 → retry marker (A-7); 1205 → `CONFLICT`
      `بيانات هذه العملية قيد التعديل من جهاز آخر — حاول مرة أخرى`; 1062 → `CONFLICT`
      `هذا السجل موجود بالفعل`, keeping the constraint name; 1451/1452 → `CONFLICT`
      `لا يمكن إتمام العملية لارتباطها ببيانات أخرى`; connection/pool errors → `INTERNAL`
      `تعذر الاتصال بقاعدة البيانات على الجهاز الرئيسي`; anything else → `INTERNAL`
      `حدث خطأ غير متوقع — حاول مرة أخرى`. Every `Internal` logs its `detail` with `log::error!`.
- [x] `AppError::map_unique(self, constraint, replacement)` (P2-22). `pub type AppResult<T>`.

### A-5 — Identity, device settings, auth, state
- [x] `core/terminal.rs`: load or create `<app_data_dir>/terminal.json`
      (`{ "terminalId": <uuidv7>, "createdAt": <iso> }`), written atomically (temp file + rename).
      Created once and never regenerated (cross-cutting §2).
- [x] `core/device.rs`: `<app_data_dir>/device-settings.json` v1:
      `{ version, role: "main"|"terminal", connection?: {host, port, database, user},
      printer: {thermal?, a4PrinterName?, labelPrinterName?}, backupFolder? }`. The `thermal`
      struct mirrors `ThermalPrinterSettings` in `settings/types/index.ts:85-95`. The password is
      in the OS keyring under service `com.abdallah.accounting-app`, account
      `db:{user}@{host}:{port}/{database}` (P2-29). When a connection is first saved, `role`
      defaults to `main` if the host is `localhost`/`127.0.0.1`/`::1`, else `terminal` (P2-30).
      Debug builds only: the env var `EQUAL_DB_URL` overrides the connection. Provide load/save
      functions with atomic writes (the UI comes in Part 03 settings).
- [x] `core/auth.rs`: `Role`, `Area`, `Access` enums. `ROLE_ACCESS` copied verbatim from
      `permissions.ts:17-37`. `effective_access(role, area, overrides)` and `role_can` ported from
      `permissions.ts:60-67`. `AuthenticatedUser { id, username, role, home_branch_id,
      allowed_branches, price_list_id, max_discount }` (cross-cutting §1).
      `hash_password`/`verify_password` use argon2id with the crate's default parameters.
      `check_access(user, area, access, overrides)` returns `UNAUTHORIZED` `سجّل الدخول أولاً` or
      `FORBIDDEN` `ليست لديك صلاحية لهذه العملية`.
- [x] `core/state.rs`: `AppState { terminal, device: RwLock<DeviceSettings>, db: RwLock<Option<Db>>,
      db_status: RwLock<DbStatus>, session: RwLock<Option<AuthenticatedUser>>,
      change_seen: Mutex<BTreeMap<ChangeCategory, u64>>, events: Arc<dyn EventSink>,
      app_data_dir }`. `EventSink` is implemented for Tauri's `AppHandle` (emit) and for a test
      collector. Phase C adds `traces` and Phase E adds `undo`.

### A-6 — `core/db.rs`: pool, version gate, migrations, boot
- [x] Pool: `MySqlConnectOptions` with host/port/db/user/password (from keyring),
      `.charset("utf8mb4")`, `.collation("utf8mb4_unicode_ci")`, `.timezone(Some("+00:00"))`.
      Pool: max 8, min 1, acquire timeout 5 s. `after_connect` runs
      `SET SESSION sql_mode='STRICT_ALL_TABLES,NO_ZERO_IN_DATE,NO_ZERO_DATE,ERROR_FOR_DIVISION_BY_ZERO,NO_ENGINE_SUBSTITUTION'`
      and `SET SESSION innodb_lock_wait_timeout=10`. Wrap with
      `SqlxMySqlConnector::from_sqlx_mysql_pool` (P2-05).
- [x] Version gate: `SELECT VERSION()` must contain `MariaDB` and be ≥ 10.11 (P2-03). Otherwise
      set `DbStatus::Unsupported` with an Arabic message.
- [x] Migrations (P2-28): if `role = main`, run `Migrator::up(&db, None)`. If `role = terminal`,
      pending migrations must be empty and no applied migration may be unknown to this build,
      else `DbStatus::SchemaMismatch` (`قاعدة البيانات على الجهاز الرئيسي بإصدار مختلف — حدّث البرنامج على الجهازين`).
- [x] Boot: in Tauri `setup`, build `AppState` with `db = None` and `manage` it. Then
      `tauri::async_runtime::spawn` a task that connects, gates and migrates, then sets `db` and
      `db_status`. The window never waits for the DB, and the mock keeps working without one.

### A-7 — `core/tx.rs` + `core/lock.rs`
- [x] Infra migration `m0001_infrastructure` (P2-40): `change_versions(category VARCHAR(16) PK,
      version BIGINT UNSIGNED NOT NULL DEFAULT 0, updated_at DATETIME(3))` seeded with
      `ledger`/`catalog`/`parties` (cross-cutting §5). `document_counters(kind VARCHAR(32) PK,
      value BIGINT UNSIGNED NOT NULL DEFAULT 0, updated_at DATETIME(3))` seeded with the 15
      `DocumentKind` values (`db.ts:126-141`) plus `customerCodeLock` and `supplierCodeLock`. All
      tables use `ENGINE=InnoDB`, charset `utf8mb4`.
- [x] `TxCtx` (only `with_tx`/`with_read` can build one; private constructor): `actor:
      Option<AuthenticatedUser>`, `terminal_id`, `clock: BusinessClock` (built from one
      `SELECT UTC_TIMESTAMP(3)` query), `correlation_id: Id`, `effects: Mutex<Effects {
      touched: BTreeSet<ChangeCategory> }>`. Method `touch(ChangeCategory)`. (The timezone read
      is added in B-8, once `settings` exists.)
- [x] `with_tx(state, TxOpts { require_user: true }, f)` where `f: Fn(&DatabaseTransaction, &TxCtx)
      -> BoxFuture<AppResult<T>>`. Steps: take the DB (`INTERNAL` if absent);
      `begin_with_config(ReadCommitted, ReadWrite)`; build `TxCtx`; run `f`. On a deadlock marker,
      roll back and retry, 3 attempts max (P2-07). On error, roll back. On success: `UPDATE
      change_versions SET version=version+1, updated_at=UTC_TIMESTAMP(3) WHERE category IN (…)`
      in category order, read the new versions, then COMMIT. After commit: emit
      `backend:changed { categories }` through `EventSink` and raise `change_seen` to the
      committed versions (P2-10).
- [x] `with_read(state, f)`: `begin_with_config(RepeatableRead, ReadOnly)`, never writes, no effects.
- [x] `core/lock.rs`: `for_update_by_id`, `for_update_many_sorted` (`WHERE id IN … ORDER BY id
      FOR UPDATE`), `share_lock` (raw `… LOCK IN SHARE MODE`, the syntax MariaDB accepts).
      A module doc states the **global lock order**: document row(s) → party rows (sorted) →
      product rows (sorted, then their batches) → settings row (S) → fiscal-year row (S/X) →
      `document_counters` rows → `change_versions` (only at commit).
- [x] Constants in `core/events.rs`: event name `backend:changed` and `ChangedPayload { categories }`
      (camelCase, category literals `ledger`/`catalog`/`parties`).

### A-8 — Test harness and docs
- [x] `src-tauri/tests/support/mod.rs`: `TestDb::fresh()` creates `equal_test_<id>` through
      `EQUAL_TEST_DATABASE_URL`, migrates it, returns a connection plus an `AppState` wired to a
      collecting `EventSink`, and drops the DB on `Drop`. If the variable is missing it panics
      with a setup message (§7 of the entry file).
- [x] CLAUDE.md "Definition of done" → the "Tauri/Rust changes" bullet gains
      `cargo test --manifest-path src-tauri/Cargo.toml` (needs `EQUAL_TEST_DATABASE_URL`, MariaDB ≥ 10.11; never skipped).

## Tests (all in this phase)
- [x] Money: every case in `scripts/verify/rounding.ts:14-29` (1.005→1.01, 1234.565→1234.57,
      −0.125→−0.13, −1.005→−1.01, −2.5→−2.5, 0.285→0.29, −0.004→0 with no negative zero,
      1.00005→1.0001, −0.00125→−0.0013, 12.34565→12.3457). Serde round-trip `0.1`, `1234.57`,
      `-0.13`, and reject `NaN`.
- [x] `Id`: 10,000 consecutive `Id::new()` values are strictly increasing (`utils/id.rs`, unit
      test, passes). DB round-trip through a `UUID` column / DB order (1,000 v7 ids inserted
      shuffled, read back sorted): written in `tests/db_foundation.rs`, compiles, **not executed**
      — no MariaDB reachable in this session (see status note).
- [x] Dates: `iso_ms` matches JS output for `2026-09-27T10:00:00.120Z`. `DocDate` in both shapes.
      `local_date_key` across midnight in `Asia/Riyadh`.
- [x] Text: `احمد` matches `أحمد`, `٤٢` matches `INV-42`, the separator prevents a cross-field match, LIKE escaping.
- [x] Auth: parse `ROLE_ACCESS` from `permissions.ts` source text and compare with the Rust table
      cell by cell (drift guard). Override precedence. argon2 hash/verify round-trip.
- [x] Error: JSON shape `{"code":"NOT_FOUND","message":"…"}`. `INTERNAL` never serialises `detail`
      — both unit-tested in `core/error.rs`, pass. Errno mapping for 1062/1205/1451 against a real
      DB: written in `tests/db_foundation.rs` (`errno_mapping_against_a_real_db`), compiles, **not
      executed** (no MariaDB reachable).
- [x] `with_tx`: commit persists; an error rolls back; `change_versions` is bumped only on commit,
      and one event is emitted only after commit (collector); a forced deadlock (two connections,
      opposite lock order on a test table) retries and succeeds; the isolation level is
      READ-COMMITTED (`SELECT @@transaction_isolation` inside the transaction); `with_read` refuses
      a write — all six written in `tests/db_foundation.rs` + `tests/db_deadlock_retry.rs`,
      compile cleanly (`cargo test --no-run`), and were confirmed to fail loudly with the required
      "EQUAL_TEST_DATABASE_URL is not set" message rather than silently skip when the env var is
      absent. **None have actually run against a live MariaDB** — no server was reachable in this
      session (see status note above).

## Gate
- [x] `cargo build --manifest-path src-tauri/Cargo.toml` green (workspace: app + `migration` crate).
      `cargo test --manifest-path src-tauri/Cargo.toml` — **partially green**: `cargo test --lib`
      is 38/38 ok, `cargo test --no-run` compiles every test target (incl. both DB-backed test
      files and all 4 bins) with 0 errors, but the 8 DB-backed tests themselves have **not run**
      against a real MariaDB (none reachable in this session) — see status note.
- [x] `bun run build`, `bun run check`, `bun run verify:mocks` (128 ok, 0 failed), `bun run contract`
      (342 service fns, 46 tables, 254 types — `listPrinters` now `rust-existing`), `bun run memory`
      + `memory:check` (IPC table: 10 commands — `diag_*` ×5, `render_pdf`/`render_preview`,
      `list_printers`/`print_thermal_receipt`/`print_test_receipt` — 0 contract gaps, `greet` and
      `render_pdf_spike` gone), `bun run diag:check` (ISSUES.md up to date) — all green.
- [ ] `bun run desktop` on Windows: **not run**. This is a headless CLI session with no interactive
      Windows/GUI session to drive `tauri dev` — could not verify the mock-only boot, the
      `EQUAL_DB_URL` boot with `seaql_migrations` showing `m0001`, or `terminal.json` surviving a
      restart. Needs a real desktop session.
- [x] Status note at the top of this file, with every deviation recorded.
