# 21 · 03.00 — `import` (D10 snapshot importer: MockDb snapshot → MariaDB, one transaction)

> **Status:** planned 2026-09-28, not implemented. Wave **W1** (entry file §4). Depends on: Part 02
> (entities m0001–m0015, `Id`, `DocDate`, `core::auth::hash_password`, `shared::invariants::run_all`,
> `with_tx`/`with_read`), and the Part 02 gaps **GI-1…GI-4** below (manager adds them before W1).
> 02-setup (W2) embeds this file's `LegacyImportCard.vue`; 17-backup (W2) reuses `import_snapshot`
> to restore legacy (browser-made) archives.

**Goal.** One importer (`src-tauri/src/infrastructure/import/`) that turns a `MockDb` snapshot
(`src/mocks/persist.ts` `Snapshot { version, savedAt, data }`) plus the `pdf_templates_v1`
`localStorage` templates into MariaDB rows — ids remapped to UUIDv7 in array order, every FK
rewritten, credentials argon2-hashed, settings split branch/device — inside **one** `with_tx`, and
commits only if every FK holds and `shared::invariants::run_all` is all-passed (D10). It serves
D10's three jobs: (1) the shipped one-time "import from the previous version", (2) demo data in
desktop dev and on the welcome screen, (3) Part 04's parity harness (direct Rust call).

**Read first.** [`../01-frontend-analysis/cross-cutting.md`](../01-frontend-analysis/cross-cutting.md)
§9 (the D10 spec), §1 (credentials), §2 (terminal id), §3 (settings split), §7 (dates), §8 (46-table
map) · [`../02-CORE-AND-SHARED-ARCHITECTURE.md`](../02-CORE-AND-SHARED-ARCHITECTURE.md) §9 handoff
(array-order ids, `'branch-main'`, `freetext-N`, legacy terminal ids, empty currency, C-25), P2-09,
P2-15, P2-16, P2-19, P2-20, P2-38 · [`../00-MASTER-PLAN.md`](../00-MASTER-PLAN.md) §9 D10 · mock:
`src/mocks/persist.ts:13-40,156-172` (snapshot key/shape, `SCHEMA_VERSION = 1`, empty `migrations`),
`src/mocks/db.ts:32-124` (`MockDb`), `:126-141` (`DocumentKind`), `:189-214` (blank settings +
counters), `src/mocks/seed/index.ts:33-63` (demo seed), `src/mocks/backend/opening.ts:133,161,288`
(non-row source ids), `src/mocks/backend/sales.ts:266` (`freetext-${i}`),
`src/modules/invoices/controllers/usePosStore.ts:50` (the only legacy terminal id, `'pos-1'`),
`src/modules/templates/services/templateService.ts:15-34` (`pdf_templates_v1`),
`src/modules/core/services/devToolsService.ts:19-20` (`reloadDemoData`),
`src/modules/core/helpers/countryProfiles.ts:150-154` (`DEFAULT_COUNTRY`, fallback) · Rust:
`src-tauri/src/entities/**` (64 tables), `migration/src/fk_manifest_b1.md` (156 FKs),
`core/tx.rs` (`TxOpts { require_user: false }`), `core/auth.rs:hash_password`,
`core/device.rs` (`DeviceSettings`, `save`), `utils/dates.rs` (`DocDate::parse/resolve`,
`local_date_key`), `utils/money.rs` (`round2`, `round4`, `round_qty`), `shared/invariants/mod.rs:run_all`,
`tests/architecture_rules.rs` (rules 3–7).

## 1. Commands

| Mock fn | Disposition | Rust command | Args → Return | Area / Access | Tx | Events |
|---|---|---|---|---|---|---|
| — (new; `setup/services/legacyImportService.ts` `inspectLegacySnapshot`) | port (new, Tauri-only) | `setup_inspect_legacy_snapshot` | `SetupInspectLegacySnapshotArgs { snapshotJson, templatesJson? }` → `LegacySnapshotSummary` | none (D-3) | `with_read` | — |
| — (new; `legacyImportService.importLegacySnapshot`) | port (new, Tauri-only) | `setup_import_snapshot` | `SetupImportSnapshotArgs { snapshotJson, templatesJson?, templateBranchId?, mode, replaceExisting }` → `ImportSnapshotResult` | none (D-3) | `with_tx(TxOpts { require_user: false })` | touches `ledger`, `catalog`, `parties` |
| `core.reloadDemoData` (`devToolsService.ts:19`) | switch only (calls `setup_import_snapshot`, `mode: 'demo'`) | — | — | — | — | — |

