# 21 · 01.B — `settings` contract

> **Status:** done · **Inventory:** `docs/backend/contract/settings.md`
> (regenerate with `bun run contract`) · **Mock spec:** `src/mocks/backend/branches.ts`,
> `src/mocks/backend/currency.ts`, `src/mocks/backend/revaluation.ts`, `src/mocks/backend/core.ts`
> (`logActivity`/`logAudit`/`diffFields`) · **Services:** `src/modules/settings/services/{settingsService,branchesService,backupService}.ts`
> · **Types:** `src/modules/settings/types/{index,dimensions,backup}.ts`
>
> **Note (2026-09-26):** plan 22 (invoice templates, in progress, uncommitted) adds
> `StoreSettings.printer.a4Template` / `imageTemplate` fields. `bun run contract` was re-run before
> this review and reported **no change** (309 port / 339 total), so those two fields are covered
> here as plain settings fields; nothing else in this module's surface moved.

## 1. Endpoints

| Function | Disposition (confirmed / changed + why) | Rust command | Request DTO | Response DTO | Writes (confirmed) | Shared | Undo | Notes |
|---|---|---|---|---|---|---|---|---|
| `getSettings` | port (confirmed) | `settings_get_settings` | — | `StoreSettings` | — | — | n/a (read) | Single-row read. Split into branch/device on the Rust side — see §9 D-S1. |
| `updateSettings` | port (confirmed) | `settings_update_settings` | `Partial<StoreSettings>` | `StoreSettings` | `settings`, activity, audit | activity | not undoable | Validates `storeName`, `vatNumber` (SA-shaped `3…3`, 15 digits — see §3; **note this regex is Saudi-only**, flagged as a contract issue in §8). Merges `printer` one level deep (`{ ...current.printer, ...patch.printer }`), everything else shallow-replaces. |
| `getTaxes` | port (confirmed) | `settings_get_taxes` | — | `Tax[]` | — | — | n/a (read) | |
| `saveTax` | port (confirmed) | `settings_save_tax` | `{ input: Omit<Tax,'id'>, id?: string }` | `Tax` | `taxes`, activity, audit | activity | not undoable | Create or update by optional `id`. Derives `direction`/`accountRole` from `type` (OUTPUT→sales/vatOutput, INPUT→purchase/vatInput) — never trusts client-sent `direction`/`accountRole`. Clearing another tax's `isDefault` when this one is set is a **second write inside the same mutation** the command must keep atomic. |
| `deleteTax` | port (confirmed) | `settings_delete_tax` | `{ id: string }` | — | `taxes` | — | not undoable (hard delete of master data) | Refuses if `isDefault`. **Contract gap:** mock does not check whether the tax is referenced by any posted document/line before deleting (see §8). |
| `getPaymentMethods` | port (confirmed) | `settings_get_payment_methods` | — | `PaymentMethod[]` | — | — | n/a (read) | Sorted by `sortOrder` — do the same in the SQL `ORDER BY`. |
| `savePaymentMethod` | port (confirmed) | `settings_save_payment_method` | `{ input: PaymentMethodInput, id?: string }` | `PaymentMethod` | `paymentMethods`, activity, audit | activity | not undoable | Create sets `canDelete: true` unconditionally — system-seeded presets are seeded with `canDelete: false` at setup time, never re-derived here. |
| `reorderPaymentMethods` | port (confirmed) | `settings_reorder_payment_methods` | `{ orderedIds: string[] }` | — | `paymentMethods` | — | not undoable | Sets `sortOrder = index+1` for every id present; ids not in the list keep their old order — the Rust version must match that (not a full re-sort). |
| `deletePaymentMethod` | port (confirmed) | `settings_delete_payment_method` | `{ id: string }` | — | `paymentMethods` | — | not undoable (hard delete of master data) | Refuses when `!canDelete`. **Contract gap:** no reference check against `payments`/journal lines (see §8). |
| `getBranches` | port (confirmed) | `settings_get_branches` | — | `Branch[]` | — | — | n/a (read) | |
| `createBranch` | port (confirmed) | `settings_create_branch` | `BranchInput` | `Branch` | `branches`, `accounts`, `costCenters`, activity, audit | activity | not undoable (creates dependent accounting rows) | Also creates a dedicated cash account (`accountFor('cash', branch)`) and a branch cost center in the **same operation** — three inserts that must commit together. Emits `ledger:changed` (new account added to the chart). |
| `updateBranch` | port (confirmed) | `settings_update_branch` | `{ id: string, input: Partial<BranchInput> }` | `Branch` | `branches`, `accounts` (cash-account name only, when `name` changes), activity, audit | activity | not undoable | Code-uniqueness check excludes self. Renaming the branch cascades to its cash account's display name. |
| `deactivateBranch` | port (confirmed) | `settings_deactivate_branch` | `{ id: string }` | `Branch` | `branches`, `accounts` (cash account `active=false`), activity | activity | not undoable (reversible by `reactivateBranch`, which is a distinct action, not a compensation) | Refuses if it's the only active branch, if branch stock ≠ 0 (`round2` tolerance 0.001), or if an OPEN shift exists on it. |
| `reactivateBranch` | port (confirmed) | `settings_reactivate_branch` | `{ id: string }` | `Branch` | `branches`, `accounts` (cash account `active=true`), activity | activity | not undoable | No guard conditions. |
| `getCostCenters` | port (confirmed) | `settings_get_cost_centers` | — | `CostCenter[]` | — | — | n/a (read) | |
| `createCostCenter` | port (confirmed) | `settings_create_cost_center` | `CostCenterInput` | `CostCenter` | `costCenters`, activity | activity | not undoable | Code uniqueness check. `canDelete: true` always (branch-owned cost centers are created only by `createBranch`, with `canDelete: false`). |
| `updateCostCenter` | port (confirmed) | `settings_update_cost_center` | `{ id: string, input: Partial<CostCenterInput> }` | `CostCenter` | `costCenters`, activity | activity | not undoable | Plain `Object.assign` — no field-level validation beyond existence. |
| `deleteCostCenter` | port (confirmed) | `settings_delete_cost_center` | `{ id: string }` | — | `costCenters` | — | not undoable (hard delete of master data) | Refuses when `!canDelete` (branch cost centers) or when any journal line references it (`journalEntries[].lines[].costCenterId`). |
| `getCurrencies` | port (confirmed) | `settings_get_currencies` | — | `Currency[]` | — | — | n/a (read) | |
| `getExchangeRates` | port (confirmed) | `settings_get_exchange_rates` | `{ currency?: string }` | `ExchangeRate[]` | — | — | n/a (read) | Optional filter by currency code. |
| `createCurrency` | port (confirmed) | `settings_create_currency` | `Currency` | `Currency` | `currencies` | — | not undoable | Refuses the base currency code or a duplicate code. Uppercases the code. |
| `updateCurrency` | port (confirmed) | `settings_update_currency` | `{ code: string, patch: Partial<Currency> }` | `Currency` | `currencies` | — | not undoable | Plain merge, no field validation. |
| `saveExchangeRate` | port (confirmed) | `settings_save_exchange_rate` | `ExchangeRateInput` | `ExchangeRate` | `exchangeRates` | — | not undoable | One rate per `(currency, date)` — a same-day save **overwrites** (delete-then-insert), not a history row. `rate` is accepted directly, or derived as `round2(1/inverseRate)` when only the inverse was given. Refuses non-positive/absent rate and an inactive currency. |
| `isBaseCurrencyLocked` | port (confirmed) | `settings_is_base_currency_locked` | — | `boolean` | — | — | n/a (read) | True once any journal entry exists — pure existence check on `journalEntries`, not date-scoped. |
| `setBaseCurrency` | port (confirmed) | `settings_set_base_currency` | `{ code: string }` | — | `settings.currency` | — | not undoable | Refuses once `isBaseCurrencyLocked()`. |
| `getRevaluationPreview` | port (confirmed) | `settings_get_revaluation_preview` | `{ rates: Record<string, number> }` | `FcBalanceRow[]` | — | — | n/a (read) | Pure computation over `journalEntries`/`accounts`/`customers`/`suppliers` — see §7. |
| `getDefaultRevaluationRates` | port (confirmed) | `settings_get_default_revaluation_rates` | — | `Record<string, number>` | — | — | n/a (read) | Latest rate on/before "today" per active currency, falling back to `fixedRate` for fixed currencies. |
| `postRevaluation` | port (confirmed) | `settings_post_revaluation` | `{ date: string, rates: Record<string, number> }` | `{ entryId: string, reversalEntryId: string, rows: FcBalanceRow[] }` | `journalEntries`, `counters`, activity, audit | activity, ledger, numbering, period | not undoable via the generic registry — self-reversing by construction (posts its own exact mirror entry dated the first of next month, with `allowClosedPeriod: true` on that mirror only) | Two `postJournal` calls in one logical operation; both must succeed or neither should persist. Refuses when no FC balance has `\|gainLoss\| >= 0.01`. |
| `backupSettings` | port (confirmed) | `settings_backup_settings` | — | `BackupSettings` | — | — | n/a (read) | Pure merge of `DEFAULT_BACKUP_SETTINGS` with `settings.backup` — not a separate table read once `settings` is one row (see §9 D-S1). |
| `saveBackupSettings` | port (confirmed) | `settings_save_backup_settings` | `Partial<BackupSettings>` | `BackupSettings` | `settings.backup`, activity, audit (via `updateSettings`) | activity | not undoable | Thin wrapper over `updateSettings({ backup: next })` — same transaction semantics. |
| `isTauriMode` | **frontend** (confirmed) | — | — | — | — | — | n/a | `@tauri-apps/api/core` `isTauri()` — client-only capability check, never crosses IPC. |
| `previewBackupCounts` | port (confirmed) | `settings_preview_backup_counts` | — | `Record<string, number>` | — | — | n/a (read) | Table row counts across the whole DB — a `SELECT COUNT(*)` per table in Rust, not a full snapshot read. |
| `backupNow` | port (confirmed) | `settings_backup_now` | `{ kind: BackupKind, password?: string, opts?: { suggestedPath?: string } }` | `BackupNowResult` | `settings.backup` (via `afterBackupSaved`→`saveBackupSettings`), activity, audit | activity | not undoable | **Moves to `infrastructure/backup`** (master plan §4, F10): builds an archive of the real MariaDB tables instead of the IndexedDB `MockDb` snapshot; attachments come from wherever `infrastructure` stores blobs (still TBD in Part 02). The **file save dialog / path** stays a frontend concern (`saveFile` helper, CLAUDE.md rule 21) — Rust returns bytes or writes to a path it's given, it does not own the OS dialog. Browser-mode IndexedDB history (`mock-db-backup-history`) has **no Rust equivalent** — dev/e2e only; not ported. |
| `initAutoBackup` | port (confirmed) | `settings_init_auto_backup` | — | — | `settings.backup` (indirectly, via scheduled `backupNow`) | activity | not undoable | Daily-schedule polling (`setInterval`) and the `onCloseRequested` hook are **frontend** concerns (Tauri window lifecycle) — only the "is a backup due, and doing one" logic ports to Rust as a callable command the frontend's timer/close-hook invokes. See §9 D-S2. |
| `stopAutoBackup` | **frontend** (confirmed) | — | — | — | — | — | n/a | Clears the JS interval and window listeners — no backend state. |
| `listHistory` | port (confirmed) | `settings_list_history` | — | `BackupHistoryEntry[]` | — | — | n/a (read) | Tauri-mode branch (`readDir` a configured folder, parse each `.zip`'s manifest) **stays a frontend/`plugin-fs` operation** — filesystem listing, not a DB read. Only exposed here because the current mock conflates browser-history-in-IndexedDB with file-listing; under the real backend this function has **no Rust command** — it's `frontend`. Reclassified in §8. |
| `deleteHistoryEntry` | **frontend** (confirmed) | — | — | — | — | — | n/a | `plugin-fs` file removal — no DB row. |
| `verifyHistoryEntry` | **frontend** (confirmed) | — | — | — | — | — | n/a | Re-checksums a file/IndexedDB blob — no DB access. |
| `pickRestoreFile` | **frontend** (confirmed) | — | — | — | — | — | n/a | `plugin-dialog` open-file — no DB access. |
| `previewRestore` | port (confirmed) | `settings_preview_restore` | `{ bytes: Uint8Array }` | `RestorePreview` | — | — | n/a (read) | Pure manifest-vs-`SCHEMA_VERSION` comparison — Rust needs the same schema-version constant. |
| `restoreFromArchive` | port (confirmed) | `settings_restore_from_archive` | `{ bytes: Uint8Array, password?: string }` | — | **whole DB** (every table), activity, audit | activity | not undoable by the generic registry — mitigated by an automatic `backupNow('pre-restore')` taken immediately before the overwrite | Highest-blast-radius command in this module: decrypts (if needed), runs a pre-restore backup, then replaces the entire dataset. Must run inside one transaction with a schema migration step (`migrateDb`) equivalent on the Rust side, and must **not** partially apply on failure. |
| `pickBackupFolder` | **frontend** (confirmed) | — | — | — | — | — | n/a | `plugin-dialog` open-directory — no DB access. |

## 2. DTOs → Rust

Only what differs from the generator's hint or needs a scale/enum decision.

| Type | Field | Rust type | Column | Why |
|---|---|---|---|---|
| `Tax` | `rate` | `Decimal` | `DECIMAL(9,4)` | Percentage rate, compared with `>= 0 && <= 100`; not money — scale 4 matches the mock's percentage fields elsewhere (`feePct`). |
| `Tax` | `category` | `enum TaxCategory { S, Z, E, O }` | `ENUM('S','Z','E','O')` or `CHAR(1)` | `#[serde(rename_all = "UPPERCASE")]` — literal single-letter TS union. |
| `Tax` | `type` | `enum TaxLegacyType { Output, Input }` | `ENUM('OUTPUT','INPUT')` | `#[serde(rename_all = "UPPERCASE")]`. Kept only because `salesTaxRate()`/`purchaseTaxRate()` in `backend/core.ts` still read it — do not drop even though `direction`/`accountRole` are the v2 fields. |
| `Tax` | `direction` | `enum TaxDirection { Sales, Purchase }` | `ENUM('sales','purchase')` | `#[serde(rename_all = "camelCase")]`. **Server-derived, never client-set** — the Rust command must compute it from `type`, same as the mock, and ignore any client-sent value. |
| `Tax` | `accountRole` | `Option<enum SystemAccountRole::{ VatOutput, VatInput }>` | reuse the shared `system_role` enum from `accounts` | Same derivation rule as `direction`. |
| `PaymentMethod` | `feePct` | `Decimal` | `DECIMAL(9,4)` | Percentage, `0..=100`, inert today (no settlement voucher reads it yet per the source comment) but still needs the same scale as other percentage fields for when Phase-8 card settlements start reading it. |
| `PaymentMethod` | `accountRole` | `enum SystemAccountRole { Cash, Bank, CardClearing, WalletClearing, Receivable }` | shared enum | `#[serde(rename_all = "camelCase")]`. |
| `PaymentMethod` | `type` | `enum PaymentMethodType { Cash, Card, BankTransfer, Wallet, Credit, StoreCredit }` | `ENUM(...)` | `#[serde(rename_all = "snake_case")]` — TS literals are already snake_case (`bank_transfer`, `store_credit`). |
| `PaymentMethod` | `branchOverrides` | `Vec<BranchAccountOverride>` (new struct `{ branch_id: Uuid, account_id: Uuid }`) | child table `payment_method_branch_overrides(payment_method_id, branch_id, account_id)` | Currently an inert array field; model as a child table now so Phase-9 per-branch UI doesn't need a migration later. |
| `Branch` | `id`, `cashAccountId`, `bankAccountId`, `defaultPriceListId`, `costCenterId` | `Uuid` / `Option<Uuid>` | `UUID` | Standard id fields — `id` is UUIDv7 (D2), the rest are FKs. |
| `Branch` | `nationalAddress` | `Option<Address>` (shared struct, already used by `StoreSettings`) | JSON column or normalized address columns — decide with `core`'s address type in Part 02 | Reused type, not settings-specific. |
| `CostCenter` | `budgets` | `Vec<CostCenterBudget>` (`{ fiscal_year_id: Uuid, amount: Decimal }`) | child table `cost_center_budgets(cost_center_id, fiscal_year_id, amount)` | `amount` is money → `DECIMAL(19,2)`. |
| `CostCenter` | `type` | `enum CostCenterType { Branch, Department, Project, Other }` | `ENUM(...)` | `#[serde(rename_all = "camelCase")]`. |
| `Currency` | `fixedRate` | `Option<Decimal>` | `DECIMAL(19,6)` | FX rates need more precision than money; matches `ExchangeRate.rate` below. |
| `ExchangeRate` | `rate` | `Decimal` | `DECIMAL(19,6)` | `round2` is applied only to the *derived-from-inverse* case in the mock (`currency.ts:62`) — the stored rate itself is not forced to 2dp elsewhere (e.g. `latestRate`/`toBase` multiply it against amounts that themselves get `round2`'d). Scale 6 avoids re-introducing float-like precision loss; Rust rounds the **products** of this rate (`toBase`, `convertLinesToBase`), never the rate row itself. |
| `ExchangeRateInput` | `inverseRate` | `Option<Decimal>` | not stored — request-only | Used only to derive `rate` server-side; never persisted as its own column. |
| `StoreSettings` | `printer.thermal.dpi/copies` | `i32` | `INT` | Plain integers, not decimals. |
| `StoreSettings` | `printer.a4Template` / `printer.imageTemplate` | `Option<enum A4TemplateId>` / `Option<enum ImageTemplateId>` | `ENUM(...)` or `VARCHAR` | Plan 22 fields (`src/modules/invoices/helpers/invoiceTemplates.ts`) — enum values are whatever that plan's registry defines; confirm exact literal set once plan 22 lands, since it's still in progress. Branch-level per D9. |
| `StoreSettings` | `inventoryApprovalThreshold` | `Option<Decimal>` | `DECIMAL(19,2)` | Money threshold. |
| `StoreSettings` | `pos.foreignCurrencyRate` | `Option<Decimal>` | `DECIMAL(19,6)` | FX rate, same scale reasoning as `ExchangeRate.rate`. |
| `BackupManifest` | `createdAt` | `DateTime<Utc>` | `DATETIME(3)` | Instant, not a business-local date — matches cross-cutting's "instants are UTC `DATETIME(3)`" rule (01.D). |
| `BackupManifest` | `kind` | `enum BackupKind { Manual, Auto, PreRestore }` | `ENUM('manual','auto','pre-restore')` | `#[serde(rename_all = "kebab-case")]` to match `pre-restore`'s hyphen. |
| Route fields | `link` on `ActivityEntry`/`AuditEntry` written by this module (`'/settings/branches'`, `/settings/cost-centers'`, `/settings/general'`, `/settings/taxes'`, `/settings/payment-methods'`, `/accounting/journal/${entry.id}` from `postRevaluation`) | `RouteRef { name, params }` | — | Path-string links — tracked centrally in §8 and the master 01.C task (F7), not duplicated per-field here. |

## 3. Validation and errors

| Function | Rule (source: mock check line) | Code | Exact Arabic message |
|---|---|---|---|
| `updateSettings` | `storeName` (when present) must be non-empty after trim | `VALIDATION` (default `ApiError` code) | `اسم المتجر مطلوب` |
| `updateSettings` | `vatNumber` (when present) must match `/^3\d{13}3$/` | `VALIDATION` | `الرقم الضريبي يجب أن يكون 15 رقماً يبدأ وينتهي بالرقم 3` — **Saudi-specific pattern; see §8 contract fix.** |
| `saveTax` | `name` non-empty after trim | `VALIDATION` | `اسم الضريبة مطلوب` |
| `saveTax` | `rate` in `[0, 100]` | `VALIDATION` | `النسبة يجب أن تكون بين 0 و 100` |
| `saveTax` | `category === 'E'` requires non-empty `exemptionReason` | `VALIDATION` | `سبب الإعفاء مطلوب للضرائب المعفاة` |
| `saveTax` (update path) | `id` must exist | `NOT_FOUND` | `الضريبة غير موجودة` |
| `deleteTax` | tax must exist | `NOT_FOUND` | `الضريبة غير موجودة` |
| `deleteTax` | must not be the default tax for its type | `VALIDATION` (default `ApiError` code) | `لا يمكن حذف الضريبة الافتراضية — عيّن ضريبة أخرى افتراضية أولاً` |
| `savePaymentMethod` | `name` non-empty after trim | `VALIDATION` | `اسم طريقة الدفع مطلوب` |
| `savePaymentMethod` | `feePct` in `[0, 100]` | `VALIDATION` | `نسبة العمولة يجب أن تكون بين 0 و 100` |
| `savePaymentMethod` (update path) | `id` must exist | `NOT_FOUND` | `طريقة الدفع غير موجودة` |
| `deletePaymentMethod` | method must exist | `NOT_FOUND` | `طريقة الدفع غير موجودة` |
| `deletePaymentMethod` | `canDelete` must be true | `FORBIDDEN` (default `ApiError` code, but semantically forbidden — see §8) | `لا يمكن حذف طريقة الدفع الأساسية — عطّلها بدلاً من ذلك` |
| `createBranch` | `name` non-empty after trim | `VALIDATION` | `اسم الفرع مطلوب` |
| `createBranch` | `code` non-empty after trim | `VALIDATION` | `رمز الفرع مطلوب` |
| `createBranch` | `code` unique (case-insensitive) | `VALIDATION` (default code) | `رمز الفرع مستخدم بالفعل` |
| `updateBranch` | branch must exist | `NOT_FOUND` | `الفرع غير موجود` |
| `updateBranch` | new `code` unique among other branches (case-insensitive) | `VALIDATION` (default code) | `رمز الفرع مستخدم بالفعل` |
| `deactivateBranch` | branch must exist | `NOT_FOUND` | `الفرع غير موجود` |
| `deactivateBranch` | must not be the only active branch | `VALIDATION` (default code) | `لا يمكن إلغاء تفعيل الفرع الوحيد النشط` |
| `deactivateBranch` | branch stock must be ~0 (`round2(stockLeft) > 0.001` fails) | `FORBIDDEN` | `لا يمكن إلغاء تفعيل الفرع — لا يزال يحتوي على مخزون. أنشئ تحويلاً لتفريغه أولاً` |
| `deactivateBranch` | no OPEN shift on the branch | `FORBIDDEN` | `لا يمكن إلغاء تفعيل الفرع — توجد وردية مفتوحة عليه` |
| `reactivateBranch` | branch must exist | `NOT_FOUND` | `الفرع غير موجود` |
| `createCostCenter` | `name` non-empty after trim | `VALIDATION` | `اسم مركز التكلفة مطلوب` |
| `createCostCenter` | `code` non-empty after trim | `VALIDATION` | `رمز مركز التكلفة مطلوب` |
| `createCostCenter` | `code` unique (case-insensitive) | `VALIDATION` (default code) | `رمز مركز التكلفة مستخدم بالفعل` |
| `updateCostCenter` | cost center must exist | `NOT_FOUND` | `مركز التكلفة غير موجود` |
| `deleteCostCenter` | cost center must exist | `NOT_FOUND` | `مركز التكلفة غير موجود` |
| `deleteCostCenter` | `canDelete` must be true | `FORBIDDEN` | `لا يمكن حذف مركز تكلفة الفرع` |
| `deleteCostCenter` | must not be referenced by any journal line | `FORBIDDEN` | `لا يمكن حذف مركز تكلفة له حركات مرحّلة` |
| `createCurrency` | `code` must not equal the base currency | `VALIDATION` (default code) | `هذه هي العملة الأساسية بالفعل` |
| `createCurrency` | `code` must not already exist | `VALIDATION` (default code) | `هذه العملة مضافة بالفعل` |
| `updateCurrency` | currency must exist | `NOT_FOUND` | `العملة غير موجودة` |
| `saveExchangeRate` | `currency` must be an existing (active) currency | `NOT_FOUND` | `العملة غير مفعّلة` |
| `saveExchangeRate` | resolved `rate` must be present and `> 0` | `VALIDATION` (default code) | `أدخل سعر الصرف` |
| `setBaseCurrency` | refused once `isBaseCurrencyLocked()` (any journal entry posted) | `FORBIDDEN` | `لا يمكن تغيير العملة الأساسية بعد بدء الترحيل` |
| `postRevaluation` | at least one FC balance with `\|gainLoss\| >= 0.01` at the given rates | `VALIDATION` (default code) | `لا توجد أرصدة عملات أجنبية بحاجة لإعادة تقييم بهذه الأسعار` |
| `restoreFromArchive` | encrypted archive requires a password | plain `Error` (not `ApiError`) — **contract gap, see §8** | `هذه النسخة مشفّرة — أدخل كلمة المرور` |

**Not re-checked server-side today (frontend Zod only):** none — this module has no dedicated Zod schema; all validation above is inline `ApiError` checks in the service/mock functions, which is what Rust re-implements 1:1.

## 4. Undo matrix (every function that writes)

| Function | Undoable? | Compensation (existing fn) | Refused when | Period rule (D7) |
|---|---|---|---|---|
| `updateSettings` | No | — | n/a | n/a — settings aren't period-scoped |
| `saveTax` | No | — | n/a | n/a |
| `deleteTax` | No (hard delete, master data) | — | n/a | n/a |
| `savePaymentMethod` | No | — | n/a | n/a |
| `reorderPaymentMethods` | No | — | n/a | n/a |
| `deletePaymentMethod` | No (hard delete, master data) | — | n/a | n/a |
| `createBranch` | No | — | n/a | n/a |
| `updateBranch` | No | — | n/a | n/a |
| `deactivateBranch` | No (compensating action is a distinct user-initiated call, not an "undo") | `reactivateBranch` (manual, not the undo registry) | n/a | n/a |
| `reactivateBranch` | No | — | n/a | n/a |
| `createCostCenter` | No | — | n/a | n/a |
| `updateCostCenter` | No | — | n/a | n/a |
| `deleteCostCenter` | No (hard delete, master data) | — | n/a | n/a |
| `createCurrency` | No | — | n/a | n/a |
| `updateCurrency` | No | — | n/a | n/a |
| `saveExchangeRate` | No | — | n/a | n/a |
| `setBaseCurrency` | No | — | n/a | n/a |
| `postRevaluation` | No, via the generic registry — it is **self-reversing by construction**: the function itself posts the exact mirror entry (dated first of next month) as part of the same call | n/a (built into the function, not a separate undo call) | — | The reversal mirror explicitly sets `allowClosedPeriod: true`, since by definition it posts into a future/next period; the original posting still goes through the normal `assertOpenPeriod` check |
| `saveBackupSettings` / `initAutoBackup`'s writes | No | — | n/a | n/a |
| `backupNow` | No | — | n/a | n/a |
| `restoreFromArchive` | No via the undo registry — mitigated operationally by an automatic `backupNow('pre-restore')` immediately before the overwrite, so the previous state is recoverable **by the user re-restoring that pre-restore archive**, not by an "undo" click | manual re-restore of the pre-restore backup | — | — |

No function in this module is undoable via the registry described in master plan §3 rule 7 — everything here is either a hard delete of master data (rule 7 explicitly excludes undo for that) or a settings/reference mutation with no compensating accounting operation. `postRevaluation` and `restoreFromArchive` get their own operational safety nets (self-reversal, pre-restore backup) instead.

## 5. Concurrency under D8 (several terminals on one DB)

| Race | Rows | Settled by |
|---|---|---|
| Two terminals both create a branch/cost-center/currency with the same code at once | `branches.code`, `costCenters.code`, `currencies.code` | Unique constraint on the code column (per table); the losing transaction gets a DB constraint violation, mapped to the same `VALIDATION` "code already used" `AppError` the mock throws on its pre-check. |
| Two terminals save an exchange rate for the same `(currency, date)` simultaneously | `exchangeRates` | `UNIQUE(currency, date)` constraint + `ON DUPLICATE KEY UPDATE` (or `INSERT ... ON CONFLICT` semantics) inside the transaction — last write wins, matching the mock's delete-then-insert behavior. No `SELECT … FOR UPDATE` needed since it's a pure overwrite-by-key. |
| A terminal deactivates a branch while another terminal is mid-sale/mid-shift on it | `branches.active`, `shifts`, stock rows | `deactivateBranch`'s own-branch stock and open-shift checks must run **inside the same transaction** that flips `active`, using a locking read (`SELECT … FOR UPDATE`) on the branch row and the shift check, so a shift can't open concurrently between the check and the commit. |
| Two terminals edit `StoreSettings` at once (e.g. one changes `printer`, another changes `taxes`-unrelated general fields) | `settings` (single row) | Under D8's single-shared-row model, a naive last-write-wins on the whole row would silently drop the other terminal's concurrent field. **Contract gap** — flagged in §8: `updateSettings`'s shallow-merge semantics need a row-level lock (`SELECT … FOR UPDATE`) around the read-merge-write in Rust so two concurrent partial updates don't clobber each other; the mock gets away with this because it's single-process. This is sharper once settings split into branch (shared) vs device (local) rows per §9 D-S1 — only the branch row needs this lock. |
| `postRevaluation` racing with any other poster of `journalEntries`/`counters` for the same fiscal period | `journalEntries`, `counters` (document numbering) | Same mechanism as every other ledger-posting function — covered by `shared::ledger`/`shared::numbering`'s own locking (`SELECT … FOR UPDATE` on the counter row), not specific to this module. |
| `restoreFromArchive` running while another terminal is mid-transaction on any table | **every table** | Out of scope for row-level locking — a restore must **exclusively lock the whole database** (or at minimum, refuse to run unless it can confirm no other terminal is connected) since it replaces every row. This needs a decision in 01.D/Part 02 (a maintenance-mode flag other terminals check before every command) — flagged in §8. |

## 6. Events and side effects

- **Activity/audit rows** written for: `updateSettings`, `saveTax`, `savePaymentMethod`, `createBranch`, `updateBranch` (via `logAudit` with before/after diff), `deactivateBranch`, `reactivateBranch`, `createCostCenter`, `updateCostCenter`, `postRevaluation`, `saveBackupSettings`/`backupNow`/`initAutoBackup` (via `afterBackupSaved`), `restoreFromArchive`. All use `activityKind: 'settings'` except `postRevaluation` (`'journal'`) and `restoreFromArchive`/backup calls (no explicit kind → default).
- **No `logAudit`/`logActivity` call** (silent writes): `deleteTax`, `reorderPaymentMethods`, `deletePaymentMethod`, `updateCostCenter` (wait — `updateCostCenter` *does* call `logActivity`, corrected above), `deleteCostCenter`, `createCurrency`, `updateCurrency`, `saveExchangeRate`, `setBaseCurrency`. **Contract fix candidate** (§8): several master-data mutations (currency create/update, exchange rate save, cost-center delete, payment-method delete/reorder, tax delete) have no audit trail at all today — worth flagging to the user as a possible gap to close in the mock before porting, since Rust's `shared::activity` should not have to special-case "some writes are silently unaudited" without a stated reason.
- **Events emitted:** only `createBranch` emits `ledger:changed` (new account added to the chart of accounts). No `catalog:changed` or `parties:changed` from this module.
- **Attachments:** none directly in settings services (logo/stamp/signature are inline base64 strings on `StoreSettings`, not through `AttachmentField`'s IndexedDB blob store — see the type file's comment).
- **Printing:** none.
- **Filesystem (Tauri-only, frontend side):** `backupNow`, `listHistory`, `deleteHistoryEntry`, `verifyHistoryEntry`, `pickRestoreFile`, `pickBackupFolder` all touch `@tauri-apps/plugin-fs`/`plugin-dialog` directly from the service layer (not through `saveFile` in the read/list/verify/pick cases) — these stay frontend concerns; only the byte-level archive build/restore crosses into Rust.

## 7. Aggregations (reports / analytics / dashboard / insights only)

Not primarily a reporting module, but `getRevaluationPreview` computes an aggregation:

| Output (DTO field) | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `FcBalanceRow[]` (customer/supplier rows) | `journalEntries` (flattened lines), `accounts` (to find the receivable/payable account id), `customers`/`suppliers` (name lookup) | lines where `accountId` = the receivable/payable system-role account, `partyKind` matches, `amountFc !== undefined`, `currency` set, and a rate exists in the input `rates` map for that line's currency | by `partyId` | `round2` on `fcBalance`, `baseBalance`, `revaluedBase`, `gainLoss` (`backend/revaluation.ts:46-51`) | `revaluation.ts:29-53` |
| `FcBalanceRow[]` (account rows) | `accounts` (FC-denominated, non-group), `journalEntries` (flattened lines) | accounts where `currency` is set and not a group account; lines where `accountId` matches and `amountFc !== undefined` | by `account.id` | `round2` on the same four fields (`revaluation.ts:61-65`) | `revaluation.ts:56-66` |
| Rows below `\|fcBalance\| < 0.005` are dropped entirely (not zero-filled) | — | `Math.abs(fcBalance) < 0.005` | — | — | `revaluation.ts:48`, `:63` |

`getDefaultRevaluationRates` is a lookup, not an aggregation (latest `exchangeRates` row per currency, or `fixedRate` fallback) — listed here only because it feeds the preview above.

## 8. Contract fixes needed in the mock (01.C — resolved 2026-09-27)

- [ ] **F7 (shared task, still pending — part of the shared 01.C pass, not this module alone):** this module owns 9 of the 61 path-string links: `branches.ts` (6: `/settings/branches` ×4, `/settings/cost-centers` ×2) plus `settingsService.ts:20,54,91` (`/settings/general`, `/settings/taxes`, `/settings/payment-methods`). Convert to `RouteRef` objects in the shared 01.C pass — no settings-specific behavior change needed, just the type.
- [x] **VAT-number validation is now country-aware.** `updateSettings` (`settingsService.ts`) reads `countryProfile(patch.country ?? db.settings.country).taxId.pattern`/`.label`/`.hint` instead of the hard-coded `/^3\d{13}3$/`, matching `GeneralSettingsPage.vue`'s existing client-side check. **Note:** `partyService.ts:31` (`validateCommon`) has the exact same hard-coded SA-only regex for party `vatNumber` — out of scope for this module's review (parties.md), flagged there as a carry-over of the same bug.
- [x] `deleteTax` and `savePaymentMethod`'s "can't delete" throws now pass `'FORBIDDEN'` explicitly (were the default `ApiError` code), matching `deleteCostCenter`'s pattern.
- [x] **Reference checks added.** `deleteTax` now refuses when any `invoices[].lines[].taxId` or `purchaseOrders[].lines[].taxId` references it. `deletePaymentMethod` now refuses when any `invoices[].tenders[].paymentMethodId`, `vouchers[].paymentMethodId` (receipt/payment vouchers only — `TransferVoucher`/`OwnerVoucher` have no such field, guarded with `'paymentMethodId' in v`), or `expenses[].paidFrom` (`kind: 'method'`) references it — both `FORBIDDEN`, same pattern as `deleteCostCenter`'s journal-line check.
- [x] **Audit rows added** for the destructive/financial writes that had none: `deleteTax` (`logAudit`, entity `tax`, action `delete`), `deletePaymentMethod` (`logAudit`, entity `paymentMethod`, action `delete`), `deleteCostCenter` (`logActivity`, now takes a `userId` param — `src/mocks/backend/branches.ts`), `setBaseCurrency` (`logActivity`, now takes a `userId` param — `src/mocks/backend/currency.ts`; its one other caller, `setupService.ts::applyCountryTax`, was updated to pass `session.userId`). Left as-is (not touched): `reorderPaymentMethods`, `createCurrency`, `updateCurrency`, `saveExchangeRate` — these are non-destructive/reference-data writes, not flagged as needing an audit trail.
- [ ] `restoreFromArchive`'s missing-password case still throws a plain `Error(...)`, not an `ApiError` — **not yet fixed**, carried forward as a remaining fix (low risk, dev/import-path only, does not block the gate below since it's a single well-understood line). Fix before Part 03 implements `settings_restore_from_archive`.
- [x] `listHistory` disposition overridden to `frontend` in `scripts/contract/config.ts` → `overrides['settings.listHistory']`, with the reason recorded there. `bun run contract` re-run — settings is now 35 port / 7 frontend (was 36/6); `invoices.md` also picked up two new frontend-only functions (`copyInvoiceImage`, `saveInvoiceImage`) from plan 22's in-progress `invoiceImageService.ts`, unrelated to this override — noted, not touched (out of scope for settings.md).
- [ ] Concurrency gap: `updateSettings`'s shallow-merge-without-lock behavior (§5) is safe today only because the mock is single-process. Flag for Part 02-A/F as a required `SELECT … FOR UPDATE` around the branch-settings row's read-merge-write. Not a mock fix — a Rust implementation note.
- [ ] Concurrency gap: `restoreFromArchive` has no defined behavior for "another terminal is connected" (§5) — remains an open decision, see §9.

## 9. Open questions (→ decisions in `00-MASTER-PLAN.md`)

Resolved during this review (2026-09-27, user confirmed all four): VAT regex made country-aware, `deleteTax`/`deletePaymentMethod` reference checks added, audit rows added for the destructive/financial writes, `listHistory` reclassified to `frontend`. Remaining open questions, not yet decided:

- **D-S1 (settings split, ties into master 01.D F11):** Once `settings` becomes one shared MariaDB row instead of a per-browser-tab object, `backupSettings()`/`saveBackupSettings()`'s `folder` field is genuinely per-machine (each terminal backs up to its own local/network folder) but lives inside `StoreSettings.backup`, which this file's endpoints treat as one branch-wide blob. 01.D's settings split (branch vs device) must explicitly classify `backup.folder` as **device**, and everything else under `backup` (`autoEnabled`, `autoTime`, `retention`) as **branch** policy that every terminal's auto-backup timer reads but only the Main PC (per D1, "Main PC hosts MariaDB... only the Main PC syncs") should actually *run* — otherwise every terminal in a branch would independently attempt a backup of the same DB on the same schedule. Recommend: `autoEnabled`/`autoTime`/`retention` stay branch-level policy in `settings.backup`; `initAutoBackup`'s scheduled `backupNow` call is gated to run only on the Main PC (an `AppState` flag, not a settings field). **Needs a decision.**
- **D-S2 (auto-backup scheduling ownership):** Following from D-S1 — should the daily/close-time backup *trigger* logic move entirely into Rust (a background task inside the Main PC's app, independent of any window being open), or stay a frontend-polled `setInterval` that merely *calls* a Rust command when due? The current mock's design (frontend timer + window-close hook) assumes exactly one process; multi-terminal deployment breaks that assumption regardless of which side owns the timer. Recommend: keep the frontend timer/close-hook pattern (simplest, matches "Rust ↔ Vue: a new command is called only from a service" — no new background-task infrastructure needed for Part 02), but gate `runAutoBackupIfDue`'s actual work to the Main PC only. **Needs a decision**, and it does not block porting `backupNow`/`saveBackupSettings` themselves.
- **`restoreFromArchive` multi-terminal safety** (see §5, §8): what should happen when a restore is requested while other terminals are connected to the same MariaDB? Refuse outright, force-disconnect others, or rely on operator discipline (single-branch-master assumption, D1)? **Needs a decision before Part 03 implements this command.**

## Gate

- [x] Every inventory function is in §1 with a confirmed disposition (42/42; `listHistory` overridden to `frontend`, applied and re-generated).
- [x] Every write function is in §4.
- [x] Every DTO field needing a non-default mapping is in §2.
- [x] `bun run contract:check` is green after the override (`settings.listHistory` → `frontend`, `scripts/contract/config.ts`). Also green: `bun run build`, `bun run check`, `bun run verify:mocks` (128 ok, 0 failed), `bun run memory:check` (0 new seam violations), `bun run diag:check`.
