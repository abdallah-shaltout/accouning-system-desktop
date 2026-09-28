# 21 · 03.02 — `setup` (first-run device role + provisioning + terminal pairing, the 11-step wizard, opening balances, party openings)

> **Status (2026-09-28, resumed after an API-limit cutoff):** Rust side complete —
> `domains/setup/{mod,commands,dto,undo}.rs` + all 12 `service/*.rs` files (§3), 22 commands +
> `ipc_signatures()`, `register_undo`, `tests/domain_setup.rs` (§8a) all written. Frontend switch
> lines added to `setupService.ts` (all 19 ported functions + `ensureEmptyCompanyShell`), new
> `deviceService.ts`, `validators/pairingSchema.ts`, `DeviceSetupPage.vue` + `DeviceRoleChoice.vue` +
> `TerminalPairingForm.vue` + the `/device-setup` route, and `setup/types/contract.check.ts`
> extended for the DTOs whose hand-written type now lives in `types/index.ts`
> (`OnboardingProgress`/`Patch`, `CountryTaxInput`, `PostOpeningBalancesResult`, `DeviceSetupState`,
> `PairTerminalInput`). **Still open (manager, not this implementer's edit scope):**
> (1) register the 22 commands in `generate_handler!` + call `domains::setup::register_undo` +
> stub `domains/accounting/dto.rs` re-export wiring if not already linked into `lib.rs`;
> (2) `src/router/index.ts`'s `beforeEach` guard (§6) and `src/modules/users/services/authService.ts`'s
> `isFreshInstall` one-liner (H-2) — both outside `src/modules/setup/**`;
> (3) `src/mocks/backend/opening.ts`'s `reversePartyOpeningBalance` D-10 guard (mock parity) and the
> full "TS moves first" type migration (`WizardBranchInput`/`WizardPaymentMethodInput` out of
> `mocks/backend/setup.ts`, the opening line types out of `mocks/backend/opening.ts`) — both touch
> `src/mocks/**`, outside this wave's edit boundary;
> (4) `src/modules/accounting/types/contract.check.ts` (new file, §2) — outside `src/modules/setup/**`;
> (5) `bun run bindings`/`bun run memory` need to run once the above lands (generates
> `setup/types/gen/*`, `accounting/types/gen/*`, and the `device-setup` route-map entry — GU-4).
> Wave **W2** (entry file §4). Depends on: 00-import
> (`LegacyImportCard.vue`, the onboarding source-id constants), 01-settings (`create_branch`,
> `create_currency`, `set_base_currency`, `is_base_currency_locked`, `country.rs`, `store.rs`),
> 03-users (`create_user`, `to_authenticated`, H-1/H-2), Part 02 (`shared::{ledger, stock, activity}`,
> `infrastructure::database::{provision, supervisor, pairing}`, `core::device::pair_terminal`,
> `core::db::{connect, check_version, migrate, connect_and_migrate}`), and gaps **GU-1…GU-4** below.

**Goal.** Port the 19 `port` functions of `setupService.ts` to `domains/setup/` (behaviour-exact,
including every wholesale-replace guard and the `afterGoLive` capital branch), and add the Part 02
handoff pieces: the first-run role step («هذا الجهاز هو الجهاز الرئيسي» / «جهاز كاشير») that calls
`infrastructure::database::provision::provision_main`, the terminal pairing form
(`pairing::parse_code` + `core::device::pair_terminal`), `settings.timezone` from the country profile,
and the `setup.postPartyOpening` undo compensator (phase-e E-5). The wizard is a public route, so the
Rust side gives it a **bootstrap session** while onboarding is unfinished (D-1).

**Read first.** [`../01-frontend-analysis/setup.md`](../01-frontend-analysis/setup.md) §1–§9 ·
[`../02-core-and-shared/phase-e-activity.md`](../02-core-and-shared/phase-e-activity.md) E-4/E-5 ·
[`../02-core-and-shared/phase-a3-main-pc-hosting.md`](../02-core-and-shared/phase-a3-main-pc-hosting.md) A3-2 ·
[`../02-CORE-AND-SHARED-ARCHITECTURE.md`](../02-CORE-AND-SHARED-ARCHITECTURE.md) P2-08, P2-20, P2-28,
P2-47, P2-54, P2-57, P2-59, §9 · mock: `src/modules/setup/services/setupService.ts:32-236`,
`src/mocks/backend/setup.ts:22-189`, `src/mocks/backend/opening.ts:59-350`, `src/mocks/seed/index.ts:80-121`
(`seedEmptyCompany`), `src/mocks/fixtures/accounts.ts:1-266` (`buildAccounts`),
`src/modules/core/helpers/format.ts:205-223` (`formatAddress`), `src/modules/setup/pages/SetupWizardPage.vue:32-120`,
`components/steps/StepOpening.vue:72-183`, `src/modules/parties/pages/PartyFormPage.vue:286-304`,
`src/router/index.ts:58-70`, `src/modules/users/services/authService.ts:69` (`isFreshInstall`) · Rust:
`core/device.rs` (`pair_terminal`, `debug_db_url_override`), `core/state.rs` (`boot`, `boot_managed_server`),
`core/db.rs`, `infrastructure/database/{provision,supervisor,pairing,payload,credentials,errors}.rs`,
`shared/ledger/{post,reverse,accounts}.rs`, `shared/stock/{mod,batches}.rs`, `shared/activity/{record,undo}.rs`.

## 1. Commands

| Mock fn | Disposition | Rust command | Args → Return | Area / Access | Tx | Events |
|---|---|---|---|---|---|---|
| — new (`deviceService.getDeviceSetupState`) | port (new, Tauri) | `setup_get_device_setup_state` | `()` → `DeviceSetupState` | none | `with_read` (only when connected) | — |
| — new (`deviceService.provisionMainDevice`) | port (new) | `setup_provision_main` | `()` → `DeviceSetupState` | none — refused once configured | none (infrastructure) | — |
| — new (`deviceService.pairTerminalDevice`) | port (new) | `setup_pair_terminal` | `{ host, port, code }` → `DeviceSetupState` | none — refused once configured | none | — |
| `ensureEmptyCompanyShell` | unwrapped, sync | — (switch: no-op; shell seeded by the next row) | — | — | — | — |
| `getOnboardingProgress` | port | `setup_get_onboarding_progress` | `()` → `OnboardingProgress` | session, **or** bootstrap (D-1) | `with_tx(require_user: false)` (may seed the shell) | — |
| `saveOnboardingProgress` | port | `setup_save_onboarding_progress` | `{ patch }` → `()` | Settings:Write | `with_tx` | — |
| `markStepDone` | port | `setup_mark_step_done` | `{ key, stepIndex? }` → `()` | Settings:Write | `with_tx` | — |
| `markStepSkipped` | port | `setup_mark_step_skipped` | `{ key }` → `()` | Settings:Write | `with_tx` | — |
| `applyBusinessTypeDefaults` | port | `setup_apply_business_type_defaults` | `{ businessType }` → `()` | Settings:Write | `with_tx` | — |
| `isBaseCurrencyLocked` | port | `setup_is_base_currency_locked` | `()` → `bool` | session | `with_read` | — |
| `applyCountryTax` | port | `setup_apply_country_tax` | `{ input: CountryTaxInput }` → `()` | Settings:Write | `with_tx` | — |
| `applyFiscalYear` | port | `setup_apply_fiscal_year` | `{ startMonth, startDay, goLiveDate }` → `FiscalYear` | Settings:Write | `with_tx` | — |
| `applyBranches` | port | `setup_apply_branches` | `{ branches: WizardBranchInput[] }` → `Vec<Branch>` | Settings:Write | `with_tx` | `ledger` (via `create_branch`) |
| `previewCoaTemplate` | **frontend** (override, D-6) | — | — | — | — | — |
| `applyCoaTemplate` | port | `setup_apply_coa_template` | `{ template, country?, businessType? }` → `Vec<Account>` | Settings:Write | `with_tx` | `ledger` |
| `applyPaymentMethods` | port | `setup_apply_payment_methods` | `{ methods }` → `()` | Settings:Write | `with_tx` | — |
| `getOpeningBalanceEquityNet` | port | `setup_get_opening_balance_equity_net` | `()` → `number` | Accounting:Read | `with_read` | — |
| `isFirstUsePosted` | port | `setup_is_first_use_posted` | `()` → `bool` | Accounting:Read | `with_read` | — |
| `postOpeningBalances` | port | `setup_post_opening_balances` | `{ input: OpeningEntryInput, closeTarget }` → `PostOpeningBalancesResult` | Accounting:Write | `with_tx` | `ledger`, `parties` |
| `postOpeningStock` | port | `setup_post_opening_stock` | `{ branchId, date, lines }` → `()` | Accounting:Write | `with_tx` | `ledger`, `catalog` |
| `recloseOpeningBalanceEquity` | port | `setup_reclose_opening_balance_equity` (setup.md §1 rename) | `{ date, target? }` → `()` | Accounting:Write | `with_tx` | `ledger` |
| `postPartyOpening` | port | `setup_post_party_opening` | `{ input: PartyOpeningInput }` → `Option<String>` | Parties:Write | `with_tx` | `ledger`, `parties` |
| `reversePartyOpening` | port | `setup_reverse_party_opening` | `{ entryId }` → `()` | Parties:Write | `with_tx` | `ledger`, `parties` |
| `finishOnboarding` | port | `setup_finish_onboarding` | `()` → `()` | Settings:Write | `with_tx` | — |
| `persistProgress` | drop (setup.md §8) | — (untouched) | — | — | — | — |

**22 commands** (19 ports + 3 device). "Settings:Write" commands run with the bootstrap admin during
the wizard (D-1), and with the logged-in user afterwards (`/setup/opening` is `area: 'accounting'`,
`routes/index.ts`; the party stub is on `PartyFormPage`, area `parties`).

## 2. DTOs (`domains/setup/dto.rs`, `#[ts(export_to = "setup/types/gen/")]`)

**TS moves first** (a service-returned type must live in `types/`, not in a mock file, so the Rust
binding has a contract to equal and the mock can be deleted later without touching types):
`OnboardingProgress` (`setupService.ts:52-59`), `WizardBranchInput`, `WizardPaymentMethodInput`
(`mocks/backend/setup.ts:95-165`), `OpeningCashLine`, `OpeningPartyLine`, `OpeningOtherLine`,
`OpeningEntryInput`, `OpeningStockLine`, `PartyOpeningBalanceInput` (`opening.ts:26-258`),
`AccountTemplate` (`mocks/fixtures/accounts.ts`) → `src/modules/setup/types/index.ts`; the mock files
import them back. New named types there: `CountryTaxInput` (the inline param of `applyCountryTax`,
`setupService.ts:104-110`), `PostOpeningBalancesResult { openingEntryId: string; closingEntryId?: string }`,
`CloseTarget = 'capital' | 'ownerCurrent'`, `DeviceSetupState`, `PairTerminalInput`.

```ts
export interface DeviceSetupState { configured: boolean; role: 'main' | 'terminal'; canHostDatabase: boolean; hasUsers: boolean }
export interface PairTerminalInput { host: string; port: number; code: string }
```

| Rust DTO | TS type | Notes |
|---|---|---|
| `OnboardingProgress` | moved type | `skipped`/`done: Vec<String>` always present (`?? []`, `setupService.ts:64`); `goLiveDate` `YYYY-MM-DD`; `finishedAt` `format_iso_ms`; `completedStep: Option<i32>`. |
| `OnboardingProgressPatch` | `Partial<OnboardingProgress>` | all keys optional. |
| `CountryTaxInput` | new named | `country` path-typed `CountryCode`; `extraCurrencies: { code: string; rate: number }[]` (`rate` `serde_number`). |
| `WizardBranchInput`, `WizardPaymentMethodInput` | moved | `address` = the `Address` value object (`entities/values.rs`), path-typed to `core/types/address`; `type`/`accountRole` reuse 01-settings' enums (setup.md §2, not redeclared). |
| `OpeningEntryInput` (without `createdBy`, `setupService.ts:188`), `OpeningCashLine`, `OpeningPartyLine`, `OpeningOtherLine` | moved | money `serde_number` (`amount`, `amountFc`), `rate` `serde_number`; `side` `debit/credit`; `partyKind` `customer/supplier`. The command's input type is `Omit<OpeningEntryInput,'createdBy'>` — the Rust struct simply has no `createdBy`; `contract.check.ts` compares against `Omit<…>`. |
| `OpeningStockLine` | moved | `qty`, `unitCost` `serde_number`; `expiryDate` `YYYY-MM-DD`. |
| `PartyOpeningInput` | `Omit<PartyOpeningBalanceInput,'createdBy'>` | `asOfDate` `NaiveDate` (never tz-shifted, setup.md §2). |
| `PostOpeningBalancesResult`, `DeviceSetupState`, `PairTerminalInput`, `CloseTarget`, `AccountTemplate` | new/moved | `AccountTemplate` enum `basic/standard/detailed` (camelCase) — reused by `StoreSettings.onboarding.coaTemplate`. |
| `FiscalYear`, `Account` | `accounting/types/index.ts` | **created here** in `domains/accounting/dto.rs` (`export_to = "accounting/types/gen/"`) because setup (W2) returns them before 12-accounting (W5) exists; 12-accounting extends that file and never redeclares them (H-3). |
| `Branch` | settings | reused from `domains::settings::dto` (01). |

`src/modules/setup/types/contract.check.ts`: `Equals` for every row above (00-import's entries stay).
`src/modules/accounting/types/contract.check.ts` (create): `FiscalYear`, `Account`.

## 3. Service logic (`domains/setup/service/{device,session,shell,coa,progress,country_tax,fiscal_year,branches,payment_methods,opening,party_opening}.rs`, `undo.rs`)

Common: wizard writers take `core::settings::load_shared_locked` (**exclusive**) as their first
statement — serialises wizard steps and avoids a shared→exclusive upgrade when `ledger::post` later
takes its shared settings lock (P2-13). Activity via `shared::activity::{log, log_undoable, record}`.

### 3.1 Device (`device.rs`, Windows bodies; other targets: `FORBIDDEN` `ServerFailure::UnsupportedPlatform.message_ar()`)
- **`get_device_setup_state(state)`**: first waits (≤ 60 s, 250 ms steps) while `db_status ∈ {Connecting,
  ServerStarting}` (boot connects in the background, `core/state.rs`); `configured` =
  `device.connection.is_some() || debug_db_url_override().is_some()`; `role` = `device.role`;
  `canHostDatabase` = `PayloadDir::from_app(&app).and_then(read_manifest).is_ok()`; `hasUsers` =
  connected ? `EXISTS(SELECT 1 FROM users)` (`with_read`) : `false`.
- **`provision_main(app, state)`** (P2-54 handoff):
  1. configured → `CONFLICT` `هذا الجهاز مُعدّ بالفعل`; `!canHostDatabase` → `INTERNAL` `ServerFailure::PayloadMissing.message_ar()`.
  2. `provision::provision_main(&state.server.paths, &payload, &CredentialStore::production(),
     state.terminal.terminal_id, Some(&state.app_data_dir))` (GU-1: `state.server` is a managed handle
     on every Windows machine). It writes `device-settings.json` `role: main` + `127.0.0.1` connection (C-23).
     `Err(f)` → `INTERNAL` `f.message_ar()` (+ `log::error!` with `f.code()`; C-02 veto → `CONFLICT`).
  3. `*state.device.write() = core::device::load(&state.app_data_dir)?`.
  4. `state.server.ensure_running(&payload, &creds)` (adopts the server provisioning left running).
  5. `core::db::connect_and_migrate(&app).await`; `db_status != Connected` → `INTERNAL` with
     `core::status` wording for that status (`تعذر تشغيل قاعدة البيانات المدمجة`).
  6. Return `get_device_setup_state` (`hasUsers: false` — the shell is **not** seeded here, so D10's
     "import only into an empty DB" still holds; it is seeded when the owner starts the wizard, 3.3).
- **`pair_terminal(app, state, input)`** (A3-2, P2-57):
  1. configured → `CONFLICT` `هذا الجهاز مُعدّ بالفعل`.
  2. `host.trim()` empty → `VALIDATION` `أدخل اسم الجهاز الرئيسي أو عنوانه`; `port` not in `1..=65535` →
     `VALIDATION` `رقم المنفذ غير صحيح`; `pairing::parse_code(code)` `None` → `VALIDATION`
     `رمز الاقتران غير صحيح — أدخله كما يظهر على الجهاز الرئيسي`.
  3. Probe: `core::db::connect(&ConnectionSettings { host, port, database: "equal", user: "equal_lan", last_known_address: None }, &secret)`;
     error → `VALIDATION` `تعذر الاتصال بالجهاز الرئيسي — تأكد من الاسم والرمز وأن الجهاز الرئيسي يعمل`;
     `check_version` fail → `VALIDATION` `قاعدة البيانات غير مدعومة — يلزم MariaDB 10.11 أو أحدث`;
     `migrate(db, Terminal)` mismatch → `VALIDATION` `قاعدة البيانات على الجهاز الرئيسي بإصدار مختلف — حدّث البرنامج على الجهازين`
     (the `core/status.rs` texts). Close the probe pool.
  4. `last_known_address` = first IPv4 from `tokio::net::lookup_host((host, port))` (None when it's already an IP).
  5. `core::device::pair_terminal(&state.app_data_dir, host, port, &secret, last_known)`; reload
     `state.device`; `connect_and_migrate(&app)`; return state (`hasUsers` from the Main PC's DB).

### 3.2 Bootstrap session (`session.rs`, D-1)
`ensure_bootstrap_session(state, conn)`: if `state.session` is `None` **and** the settings row's
`onboarding.finished_at` is `NULL` → the first active `admin` user (`ORDER BY created_at, id`) becomes
the session via `domains::users::service::to_authenticated` (03-users), and its id is remembered in
`session.rs`'s process-local `BOOTSTRAP_USER: Mutex<Option<Id>>`. `clear_bootstrap_session(state)`:
if `state.session.id == BOOTSTRAP_USER` → `state.session = None`, `BOOTSTRAP_USER = None`.

### 3.3 `get_onboarding_progress` (`setupService.ts:61-65`) — command body
1. `with_tx(TxOpts { require_user: false })`: if `users` and `settings` are both empty →
   `shell::seed_company_shell(conn, cx)` (the Rust `ensureEmptyCompanyShell`, `setupService.ts:41-43`);
   `users` empty but a settings row exists → `INTERNAL` (inconsistent DB, never auto-repaired).
2. Read `settings.onboarding` → DTO (`skipped/done ?? []`).
3. After commit: `ensure_bootstrap_session`. No session afterwards (onboarding finished, nobody
   logged in) → `UNAUTHORIZED` `سجّل الدخول أولاً` (a finished wizard is not public data).

**`seed_company_shell`** = `seedEmptyCompany('EG')` (`seed/index.ts:80-121`), in FK order, no audit
rows (the mock writes none): accounts = `coa::build_accounts(Standard, Some("EG"), None)`; fiscal years
`[y−1 (closed), y (open)]` with `y = cx.clock.today().year()`, names `y−1`/`y`, `01-01`…`12-31`
(`fixtures/accounts.ts:260-266`); branch `الفرع الرئيسي`/`MAIN` with `cash_account_id` = the `1110`
account, `can_delete = false` (inserted with `cost_center_id NULL`) → cost center `CC-MAIN`/
`الفرع الرئيسي`/`branch`/`can_delete false`/`branch_id` → branch `cost_center_id` set; taxes from
`country_profile("EG")`: `<label> (مبيعات)` OUTPUT/sales/vatOutput/default and `<label> (مشتريات)`
INPUT/purchase/vatInput/default, category `S`; payment methods `نقداً` (cash/cash, pos+payments, sort 1)
and `آجل` (credit/receivable, pos only, sort 2), both `can_delete = false`; settings row: `شركتي`, `EGP`,
`EG`, `default_tax_id` = the sales VAT, `INV-`, printer `{ mode: a4, thermalWidthMm: 80 }`,
`prices_include_tax = true`, `default_branch_id` = MAIN, **`timezone = 'Africa/Cairo'`** (handoff §9);
user `admin`/`المدير`/admin/`max_discount 100`/active + `credentials` = `hash_password("admin123")`
(`spawn_blocking`).

**`coa.rs`** — `build_accounts(template, country, business_type) -> Vec<AccountRow>`: a line-for-line
port of `fixtures/accounts.ts` (`NORMAL_SIDE`, `leaf`, `group`, `rootRows`, `standardAccountRows`,
`basicAccountRows`, `detailedAccountRows`, `saAddonRows`, `pharmacyAddonRows`, `buildAccounts`:
rows + `SA` add-on + `pharmacy` add-on, `parentId` resolved by `parentCode`, `canDelete ?? true`).

### 3.4 Progress (`progress.rs`, `setupService.ts:67-88,231-234`)
- `save_onboarding_progress(patch)`: shallow-merge present keys into `onboarding` (`{ ...o, ...patch }`).
- `mark_step_done(key, step_index)`: append `key` to `done` if absent (Set order); `completedStep = step_index` when given.
- `mark_step_skipped(key)`: same for `skipped`.
- `finish_onboarding`: `finished_at = cx.clock.now`; after commit `clear_bootstrap_session` (the wizard
  then routes to login, `SetupWizardPage.vue` "ready" step).

### 3.5 Structure steps
- **`apply_business_type_defaults(bt)`** (`setup.ts:22-39`): live `units` count > 0 → no-op; else insert
  `قطعة/pc` + (`pharmacy`: `علبة/box`, `شريط/strip`; `supermarket`: `كرتونة/ctn`; `services`: `خدمة`
  (no symbol); `clothing`: `طقم/set`).
- **`apply_country_tax(input)`** (`setupService.ts:104-136`): 1) `settings::is_base_currency_locked` →
  `FORBIDDEN` `لا يمكن تغيير الدولة أو العملة الأساسية بعد أول ترحيل`; 2) `p = country_profile(input.country)`;
  3) `row.currency != input.currency` → `settings::set_base_currency(input.currency)` (logs its own
  activity); 4) `prices_include_tax`, `country = p.code`, **`timezone = p.timezone`** (handoff §9); live
  taxes with `account_role = vatOutput` → `rate = p.vat_rate`, `name = "<label> (مبيعات)"`, `vatInput` →
  `(مشتريات)`; 5) each `extraCurrencies` entry with a non-empty code not already present (exact) →
  `settings::create_currency({ code, nameAr: code, symbol: code, decimals: 2, active: true, fixed: false, fixedRate: rate })`
  (its base-currency refusal propagates, Q-3); 6) `extraCurrencies.length > 0` → `features.currencies = true`;
  `vatRegistered` is ignored (`:135`).