2 commands. They live in `src-tauri/src/infrastructure/import/commands.rs` with their own
`pub fn ipc_signatures() -> Vec<IpcSig>` (the manager chains it into `domains::all_ipc_signatures()`
and registers both in `generate_handler!`). Command names use the `setup_` prefix because the
frontend service that calls them lives in the `setup` module (entry §3.2, `<domain>_<fn>`).

## 2. DTOs (`infrastructure/import/dto.rs`, `#[ts(export_to = "setup/types/gen/")]`)

New TS types are written **first** in `src/modules/setup/types/index.ts` (the TS type is the contract):

```ts
export type ImportMode = 'legacy' | 'demo';
export interface LegacySnapshotSummary {
  schemaVersion: number; savedAt?: string; company: string;
  counts: Record<string, number>;           // tableCounts() keys, same order (backupArchive.ts:63-70)
  branches: { id: string; name: string; code: string }[];   // snapshot ids (pre-remap), array order
  hasTemplates: boolean; targetEmpty: boolean;
}
export interface ImportSnapshotResult { counts: Record<string, number>; roundedValues: number; defaultBranchId: string }
```

| Rust DTO | TS type | Notes |
|---|---|---|
| `LegacySnapshotSummary` | `setup/types/index.ts` (new) | `counts` serialised through an **ordered** map (`Vec<(String, i64)>` with a custom `Serialize`) in `tableCounts` order (`db.ts:145-215` key order, arrays only, then `attachments: 0`) — never a `BTreeMap` (would sort keys). `#[ts(type = "Record<string, number>")]`. `savedAt` optional. |
| `LegacySnapshotBranch` | inline object type | `#[ts(export)]` as a named type is fine only if `contract.check.ts` compares the parent; use `#[ts(inline)]` on the field. |
| `ImportSnapshotResult` | new | `counts` as above (post-import row counts, same keys); `roundedValues: i64` (`#[ts(type = "number")]`); `default_branch_id: Id` `#[ts(type = "string")]`. |
| `ImportMode` | new | `#[serde(rename_all = "lowercase")]` enum `Legacy`, `Demo`. |
| `SetupInspectLegacySnapshotArgs`, `SetupImportSnapshotArgs` | args | camelCase; `snapshot_json: String`, `templates_json: Option<String>`, `template_branch_id: Option<String>` (a **snapshot** id, remapped inside), `replace_existing: bool`. The snapshot travels as a JSON **string** so ts-rs never has to type the whole `MockDb`. |

