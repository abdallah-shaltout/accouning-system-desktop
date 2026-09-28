# 21 · 03.01 — `settings` (store settings with the branch/device split, taxes, payment methods, branches, cost centers, currencies, revaluation, LAN sharing + server-failure screen)

> **Status (2026-09-28): code complete, not yet compiled/tested.** All 33 commands, DTOs, service
> logic, frontend switch lines, `NetworkSettingsPage.vue`/`useBackendHealth.ts`/
> `ServerFailureScreen.vue`, `contract.check.ts` and `tests/domain_settings.rs` are written per
> this spec. Not run through `cargo check` (manager's throttled pass) or any `bun run` gate yet —
> see "Needs from manager" in this wave's final report. Test/gate items are ⏳ deferred to the
> time-boxed test pass (entry file §3.6). Depends on: 00-import (only for its tests' fixtures),
> Part 02 (`core::settings::{load, load_shared_locked}`, `core::device`, `shared::{ledger,
> activity}`, `infrastructure::database::{lan, pairing, credentials, supervisor}`), and gaps
> **GS-1…GS-3** below (confirmed already fixed per `_part02-gaps.md`, not re-verified by this
> implementer). `backupService.ts` is **not** here — it is 17-backup.

**Goal.** Port the 28 `port` functions of `settingsService.ts` + `branchesService.ts` to
`domains/settings/`, with `settings_get_settings`/`settings_update_settings` merging and splitting the
device-scoped fields (`device-settings.json`) exactly as cross-cutting §3 specifies, so pages keep one
`StoreSettings` shape. Add the Part 02 handoff UI: the LAN-sharing toggle, the pairing-code display and
the server-failure screen driven by `BackendStatus.server` (5 new commands). Remove the dead
`StoreSettings.theme` (C-14).

**Read first.** [`../01-frontend-analysis/settings.md`](../01-frontend-analysis/settings.md) §1–§9 ·
[`../01-frontend-analysis/cross-cutting.md`](../01-frontend-analysis/cross-cutting.md) §3 (field table,
device store), §7 (timezone) · [`../02-CORE-AND-SHARED-ARCHITECTURE.md`](../02-CORE-AND-SHARED-ARCHITECTURE.md)
C-14, P2-16, P2-20, P2-22, P2-47, P2-54, P2-56–P2-58, §9 · [`../02-core-and-shared/phase-a3-main-pc-hosting.md`](../02-core-and-shared/phase-a3-main-pc-hosting.md)
A3-1/A3-2 · mock: `src/modules/settings/services/settingsService.ts:9-122`,
`branchesService.ts:16-115`, `src/mocks/backend/branches.ts:42-261`, `currency.ts:14-59`,
`revaluation.ts:26-139`, `core.ts:352-364` (`logActivity`) · types: `src/modules/settings/types/index.ts:21-218`,
`types/dimensions.ts:13-99` · `src/modules/core/helpers/countryProfiles.ts:31-154` · Rust:
`core/settings.rs`, `core/device.rs`, `core/status.rs`, `core/dto.rs` (`BackendStatus`),
`entities/org/{settings,taxes,payment_methods,branches,cost_centers,cost_center_budgets,currencies,exchange_rates,accounts}.rs`,
`entities/values.rs` (`PrinterSettings`, `BackupPolicy`, `OnboardingState`, …),
`infrastructure/database/{lan,pairing,credentials,supervisor,errors,payload}.rs`.

## 1. Commands

Reads: `let actor = state.session.read().unwrap().clone();` then `with_read`; "session" = actor must be
`Some` else `UNAUTHORIZED` `سجّل الدخول أولاً` (every role reads these: `MoneyText`, POS, invoices — D-2).

| Mock fn | Disposition | Rust command | Args → Return | Area / Access | Tx | Events |
|---|---|---|---|---|---|---|
| `getSettings` | port | `settings_get_settings` | `()` → `StoreSettings` | session | `with_read` | — |
| `updateSettings` | port | `settings_update_settings` | `{ patch: StoreSettingsPatch }` → `StoreSettings` | Settings:Write (+ Users:Write when `roleAccessOverrides` present, D-3) | `with_tx` | — |
| `getTaxes` | port | `settings_get_taxes` | `()` → `Vec<Tax>` | session | `with_read` | — |
| `saveTax` | port | `settings_save_tax` | `{ input: TaxInput, id? }` → `Tax` | Settings:Write | `with_tx` | — |
| `deleteTax` | port | `settings_delete_tax` | `{ id }` → `()` | Settings:Write | `with_tx` | — |
| `getPaymentMethods` | port | `settings_get_payment_methods` | `()` → `Vec<PaymentMethod>` | session | `with_read` | — |
| `savePaymentMethod` | port | `settings_save_payment_method` | `{ input: PaymentMethodInput, id? }` → `PaymentMethod` | Settings:Write | `with_tx` | — |
| `reorderPaymentMethods` | port | `settings_reorder_payment_methods` | `{ orderedIds }` → `()` | Settings:Write | `with_tx` | — |
| `deletePaymentMethod` | port | `settings_delete_payment_method` | `{ id }` → `()` | Settings:Write | `with_tx` | — |
| `getBranches` | port | `settings_get_branches` | `()` → `Vec<Branch>` | session | `with_read` | — |
| `createBranch` | port | `settings_create_branch` | `{ input: BranchInput }` → `Branch` | Settings:Write | `with_tx` | `ledger` |
| `updateBranch` | port | `settings_update_branch` | `{ id, input: BranchPatch }` → `Branch` | Settings:Write | `with_tx` | — |
| `deactivateBranch` | port | `settings_deactivate_branch` | `{ id }` → `Branch` | Settings:Write | `with_tx` | — |
| `reactivateBranch` | port | `settings_reactivate_branch` | `{ id }` → `Branch` | Settings:Write | `with_tx` | — |
| `getCostCenters` | port | `settings_get_cost_centers` | `()` → `Vec<CostCenter>` | session | `with_read` | — |
| `createCostCenter` | port | `settings_create_cost_center` | `{ input: CostCenterInput }` → `CostCenter` | Settings:Write | `with_tx` | — |
| `updateCostCenter` | port | `settings_update_cost_center` | `{ id, input: CostCenterPatch }` → `CostCenter` | Settings:Write | `with_tx` | — |
| `deleteCostCenter` | port | `settings_delete_cost_center` | `{ id }` → `()` | Settings:Write | `with_tx` | — |
| `getCurrencies` | port | `settings_get_currencies` | `()` → `Vec<Currency>` | session | `with_read` | — |
| `getExchangeRates` | port | `settings_get_exchange_rates` | `{ currency? }` → `Vec<ExchangeRate>` | session | `with_read` | — |
| `createCurrency` | port | `settings_create_currency` | `{ input: Currency }` → `Currency` | Settings:Write | `with_tx` | — |
| `updateCurrency` | port | `settings_update_currency` | `{ code, patch: CurrencyPatch }` → `Currency` | Settings:Write | `with_tx` | — |
| `saveExchangeRate` | port | `settings_save_exchange_rate` | `{ input: ExchangeRateInput }` → `ExchangeRate` | Settings:Write | `with_tx` | — |
| `isBaseCurrencyLocked` | port | `settings_is_base_currency_locked` | `()` → `bool` | session | `with_read` | — |
| `setBaseCurrency` | port | `settings_set_base_currency` | `{ code }` → `()` | Settings:Write | `with_tx` | — |
| `getRevaluationPreview` | port | `settings_get_revaluation_preview` | `{ rates }` → `Vec<FcBalanceRow>` | Settings:Read | `with_read` | — |
| `getDefaultRevaluationRates` | port | `settings_get_default_revaluation_rates` | `()` → `OrderedNumberMap` | Settings:Read | `with_read` | — |
| `postRevaluation` | port | `settings_post_revaluation` | `{ date, rates }` → `RevaluationResult` | Settings:Write | `with_tx` | `ledger` (+`parties`, via `ledger::post`, P2-12) |
| — new (`networkService.getLanSharingStatus`) | port (new, Tauri) | `settings_get_lan_sharing_status` | `()` → `LanSharingStatus` | Settings:Read | `with_read` for the access check only; the rest reads `AppState`, `server.json`, keyring | — |
| — new (`networkService.enableLanSharing`) | port (new) | `settings_enable_lan_sharing` | `()` → `PairingInfo` | Settings:Write, role main | `with_read` access check → infrastructure call (no tx) → one `with_tx` for the activity row | — |
| — new (`networkService.disableLanSharing`) | port (new) | `settings_disable_lan_sharing` | `{ confirmDisconnect }` → `LanSharingStatus` | Settings:Write, role main | same as enable | — |
| — new (`networkService.rotatePairingCode`) | port (new) | `settings_rotate_pairing_code` | `()` → `PairingInfo` | Settings:Write, role main | same as enable | — |
| — new (`networkService.reconnectBackend`) | port (new) | `settings_reconnect_backend` | `()` → `()` | **none** (D-9) | none | — |

**33 commands.** `stay-frontend`/other-file functions are untouched here: `backupService.ts`'s 13
functions are 17-backup; `isTauriMode`, `listHistory`, `deleteHistoryEntry`, `verifyHistoryEntry`,
`pickRestoreFile`, `pickBackupFolder`, `stopAutoBackup` stay frontend (settings.md §1).

## 2. DTOs (`domains/settings/dto.rs`, `#[ts(export_to = "settings/types/gen/")]`)

| Rust DTO | TS type | Notes |
|---|---|---|
| `StoreSettings` | `types/index.ts:97-218` (**after** removing `theme`, C-14) | Optional TS fields = `Option` + `#[ts(optional)]` + `skip_serializing_if` (absent, never `null`). `country`: `#[ts(type = "import('@/modules/core/helpers/countryProfiles').CountryCode")]`; `nationalAddress`: Address type path; `printer.a4Template/imageTemplate`: `#[ts(type = …A4TemplateId/ImageTemplateId import)]` (plan-22 unions, not enumerated in Rust — stored as `String`); `roleAccessOverrides`: `#[ts(type = "import('@/modules/users/helpers/permissions').RoleAccessOverrides")]`; `insightThresholds`: `#[ts(type = "Partial<import('@/modules/core/services/insightTypes').InsightThresholds>")]`; `backup`: `#[ts(type = "import('./backup').BackupSettings")]` (17-backup owns that DTO; the field is typed by path so W1 compiles alone). Money/rate numbers (`inventoryApprovalThreshold`, `pos.foreignCurrencyRate`) `serde_number` + `#[ts(type = "number")]`. Dates: `accounting.lockDate` `YYYY-MM-DD`; `onboarding.finishedAt` = `format_iso_ms`. |
| `StoreSettingsPatch` | `Partial<StoreSettings>` | Every top-level key `Option`; `printer: Option<PrinterPatch>` (all inner keys optional). Absent key = unchanged (JSON drops `undefined`). |
| `Tax`, `TaxInput` | `Tax` (`:21-37`), `Omit<Tax,'id'>` | `rate: Decimal` `serde_number`; `category` enum `S/Z/E/O`; `type` enum `OUTPUT/INPUT` (`#[serde(rename = "type")] kind`); `direction`, `accountRole` enums; `exemptionReason` optional. Client `direction`/`accountRole` are **ignored** (server-derived). |
| `PaymentMethod`, `PaymentMethodInput` | `:48-70` | `type` snake_case enum; `accountRole` camelCase enum; `feePct` `serde_number`; `branchOverrides?: {branchId, accountId}[]`; `sortOrder: i32` (column `i16`). |
| `Branch`, `BranchInput`, `BranchPatch` | `dimensions.ts:13-37` | `createdAt` = `format_iso_ms(created_at)` (B-1: the column is the DTO field). `BranchPatch` = `Partial<BranchInput>`. |
| `CostCenter`, `CostCenterBudget`, `CostCenterInput`, `CostCenterPatch` | `dimensions.ts:43-65` | `type` enum (`#[serde(rename = "type")]`, column `kind`); `budgets` from `cost_center_budgets` (position order); `amount` `serde_number`. |
| `Currency`, `CurrencyPatch`, `ExchangeRate`, `ExchangeRateInput` | `dimensions.ts:71-99` | `fixedRate`, `rate`, `inverseRate` `serde_number`; `ExchangeRate.date` = `YYYY-MM-DD`. |
| `FcBalanceRow` | `revaluation.ts:13-23` → **move the interface to `settings/types/index.ts`** (a service-returned type must live in `types/`, CLAUDE.md architecture; `revaluation.ts` re-imports it) | `kind` enum `customer/supplier/account`; four `serde_number` money fields. |
| `RevaluationResult` | new `settings/types/index.ts`: `{ entryId: string; reversalEntryId: string; rows: FcBalanceRow[] }` (the inline return type of `postRevaluation`, `branchesService.ts:112`) | the service's annotated return type becomes `Promise<RevaluationResult>`. |
| `OrderedNumberMap` | `Record<string, number>` | ordered `Vec<(String, Decimal)>` + custom `Serialize` (currency order), `#[ts(type = "Record<string, number>")]`. Input `rates: Record<string, number>` deserialises into `BTreeMap<String, Decimal>` (order irrelevant on input). |
| `LanSharingStatus`, `PairingInfo` | **new** `settings/types/network.ts`: `PairingInfo { hostName; addresses: string[]; port: number; code: string }`, `LanSharingStatus { role: 'main' \| 'terminal'; provisioned: boolean; lanSharing: boolean; connectedTerminals: number; pairing?: PairingInfo; mainHost?: string }` | `PairingInfo` DTO is a domain struct built from `infrastructure::database::pairing::PairingInfo` (that one is not `TS`). `port: u16` `#[ts(type = "number")]`. |
| Args structs | — | `SettingsUpdateSettingsArgs { patch }`, `SettingsSaveTaxArgs { input, id: Option<String> }`, … one per command, camelCase; no-arg commands use `ipc_sig!(cmd, (), T)`. |

`src/modules/settings/types/contract.check.ts` (new): `Equals` for `StoreSettings`, `Tax`, `PaymentMethod`,
`PaymentMethodInput`, `Branch`, `BranchInput`, `CostCenter`, `CostCenterInput`, `CostCenterBudget`,
`Currency`, `ExchangeRate`, `ExchangeRateInput`, `FcBalanceRow`, `RevaluationResult`, `PairingInfo`,
`LanSharingStatus`. `StoreSettingsPatch` vs `Partial<StoreSettings>`: if `Equals` fails only because
ts-rs cannot express the nested `Partial` of `printer`, keep the check on `StoreSettings` and add
`// contract-ok: request-only Partial, validated field-by-field in Rust` (entry §3.4).

## 3. Service logic (`domains/settings/service/{store,taxes,payment_methods,branches,cost_centers,currency,revaluation,network,country}.rs`)

Common: every activity row goes through `shared::activity::log(conn, cx, &state.undo, kind, message, date, link)`
(= `logActivity`, `core.ts:352`) or `record(...)` (= `logAudit`); `date: None` means `cx.clock.now`.
Lists keep mock array order = `ORDER BY created_at, id` (entry §3.3); soft-delete tables read `find_live()`.

**`country.rs`** (pub; 00-import and 02-setup reuse it): `CountryProfile { code, currency_code, vat_rate,
vat_label, prices_include_tax_default, tax_id_label, tax_id_hint, tax_id_valid: fn(&str) -> bool, timezone }`
for `EG` (`EGP`, 14, `ضريبة القيمة المضافة 14%`, `true`, `رقم التسجيل الضريبي`, `9 أرقام`, `^\d{9}$`,
`Africa/Cairo`) and `SA` (`SAR`, 15, `ضريبة القيمة المضافة 15%`, `true`, `الرقم الضريبي`,
`15 رقماً يبدأ وينتهي بالرقم 3`, `^3\d{13}3$`, `Asia/Riyadh`) — values copied from
`countryProfiles.ts:78-146`; validators hand-written (no `regex` crate). `country_profile(code: Option<&str>)`
falls back to `EG` exactly like `countryProfile()` (`:152-154`). `country_timezone(code)` → `Option<&'static str>`.

**`get_settings(conn, device: &DeviceSettings) -> StoreSettings`** (`settingsService.ts:9-12`):
`core::settings::load` → DTO; then **merge device fields** (cross-cutting §3): `printer.thermal`,
`printer.a4PrinterName`, `printer.labelPrinterName` from `device.printer`; `backup.folder` from
`device.backup_folder` (if the row has no `backup` but the device has a folder → `backup = DEFAULT_BACKUP_SETTINGS + folder`).
The row's own `printer` JSON device keys are ignored (never written, D-4).

**`update_settings(conn, cx, reg, patch, device) -> (DeviceSettings /*new*/, ())`** (`:14-26`):
1. `patch.storeName` present and `trim()` empty → `VALIDATION` `اسم المتجر مطلوب` (`:16`).
2. `patch.vatNumber` truthy → `p = country_profile(patch.country.or(row.country))` (row read unlocked
   first); `!p.tax_id_valid(v)` → `VALIDATION` `` `${label} يجب أن يكون ${hint}` `` (`:17-21`).
3. `load_shared_locked` (settings row `FOR UPDATE`, first in the global lock order).
4. Shallow merge in key order of `StoreSettings`: each present top-level key replaces the column;
   `printer` merges one level (`{ ...current.printer, ...patch.printer }`, `:23`) — **device keys**
   (`thermal`, `a4PrinterName`, `labelPrinterName`) go to the new `DeviceSettings.printer` instead;
   `backup` (whole object) → row `backup` without `folder`, and `folder` (present or absent in the new
   object) → `device.backup_folder` (the mock replaces the object wholesale, so an absent `folder` clears it).
   `country` changed → also set `timezone = country_timezone(new)` (D-5; Rust-only column).
5. Update the row (`updated_at = cx.clock.now`).
6. `log(Settings, 'تحديث إعدادات المتجر', None, RouteRef::list("settings-general"))` (`:24`).
7. Command: keeps the old `DeviceSettings` in memory, computes the new one from the patch (a pure
   function run before the transaction), saves it with `core::device::save`, runs `with_tx`; on `Err`
   it saves the old one back and leaves `state.device` untouched; on `Ok` it replaces `state.device`
   (D-6). Returns `get_settings` of the committed state.

**Taxes** (`settingsService.ts:28-71`):
- `save_tax`: validations in order `اسم الضريبة مطلوب` (name trim), `النسبة يجب أن تكون بين 0 و 100`
  (`0 ≤ rate ≤ 100`), `سبب الإعفاء مطلوب للضرائب المعفاة` (`E` + trimmed reason empty) — all `VALIDATION`
  (`:35-39`); `direction`/`account_role` derived from `type` (`:41-42`). Then `isDefault` → `UPDATE taxes
  SET is_default = 0 WHERE type = ? AND deleted_at IS NULL` (`:46`); then update (`FOR UPDATE` by id,
  missing → `NOT_FOUND` `الضريبة غير موجودة`, `:50`) or insert; `exemption_reason = E ? trim : NULL`;
  `name` is stored **untrimmed** (Q-2). `log(Settings, "حفظ الضريبة \"<name>\"", …, settings-taxes)` (`:58`).
- `delete_tax`: live row `FOR UPDATE` → `NOT_FOUND`; `is_default` → `FORBIDDEN`
  `لا يمكن حذف الضريبة الافتراضية — عيّن ضريبة أخرى افتراضية أولاً`; `EXISTS invoice_lines.tax_id = ?` or
  `purchase_order_lines.tax_id = ?` → `FORBIDDEN` `لا يمكن حذف ضريبة مستخدمة في مستندات مرحّلة` (`:64-68`);
  soft delete (P2-16); `record(entity 'tax', action Delete, label name, message "حذف الضريبة \"<name>\"",
  link settings-taxes, activity_kind Settings)` (`:70`).

**Payment methods** (`:77-122`): list `ORDER BY sort_order, created_at, id` (stable JS sort, `:79`).
`save_payment_method`: `اسم طريقة الدفع مطلوب`, `نسبة العمولة يجب أن تكون بين 0 و 100`, then update
(`FOR UPDATE`, missing → `NOT_FOUND` `طريقة الدفع غير موجودة`, present keys only — `Object.assign`) or insert
with `can_delete = true`; `log(Settings, "حفظ طريقة الدفع \"<name>\"", …, settings-payment-methods)`.
`reorder_payment_methods`: lock the listed live ids `FOR UPDATE` in **id order** (global lock order),
then `sort_order = index + 1` per listed id; unknown ids ignored; unlisted keep their order; no audit
(`:102-108`). `delete_payment_method`: `NOT_FOUND`; `!can_delete` → `FORBIDDEN`
`لا يمكن حذف طريقة الدفع الأساسية — عطّلها بدلاً من ذلك`; in use (`invoice_tenders.payment_method_id`,
`vouchers.payment_method_id`, `expenses.paid_from_kind = 'method' AND paid_from_payment_method_id`) →
`FORBIDDEN` `لا يمكن حذف طريقة دفع مستخدمة في مستندات مرحّلة`; soft delete; `record(entity 'paymentMethod', Delete, …)` (`:110-122`).

**Branches** (`branches.ts:96-202`) — `create_branch` is `pub` (02-setup `applyBranches` reuses it):
1. `اسم الفرع مطلوب`, `رمز الفرع مطلوب` (trim), code taken case-insensitively (all branches) →
   `VALIDATION` `رمز الفرع مستخدم بالفعل` (`:101-105`); the unique index maps to the same text (P2-22).
2. Insert: `name` trimmed, `code` trimmed + upper, `address`, `phone`, `receipt_header`,
   `bank_account_id`, `default_price_list_id`, `active ?? true`, `can_delete = false`, `created_at = now`
   (`:106-118`) — `nationalAddress` is **not** copied (Q-3).
3. Cash account (`:42-79`): lock the parent of the first live `cash`-role account with `branch_id NULL`
   (`FOR UPDATE`, serialises code allocation); code = first free `1111`…`1119` among that parent's
   children matching `^111\d$`, else `1119<live account count>`; insert `الصندوق — <name>`, ASSET/cash/
   DEBIT, `system_role cash`, `branch_id`, `allow_manual`, `active`, `can_delete = false`.
4. Cost center `CC-<code>` / name / `branch` / `can_delete = false` / `branch_id` (`:82-94`); a live
   code clash → `VALIDATION` `رمز مركز التكلفة مستخدم بالفعل` (D-7).
5. Set `cash_account_id`, `cost_center_id`; `log(Settings, "إضافة فرع \"<name>\" (<code>)", created_at, settings-branches)`;
   `cx.touch(Ledger)` (`emit('ledger:changed')`, `:127`).
- `update_branch` (`:131-168`): `FOR UPDATE` → `NOT_FOUND` `الفرع غير موجود`; code check exactly `:134`;
  apply only present keys among name(trim)/code(trim upper)/address/phone/receiptHeader/bankAccountId/
  defaultPriceListId; `name` truthy + `cash_account_id` → rename that account `الصندوق — <name>`;
  `record(entity 'branch', Update, before/after = diff_fields over those 7 keys in that order, message
  "تعديل بيانات الفرع \"<name>\"", settings-branches, Settings)`.
- `deactivate_branch` (`:171-190`): `FOR UPDATE` → `NOT_FOUND`; inactive → return unchanged (no audit);
  live active branches `≤ 1` → `VALIDATION` `لا يمكن إلغاء تفعيل الفرع الوحيد النشط`;
  `round2(Σ |product_branch_stock.qty| of live products for this branch) > 0.001` → `FORBIDDEN`
  `لا يمكن إلغاء تفعيل الفرع — لا يزال يحتوي على مخزون. أنشئ تحويلاً لتفريغه أولاً`;
  `EXISTS shifts WHERE status='OPEN' AND branch_id = ? FOR UPDATE` → `FORBIDDEN`
  `لا يمكن إلغاء تفعيل الفرع — توجد وردية مفتوحة عليه`; set inactive + cash account inactive;
  `log(Settings, "إلغاء تفعيل الفرع \"<name>\"", …)`.
- `reactivate_branch` (`:192-202`): `NOT_FOUND`; active + cash account active; log `إعادة تفعيل الفرع "<name>"` (no guard, Q-4).

**Cost centers** (`:215-261`): list live `ORDER BY created_at, id` with budgets.
`create`: `اسم مركز التكلفة مطلوب`, `رمز مركز التكلفة مطلوب`, live code ci-taken → `رمز مركز التكلفة مستخدم بالفعل`;
`code`/`name` trimmed (code **not** uppercased); `can_delete = true`; budgets → child rows;
log `إضافة مركز تكلفة "<name>"`. `update`: `NOT_FOUND` `مركز التكلفة غير موجود`; `Object.assign`
semantics (present keys, no validation, no trim — Q-5; a code clash hits the unique index → same
`VALIDATION` text); `budgets` present → replace child rows; log `تعديل مركز التكلفة "<name after>"`.
`delete`: `NOT_FOUND`; `!can_delete` → `FORBIDDEN` `لا يمكن حذف مركز تكلفة الفرع`; `EXISTS journal_lines.cost_center_id`
→ `FORBIDDEN` `لا يمكن حذف مركز تكلفة له حركات مرحّلة`; soft delete; log `حذف مركز التكلفة "<name>"`.

**Currency** (`currency.ts:14-59`) — `is_base_currency_locked` is `pub` (02-setup reuses: one function, setup.md §8):
- `is_base_currency_locked` = `EXISTS (SELECT 1 FROM journal_entries)` (`:50-52`).
- `set_base_currency(code)` `pub`: locked → `FORBIDDEN` `لا يمكن تغيير العملة الأساسية بعد بدء الترحيل`;
  `load_shared_locked`; `currency = upper(code)`; log `تغيير العملة الأساسية إلى <UPPER>` → settings-general (`:54-59`).
- `create_currency` `pub`: `input.code == base` (case-**sensitive**, before uppercasing) → `VALIDATION`
  `هذه هي العملة الأساسية بالفعل`; exact code exists → `هذه العملة مضافة بالفعل`; store upper (`:30-37`);
  a PK clash from the upper-casing (Q-6) → same `هذه العملة مضافة بالفعل`. No audit (mock has none).
- `update_currency(code, patch)`: missing → `NOT_FOUND` `العملة غير موجودة`; present keys except `code`
  (D-8); no audit.
- `save_exchange_rate`: currency missing → `NOT_FOUND` `العملة غير مفعّلة` (the currency row is locked
  `FOR UPDATE` — serialises same-currency saves); `rate = input.rate ?? (inverseRate ? round2(1 / inverseRate) : None)`;
  `rate` absent or `≤ 0` → `VALIDATION` `أدخل سعر الصرف`; `date = localDateKey(input.date)` (DocDate
  resolve → day); **delete** the `(currency, date)` row then **insert** a new row with a new id (the mock
  moves the row to the end of the array, `:62-73`, so list order stays equal).

**Revaluation** (`revaluation.ts`; lives here because its three functions are `settings.*`, D-10):
- `open_fc_balances(conn, rates)` (`:26-69`): receivable/payable account = **first** live account with that
  `system_role` (`ORDER BY created_at, id`); lines `ORDER BY journal_entries.created_at, journal_entries.id,
  journal_lines.position` with that account, matching `party_kind`, `amount_fc IS NOT NULL`, `currency`
  non-empty; group by party in first-appearance order; `currency` = first line's; `rates[currency]`
  absent/0 → skip; `fcBalance = round2(Σ sign·amountFc)` with sign by side (`:46`), `baseBalance =
  round2(Σ(debit−credit) · (supplier ? −1 : 1))`; skip `|fcBalance| < 0.005`; `revaluedBase =
  round2(fcBalance·rate)`; name = party name else id; `gainLoss = round2(revaluedBase − baseBalance)`.
  Then live non-group accounts with a `currency` (array order): lines with `amount_fc IS NOT NULL`; none →
  skip; `fcBalance = round2(Σ(debit−credit) == 0 ? 0 : Σ amountFc·(debit>0 ? 1 : −1))` (Q-7); same tail.
- `default_revaluation_rates` (`:72-80`): active currencies in order; latest rate by `date DESC` with no
  date cutoff (`'9999-99-99'`, Q-8); else `fixed && fixed_rate` → `fixed_rate`.
- `post_revaluation(date, rates)` (`:96-139`): rows with `|gainLoss| ≥ 0.01`; none → `VALIDATION`
  `لا توجد أرصدة عملات أجنبية بحاجة لإعادة تقييم بهذه الأسعار`; lines per row (account/receivable+customer/
  payable+supplier, side by sign, description `إعادة تقييم <cur> — <name>`), `totalGain = round2(running)`,
  plus `fxGain` credit / `fxLoss` debit `صافي إعادة تقييم العملات`; `ledger::post { date, "إعادة تقييم العملات بتاريخ <date>",
  System, source { kind "fxReval", id: Id::new(), number "FXR-<date>" }, allow_closed_period: false }`;
  mirror = a **new** `ledger::post` (not `ledger::reverse` — the mock posts fresh mirrored lines with only
  account/description/party, `:132`) dated `first_of_next_month(date)`, `"عكس إعادة تقييم العملات بتاريخ <date>"`,
  source number `FXR-<date>`, `allow_closed_period: true`; `log(Journal, "إعادة تقييم العملات بتاريخ <date> (<n> رصيد)",
  Some(date), RouteRef::detail("journal-entry", entry.id))`. Returns `{ entryId, reversalEntryId, rows }`.

**Network (`network.rs`, Windows bodies; other targets return `FORBIDDEN` with
`ServerFailure::UnsupportedPlatform.message_ar()`)** — Part 02 handoff §9, P2-54/56/57:
- Common gate for the 3 writes: `state.device.role == Main` else `FORBIDDEN` `هذا الإعداد متاح على الجهاز الرئيسي فقط`;
  `payload = PayloadDir::from_app(&app).unwrap_or_else(PayloadDir::dev)`, `creds = CredentialStore::production()`
  (the `boot_managed_server` pattern, `core/state.rs`).
- `get_lan_sharing_status`: terminal → `{ role: terminal, provisioned: false, lanSharing: false,
  connectedTerminals: 0, mainHost: device.connection.host }`; main → `server.json` (`state_file::load`) →
  `provisioned`, `lanSharing`; when sharing: `pairing = { host_name(), non_loopback_ipv4_addresses(), port,
  format_code(creds.get_lan_secret(data_dir_id)) }` and `connectedTerminals = lan::connected_terminal_count(port, root)`.
- `enable_lan_sharing`: `lan::enable_lan_sharing(&state.server, &creds, &payload, FirewallMode::Real)`
  → `lan::on_lan_sharing_enabled(&app, &state.server)` → then one `with_tx` writing
  `log(Settings, 'تفعيل مشاركة قاعدة البيانات على الشبكة', …, settings-network)`. Error map (D-11):
  `PermissionDeclined` → `FORBIDDEN`, `NotProvisioned` → `VALIDATION`, every other `LanError` →
  `INTERNAL`, message = `LanError::message_ar()`.
- `disable_lan_sharing(confirm_disconnect)`: `connected_terminal_count > 0 && !confirm` → `CONFLICT`
  `أجهزة الكاشير متصلة الآن (<n>) — أكّد الإيقاف لقطع اتصالها` (D-12); `lan::disable_lan_sharing` →
  `on_lan_sharing_disabled(&app)` → log `إيقاف مشاركة قاعدة البيانات على الشبكة`.
- `rotate_pairing_code`: `lan::rotate_pairing_code` → log `تغيير رمز اقتران أجهزة الكاشير`.
- `reconnect_backend` (D-9): main with `server.json` ready → `state.server.ensure_running(&payload, &creds)`;
  then (any role) `core::db::connect_and_migrate(&app)`. Never touches data; returns `()`; the UI re-reads
  `core_backend_status`.

## 4. Concurrency (D8, settings.md §5)
- Settings row: every writer takes `load_shared_locked` (`FOR UPDATE`) **first** (global order: settings S
  first) — `update_settings`, `set_base_currency`. Device fields never contend (per machine).
- Branch/cost-center/currency codes: pre-check + unique index (`AppError::map_unique` → the pre-check's
  exact text). Branch cash-account codes: parent-account lock (§3 branches step 3).
- Exchange rate `(currency, date)`: currency row `FOR UPDATE` + `uq(currency, date)` — last write wins, as the mock.
- `deactivate_branch`: branch row `FOR UPDATE`; the open-shift check is a locking read. 08-invoices'
  `openShift` must take `branches` row `LOCK IN SHARE MODE` for the shift's branch so it cannot slip in
  between (handoff H-2).
- `reorder_payment_methods`: rows locked in id order.
- `post_revaluation`: period/numbering locks inside `ledger::post` (P2-13, P2-21).
- LAN toggles restart the server; in-flight commands on other terminals fail with the pool's
  connection error (accepted: an explicit admin action, confirmed in the UI when terminals are connected).

## 5. Undo
Not undoable via the registry (settings.md §4; phase-e E-5 lists no settings action). `post_revaluation`
is self-reversing by construction. No `register_undo`.

## 6. Frontend switch lines and new frontend pieces

`src/modules/settings/services/settingsService.ts` — first line inside each `wrap`:
`getSettings` → `if (usesRust('settings')) return backendCall('settings_get_settings');`;
`updateSettings` → `…('settings_update_settings', { patch })`; `getTaxes`; `saveTax` → `{ input, id }`;
`deleteTax` → `{ id }` (then `return;`); `getPaymentMethods`; `savePaymentMethod` → `{ input, id }`;
`reorderPaymentMethods` → `{ orderedIds }`; `deletePaymentMethod` → `{ id }`.
`src/modules/settings/services/branchesService.ts`: `getBranches`, `createBranch` `{ input }`,
`updateBranch` `{ id, input }`, `deactivateBranch` `{ id }`, `reactivateBranch` `{ id }`, `getCostCenters`,
`createCostCenter` `{ input }`, `updateCostCenter` `{ id, input }`, `deleteCostCenter` `{ id }`,
`getCurrencies`, `getExchangeRates` `{ currency }`, `createCurrency` `{ input }`, `updateCurrency`
`{ code, patch }`, `saveExchangeRate` `{ input }`, `isBaseCurrencyLocked`, `setBaseCurrency` `{ code }`,
`getRevaluationPreview` `{ rates }`, `getDefaultRevaluationRates`, `postRevaluation` `{ date, rates }`.
Void-returning ones: `{ await backendCall(...); return; }`.

**C-14:** delete `theme` from `StoreSettings` (`types/index.ts:122`) and its three writers
(`src/mocks/db.ts:194`, `src/mocks/seed/index.ts:117`, `src/mocks/fixtures/settings.ts:41`); `bun run build`
proves no reader remains.

**New (dormant until `usesRust('settings')`):**
- `src/modules/settings/services/networkService.ts`: `getLanSharingStatus`, `enableLanSharing`,
  `disableLanSharing(confirmDisconnect)`, `rotatePairingCode`, `reconnectBackend` — each `wrap('settings.<fn>')`,
  calling its command; outside `usesRust('settings')` they throw `ApiError('متاح في نسخة سطح المكتب فقط', 'FORBIDDEN')`.
- `src/modules/settings/pages/NetworkSettingsPage.vue` (route `settings-network`, `/settings/network`,
  `meta { title: 'الشبكة وقاعدة البيانات', section, area: 'settings' }`, in `routes/index.ts`), built on the
  settings page layout + `PageHeader`: a role card («هذا الجهاز هو الجهاز الرئيسي» / «جهاز كاشير متصل بـ <mainHost>»);
  on the Main PC an `AppSwitch` «السماح لأجهزة الكاشير بالاتصال بهذا الجهاز» with the note «سيطلب ويندوز
  إذناً مرة واحدة»; when on, a pairing card (host name, addresses, port and the code, each `dir="ltr"` + `num`),
  «تغيير رمز الاقتران» (confirm dialog: «ستحتاج كل أجهزة الكاشير إلى الاقتران من جديد»), connected
  terminals count; disabling with terminals connected re-asks with the `CONFLICT` text and resends
  `confirmDisconnect: true`. Outside Rust mode the page shows the shared empty state «متاح في نسخة سطح المكتب».
- `SettingsTabs.vue`: `{ to: { name: 'settings-network' }, label: 'الشبكة', show: auth.can('settings') && usesRust('settings') }`;
  `settings/commands.ts`: palette entry «الشبكة وقاعدة البيانات» → `{ name: 'settings-network' }` (rule 24).
- `src/modules/settings/controllers/useBackendHealth.ts` (Pinia store): when `isTauri() && usesRust('settings')`,
  polls `getBackendStatus()` every 5 s; `failure` = `status.server?.state === 'failed'` → `{ message: server.failure.message, code: server.failure.code }`,
  or `!status.connected && status.error` → `{ message: status.error }`.
- `src/modules/settings/components/ServerFailureScreen.vue`: full-screen blocking state (tokens only, RTL,
  `no-print`): title «تعذر تشغيل قاعدة البيانات» (main) / «تعذر الاتصال بالجهاز الرئيسي» (terminal), the
  message, `رمز الخطأ: <code>`, primary «إعادة المحاولة» (`reconnectBackend`), secondary «تصدير ملف التشخيص»
  (the existing diagnostics support-bundle service — never a second export path). Mounted once in
  `src/App.vue` (`<ServerFailureScreen v-if="health.failure" />`). Added to `/dev/ui` (`DevUiPage.vue`,
  light/dark/RTL) and `docs/design_system.md` (it's a shared full-screen state, rule 3).

## 7. Known mock quirks (kept, not fixed) and decisions

**Quirks kept:** Q-1 `saveTax` clears other defaults before the `NOT_FOUND` check — the mock keeps that
side effect, Rust rolls it back (one transaction; only the error path differs). Q-2 tax/`name` stored
untrimmed. Q-3 `createBranch`/`updateBranch` ignore `nationalAddress`. Q-4 `reactivateBranch` has no
guard and logs even when already active. Q-5 `updateCostCenter` validates nothing. Q-6 `createCurrency`
compares codes case-sensitively before uppercasing. Q-7 the account-row `Σ(debit−credit) == 0` test is
exact in Rust, float-noisy in JS. Q-8 default revaluation rates include future-dated rates. Q-9
`updateSettings` does not enforce the base-currency lock on `currency` (the UI disables it) — **recommend
fixing mock + Rust together later**; kept for parity. Q-10 `revaluation` mirror entry is not linked by
`reversal_of_id`.

**Decisions (strictest option, logged):**
- D-1 Device fields (printer hardware, `backup.folder`) live only in `device-settings.json`; the row never stores them.
- D-2 Reads need a session but no area (every role renders currency/branches/taxes); writes need Settings:Write.
- D-3 A patch touching `roleAccessOverrides` also needs Users:Write (it is the role-matrix page's field, area `users`).
- D-4 Device keys already present in the row's `printer` JSON are ignored on read.
- D-5 Changing `country` updates `settings.timezone` (P2-08) — Rust-only, no DTO impact.
- D-6 Device file written before the transaction and restored on failure (no half-applied patch).
- D-7 A branch cost-center code clash is refused (the mock would create a duplicate).
- D-8 `updateCurrency` never changes the PK `code` (the only caller sends `{ active }`, `CurrenciesSettingsPage.vue:105`).
- D-9 `settings_reconnect_backend` needs no session: the failure screen shows before anyone can log in; it only restarts/reconnects.
- D-10 Revaluation is ported here (its functions are `settings.*`); 12-accounting must not re-port it (entry §4 row 12 lists "revaluation" — manager to reword).
- D-11 `LanError` → `FORBIDDEN`/`VALIDATION`/`INTERNAL` (C-02: if `INTERNAL` is vetoed, those map to `CONFLICT`).
- D-12 Disabling LAN sharing while terminals are connected needs explicit confirmation.
- D-13 settings.md §9 D-S1 adopted: `backup.folder` device, `autoEnabled/autoTime/retention` branch (17-backup runs it Main-PC-only, D-S2).

**Handoffs:** H-1 17-backup reads/writes `settings.backup` through `store.rs`'s `pub` merge helpers.
H-2 08-invoices `openShift` takes the branch row in share mode (§4). H-3 02-setup reuses `create_branch`,
`create_currency`, `set_base_currency`, `is_base_currency_locked`, `country.rs`.

## 8. Tests

**(a) `src-tauri/tests/domain_settings.rs`:**
- get/update: device keys routed to a temp `device-settings.json` and merged back; `backup.folder` absent in a patch clears it; `storeName` blank → text; EG `vatNumber` `12345678` → `رقم التسجيل الضريبي يجب أن يكون 9 أرقام`; SA valid `310000000000003` passes; `country` change sets `timezone`; `roleAccessOverrides` by a manager → `FORBIDDEN`; activity row `تحديث إعدادات المتجر` → `settings-general`.
- concurrent `update_settings` on two connections (printer vs storeName) → both fields present after both commit.
- taxes: each message in order; second default clears the first; `NOT_FOUND` leaves defaults unchanged; delete default → `FORBIDDEN`; delete used by an invoice line → `FORBIDDEN`; delete soft-deletes and writes an audit `delete` row; list excludes it.
- payment methods: order by `sort_order`; reorder with an unknown id; delete `can_delete = false` / in-use (tender, voucher, expense) → texts.
- branches: create → cash account `1111` under the main cash parent + `CC-<CODE>` + `ledger` version bumped; second → `1112`; duplicate `main` vs `MAIN` → text; parallel creates → one `VALIDATION`; update diff rows; deactivate: only active / stock / open shift texts; reactivate.
- cost centers: messages; delete with journal line → `FORBIDDEN`; budgets replaced.
- currency: base lock texts; exchange rate same-day resave replaces and moves to the end; inverse `4` → `0.25`.
- revaluation: seeded FC invoice + rate → preview row values (round2 points); post → two entries, mirror dated the 1st of next month in a closed period allowed; `run_all` green.
- network: non-Windows → `FORBIDDEN`; terminal role → `FORBIDDEN` text (Windows-only real toggles stay in the deferred manual/VM testing plan).

**(b) Parity cases (Part 04):** `settings-update-merge`, `settings-tax-crud`, `settings-payment-methods-order`,
`settings-branch-lifecycle` (create → update → deactivate refusals → reactivate), `settings-cost-centers`,
`settings-currency-rates`, `settings-revaluation-post` (GL after), each also checked with `run_all`.

## 9. Checklist

- [x] Confirm GS-1…GS-3 (manager, before W1) — taken as already fixed per `_part02-gaps.md`'s status column; not independently re-verified.
- [x] TS first: remove `theme` + 3 writers (C-14); move `FcBalanceRow` to `settings/types/index.ts`; add `RevaluationResult`; new `settings/types/network.ts`.
- [x] `domains/settings/{mod,commands,dto}.rs` + `service/{store,taxes,payment_methods,branches,cost_centers,currency,revaluation,network,country}.rs`.
- [x] `country.rs` first (00-import and 02-setup depend on it), then `store.rs`, then the rest in §3 order, each step commented with its mock line.
- [x] `commands.rs`: 33 commands; device-file write/restore around `update_settings`; network commands with `AppHandle`.
- [x] `ipc_signatures()` (33 lines); ask the manager to register them in `generate_handler!` + `pub mod settings;`.
- [x] Switch lines (§6) in `settingsService.ts`, `branchesService.ts`.
- [x] `networkService.ts`, `NetworkSettingsPage.vue` + route/tab/palette, `useBackendHealth.ts`, `ServerFailureScreen.vue` + `App.vue` mount + `/dev/ui` + `docs/design_system.md`.
- [x] `settings/types/contract.check.ts`.
- [x] `tests/domain_settings.rs` (§8a) written — ⏳ deferred: not run, no DB available to this implementer. Parity list to Part 04 (§8b) recorded in the final report.
- [ ] Ask the manager to run `bun run memory` (new route/page/service/component) — route map regenerates.
- [x] Status note at the top of this file.

## Gate

`cargo check --workspace --all-targets` clean (manager's throttled run) · tests written (not run) ·
switch lines in place · `contract.check.ts` compiles in `bun run build` · `bun run check` (RTL/tokens/
route-object guards on the new page/components) · `bun run memory:check` 33 commands, 0 gaps. DB
tests and parity cases run in the deferred, time-boxed pass; the real UAC/firewall/second-PC checks
go to the final testing plan (master §8).

## Part 02 gaps (manager tasks, before W1)

- **GS-1** Session-only read guard is inline (no new API). Area-checked reads call
  `core::settings::require(tx, actor.as_ref(), area, access)` inside `with_read` (it takes
  `Option<&AuthenticatedUser>`, `core/settings.rs` `require`) — no change, listed for the manager to confirm.
- **GS-2** `AppHandle` access in commands: network commands take `app: tauri::AppHandle` as a Tauri-injected
  parameter (not part of args) — no core change; listed so `ipc_sig!`/scanner parsing of a 3-parameter
  command is verified by the manager.
- **GS-3** `infrastructure::database::payload::PayloadDir::from_app` and `CredentialStore::production`
  must be `pub` to `domains::settings` (they are used by `core/state.rs` today — confirm visibility).