- **`apply_fiscal_year(start_month, start_day, go_live)`** (`setup.ts:67-93`, `setupService.ts:140-145`):
  any journal entry → `FORBIDDEN` `لا يمكن تغيير السنة المالية بعد بدء الترحيل`; `ref = go_live` (a date;
  Q-4); `start = js_date(ref.year, m, d)`; `start > ref` → `js_date(ref.year − 1, m, d)`; `end =
  js_date(start.year + 1, start.month, start.day) − 1 day`, where `js_date` normalises day/month overflow
  exactly like JS `new Date(y, m−1, d)` (Feb 30 → Mar 2); `name = start.year`. The first fiscal year
  row (`created_at, id`) is **updated** in place (id kept — `db.fiscalYears[0]?.id`, `:85`), `is_closed =
  false`, `closed_by/closing_entry_id = NULL`; every other row is deleted; none → insert.
  `onboarding.goLiveDate = go_live`. Returns `FiscalYear`.
- **`apply_branches(list)`** (`setup.ts:106-151`): empty → `VALIDATION` `أضف فرعاً واحداً على الأقل`;
  main = the `default_branch_id` branch (the mock's `'branch-main'`, P2-20) else the first; rename:
  `name = first.name` (untrimmed), `code = upper(first.code)` (untrimmed), `national_address = first.address`,
  `address = format_address(first.address)` or NULL (`format.ts:205-223` ported to `shell.rs`); for each
  other entry: code taken (ci, including the renamed main) → `VALIDATION` `رمز الفرع "<code>" مستخدم بالفعل`,
  else `settings::create_branch({ name, code, nationalAddress: address, address: format_address(address), active: true })`;
  `rest.len() > 0` → `features.branches = true`. Returns all branches in list order.
- **`apply_coa_template(template, country = "EG", bt)`** (`setup.ts:42-55`, `setupService.ts:161-166`):
  any journal entry → `FORBIDDEN` `لا يمكن تغيير شجرة الحسابات بعد بدء الترحيل`; rows =
  `build_accounts`; **identity by code** (D-7, mirrors the mock's `acc-<code>` ids): soft-delete live
  accounts whose code is not in rows; update in place those whose code is; insert the rest; set
  `created_at = now + i ms` in template order so `ORDER BY created_at, id` equals template order;
  `onboarding.coaTemplate = template`; `cx.touch(Ledger)`. Returns rows as `Account` DTOs in template order.
- **`apply_payment_methods(methods)`** (`setup.ts:168-189`): any journal entry → `FORBIDDEN`
  `لا يمكن تغيير طرق الدفع بعد بدء الترحيل`; soft-delete all live methods; insert each (`fee_pct 0`,
  `show_in_pos/payments true`, `sort_order = i+1`, `active`, `can_delete = true`).

### 3.6 Opening (`opening.rs`, `opening.ts:59-240`)
Constants: `ONBOARDING_SOURCE_ID`, `ONBOARDING_CLOSE_SOURCE_ID` (re-exported from 00-import's `idmap.rs`, D-7 there).
- `get_opening_balance_equity_net` = `round2(Σ debit − credit)` over `journal_lines` of
  `resolve_account(OpeningBalanceEquity)` (`:65-74`); `is_first_use_posted` = `EXISTS journal_entries
  WHERE source_kind IN ('invoice','purchaseOrder')` (`:60-62`).
- `build_opening_lines(input)` (`:80-119`): cash lines (explicit account, `currency/amountFc/rate`),
  customer lines (`receivable` + party), supplier lines (`payable` + party), other lines; any
  `amount == 0` skipped; `diff = round2(Σdebit − Σcredit)`; `|diff| > 0.001` → 3900 line on the short side.
- `post_opening_balances(input, target)` (`setupService.ts:187-203`): lock settings (exclusive) →
  `ledger::post { date: input.date, "القيد الافتتاحي", Opening, source { "opening", ONBOARDING_SOURCE_ID, "OPENING" }, allow_closed_period: true }`
  → `log(Journal, 'ترحيل القيد الافتتاحي', None, RouteRef::list("journal"))` → `close_opening_balance_equity(input.date, target)`
  → `onboarding.openingEntryId = entry.id`, `closingEntryId = closing.map(id)` (None clears) → result.
- `close_opening_balance_equity(date, target)` (`:148-168`): `net` (reads this transaction's own
  writes); `|net| < 0.01` → `None`; `net > 0` → Cr 3900 / Dr target, else mirror; `ledger::post { date,
  "إقفال حساب الأرصدة الافتتاحية (3900) إلى " + (capital ? "رأس المال" : "جاري المالك"), Closing,
  source { "opening", ONBOARDING_CLOSE_SOURCE_ID, "OPENING-CLOSE" }, allow_closed_period: true }`;
  `log(Journal, 'إقفال حساب الأرصدة الافتتاحية', None, list "journal")`. `reclose_…` = this alone.
- `post_opening_stock(branch_id, date, lines)` (`:203-240`): branch must exist → `NOT_FOUND`
  `الفرع غير موجود` (D-8); `valid = qty > 0`; none → return; `shared::stock::lock_products(ids)` (sorted);
  per line in input order: `value = round2(qty × (unitCost || product.cost_price || 0))`, `total += value`;
  `stock::apply_change(p, qty, value, StockIn, StockRef { Id::new() /*adjRef*/, "OPENING-STOCK" }, date, Some(branch))`;
  `track_batches && batchNo` → `stock::receive_batch(product, qty, unitCost || cost_price, batchNo, expiry, date, ref)`;
  `total = round2(total)`; `≤ 0` → return (Q-5); `ledger::post` Dr `inventory` (branch ctx) / Cr 3900,
  `"رصيد افتتاحي للمخزون — <branch name>"`, Opening, source `{ "opening", adjRef.id, "OPENING-STOCK" }`,
  `allow_closed_period: true`; `log(Stock, "رصيد افتتاحي للمخزون — <n> صنف", Some(date), list "movements")`.
  Unknown product → `NOT_FOUND` `المنتج غير موجود` (`core.ts:251-255`).

### 3.7 Party opening (`party_opening.rs`, `opening.ts:266-350`)
- **`post_party_opening(input)`**: `amount == 0` → `None` (no write); party must exist with that kind →
  `NOT_FOUND` `العميل غير موجود` / `المورد غير موجود` (`partyService.ts:116,174`; D-9);
  `afterGoLive = goLiveDate && asOfDate > goLiveDate`; role receivable/payable; `partyDebit = customer ?
  side == debit : side == credit`; counter = `capital` (after go-live) or `openingBalanceEquity`;
  `ledger::post { asOfDate, "رصيد افتتاحي — <عميل|مورد>" + (after ? " (بعد تاريخ البدء — أُقفل مباشرة إلى رأس المال)" : ""),
  Opening, source { "opening", Id::new(), "OPENING" }, allow_closed_period: true }`;
  `log_undoable(Party, "رصيد افتتاحي (<js_number_string(amount)>) — <عميل|مورد>", None,
  RouteRef::detail("customer"|"supplier", partyId), UndoSpec { action_type: "setup.postPartyOpening", payload: { entryId } })`.
  Returns `Some(entry.id)`.
- **`reverse_party_opening(conn, cx, reg, entry_id, allow_closed_period, reason: Option<String>) -> TxResult<Id /*audit id*/>`**:
  1. entry missing → `NOT_FOUND` `القيد غير موجود` (`:309-310`).
  2. not a party opening (`source_kind != 'opening'` or no party line) → `VALIDATION`
     `هذا القيد ليس رصيداً افتتاحياً لطرف` (D-10, mock gets the same guard).
  3. `EXISTS payment_allocations WHERE target_kind = 'opening' AND target_id = ?` → `FORBIDDEN`
     `لا يمكن التراجع عن رصيد افتتاحي له تخصيص دفعة — أزل التخصيص أولاً` (`:311-312`).
  4. `ledger::reverse { original_id, date: cx.clock.now.date_naive() (UTC, `:327`, Q-6), "عكس: <description>",
     Opening, allow_closed_period, reason: None, dims: MirrorDims::Keep }` — already reversed →
     `ledger::reverse`'s `هذا القيد معكوس بالفعل` (Q-7).
  5. `record(… message 'التراجع عن رصيد افتتاحي', link customer/supplier of the first party line,
     activity_kind Party, reason)` (`:337-348`) → audit id.
  The command passes `allow_closed_period = true, reason = None` (mock); the compensator passes D7's value.

## 4. Concurrency (D8, setup.md §5)
- Wizard: single operator on the Main PC before terminals exist (D1); still every wizard writer takes the
  settings row exclusively first (§3), so two windows can't interleave a wholesale replace.
- Replace guards (`journal_entries` non-empty → `FORBIDDEN`) run after the settings lock; any poster
  takes the settings row in share mode (P2-13), so a post cannot slip between guard and replace.
- Opening posts: numbering/period locks inside `ledger::post`; products locked sorted by id (`lock_products`).
- `reverse_party_opening` vs a payment allocating the same entry: `ledger::reverse` locks the entry
  `FOR UPDATE` before posting; 09-payments' allocation to an `opening` target must lock that journal
  entry `LOCK IN SHARE MODE` (H-4), so the allocation check in step 3 is race-free.
- Provision/pair: guarded by `configured`; a second concurrent call hits `provision_main`'s own
  `server.json` state machine (idempotent/resumable) and then `CONFLICT`.

## 5. Undo
Registers **one** compensator (phase-e E-5): `domains/setup/undo.rs` `PartyOpeningCompensator {
action_type: "setup.postPartyOpening", area: Parties }`; `compensate(tx, cx, original, req)`: reads
`payload.entryId`, calls `reverse_party_opening(tx, cx, reg, entry_id, allow_closed_period =
cx.actor.role == Admin (D7), Some(req.reason))` and returns its audit id (the registry links
`undo_of/undone_by`). `domains/setup/mod.rs` `pub fn register_undo(r: &mut UndoRegistry)`. Every other
setup write is "not undoable via the registry" (setup.md §4). The step-8 opening/closing entries get no
reversal: no UI calls one (setup.md §8 last bullet; later note).

## 6. Frontend switch lines and new frontend pieces

`src/modules/setup/services/setupService.ts` (first line inside each `wrap`): `getOnboardingProgress`,
`saveOnboardingProgress` `{ patch }`, `markStepDone` `{ key, stepIndex }`, `markStepSkipped` `{ key }`,
`applyBusinessTypeDefaults` `{ businessType }`, `isBaseCurrencyLocked`, `applyCountryTax` `{ input }`,
`applyFiscalYear` `{ startMonth, startDay, goLiveDate }`, `applyBranches` `{ branches }`,
`applyCoaTemplate` `{ template, country, businessType }`, `applyPaymentMethods` `{ methods }`,
`getOpeningBalanceEquityNet`, `isFirstUsePosted`, `postOpeningBalances` `{ input, closeTarget }`,
`postOpeningStock` `{ branchId, date, lines }`, `recloseOpeningBalanceEquity` `{ date, target }`,
`postPartyOpening` `{ input }` (`?? undefined` — Rust `null` → TS `undefined`), `reversePartyOpening`
`{ entryId }`, `finishOnboarding`. `ensureEmptyCompanyShell`: `if (usesRust('setup')) return;`.
`previewCoaTemplate`, `persistProgress`: untouched.

**Mock fix (parity with D-10):** `src/mocks/backend/opening.ts` `reversePartyOpeningBalance` gets the
same "not a party opening" guard after the `NOT_FOUND` check; `bun run verify:mocks` stays 128/0.

**New (dormant until `usesRust('setup')`):**
- `src/modules/setup/services/deviceService.ts`: `getDeviceSetupState`, `refreshDeviceSetupState`,
  `ensureDeviceSetupState` (cached promise), `isFreshInstallCached()` (sync: `!cache.hasUsers`),
  `provisionMainDevice`, `pairTerminalDevice(input)` — each `wrap('setup.<fn>')`.
- `src/router/index.ts` `beforeEach`, first lines: `if (usesRust('setup')) { const d = await
  ensureDeviceSetupState(); if (!d.configured) return to.name === 'device-setup' ? true : { name: 'device-setup' }; }`.
- `src/modules/users/services/authService.ts` `isFreshInstall` (03-users H-2): `if (usesRust('setup')) return isFreshInstallCached();`.
- 00-import's W2 follow-up: `await refreshDeviceSetupState();` after the successful import in
  `legacyImportService.importLegacySnapshot` and in `devToolsService.reloadDemoData`'s Rust branch.
- Route `device-setup` (`/device-setup`, `meta { public: true, layout: 'blank', title: 'إعداد الجهاز' }`) →
  `src/modules/setup/pages/DeviceSetupPage.vue` (blank layout like `/welcome`): `DeviceRoleChoice.vue` —
  two cards «هذا الجهاز هو الجهاز الرئيسي» (يحفظ بيانات نشاطك التجاري — اختره لجهاز واحد فقط في الفرع;
  disabled with «هذا الجهاز لا يدعم تشغيل قاعدة البيانات» when `!canHostDatabase`) and «جهاز كاشير»
  (يتصل بالجهاز الرئيسي عبر الشبكة المحلية); main → progress «جارٍ إعداد قاعدة البيانات على هذا الجهاز —
  قد يستغرق ذلك بضع دقائق» (polls `getBackendStatus()` for `server.state`) → then, when
  `hasLegacySnapshot()`, 00-import's `LegacyImportCard` with a secondary «ابدأ من جديد»; else
  `router.replace({ name: 'welcome' })`. Terminal → `TerminalPairingForm.vue` (`FormField` + `AppInput`
  for host, port (default `3406`) and code, the latter two `dir="ltr"`, Zod schema, submit with the
  error text inline) → `router.replace({ name: 'login' })`. All new copy lives in these components.
- Palette: none (a first-run page is never navigated to by choice).

## 7. Known mock quirks (kept) and decisions

**Quirks kept:** Q-1 wizard writes carry `userId ''` in the mock and the bootstrap admin in Rust — the
parity harness maps `'' ↔ bootstrap admin`. Q-2 `applyBranches` stores the first branch's name/code
untrimmed and never renames its cash account. Q-3 `applyCountryTax`'s `createCurrency` refusal (extra
currency = base) aborts the step — the mock keeps the earlier writes, Rust rolls back. Q-4
`setFiscalYear` parses `goLiveDate` as UTC midnight; identical for UTC+ zones (EG/SA). Q-5 opening stock
with zero total leaves stock movements without a journal. Q-6 the party-opening reversal is dated with
the UTC day (`toISOString().slice(0,10)`). Q-7 a second reversal: mock double-reverses, Rust refuses
(`ledger::reverse`) — error path only. Q-8 `applyCoaTemplate` after `applyBranches` leaves extra branch
cash accounts pointing at soft-deleted accounts (the mock's dangling ids). Q-9 `applyPaymentMethods`
makes the new cash/credit methods deletable.

**Decisions (strictest option, logged):**
- D-1 **Bootstrap session**: while `onboarding.finished_at` is NULL and nobody is logged in, the first
  admin becomes this process's session (set by `setup_get_onboarding_progress`, cleared by
  `setup_finish_onboarding`). It grants exactly what the public wizard already reaches in the mock, never
  outlives onboarding, and satisfies 03-users H-1 (`users_create_user` stays authorised).
- D-2 The role step is dormant (route guard only under `usesRust('setup')`), like every Part 03 switch.
- D-3 Provisioning does **not** seed the company shell; the shell is seeded when the wizard starts, so an
  import (00) always finds an empty database.
- D-4 `settings.timezone` = country profile (`Africa/Cairo`, `Asia/Riyadh`) in the shell and in `applyCountryTax`.
- D-5 Pairing verifies the connection, server version and schema **before** saving anything.
- D-6 `previewCoaTemplate` stays frontend (pure, synchronous, used in a `computed` in `StepCoa.vue:16`);
  Rust's `build_accounts` is pinned equal to the TS one by a fixture test (§8).
- D-7 `applyCoaTemplate` keeps account identity by code (the mock's ids are `acc-<code>`).
- D-8 `postOpeningStock` refuses an unknown branch (the mock would write a dangling id).
- D-9 Party openings refuse an unknown party with the parties domain's own texts.
- D-10 `reversePartyOpening` only reverses party-opening entries (it would otherwise let a Parties:Write
  user reverse any entry, bypassing `reverseJournal`'s MANUAL-only rule) — applied to the mock too.
- D-11 Wizard writers lock the settings row exclusively first (no lock upgrade with `ledger::post`).

**Handoffs:** H-1 03-users: `to_authenticated` must be `pub`. H-2 `isFreshInstall` one-liner in
`authService.ts` is added by this file's implementer after W1. H-3 12-accounting extends
`domains/accounting/dto.rs` (`FiscalYear`, `Account`) created here. H-4 09-payments locks an `opening`
allocation target entry in share mode.

## 8. Tests

**(a) `src-tauri/tests/domain_setup.rs`:**
- shell: empty DB + `get_onboarding_progress` → shell rows exactly (2 taxes at 14 %, 2 methods, MAIN + `CC-MAIN` + `1110`, 2 FYs, `timezone = Africa/Cairo`, `admin123` verifies); called twice → no second seed; bootstrap session set, cleared by `finish_onboarding`; after finish with no session → `UNAUTHORIZED`.
- `coa`: `build_accounts` for 3 templates × {EG, SA} × {none, pharmacy} equals `tests/fixtures/coa-templates.json` (written by `bun run verify:export-snapshot`, 00-import) — codes, names, parents, kinds, subtypes, sides, roles, flags, order.
- country tax: EG→SA switches currency (activity row), rates 15, names, timezone `Asia/Riyadh`; extra currency added once; locked after a journal → text.
- fiscal year: go-live `2026-03-15`, start `01/01` → `2026-01-01…2026-12-31`; start `07/01` → `2025-07-01…2026-06-30`; `02/29` normalisation; id of the first row kept, second row deleted; locked text.
- branches: rename main + create two (cash `1111`, `1112`), duplicate code text, empty list text, `features.branches`.
- coa apply: ids of shared codes kept, extra accounts soft-deleted, order = template, locked text.
- payment methods: replace, locked text.
- opening: balanced entry + 3900 line; closing to capital/owner current; `onboarding.openingEntryId/closingEntryId` stored; `reclose` no-op at zero; opening stock per branch with a batch product (batch row, movement, entry Dr inventory[branch]); `run_all` green after each.
- party opening: before/after go-live counter account; zero amount → `None`, no rows; unknown party text; audit row `is_undoable`, `action_type = setup.postPartyOpening`.
- reverse: missing, not-opening, allocated, twice → texts; success mirror + `reversed = true`.
- undo: `shared::activity::undo` on the party-opening audit row → mirror posted, rows linked; closed period: non-admin `FORBIDDEN`, admin OK.
- device (non-Windows CI): all three commands → `FORBIDDEN` unsupported; pairing input validation texts (unit tests on the validator fn); real provision/pair runs are in the final testing plan.

**(b) Parity cases (Part 04):** `setup-wizard-eg` (full wizard incl. opening balances + stock), `setup-wizard-sa`,
`setup-coa-switch`, `setup-branches-after-coa`, `setup-party-opening-before-after-golive`,
`setup-party-opening-reverse`; each compares DTOs + GL + stock + party balances (ids and `'' ↔ admin` mapped).

## 9. Checklist

- [ ] Confirm GU-1…GU-4 and 01/03 handoffs (H-1) in place. *(manager — needs Part 02 state)*
- [x] New types added to `setup/types/index.ts` (`OnboardingProgress`/`Patch`, `CountryTaxInput`,
      `PostOpeningBalancesResult`, `CloseTarget`, `DeviceSetupState`, `DeviceRole`, `PairTerminalInput`).
      **Partial:** `WizardBranchInput`/`WizardPaymentMethodInput` and the opening line types
      (`OpeningCashLine`/`OpeningPartyLine`/`OpeningOtherLine`/`OpeningEntryInput`/`OpeningStockLine`/
      `PartyOpeningBalanceInput`) still live in `src/mocks/backend/{setup,opening}.ts` — moving them
      touches `src/mocks/**`, outside this implementer's edit boundary. *(needs manager)*
- [x] `domains/setup/{mod,commands,dto,undo}.rs` + `service/*` in §3 order (`coa.rs`, `shell.rs` first).
- [x] `domains/accounting/dto.rs` stub with `FiscalYear`, `Account` (H-3).
- [x] 22 commands + `ipc_signatures()` + `register_undo` written; **not yet wired into `generate_handler!`/
      `lib.rs`'s undo-registry bootstrap** — ask the manager to register them and call
      `domains::setup::register_undo`.
- [x] Switch lines (§6) in `setupService.ts`, `deviceService.ts`, `DeviceSetupPage.vue` + route +
      `DeviceRoleChoice.vue` + `TerminalPairingForm.vue` (+ `validators/pairingSchema.ts`).
      **Not done (outside `src/modules/setup/**`):** mock guard D-10 in `src/mocks/backend/opening.ts`,
      the router guard in `src/router/index.ts`, the `authService.ts` `isFreshInstall` one-liner (H-2).
- [x] `setup/types/contract.check.ts` extended for the types now in `types/index.ts`.
      **Not done:** `accounting/types/contract.check.ts` (new file, outside `src/modules/setup/**`).
- [x] `tests/domain_setup.rs` (§8a) written (not run, per the no-cargo hard rule). Parity list to
      Part 04 (§8b) unchanged from this file's own §8b — no new cases added this pass.
- [ ] Ask the manager to run `bun run bindings`, `bun run memory` (route, page, service, components)
      and `verify:mocks`.
- [x] Status note at the top of this file.

## Gate

`cargo check --workspace --all-targets` clean · tests written (not run) · switch lines + mock guard
in place · `contract.check.ts` compiles · `bun run check` green on the new page/components ·
`bun run memory:check` 22 commands, 0 gaps. DB tests/parity in the deferred pass; real provisioning,
UAC and second-PC pairing in the final testing plan.

## Part 02 gaps (manager tasks, before W2)

- **GU-1** `core::state::boot`: on Windows, attach `ServerHandle::new(ServerPaths::machine())` whenever
  `ServerPaths::machine()` resolves (today only when `server.json` already exists), so first-run
  provisioning mid-session is supervised, restartable, LAN-toggleable and shut down cleanly on exit.
  `build_server_status` already returns `None` for terminals, so status is unaffected.
- **GU-2** `core::db::connect`, `check_version`, `migrate` are `pub` today; confirm `connect_and_migrate`
  can be awaited from a command (it takes `&AppHandle`) and is safe to call a second time (resets
  `db_status`, replaces `AppState.db`).
- **GU-3** `infrastructure::database::pairing::parse_code` is `#![cfg(windows)]`; the command body is
  `#[cfg(windows)]` with a non-Windows stub — no change, confirm the stub pattern is accepted by the
  `ipc_manifest_matches_handler` test (both cfgs register the same command name).
- **GU-4** `tests/architecture_rules.rs` route-name rule must know `device-setup` (after `bun run memory`
  regenerates `route-map.gen.d.ts`).