`src/modules/setup/types/contract.check.ts` (create if 02-setup hasn't yet; 02 appends its own):
`Expect<Equals<GenLegacySnapshotSummary, LegacySnapshotSummary>>`, `<GenImportSnapshotResult, ImportSnapshotResult>`, `<GenImportMode, ImportMode>`.

**Frozen reader model (`infrastructure/import/model.rs`).** Private serde structs for the
`SCHEMA_VERSION = 1` snapshot — one per `MockDb` table, camelCase, `#[serde(default)]` on every
optional TS field, unknown fields ignored. Deliberately **not** the domains' DTOs: this is the
frozen v1 file format and must keep reading old snapshots after domain DTOs evolve (D-2). Money,
qty, cost and rate fields deserialise as `serde_json::Number` → `Decimal` through the number's
shortest text (`Decimal::from_str(&n.to_string())`), never through `f64` (architecture rule 1).

## 3. Service logic

Files: `infrastructure/import/{mod,commands,dto,model,idmap,order,tables/*.rs,settings,templates,run}.rs`
(split `tables/` by entity group: `org.rs`, `catalog.rs`, `inventory.rs`, `parties.rs`, `sales.rs`,
`purchases.rs`, `payments.rs`, `expenses.rs`, `journal.rs`, `platform.rs`).

### 3.1 `inspect(conn, args) -> LegacySnapshotSummary` (`with_read`)
1. `serde_json::from_str::<SnapshotV1Envelope>` (`{ version, savedAt?, data }`, `persist.ts:20-24`) → parse
   error → `VALIDATION` `ملف البيانات غير صالح — تعذرت قراءته`.
2. `version > 1` (`persist.ts:18`) → `VALIDATION` `هذه البيانات من إصدار أحدث — حدّث البرنامج أولاً`.
   `version < 1` cannot exist (`migrations` is empty, `persist.ts:31`); treat as `1`.
3. `company = data.settings.storeName || 'company'` (`backupArchive.ts:130`); `counts` = array lengths
   in `tableCounts` order (`backupArchive.ts:63-70`); `branches` = `data.branches` (id, name, code);
   `hasTemplates` = `templatesJson` parses to a non-empty array; `targetEmpty` = step 3.2.1's check.

### 3.2 `import_snapshot(conn, cx, input: ImportInput, opts: ImportOpts) -> TxResult<ImportReport>`
`pub` — command, 17-backup (legacy archive restore) and Part 04's harness all call this one function.
`ImportOpts { mode, replace_existing, adopt_terminal: Option<Id> }`.

1. **Target must be empty** (cross-cutting §9 "Idempotent"): `SELECT (SELECT COUNT(*) FROM users) +
   (SELECT COUNT(*) FROM settings) + (SELECT COUNT(*) FROM journal_entries)`. Non-zero and
   `!replace_existing` → `CONFLICT` `قاعدة البيانات تحتوي على بيانات بالفعل — الاستيراد ممكن فقط إلى قاعدة بيانات فارغة`.
   `replace_existing` in a **release** build (`!cfg!(debug_assertions)`) → `FORBIDDEN`
   `غير مسموح في النسخة النهائية`; in a debug build → `wipe_business_rows(conn)` (§3.3).
2. Parse + version check exactly as 3.1 steps 1–2.
3. **Timezone first.** `tz = settings::country_timezone(data.settings.country)` (the same table
   01-settings' `service/country.rs` owns: `EG → Africa/Cairo`, `SA → Asia/Riyadh`, unknown/absent →
   `None` = OS timezone, P2-08). Every ISO-instant `date` in the snapshot becomes a `DocDate` with
   `day = utils::dates::local_date_key(instant, tz)` and `instant = Some(instant)`; a `YYYY-MM-DD`
   string becomes `day` only (P2-09). Unparseable → `VALIDATION` `تاريخ غير صالح في <table>`.
4. **Id map** (`idmap.rs`, `IdMap { map: HashMap<String, Id> }`, lives for this call only):
   - Pass 1 walks every table **in `MockDb` array order** and calls `assign(old_id)` = `Id::new()`
     per row, so UUIDv7s ascend in array order (handoff §9). Child rows (lines, tenders,
     allocations, movements…) get their ids in parent order, then child index order.
   - `resolve(old) -> Id` for typed FK columns: unknown key → **not** inserted into the map; the
     column is written with a fresh `Id` only if the column has no FK (polymorphic: `journal_entries.source_id`,
     `audit.entity_id`, `stock_movements.ref_id`, `product_batches.source_ref_id`,
     `payment_allocations.target_id` — B-1) via `resolve_or_mint(old)` (same old string → same new
     `Id` for the whole import). A typed FK column with an unknown key is written as-is through
     `resolve_or_mint` too and the **database** rejects it (step 12).
   - Fixed mappings: `'onboarding'` → `setup::ONBOARDING_SOURCE_ID`, `'onboarding-close'` →
     `setup::ONBOARDING_CLOSE_SOURCE_ID` (`opening.ts:133,161`; the constants are owned by 02-setup —
     until W2, define them in `infrastructure/import/idmap.rs` as `pub const` and 02-setup
     re-exports them, D-7).
   - `productId` starting with `freetext-` (`sales.ts:266`, `seed/branches9.ts:119`) or any line with
     `isFreeText: true` → `product_id NULL` (handoff §9).
   - Legacy terminal strings (`HeldSale.terminalId`, `Shift.terminalId`, `OpenShiftInput`) → one new
     `Id` each (handoff §9) — **except** `opts.adopt_terminal = Some(t)` (job 1 only) **and** exactly
     one distinct terminal string exists in the snapshot → it maps to `t` (this machine's
     `AppState.terminal.terminal_id`), so an open shift/held sale keeps working on this till (D-5).
5. **Rounding to column scale** (P2-19): every Decimal is checked against its column scale; money
   `(19,2)` → `round2`, qty → `round_qty`, cost `(19,4)` → `round4`, rate `(19,6)` and `%` `(9,4)` are
   never rounded (scale fits JS doubles already written by `round2`/`round4`). Each value that actually
   changed increments `report.rounded_values` (D-4). No other arithmetic is done — totals are copied,
   never recomputed (the mock already posted them; invariants in step 13 prove they tie).
6. **Insert rows in `order::IMPORT_ORDER`** (FK-safe, **FK checks stay ON**, D-6). The order (derived
   from `migration/src/fk_manifest_b1.md`; the test in §8 proves it covers all 156 FKs):
   `currencies, exchange_rates, branches*, accounts*, cost_centers*, fiscal_years*, cost_center_budgets,
   taxes, payment_methods, users*, credentials, units, categories*, price_lists, party_groups, parties,
   party_phones, custom_field_defs, products, product_prices, product_branch_stock, product_batches,
   settings, journal_entries*, journal_lines, journal_drafts, journal_draft_lines, journal_templates,
   invoices, invoice_lines, invoice_tenders, refunds, refund_lines, quotations, quotation_lines,
   held_sales, shifts, shift_movements, purchase_orders, purchase_order_lines, purchase_returns,
   purchase_return_lines, payments, payment_allocations, vouchers, card_settlements,
   card_settlement_groups, expense_categories, expenses, recurring_expenses, stock_adjustments,
   stock_adjustment_lines, stock_counts, stock_count_lines, stock_transfers, stock_transfer_lines,
   stock_movements, debit_note_drafts, party_history, approval_requests, print_templates, audit*, activity`.
   `*` = the table has **deferred FK columns** (`order::DEFERRED`): self-references (`accounts.parent_id`,
   `categories.parent_id`, `cost_centers.parent_id`, `audit.undo_of/undone_by` — always NULL on import),
   forward references (`branches.cash_account_id/bank_account_id/cost_center_id/default_price_list_id`,
   `cost_centers.manager_user_id`, `fiscal_years.closing_entry_id/closed_by`, `users.price_list_id`,
   `journal_entries.reversal_of_id`), each inserted `NULL` in phase A and set by one `UPDATE … WHERE id = ?`
   per row in phase B (step 11). Any further forward FK the manifest shows is added to `DEFERRED`
   (the §8 order test fails until it is).
   Per-table mapping = the entity file + the module's analysis §2 + the TS type; the transforms that
   are **not** a plain field copy:
   - `customers` + `suppliers` → one `parties` table, `kind` = source array (P2-15); `phones` → `party_phones`
     (position = index). Per-row `ActiveModel::insert` (never `insert_many`) for `products`, `parties`,
     `journal_entries`, `vouchers`, so the `before_save` hook computes `search_normalized` (phase-b note, P2-38).
   - `products.stockByBranch` → `product_branch_stock`; per-price-list overrides → `product_prices` (C-08).
   - Every child list → its child table with `position` = array index (B-1 "Child lists").
   - Soft-delete tables get `deleted_at = NULL` (the mock hard-deletes, so every snapshot row is live).
   - `created_at` = the row's own TS `createdAt` when its type has one (B-1: that column **is** the
     DTO field); otherwise `import_base + i ms` (i = array index), so `ORDER BY created_at, id`
     reproduces array order (handoff §9, D-8). `updated_at = created_at`. `sync_status = 'local'`.
   - JSON columns (`held_sales.cart`, `journal_templates.lines`, `payment_methods.branch_overrides`,
     `users.allowed_branches`, `shifts` JSON, `approval_requests` payload/link, `audit.before/after/link`,
     `activity.link`, `party_history.link`, settings sub-objects): `idmap::remap_json(value)` replaces
     every JSON **string that exactly equals a key assigned in pass 1** with the new id string — only
     row ids, never arbitrary text. Route links (`AppRoute`, already objects since 01.C) get
     `params.id` remapped the same way (cross-cutting §9).
   - `attachmentIds` string lists are copied verbatim (they key the browser's IndexedDB blob store —
     C-16 stays open; see §7 D-9).
   - `audit` rows: `action_type = NULL`, `payload = NULL`, `is_undoable = false`, `undo_of/undone_by = NULL`,
     `terminal_id = NULL` (legacy rows carry no terminal). `audit` is optional in old snapshots (`db.ts:123`).
   - `settings.theme` is ignored (C-14).
