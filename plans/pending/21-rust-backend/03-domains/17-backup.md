# 21 · 03.17 — `backup` (backup/restore through SQL in the existing archive format, auto backup on the Main PC, automatic backup before pending migrations)

> **Status:** code complete 2026-09-28 (this implementer), cargo not run this session (hard rule: no
> build/test commands — the manager compiles after the wave). Wave **W2** (entry file §4). Depends
> on: 00-import (`import_snapshot` for legacy browser archives, GI-1 rule exemption), 01-settings
> (`store.rs` backup-policy merge + device-file helpers, `network.rs` terminal count), Part 02
> (`with_read`, `with_tx`, `shared::invariants::run_all`, `infrastructure::database::{paths, lan,
> credentials}`, `migration::Migrator`), and gaps **GB-1…GB-6** below — all six were already done by
> the manager before this wave (`with_read_on`, the `PreMigrationBackup` trait + `DbStatus::
> MigrationBackupFailed` + `core/status.rs` message, `ServerPaths::backups()`, the GB-5 crates, and
> `infrastructure/backup/**`'s `architecture_rules.rs` exemption all already exist). **Still needed
> from the manager** (outside this implementer's allowed files): switch `core::db::migrate`'s
> `NoPendingMigrationBackup` call site to `infrastructure::backup::pre_migration::
> RealPreMigrationBackup`, add `pub mod backup;` to `infrastructure/mod.rs`, register the 8
> `settings_*` commands in `lib.rs`'s `generate_handler!` and chain `infrastructure::backup::commands
> ::ipc_signatures()` into `domains/mod.rs`'s `all_ipc_signatures()` (mirrors `infrastructure::import`'s
> own two registration points), then run the full definition-of-done gate (cargo build/test/
> bindings:check, `bun run build`, `bun run memory`).

**Goal.** `src-tauri/src/infrastructure/backup/` builds and restores the **same zip archive** that
`settings/helpers/backupArchive.ts` builds (manifest, `data.json`, optional AES-GCM `payload.enc`,
SHA-256 checksum), reading the database **through SQL inside one `with_read` snapshot** (P2-52, no
`mariadb-dump`), restoring inside **one** transaction that ends with the invariants green, never
changing `APP_NAME = 'accounting-app'`. It ports `backupService.ts`'s backend work (8 commands),
runs the daily/close auto backup only on the Main PC (settings.md §9 D-S1/D-S2), and takes an
automatic backup before any pending migration (P2-52 handoff).

**Read first.** [`../01-frontend-analysis/settings.md`](../01-frontend-analysis/settings.md) §1
(`backupSettings` … `pickBackupFolder` rows), §2 (`BackupManifest`, `BackupKind`), §3 (restore message),
§4, §5 (restore row), §8 (open restore fix), §9 (D-S1, D-S2, restore multi-terminal) ·
[`../01-frontend-analysis/cross-cutting.md`](../01-frontend-analysis/cross-cutting.md) §3 (`backup.folder`
= device) · [`../02-CORE-AND-SHARED-ARCHITECTURE.md`](../02-CORE-AND-SHARED-ARCHITECTURE.md) P2-44,
P2-52, P2-28, C-24, C-27, §9 · mock/frontend: `src/modules/settings/services/backupService.ts:36-489`,
`helpers/backupArchive.ts:17-268`, `helpers/backupCrypto.ts:8-73`, `types/backup.ts:8-75`,
`controllers/useBackupStore.ts:24-27`, `components/RestoreBackupModal.vue:31-60`,
`pages/BackupSettingsPage.vue:60-82,255`, `src/mocks/persist.ts:18,31` · Rust: `core/tx.rs`,
`core/db.rs:105-126` (`migrate`), `core/state.rs` (`DbStatus`), `infrastructure/database/paths.rs`,
`infrastructure/database/lan.rs:342` (`connected_terminal_count`), `migration/src/lib.rs`,
`entities/values.rs:269-284` (`BackupPolicy`), `tests/architecture_rules.rs`.

## 1. Commands

| Mock fn | Disposition | Rust command | Args → Return | Area / Access | Tx | Events |
|---|---|---|---|---|---|---|
| `backupSettings` | port (**sync → async**, D-1) | `settings_backup_settings` | `()` → `BackupSettings` | session | `with_read` | — |
| `saveBackupSettings` | port | `settings_save_backup_settings` | `{ patch }` → `BackupSettings` | Settings:Write | `with_tx` | — |
| `previewBackupCounts` | port | `settings_preview_backup_counts` | `()` → `OrderedCounts` | Settings:Read | `with_read` | — |
| `backupNow` (manual path) | port (split, D-2) | `settings_build_backup_archive` | `{ kind, password? }` → `BackupArchive` | Settings:Write | `with_read` | — |
| `backupNow` → `afterBackupSaved` | port (split, D-2) | `settings_record_backup_saved` | `{ manifest, kind }` → `()` | Settings:Write | `with_tx` | — |
| `initAutoBackup` → `runAutoBackupIfDue` / `runCloseBackup` | port of the logic only; the timer + close hook stay frontend (settings.md §1, D-S2) | `settings_run_auto_backup_if_due` | `{ trigger: 'schedule' \| 'close' }` → `AutoBackupOutcome` | none (D-6) | `with_read` + `with_tx` | — |
| `previewRestore` | port (**sync → async**, D-1) | `settings_preview_restore` | `{ archiveBase64 }` → `RestorePreview` | Settings:Read | none (bytes only) | — |
| `restoreFromArchive` | port | `settings_restore_from_archive` | `{ archiveBase64, password? }` → `()` | Settings:Write **and** Users:Write (D-7) | `with_read` (pre-restore) + one `with_tx` | `ledger`, `catalog`, `parties` |
| — (P2-52) | no IPC | `backup::pre_migration::backup_before_migrations` | called by `core::db::migrate` (GB-2) | — | `with_read_on` (GB-1) | — |

**8 commands**, in `src-tauri/src/infrastructure/backup/commands.rs` with their own
`ipc_signatures()` (chained by the manager, GI-4). Names keep the `settings_` prefix: the service is
`settings/services/backupService.ts`, so the domain flips with `usesRust('settings')`. Frontend-only
and untouched: `isTauriMode`, `listHistory` (override, settings.md §8), `deleteHistoryEntry`,
`verifyHistoryEntry`, `pickRestoreFile`, `pickBackupFolder`, `stopAutoBackup`, `initAutoBackup` itself.

## 2. DTOs (`infrastructure/backup/dto.rs`, `#[ts(export_to = "settings/types/gen/")]`)

New TS types go first into `src/modules/settings/types/backup.ts`:

```ts
export interface BackupArchive { manifest: BackupManifest; fileName: string; archiveBase64: string }
export type AutoBackupTrigger = 'schedule' | 'close';
export interface AutoBackupOutcome { ran: boolean; path?: string; skipped?: 'not-main' | 'disabled' | 'not-due' | 'no-folder'; error?: string }
```

| Rust DTO | TS type | Notes |
|---|---|---|
| `BackupManifest` | `types/backup.ts:8-18` | `app` = `"accounting-app"` (`APP_NAME`, never changed); `appVersion` = `env!("CARGO_PKG_VERSION")`; `schemaVersion: u32` (`#[ts(type = "number")]`); `createdAt` `format_iso_ms`; `counts` ordered map (below); `kind: BackupKind`. Serialised pretty with 2-space indent like `JSON.stringify(manifest, null, 2)` (`backupArchive.ts:136`). |
| `BackupKind` | `types/backup.ts:27` | `#[serde(rename_all = "kebab-case")]` `Manual`, `Auto`, `PreRestore` (settings.md §2). |
| `BackupSettings` | `types/backup.ts:42-61` | `folder` = device (cross-cutting §3); instants `format_iso_ms`; `lastAutoRunDate` `YYYY-MM-DD`; `retention: i32`; optional keys absent (never `null`). 01-settings' `StoreSettings.backup` is typed by path to this TS type (01 §2). |
| `BackupSettingsPatch` | `Partial<BackupSettings>` | every key optional; an absent key = unchanged. The two places the mock clears keys by passing `undefined` (`afterBackupSaved`, `:196`) are Rust-internal code paths (§3.4), because `undefined` never crosses JSON. |
| `RestorePreview` | `types/backup.ts:71-75` | `compatibilityNote` optional. |
| `BackupArchive`, `AutoBackupOutcome`, `AutoBackupTrigger` | new | `archiveBase64` standard base64 (the `pdf_base64` precedent, `core/services/pdfService.ts:516`); `skipped` kebab-case enum. |
| `OrderedCounts` | `Record<string, number>` | `Vec<(&'static str, i64)>` + custom `Serialize` preserving key order; `#[ts(type = "Record<string, number>")]`. |
| Args | — | `SettingsSaveBackupSettingsArgs { patch }`, `SettingsBuildBackupArchiveArgs { kind, password: Option<String> }`, `SettingsRecordBackupSavedArgs { manifest, kind }`, `SettingsRunAutoBackupIfDueArgs { trigger }`, `SettingsPreviewRestoreArgs { archive_base64 }`, `SettingsRestoreFromArchiveArgs { archive_base64, password }`. |

`settings/types/contract.check.ts` (01 creates it; append): `BackupManifest`, `BackupKind`,
`BackupSettings`, `RestorePreview`, `BackupArchive`, `AutoBackupOutcome`, `AutoBackupTrigger`.

## 3. Service logic (`infrastructure/backup/{mod,commands,dto,archive,crypto,dataset,counts,upgrade,auto,restore,pre_migration}.rs`)

### 3.1 Archive format (`archive.rs`, `crypto.rs`) — byte-compatible with `backupArchive.ts`
- Zip (`zip` crate, deflate) entries: `manifest.json` always plaintext; unencrypted → `data.json` (+
  `attachments/<id>.{blob,json,thumb}` when present, read-only support); encrypted → `payload.enc` +
  `crypto.json { saltB64, ivB64, iterations }` (`:94-118`).
- Pack format for the encrypted payload (`:144-175`): 4-byte little-endian header length, JSON
  `{ "names": [[name, len], …] }`, then the bytes in that order.
- Crypto (`backupCrypto.ts:8-67`): PBKDF2-HMAC-SHA256, `iterations` = 150 000 on write (read from
  `crypto.json` on read), 16-byte salt, 256-bit key, AES-256-GCM with a 12-byte IV, ciphertext‖tag (the
  WebCrypto layout); salt/IV from `getrandom`. Wrong password / tag failure → `VALIDATION`
  `كلمة المرور غير صحيحة، أو الملف تالف` (`:65`).
- Checksum (`:120-123`, `backupService.ts:263-279`): SHA-256 hex of `payload.enc` (encrypted) or of the
  payload files concatenated in **sorted name order** (unencrypted).
- Read errors, exact texts: no/unreadable `manifest.json` → `VALIDATION`
  `ملف النسخة الاحتياطية غير صالح: manifest.json مفقود` (`:204`); encrypted without `payload.enc`/`crypto.json`
  → `ملف النسخة الاحتياطية مشفّر لكن بياناته مفقودة` (`:218`); no `data.json` → `ملف النسخة الاحتياطية غير صالح: data.json مفقود`
  (`:237`); invalid base64 argument → the manifest text (it cannot be a readable archive).
- `backup_file_name(store_name, local_now)` = `backup-<slug>-<YYYYMMDD-HHmm>.zip` (`:72-82`): slug =
  trimmed name or `company`, runs of `\/:*?"<>|` → `-`, whitespace runs → `-`, first 40 characters;
  `local_now` via the business clock timezone (the mock uses local time).

### 3.2 Dataset (`dataset.rs`) — the `data.json` of Rust-made archives (schema-agnostic, lossless)
`DataSetV1 { format: "equal-db", dbSchemaVersion, migrations: [names], tables: [{ name, columns, rows: [[string | null]] }] }`.
- Tables: `information_schema.TABLES` of `DATABASE()`, `BASE TABLE`, excluding `seaql_migrations` and
  `change_versions` (C-11: per-branch mechanics; bumped after a restore instead). `document_counters`
  **is** included (numbering continues correctly after a restore).
- Columns: `information_schema.COLUMNS` by `ORDINAL_POSITION`, **excluding** `EXTRA LIKE '%GENERATED%'`
  (`_key`, `_live`, `open_key`, `default_key`, P2-09/P2-16/C-15); a `BLOB/BINARY/VARBINARY/BIT` column
  anywhere → `INTERNAL` (none exist; §8 test pins it).
- Dump: `SELECT CAST(<col> AS CHAR CHARACTER SET utf8mb4) … FROM <t> ORDER BY <primary key>` — exact
  text for `DECIMAL`, `UUID`, `DATETIME(3)`, `DATE`, `JSON`, `ENUM`, `TINYINT`, text; SQL `NULL` → `null`.
  Identifiers quoted with backticks from `information_schema` names only (no user input reaches SQL).
- Load order (`topo_order`): Kahn's sort over `information_schema.KEY_COLUMN_USAGE` FKs; self-references
  and cycle edges are broken by **deferring nullable FK columns** (inserted `NULL`, then one
  `UPDATE … SET col = ? WHERE <pk> = ?` per row); a cycle with no nullable edge → `INTERNAL` (test pins none).
  FK checks stay **on** (no session-variable state, same rule as 00-import D-6).
- `wipe(conn, order)`: `UPDATE <t> SET <deferred> = NULL` for every table with deferred columns, then
  `DELETE FROM` every table in **reverse** order. `load(conn, dataset, order)`: batched multi-row
  `INSERT INTO <t> (<archive columns>) VALUES (?, …)` (≤ 200 rows/statement) with string parameters,
  then the deferred `UPDATE`s. An archive column missing from the current table → `VALIDATION`
  `هذه النسخة غير متوافقة مع إصدار قاعدة البيانات الحالي` (only reachable if an upgrader is missing).
- `db_schema_version(conn)` = `100 + COUNT(*) FROM seaql_migrations`; `build_schema_version()` =
  `100 + migration::Migrator::migrations().len()` (m0001–m0015 → **115**). Values `< 100` are the mock's
  `SCHEMA_VERSION` space (`persist.ts:18` = 1), so the two formats never collide (D-3).

### 3.3 Row upgraders (`upgrade.rs`)
`ROW_UPGRADES: &[(u32 /*from*/, fn(&mut DataSetV1) -> Result<(), String>)]` — empty at baseline 115.
Restoring version `v < build` runs every upgrader `v..build` in order (the Rust analogue of the mock's
`migrations` record, `backupService.ts:316-323`). A unit test fails when `build_schema_version()` grows
without an entry for each new version (identity upgraders are allowed and must be written explicitly).

### 3.4 Settings (`auto.rs` helpers; `store.rs` from 01-settings)
- `backup_settings(conn, device)` = `{ ...DEFAULT_BACKUP_SETTINGS (autoEnabled false, autoTime "20:00",
  retention 14), ...settings.backup, folder: device.backup_folder }` (`backupService.ts:36-38`, `types/backup.ts:63-68`).
- `save_backup_settings(patch)` (`:40-44`): `next = { ...backup_settings(), ...patch }` → 01-settings
  `store::update_settings({ backup: next })` (same transaction, same device-file write/restore, same
  `تحديث إعدادات المتجر` activity row).
- `record_backup_saved(manifest, kind)` (`afterBackupSaved`, `:193-198`): `save_backup_settings` with
  `lastBackupAt = manifest.createdAt`, `lastBackupKind = kind` **and** `lastBackupFailedAt`/`lastBackupError`
  removed; then `log(Settings, kind == manual ? 'إنشاء نسخة احتياطية يدوية' : kind == auto ? 'نسخة احتياطية تلقائية' : 'نسخة احتياطية قبل الاستعادة', Some(manifest.createdAt), None)`.
- `record_backup_failed(message)` (`:390-391`): sets `lastBackupFailedAt = now`, `lastBackupError = message` — its own `with_tx`, best effort.

### 3.5 `build_backup_archive(kind, password)` — `with_read` (one REPEATABLE READ snapshot, P2-06)
Dataset dump (3.2) + `counts` (3.6) + `company = storeName || 'company'` in the **same** snapshot →
manifest (`schemaVersion = db_schema_version`, `createdAt = now`, `encrypted = password.is_some()`)
→ zip (3.1) → `BackupArchive { manifest, fileName, archiveBase64 }`. No write happens here (the file is
saved by the frontend's `saveFile` — CLAUDE.md rule 21 — then `settings_record_backup_saved` runs).

### 3.6 Counts (`counts.rs`) — parity with `tableCounts` (`backupArchive.ts:63-70`)
Fixed ordered list = the `MockDb` array keys in `blankDb()` order (`db.ts:145-215`): `users, accounts,
journalEntries, journalDrafts, journalTemplates, fiscalYears, categories, units, priceLists, products,
stockAdjustments, stockMovements, productBatches, customFieldDefs, stockCounts, debitNoteDrafts,
customers, suppliers, partyGroups, partyHistory, invoices, refunds, quotations, heldSales, shifts,
purchaseOrders, purchaseReturns, payments, taxes, paymentMethods, expenseCategories, expenses,
recurringExpenses, vouchers, cardSettlements, branches, costCenters, stockTransfers, currencies,
exchangeRates, approvalRequests, activity, audit`, then `attachments: 0` (D-5). Each maps to one
`SELECT COUNT(*)` of **live** rows (soft-delete tables `deleted_at IS NULL`; `customers`/`suppliers` =
`parties` by `kind`). `settings_preview_backup_counts` returns exactly this.

### 3.7 `run_auto_backup_if_due(trigger)` (`runAutoBackupIfDue`/`runCloseBackup`, `:374-408`)
1. `device.role != Main` and no debug DB URL → `{ ran: false, skipped: 'not-main' }` (D-S1/D-S2: every
   terminal's timer reads the shared policy, only the Main PC writes).
2. `s = backup_settings`; `!s.autoEnabled` → `skipped: 'disabled'`.
3. `schedule` only: `todayKey = cx.clock.now` **UTC** date (`toISOString().slice(0,10)`, Q-1);
   `s.lastAutoRunDate == todayKey` → `not-due`; `(local hour, minute) < autoTime` (business-clock
   timezone) → `not-due`.
4. `!s.folder` → `no-folder` (`:383`).
5. Build (3.5, `kind: auto`, no password) → write `<folder>/<fileName>` (create the folder; temp file +
   rename) → `record_backup_saved(manifest, auto)` → `schedule`: `save_backup_settings({ lastAutoRunDate: todayKey })`
   → prune: every `*.zip` in the folder whose manifest parses, newest `createdAt` first, delete beyond
   `retention` (errors ignored, `:359-372`) → `{ ran: true, path }`.
6. Any error in 5 → `record_backup_failed(message)` → `{ ran: false, error: message }` (no `Err` —
   the mock swallows, `:389-392`).

### 3.8 `preview_restore(archive)` (`:304-314`)
Read the manifest (3.1). `v = schemaVersion`:
- `v < 100` (browser archive): `v ≤ 1` → compatible, note `سيتم ترقية بيانات هذه النسخة تلقائياً إلى الإصدار الحالي عند الاستعادة`
  (it is imported through 00-import); `1 < v < 100` → incompatible, note `هذه النسخة أُنشئت بإصدار أحدث من التطبيق الحالي — يلزم تحديث التطبيق قبل الاستعادة`.
- `v ≥ 100`: `v > build` → incompatible + the same "أحدث" note; `v < build` → compatible + the "ترقية" note; equal → compatible, no note.

### 3.9 `restore_from_archive(archive, password)` (`:331-351`)
1. **Gates** (settings.md §9, decided D-8): `device.role == Main` (or debug DB URL) else `FORBIDDEN`
   `الاستعادة متاحة على الجهاز الرئيسي فقط`; LAN sharing on and `lan::connected_terminal_count > 0` →
   `CONFLICT` `أغلق البرنامج على أجهزة الكاشير أولاً — عدد الأجهزة المتصلة: <n>`.
2. Parse; `encrypted && password` empty → `VALIDATION` `هذه النسخة مشفّرة — أدخل كلمة المرور` (closes
   settings.md §8's plain-`Error` item); decrypt; **checksum mismatch** → `VALIDATION`
   `المجموع الاختباري غير مطابق — الملف قد يكون تالفاً` (D-9); incompatible (3.8) → `VALIDATION` with that note.
3. **Pre-restore backup** (settings.md §4 mitigation): `build_backup_archive(PreRestore, None)` → write to
   `<backups root>/pre-restore/<fileName>` (GB-4) → `record_backup_saved(manifest, PreRestore)` (own tx).
   Any failure → `INTERNAL` `تعذر أخذ نسخة احتياطية قبل الاستعادة — لم تتم الاستعادة`; nothing restored.
4. **One `with_tx`** (`cx.require(Settings, Write)` + `cx.require(Users, Write)`):
   - Browser archive (`v < 100`): `dataset::wipe` → `import::import_snapshot(conn, cx, { version: v,
     savedAt: manifest.createdAt, data: data.json }, ImportOpts { mode: Legacy, replace_existing: false,
     adopt_terminal: Some(this terminal) })` (00-import §3.2, which ends with `run_all`).
   - Rust archive (`v ≥ 100`): run upgraders (3.3) → `dataset::wipe` → `dataset::load` →
     `shared::invariants::run_all(conn)`; first failure → `VALIDATION`
     `تعذر الاستعادة — البيانات لا تحقق قاعدة "<doc>": <message>` (rollback — the database is untouched).
   - `log(Settings, 'استعادة من نسخة احتياطية', None, None)` with `user_id` = the actor if that id exists in
     the restored `users`, else the first active admin of the restored data (FK-safe, D-10).
   - `cx.touch(Ledger)`, `touch(Catalog)`, `touch(Parties)`.
5. After commit: `*state.session.write() = None` (restored users/passwords may differ, D-11); for a
   browser archive the importer's device fields are merged into `device-settings.json` like 00-import step 15.

### 3.10 Pre-migration backup (`pre_migration.rs`, P2-52 handoff) — no IPC
`backup_before_migrations(db: &DatabaseConnection, out_dir: &Path) -> Result<Option<PathBuf>, BackupError>`:
`applied = Migrator::get_applied_migrations`, `pending = get_pending_migrations`; either empty → `Ok(None)`
(fresh install or nothing to do). Else dump through `core::tx::with_read_on(db, …)` (GB-1) → archive
(`kind: Auto`, unencrypted — the folder is as protected as the data dir itself, P2-44) → write
`<out_dir>/pre-migration/<fileName>` → keep the newest 10 files there → `Ok(Some(path))`. No settings or
activity rows (the schema is the **old** one; entities describe the new one). Called by
`core::db::migrate` for `DeviceRole::Main` **before** `Migrator::up` (GB-2); `Err` → migrations are not
run and `DbStatus::MigrationBackupFailed` (GB-3) is set. `out_dir` = `ServerPaths::backups()` on a
managed server (GB-4), else `<app_data_dir>/backups`.

## 4. Concurrency (D8)
- Backups read one consistent snapshot (`with_read`, REPEATABLE READ READ ONLY, P2-06) — terminals keep
  writing; the archive is the state at snapshot start.
- Restore: Main PC only and refused while LAN terminals are connected (D-8); the whole replace is one
  READ COMMITTED transaction on this app's own pool. The pre-restore archive is taken just before, in a
  separate snapshot (nothing else can write in between: no terminals, and this window is modal).
- `record_backup_saved`/`save_backup_settings` take the settings row `FOR UPDATE` through 01's `store.rs`.
- Auto backup: only the Main PC writes; two triggers (schedule + close) are serialised by the settings
  row lock in `record_backup_saved`; a duplicate file name within one minute is overwritten atomically (rename).
- Pre-migration backup runs at boot before any command can be served (the pool isn't published yet).

## 5. Undo
Not undoable via the registry (settings.md §4). Restore's safety net is the automatic pre-restore
archive (§3.9 step 3), re-restorable by the user.

## 6. Frontend switch lines (`src/modules/settings/services/backupService.ts`)

- `backupSettings` becomes `async … Promise<BackupSettings>` (D-1): `if (usesRust('settings')) return backendCall('settings_backup_settings');`
  + mock body unchanged; callers get `await`: `controllers/useBackupStore.ts:25` (`settings.value = await backupService.backupSettings()`),
  and the internal `saveBackupSettings`, `runAutoBackupIfDue`, `runCloseBackup`, `pruneFileHistory`, `listHistory`.
- `saveBackupSettings`: `…('settings_save_backup_settings', { patch })`.
- `previewBackupCounts`: `…('settings_preview_backup_counts')`.
- `backupNow`: `if (usesRust('settings')) { const a = await backendCall('settings_build_backup_archive', { kind, password }); const bytes = base64ToBytes(a.archiveBase64); const path = await saveFile(bytes, { suggestedName: a.fileName, kind: 'backup', silent: true }) as string | null; if (!path) return { manifest: a.manifest, sizeBytes: bytes.length, cancelled: true }; await backendCall('settings_record_backup_saved', { manifest: a.manifest, kind }); return { manifest: a.manifest, sizeBytes: bytes.length, path }; }`
  (`opts.suggestedPath` is only ever passed by the auto helpers, which take the branch below instead).
- `runAutoBackupIfDue` / `runCloseBackup` (unwrapped internals): `if (usesRust('settings')) { const o = await backendCall('settings_run_auto_backup_if_due', { trigger: 'schedule' | 'close' }); if (o.error) log.error('settings.autoBackup', o.error); return; }`.
- `previewRestore` becomes `async` (D-1): `…('settings_preview_restore', { archiveBase64: bytesToBase64(bytes) })`;
  `RestoreBackupModal.vue:35` → `const preview = await backupService.previewRestore(bytes);` (already inside an `async` fn and `try`).
- `restoreFromArchive`: `if (usesRust('settings')) { await backendCall('settings_restore_from_archive', { archiveBase64: bytesToBase64(bytes), password }); await restoreLocalAttachments(bytes, password); return; }`
  where `restoreLocalAttachments` = the existing `parseArchive`/`decryptArchive` + `replaceAllAttachments`
  steps (`:332-347`) applied only when the archive carries `attachments/` (browser archives) — attachment
  blobs stay per machine until C-16 is decided (D-4).
- `bytesToBase64`/`base64ToBytes`: one shared pair in `settings/helpers/backupArchive.ts` (no second copy).
- `restoreFromArchive`'s mock path: the plain `Error('هذه النسخة مشفّرة — أدخل كلمة المرور')` becomes
  `new ApiError(…, 'VALIDATION')` (settings.md §8 open fix, one line, both backends now agree).

## 7. Known mock quirks (kept) and decisions

**Quirks kept:** Q-1 `lastAutoRunDate` is the UTC day while "due" uses local time (`:377-381`) — near
midnight in UTC+2/+3 the two disagree; ported as-is. Q-2 `backupNow` in the mock opens a save dialog
even for `pre-restore` and a cancel lets the restore continue — Rust never skips it (D-12). Q-3 manifest
`counts` for Rust archives are live-row counts; browser ones counted array lengths — equal by construction.

**Decisions (strictest option, logged):**
- D-1 `backupSettings` and `previewRestore` become async: a synchronous function cannot cross IPC. Two
  call sites gain `await` (a controller and a component — no page changes).
- D-2 `backupNow` splits into build (read-only, returns bytes) + record-saved (after the native Save
  dialog succeeds), so the OS dialog stays in the frontend (rule 21) and a cancelled save records nothing.
- D-3 Rust archives carry a table dump (`format: "equal-db"`, `schemaVersion ≥ 100`) instead of a
  `MockDb` object: lossless (argon2 hashes, soft-deleted rows, undo links, `sync_status`) and ids stay
  stable across a restore; browser archives (`schemaVersion 1`) restore through the D10 importer.
- D-4 No attachment blobs in Rust archives while C-16 is open; a browser archive's attachments are
  restored into this machine's local store by the existing frontend code.
- D-5 `counts` keys stay the `MockDb` names (the UI prints raw keys, `BackupSettingsPage.vue:255`) with `attachments: 0`.
- D-6 `settings_run_auto_backup_if_due` needs no session (the timer and close hook run before/after
  login, like the mock); it only ever writes to the admin-configured device folder.
- D-7 Restore requires Settings:Write **and** Users:Write (it replaces users and credentials).
- D-8 settings.md §9 "restore multi-terminal safety" decided: Main PC only, refused while LAN terminals
  are connected (no maintenance-mode flag — Part 02 §10 "later").
- D-9 Restore verifies the archive checksum first (the mock does not; same text as "verify").
- D-10 The restore activity row is attributed to a user that exists in the restored data.
- D-11 Restore ends every session on this machine; the modal already reloads the app.
- D-12 The pre-restore backup is automatic and mandatory, written to the machine backups folder.
- D-13 settings.md §9 D-S1/D-S2 adopted: frontend timer + close hook call Rust; only the Main PC runs it.
- D-14 A failed pre-migration backup blocks the migration (zero data loss over availability).

## 8. Tests

**(a) `src-tauri/tests/domain_backup.rs`:**
- `dataset_schema_is_dumpable`: no binary columns; `topo_order` covers every FK; every cycle has a nullable edge.
- round trip: seed via 00-import's demo fixture → build (plain) → wipe → load → every table's rows equal
  (text compare), `run_all` green, `document_counters` equal, `change_versions` bumped not restored.
- encrypted round trip; wrong password → text; missing password → text; flipped byte → checksum text.
- **compatibility with the TS format:** `tests/fixtures/backup-browser-plain.zip` and
  `backup-browser-encrypted.zip` (password `1234`), produced by `bun run verify:export-snapshot` through
  the real `buildBackupArchive` (00-import script, extended) → Rust reads manifests, decrypts, checksums
  match; restoring them runs the importer (counts equal the snapshot).
- preview notes for versions 1, 50, 114, 115, 116 (build = 115 in the test via the real migrator count).
- counts: key order equals the fixed list; customers/suppliers split; soft-deleted tax not counted.
- auto: terminal role → `not-main`; disabled; not due; no folder; due → file written, settings updated,
  retention prunes to N; a failing folder (read-only path) → `lastBackupError` stored, `ran: false`.
- restore: invariant-breaking archive (hand-edited dump) → rollback, DB unchanged; activity row user
  fallback when the actor isn't in the archive; session cleared.
- pre-migration: fresh DB → `None`; DB migrated to m0014 with m0015 pending → file written in
  `pre-migration/`, 11 runs keep 10 files; out-dir not writable → `Err` (the caller's status test is GB-3's).
- `upgrade_registry_covers_every_version` (unit).

**(b) Parity cases (Part 04):** `backup-settings-save` (settings row + activity rows), `backup-counts`,
`backup-restore-browser-archive` (mock `restoreFromArchive` vs Rust on the same browser archive →
identical DTOs/GL after, ids mapped), `backup-auto-run` (settings fields after a due run).

## 9. Checklist

- [x] Confirm GB-1…GB-6 (manager, before W2) and 01-settings' `store.rs` helpers are `pub`. — all six confirmed already in place (see status note above).
- [x] TS: new types in `types/backup.ts`; `bytesToBase64`/`base64ToBytes` in `backupArchive.ts`; `ApiError` fix in the mock restore path.
- [x] `archive.rs` + `crypto.rs` (format + crypto, unit-tested against the TS fixtures). — written by the previous implementer, verified complete.
- [x] `dataset.rs`, `counts.rs`, `upgrade.rs`. — written by the previous implementer, verified complete.
- [x] `auto.rs` (settings helpers, run-if-due), `restore.rs` (preview + restore), `pre_migration.rs`. — `auto.rs`/`pre_migration.rs` written by the previous implementer (this pass added `build_dataset_archive`/`build_and_write_or_read_archive` to `auto.rs` for §3.5); `restore.rs` written this pass.
- [x] `commands.rs`: 8 commands + `ipc_signatures()`; ask the manager to register them. — written this pass; registration is listed under "Needs from manager" above (outside this implementer's allowed files).
- [x] Switch lines + the two `await` call-site edits (§6).
- [x] `settings/types/contract.check.ts` entries.
- [x] `tests/domain_backup.rs` written (not run, per the no-cargo hard rule) — fixture generation in `scripts/verify/export-snapshot.ts` (§8a) and the Part 04 parity list are **not** done (outside this implementer's allowed files/scope; needs the manager or a later wave).
- [ ] Update CLAUDE.md's backup wording only via the manager (master §8 "the backup path") — CLAUDE.md's "Workflow" section does not currently name a specific "backup path" sentence to update; flagged for the manager to check against the final merged state.
- [x] Status note at the top of this file.

## Gate

`cargo check --workspace --all-targets` clean (with GB-5 crates) · `architecture_rules` passes (GI-1) ·
tests written (not run) · switch lines in place · `contract.check.ts` compiles in `bun run build` ·
`bun run memory:check` 8 commands, 0 gaps. DB tests/parity in the deferred pass; a real upgrade with a
pending migration on the managed server goes to the final testing plan.

## Part 02 gaps (manager tasks, before W2)

- **GB-1** `core::tx::with_read_on(conn: &DatabaseConnection, f)` — `with_read` without `AppState`
  (REPEATABLE READ, READ ONLY), for the boot-time pre-migration dump (rule 2: transactions only in `core/tx.rs`).
- **GB-2** `core::db::migrate` (`Main` branch): call `infrastructure::backup::pre_migration::backup_before_migrations`
  before `Migrator::up`; on `Err` return an error that `connect_and_migrate` maps to GB-3.
- **GB-3** `DbStatus::MigrationBackupFailed` + `core/status.rs` message `تعذر أخذ نسخة احتياطية قبل تحديث قاعدة البيانات — لن يتم التحديث. وفّر مساحة على القرص ثم أعد تشغيل البرنامج`
  (drives 01-settings' `ServerFailureScreen` via `BackendStatus.error`).
- **GB-4** `ServerPaths::backups()` = `database\backups\` (created by `ensure_dirs`; never under `$INSTDIR`/`$APPDATA`, P2-44).
- **GB-5** `Cargo.toml`: `zip` (deflate only), `aes-gcm`, `pbkdf2` (+ `sha2`, `hmac` as its features need),
  `sha2`, `base64` (`getrandom 0.3` is already present).
- **GB-6** `tests/architecture_rules.rs`: `infrastructure/backup/**` joins GI-1's row-mover allowlist
  (raw `DELETE`/`INSERT` on every table, `document_counters` included).