7. **Credentials** (cross-cutting §1, §9): for each `(username, password)` in `data.credentials`
   (`db.ts:34-35`), find the **imported** user with `username ===` (exact, case-sensitive — the mock's
   map key) → `credentials(user_id, password_hash = core::auth::hash_password(pw))`, hashing in
   `tokio::task::spawn_blocking`. A credential with no matching user is skipped and counted in the
   report log line (the mock could never log in with it either). A user with no credential gets no row.
8. **Settings row** (`settings.rs`, cross-cutting §3): exactly one row. Branch fields copied from
   `data.settings`; `currency` empty/absent → `countryProfile(DEFAULT_COUNTRY).currency.code` = `EGP`
   (`currency.ts:15`, handoff §9); `default_tax_id`, `accounting.defaultPurchaseAccountId`,
   `onboarding.openingEntryId/closingEntryId` remapped; `timezone` = step 3's `tz` name;
   `default_branch_id` = `resolve('branch-main')` when that id exists, else the **first** branch in
   array order (P2-20, D-10); no branch at all → `VALIDATION` `لا يوجد فرع في البيانات المستوردة`.
   Device fields (`printer.thermal`, `printer.a4PrinterName`, `printer.labelPrinterName`,
   `backup.folder`) are **not** written to the row; in `mode = legacy` they are returned in
   `ImportReport.device_fields` and the command writes them to `device-settings.json` **after**
   commit (step 15) — only the keys the device file does not already have (never overwrite this
   machine's own configuration).
9. **Print templates** (`templates.rs`, D9): `templatesJson` absent/empty → none. Branch =
   `template_branch_id` (remapped) when given, else the only branch; **more than one branch and no
   `template_branch_id`** → `VALIDATION` `اختر الفرع الذي تنتمي إليه قوالب الطباعة` (cross-cutting §9 —
   the UI asks, never guesses). Each `PdfTemplate` → `print_templates` row (fields per
   `templates.md` §2); if two defaults of one kind arrive, the first keeps `is_default` and the rest
   are cleared (the generated `default_key` unique, C-15) and counted in the log line.
10. **Counters**: `data.counters[kind]` → `shared::numbering::set_counter(conn, kind, value)` (GI-2)
    for the 15 `DocumentKind`s (`db.ts:126-141`); the two `*CodeLock` rows stay 0.
11. **Phase B**: every deferred FK column from step 6 is set (`UPDATE <table> SET <col> = ? WHERE id = ?`).
12. **FK failure mapping**: MariaDB errno 1452 anywhere in steps 6–11 → `VALIDATION`
    `تعذر الاستيراد: مرجع غير موجود في <table> — أرسل ملف التشخيص للدعم`, where `<table>` is parsed from
    the constraint name `fk_<table>_<col>` (P2-22); the raw error goes to `log::error!` (18.B).
13. **Invariants** (D10): `shared::invariants::run_all(conn)` on the same transaction; the first
    `!passed` result → `VALIDATION` `تعذر الاستيراد — البيانات لا تحقق قاعدة "<doc>": <message>`,
    every failed result logged. The whole transaction rolls back (rule 4).
14. `cx.touch(Ledger)`, `cx.touch(Catalog)`, `cx.touch(Parties)` (every category changed).
15. Return `ImportReport { counts (tableCounts keys, post-import `COUNT(*)` of live rows, customers/
    suppliers split by `parties.kind`), rounded_values, default_branch_id, device_fields }`. The
    command (not the service) then, on `Ok`: writes `device_fields` via `core::device::load` + merge +
    `save`, updates `state.device`, and logs `log::info!(target: "import", …)` with the counts.

### 3.3 `wipe_business_rows(conn)` (debug builds only)
`DELETE` every table in **reverse** `IMPORT_ORDER` after `UPDATE … SET <deferred col> = NULL`
(same `DEFERRED` list), then `set_counter(kind, 0)` for all 17 counters. `change_versions` untouched
(the touches in step 14 bump them). Guarded by `debug_assert!`-free runtime `cfg!(debug_assertions)`
check at the top — a release build returns `FORBIDDEN` before any SQL.

### 3.4 Command bodies (`commands.rs`)
- `setup_inspect_legacy_snapshot`: `with_read(&state, |tx| inspect(tx, args))`.
- `setup_import_snapshot`: role gate first — `state.device.role == Main` **or**
  `core::device::debug_db_url_override().is_some()` (dev DB), else `FORBIDDEN`
  `استيراد البيانات متاح على الجهاز الرئيسي فقط` (D-3). `adopt_terminal = (mode == Legacy).then(|| state.terminal.terminal_id)`.
  `with_tx(&state, TxOpts { require_user: false }, |tx, cx| import_snapshot(tx, cx, input.clone(), opts))`,
  then step 15's device write. The JSON string is parsed once outside the closure (the closure may
  re-run on a deadlock retry, P2-07) and the parsed model is `Arc`-shared into it.

## 4. Concurrency (D8)
Runs only against an empty database on the Main PC, before any terminal can have data to write
(D1). The empty-check (3.2.1) and the inserts share one READ COMMITTED transaction; a concurrent
second import collides on `uq_settings_singleton` / `uq_users_username` → the loser gets the step-1
`CONFLICT` text via `AppError::map_unique`. No numbering or stock locks are taken (rows are copied,
not posted; counters are set, not incremented).

## 5. Undo
Not undoable via the registry (not a business action; phase-e E-5 lists none). The only way back is
a fresh database; the legacy IndexedDB snapshot is never deleted by the importer (D-11).

## 6. Frontend switch lines and new frontend pieces

- `src/mocks/persist.ts`: new export `readPersistedSnapshot(): Promise<Snapshot | undefined>` = `idbGet(SNAPSHOT_KEY)`
  without loading it into `db` (used only by the service below — seam rule: services may import mocks).
- **New** `src/modules/setup/services/legacyImportService.ts` (all Tauri-only; outside `usesRust('setup')`
  each throws `ApiError('الاستيراد متاح في نسخة سطح المكتب فقط', 'FORBIDDEN')`):
  - `hasLegacySnapshot()` — `wrap('setup.hasLegacySnapshot')`, frontend-only: snapshot exists **and**
    `localStorage['equal.legacyImportedAt']` unset.
  - `inspectLegacySnapshot()` — `backendCall('setup_inspect_legacy_snapshot', { snapshotJson, templatesJson })`,
    where `snapshotJson = JSON.stringify(await readPersistedSnapshot())`, `templatesJson = localStorage.getItem('pdf_templates_v1') ?? undefined`.
  - `importLegacySnapshot(templateBranchId?: string)` — `backendCall('setup_import_snapshot', { snapshotJson, templatesJson, templateBranchId, mode: 'legacy', replaceExisting: false })`;
    on success sets `localStorage['equal.legacyImportedAt']` and calls 02-setup's `refreshDeviceSetupState()`.
- `src/modules/core/services/devToolsService.ts` `reloadDemoData` (first line inside `wrap`):
  `if (usesRust('setup')) { seedDatabase(); await backendCall('setup_import_snapshot', { snapshotJson: JSON.stringify({ version: SCHEMA_VERSION, savedAt: new Date().toISOString(), data: db }), mode: 'demo', replaceExisting: import.meta.env.DEV }); await refreshDeviceSetupState(); return; }`
  (`seedDatabase()` only builds the snapshot in memory — the mock is not the backend in this mode).
- **New** `src/modules/setup/components/LegacyImportCard.vue` (embedded by 02-setup's device page):
  `inspectLegacySnapshot()` → shows `company` + non-zero counts; an `AppSelect` of `branches` only when
  `branches.length > 1 && hasTemplates`; primary button «استيراد بياناتك من الإصدار السابق»; the C-25
  notice as `text-caption`: «يتم نقل بياناتك إلى قاعدة البيانات الجديدة على هذا الجهاز. لا تُلغِ تثبيت
  البرنامج قبل انتهاء النقل.»; errors via `errorMessage(err)`. Added to `/dev/ui` + `docs/design_system.md`
  only if it becomes shared (it is setup-local — rule 3 does not apply).

## 7. Known mock quirks (kept) and decisions

**Quirks kept:** Q-1 snapshot money that escaped `round2` (float noise) is rounded to column scale and
counted, not rejected (D-4). Q-2 array order vs `createdAt` order may differ in backdated seed data;
the importer keeps `createdAt` (it is a DTO field) and Part 04 surfaces any list-order diff as a
parity case (D-8). Q-3 credential keys are matched case-sensitively, like the mock's map.

**Decisions (strictest option, logged):**
- D-1 One importer for all three jobs; the Part 04 harness calls `import_snapshot` directly (no IPC).
- D-2 A frozen private v1 reader model instead of domain DTOs (format stability; see §2).
- D-3 No session is required (the target DB is empty — nobody can log in yet); only the Main PC (or
  the debug DB URL) may import, so a terminal never pushes its browser data into the branch DB.
- D-4 Values beyond column scale are rounded with the one rounding rule and reported (zero silent change).
- D-5 Job 1 adopts the single legacy terminal id (`'pos-1'`) as this machine's terminal id so an open
  shift/held sale survives the upgrade; jobs 2/3 follow the handoff (one new `Id` per string).
- D-6 FK checks stay **on**; cycles are broken by the explicit `DEFERRED` list (no
  `foreign_key_checks = 0` session state that could leak into the pool).
- D-7 `'onboarding'`/`'onboarding-close'` source ids map to fixed UUID constants shared with 02-setup,
  so imported and Rust-posted opening entries use the same source id (parity maps 1:1).
- D-8 `created_at` = TS `createdAt` when present, else a monotonic `import_base + i ms`.
- D-9 Attachment blobs are not imported (C-16 open); ids are kept so the local blob store still resolves them.
- D-10 `default_branch_id` = remapped `'branch-main'`, else the first branch (P2-20).
- D-11 The legacy IndexedDB snapshot is never deleted (zero data loss); `equal.legacyImportedAt` only
  hides the offer.
- D-12 Device fields are merged into `device-settings.json` only where the file has no value.

## 8. Tests

**(a) `src-tauri/tests/domain_import.rs`** (DB-backed, `TestDb::fresh()`; fixtures below):
- `import_order_covers_every_fk`: reads `information_schema.KEY_COLUMN_USAGE` of a migrated DB; every
  FK `(t.c → r)` has `index(r) < index(t)` in `IMPORT_ORDER` or `(t, c) ∈ DEFERRED` (and `c` nullable).
- Demo snapshot (fixture `tests/fixtures/mock-snapshot-demo.json`, written by the new
  `bun run verify:export-snapshot` = `seedDatabase(new Date('2026-06-30T09:00:00Z'), 'SA')` →
  `{ version: 1, savedAt, data }`; gitignored; the test fails with a clear message when it's missing —
  never skips): imports, `run_all` all passed, `counts` equal the snapshot's `tableCounts` (customers/
  suppliers split), `document_counters` equal `data.counters`, ids ascend in array order per table,
  `ORDER BY created_at, id` of `journal_entries` equals array order.
- Second import into the now non-empty DB → exact `CONFLICT` text; `replace_existing` in a debug test
  build wipes and re-imports; (release behaviour covered by a unit test on the guard fn).
- Credentials: `admin`'s hash verifies `admin123` via `core::auth::verify_password`; stored hash starts `$argon2id$`.
- Small hand-written fixture `tests/fixtures/mock-snapshot-edge.json` (checked in): `freetext-1` line →
  `product_id NULL`; `'onboarding'` source → `ONBOARDING_SOURCE_ID`; one `'pos-1'` shift with
  `adopt_terminal` → that terminal id; without → a fresh id shared by its held sale; empty
  `settings.currency` → `EGP`; `theme` ignored; audit row `is_undoable = false`; JSON `cart.productId` remapped.
- Dangling `invoice.customerId` → `VALIDATION` text naming `invoices`, nothing committed.
- A snapshot whose GL does not tie (hand-broken journal line) → invariant `VALIDATION`, nothing committed.
- Two branches + templates, no `template_branch_id` → the branch-picker `VALIDATION`; with it → rows on that branch.
- Two defaults of one template kind → only the first stays default.
- `version: 2` → newer-version `VALIDATION`.

**(b) Parity cases (Part 04):** the harness's base state for **every** domain case is this importer
applied to the same snapshot on both backends; plus `import-demo-sa`, `import-demo-eg` (full seed,
compare every list DTO after import, ids mapped) and `import-edge` (the edge fixture).

## 9. Checklist

- [ ] Confirm GI-1…GI-4 are in place (manager, before W1).
- [ ] TS types in `setup/types/index.ts` (§2) first.
- [ ] `infrastructure/import/mod.rs` (`pub mod commands; dto; model; idmap; order; tables; settings; templates; run;` + `ipc_signatures()`).
- [ ] `model.rs` — v1 reader structs for the 43 array tables + `settings` + `counters` + `credentials`.
- [ ] `idmap.rs` — `assign`, `resolve`, `resolve_or_mint`, `remap_json`, fixed mappings, terminal rule.
- [ ] `order.rs` — `IMPORT_ORDER`, `DEFERRED`.
- [ ] `tables/*.rs` — one `insert_<table>` per entity, transforms per §3.2.6, rounding per §3.2.5.
- [ ] `settings.rs` (§3.2.8, uses 01-settings `country_timezone`), `templates.rs` (§3.2.9).
- [ ] `run.rs` — `inspect`, `import_snapshot` (steps 1–15 in order, each citing its source), `wipe_business_rows`.
- [ ] `commands.rs` — 2 commands (§3.4) + the post-commit device write.
- [ ] `persist.ts` `readPersistedSnapshot`; `legacyImportService.ts`; `devToolsService.ts` switch; `LegacyImportCard.vue`.
- [ ] `scripts/verify/export-snapshot.ts` + `package.json` `verify:export-snapshot`; gitignore the fixture.
- [ ] `setup/types/contract.check.ts` entries.
- [ ] `tests/domain_import.rs` + `tests/fixtures/mock-snapshot-edge.json` (§8a); parity list (§8b) handed to Part 04.
- [ ] Ask the manager: register 2 commands, chain `ipc_signatures()`, run `bun run memory` (new service file, component, script).
- [ ] Status note at the top of this file.

## Gate

`cargo check --workspace --all-targets` clean (manager's throttled run; `architecture_rules` passes
with GI-1) · tests written (not run) · switch line + new service in place · `contract.check.ts`
compiles in `bun run build` · `bun run memory:check` shows 2 commands, 0 contract gaps. DB tests and
parity cases run in the deferred, time-boxed pass.

## Part 02 gaps (manager tasks, before W1)

- **GI-1** `tests/architecture_rules.rs` rules 3–7: allow `src/infrastructure/import/**` and
  `src/infrastructure/backup/**` ("verbatim row movers": they copy already-posted history; integrity is
  proven by DB FKs + `run_all` before commit). Rule 1 (no `f32/f64`) and rule 2 (tx only in `core/tx.rs`) still apply.
- **GI-2** `shared::numbering::set_counter(conn, kind: DocumentKind, value: i64)` (`UPDATE document_counters
  SET value = ? WHERE kind = ?`) — the only sanctioned non-increment write (keeps rule 6's counter check).
- **GI-3** `Cargo.toml`: none new for this file (argon2, serde_json, uuid v7 present). Verify `uuid ≥ 1.9`
  (monotonic `now_v7` within a process) — the §8 ascending-id assertion depends on it.
- **GI-4** `domains::all_ipc_signatures()` chains `infrastructure::import::commands::ipc_signatures()`
  (and later `infrastructure::backup::commands::ipc_signatures()`, 17-backup).
