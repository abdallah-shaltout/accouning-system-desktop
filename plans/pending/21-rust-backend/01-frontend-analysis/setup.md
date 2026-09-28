# 21 · 01.B — `setup` contract

> **Status:** done (2026-09-27) · **Inventory:** `docs/backend/contract/setup.md`
> (regenerate with `bun run contract`) · **Mock spec:** `src/mocks/backend/setup.ts`,
> `src/mocks/backend/opening.ts`, `src/mocks/backend/core.ts` (`postJournal`/`assertOpenPeriod`),
> `src/mocks/backend/branches.ts` (`createBranch`, reused), `src/mocks/backend/currency.ts`
> (`createCurrency`, `setBaseCurrency`, reused), `src/mocks/fixtures/accounts.ts` (`buildAccounts`)
> · **Services:** `src/modules/setup/services/setupService.ts` · **Types:**
> `src/modules/setup/types/index.ts`
>
> This is the one-time 11-step onboarding wizard (docs/v2/05-onboarding.md). It writes into the
> same tables `settings.md` and `accounting`/`products` own — it has almost no state of its own
> beyond `settings.onboarding` and reuses `createBranch`/`createCurrency`/`setBaseCurrency` from
> the already-reviewed `settings` module rather than duplicating their logic.

## 1. Endpoints

| Function | Disposition (confirmed / changed + why) | Rust command | Request DTO | Response DTO | Writes (confirmed) | Shared | Undo | Notes |
|---|---|---|---|---|---|---|---|---|
| `ensureEmptyCompanyShell` | **dev/setup-only, not a `wrap()`ped endpoint** — confirmed, no change | n/a (not in the 21-function inventory; called synchronously before the wizard mounts) | — | — | `users`, `branches`, `accounts`, `taxes`, `paymentMethods` (via `seedEmptyCompany`) | — | n/a | Idempotent no-op once `db.users` is non-empty. Rust equivalent: a one-time bootstrap the app runs on first launch against an empty MariaDB (schema migration + a minimal seed row set), not a callable command — listed for completeness per the template's "work only from the inventory" rule, since it's the thing that makes step 1 possible. |
| `persistProgress` | port (confirmed) | n/a — **drops**, see §8 | — | — | — (flushes the whole snapshot to IndexedDB) | — | n/a | Pure mock-persistence artifact (`flushSnapshot()` from `mocks/persist.ts`) — under MariaDB every write already commits durably inside its own transaction, so this function has no Rust equivalent at all. Reclassifying to `drop` in §8. |
| `getOnboardingProgress` | port (confirmed) | `setup_get_onboarding_progress` | — | `OnboardingProgress` | — | — | n/a (read) | Reads `settings.onboarding`, defaulting `skipped`/`done` to `[]`. |
| `saveOnboardingProgress` | port (confirmed) | `setup_save_onboarding_progress` | `Partial<OnboardingProgress>` | — | `settings.onboarding` | — | not undoable | Shallow merge into `settings.onboarding`. |
| `markStepDone` | port (confirmed) | `setup_mark_step_done` | `{ key: string, stepIndex?: number }` | — | `settings.onboarding` | — | not undoable | `done` is a de-duplicated set (client sends a step key, server adds it if absent); optionally bumps `completedStep`. |
| `markStepSkipped` | port (confirmed) | `setup_mark_step_skipped` | `{ key: string }` | — | `settings.onboarding` | — | not undoable | Same de-dup-set pattern for `skipped`. |
| `applyBusinessTypeDefaults` | port (confirmed) | `setup_apply_business_type_defaults` | `{ businessType: string }` | — | `units` | — | not undoable | **Guarded by `db.units.length > 0`** — a no-op if units already exist, so calling it twice (e.g. revisiting the step) never overwrites owner edits. Rust must preserve this same "only seed if empty" guard, not just "insert." |
| `isBaseCurrencyLocked` | port (confirmed) | `setup_is_base_currency_locked` | — | `boolean` | — | — | n/a (read) | **Duplicate logic of `settings.isBaseCurrencyLocked`** (`journalEntries.length > 0`) — same check, reimplemented in a second file. Flagged in §8: Rust should have exactly one `shared::` function for this, called from both `setup_*` and `settings_*` commands. |
| `applyCountryTax` | port (confirmed) | `setup_apply_country_tax` | `{ country, currency, vatRegistered, pricesIncludeTax, extraCurrencies: {code,rate}[] }` | — | `settings.{currency,pricesIncludeTax,country,features}`, `taxes`, `currencies`, activity (via `setBaseCurrency`/`createCurrency`) | activity | not undoable | **Composes three settings-module writes in one wizard step**: `setBaseCurrency` (only if the currency actually changed), a direct rewrite of the two seeded VAT tax rows (`tax-vat-out`/`tax-vat-in`, hard-coded ids) from the chosen `countryProfile`, and `createCurrency` for each valid, not-yet-existing `extraCurrencies` entry (silently skips blank codes and duplicates — no error). All of this must commit as one transaction — a partial apply (e.g. currency changed but tax rewrite failed) would leave `settings.currency` and `settings.taxes` inconsistent. **Hard-coded `tax-vat-out`/`tax-vat-in`** ids are a direct dependency on `seedEmptyCompany`'s exact seed ids — flagged in §8. |
| `applyFiscalYear` | port (confirmed) | `setup_apply_fiscal_year` | `{ startMonth: number, startDay: number, goLiveDate: string }` | `FiscalYear` | `fiscalYears` (**wholesale replace** — always exactly one row), `settings.onboarding.goLiveDate` | — | not undoable | `setFiscalYear` always sets `db.fiscalYears = [fy]` — this only works because onboarding runs before any fiscal year exists. Once fiscal years are real multi-row data (`accounting` module), Rust's version of this command must refuse or special-case being called a second time after go-live — see §8. |
| `applyBranches` | port (confirmed) | `setup_apply_branches` | `{ branches: WizardBranchInput[] }` | `Branch[]` | `branches`, `accounts`, `costCenters`, `settings.features.branches`, activity (via reused `createBranch`) | activity | not undoable | **Renames the seeded main branch** for the first entry (never creates a duplicate "main" branch), then calls the real `createBranch` for every subsequent one — same function `settings.createBranch` uses, so its account/cost-center creation and audit trail are identical. Silently **skips** (no error) any branch whose code already exists, rather than throwing — inconsistent with `createBranch`'s own uniqueness `ApiError`; flagged in §8. |
| `previewCoaTemplate` | port (confirmed) | `setup_preview_coa_template` | `{ template: AccountTemplate, country?: CountryCode, businessType?: string }` | `Account[]` | — (pure computation, no `db` touch) | — | n/a (read) | Same `buildAccounts()` call as `applyCoaTemplate`, just not persisted — a preview endpoint. |
| `applyCoaTemplate` | port (confirmed) | `setup_apply_coa_template` | `{ template: AccountTemplate, country?: CountryCode, businessType?: string }` | `Account[]` | `accounts` (**wholesale replace**), `settings.onboarding.coaTemplate` | — | not undoable | **Refuses once any journal entry exists** (`journalEntries.length > 0` → `FORBIDDEN`) — this is the one function in this module with its own explicit guard against re-running after go-live. Rust must keep exactly this guard, not the softer "silently overwrite" pattern seen in `applyFiscalYear`/`applyBranches`. |
| `applyPaymentMethods` | port (confirmed) | `setup_apply_payment_methods` | `{ methods: WizardPaymentMethodInput[] }` | — | `paymentMethods` (**wholesale replace**) | — | not undoable | Unconditionally replaces the entire `paymentMethods` table with the wizard's step-7 list — no guard at all (unlike `applyCoaTemplate`). If a payment method is already referenced by a posted document by the time this step re-runs (revisiting the wizard), this would silently orphan that reference. Flagged in §8 — same class of risk as `applyFiscalYear`. |
| `getOpeningBalanceEquityNet` | port (confirmed) | `setup_get_opening_balance_equity_net` | — | `number` | — | — | n/a (read) | Sums all `openingBalanceEquity` (3900) journal lines' `debit - credit`, `round2`'d. Used to render the step-8 "does 3900 net to zero" check and to decide whether `recloseOpeningBalanceEquity` needs to do anything. |
| `isFirstUsePosted` | port (confirmed) | `setup_is_first_use_posted` | — | `boolean` | — | — | n/a (read) | True once any `SYSTEM`-posted invoice or purchase-order journal entry exists (`sourceRef.kind` check) — gates whether the opening entry/stock can still be edited ("before first use," docs/v2/05 §3). |
| `postOpeningBalances` | port (confirmed) | `setup_post_opening_balances` | `{ input: Omit<OpeningEntryInput,'createdBy'>, closeTarget: 'capital'\|'ownerCurrent' }` | `{ openingEntryId: string, closingEntryId?: string }` | `journalEntries` (2 entries: OPENING + CLOSING), `counters`, activity, `settings.onboarding.{openingEntryId,closingEntryId}` | activity, ledger, numbering, period | see §4 — reversible only via the wizard's own "edit before first use" flow, not the generic undo registry | Two `postJournal` calls (`postOpeningEntry` then `closeOpeningBalanceEquity`) plus a settings write — **all three must commit as one transaction**. Both journal posts pass `allowClosedPeriod: true` deliberately (opening entries predate any fiscal-year lock by construction). |
| `postOpeningStock` | port (confirmed) | `setup_post_opening_stock` | `{ branchId: string, date: string, lines: OpeningStockLine[] }` | — | `journalEntries`, `counters`, `productBatches`, `stockMovements`, activity | activity, ledger, numbering, period, stock | see §4 | Per-branch (unlike the legacy `postOpeningStockDefault`, which this service **never calls** — only the per-branch form is wired up). Filters out non-positive-qty lines; if nothing survives the filter, silently returns without posting (no error, no entry). Loops `applyStockChange` (+ `receiveBatch` for batch-tracked products) per line, then one journal entry for the batch total — all inside one logical operation that must commit atomically. |
| `recloseOpeningBalanceEquity` | port (confirmed) | `setup_recloseopeningbalanceequity` → **rename to `setup_reclose_opening_balance_equity`** in the Rust command name (the generator emits the TS name verbatim; note it here so Part 03 doesn't ship the un-snake-cased form) | `{ date: string, target?: 'capital'\|'ownerCurrent' }` | — | `journalEntries`, `counters`, activity (only if 3900 is non-zero) | activity, ledger, numbering, period | not undoable via the registry — but it's naturally idempotent (no-op if 3900 is already ~0) | Called after the opening entry OR opening stock changes, to re-zero 3900. Idempotent by construction (`closeOpeningBalanceEquity` returns `undefined` and posts nothing if `\|net\| < 0.01`). |
| `postPartyOpening` | port (confirmed) | `setup_post_party_opening` | `{ input: Omit<PartyOpeningBalanceInput,'createdBy'> }` | `string \| undefined` (entry id) | `journalEntries`, `counters`, activity | activity, ledger, numbering, period | see §4 | **Not onboarding-exclusive** — also called from the party-form "رصيد سابق من نظام قديم" stub any time after setup, not just during the wizard (confirmed by the doc comment in `opening.ts`). Silently does nothing (`return undefined`) if `input.amount` is falsy — no error. **Date-dependent branching**: if `asOfDate > settings.onboarding.goLiveDate`, the counter-entry goes to `capital` instead of `openingBalanceEquity`, to keep 3900 at zero for balances added after go-live — this branch must be preserved exactly, since it's an accounting-correctness rule (docs/v2/05 §4), not a UI nicety. |
| `reversePartyOpening` | port (confirmed) | `setup_reverse_party_opening` | `{ entryId: string }` | — | `journalEntries` (+ marks the original `reversed: true`), `counters` | ledger, numbering, period | **this IS the compensation for `postPartyOpening`** — see §4 | Mutates the **original** entry object in place (`originalRef.reversed = true`) as a side effect of posting the reversal — same pattern `reverseJournal` uses elsewhere in the app. Throws a plain `Error`, not `ApiError`, when the entry doesn't exist — flagged in §3/§8. **No caller-side check here** that the party-opening entry hasn't already had a payment allocated against it — the doc comment says "callers check allocation state before calling this," meaning the guard lives in the calling Vue page today, not in the mock. This must move into the Rust command itself (never trust a caller-side-only guard across an IPC boundary) — flagged in §8. |
| `finishOnboarding` | port (confirmed) | `setup_finish_onboarding` | — | — | `settings.onboarding.finishedAt` | — | not undoable | Just stamps a timestamp — no validation that any step was actually completed. |
| `uid` | **frontend/unwrapped re-export** (confirmed, no change) | n/a | — | — | — | — | n/a | `export { uid }` from `setupService.ts` — a plain re-export of the mock's id generator for wizard-local temp keys (e.g. list item `key`s in the branches/payment-methods step forms), never a backend call. Already excluded from the 21-function inventory. |

## 2. DTOs → Rust

| Type | Field | Rust type | Column | Why |
|---|---|---|---|---|
| `OnboardingProgress` | `goLiveDate` | `NaiveDate` | `DATE` | Business-local date (the opening-balance date), not an instant — matches cross-cutting's date-vs-instant split. |
| `OnboardingProgress` | `finishedAt` | `Option<DateTime<Utc>>` | `DATETIME(3)` | Instant. |
| `OnboardingProgress` | `skipped` / `done` | `Vec<String>` | child table `onboarding_steps(step_key, status)` or a JSON column — **recommend JSON** (`settings.onboarding` is already a settings sub-blob under D-S1's "branch" classification from `settings.md`; a child table is overkill for a small, wizard-lifetime-only set) | Not itself accounting data — no need for the relational rigor a child table would add. |
| `WizardBranchInput` | `address` | `Option<Address>` (shared struct, same as `Branch.nationalAddress`) | reuse `settings.md`'s `Branch` mapping | Same address shape, no new type. |
| `AccountTemplate` | (bare string union) | `enum AccountTemplate { Basic, Standard, Detailed }` | `ENUM('basic','standard','detailed')` | `#[serde(rename_all = "camelCase")]` — matches `StoreSettings.onboarding.coaTemplate`'s same three literals (already listed in `settings.md`). One enum, reused by both modules — do not redeclare. |
| `WizardPaymentMethodInput` | `type` | reuse `PaymentMethodType` enum from `settings.md` §2 | — | Same literal set (`cash`/`card`/`bank_transfer`/`wallet`/`credit`/`store_credit`). |
| `WizardPaymentMethodInput` | `accountRole` | reuse `SystemAccountRole` enum from `settings.md` §2 | — | Same literal set as `PaymentMethod.accountRole`. |
| `OpeningEntryInput.cash[].amount` / `.amountFc` / `.rate` | `amount` | `Decimal` | `DECIMAL(19,2)` | Money — `round2` throughout `buildOpeningLines`. |
| `OpeningEntryInput.cash[].rate` | `rate` | `Decimal` | `DECIMAL(19,6)` | FX rate — same scale as `ExchangeRate.rate` in `settings.md`. |
| `OpeningPartyLine.amount` / `OpeningOtherLine.amount` | `amount` | `Decimal` | `DECIMAL(19,2)` | Money. |
| `OpeningPartyLine.side` / `OpeningOtherLine.side` | `side` | `enum PostingSide { Debit, Credit }` | `ENUM('debit','credit')` | `#[serde(rename_all = "lowercase")]`. |
| `OpeningStockLine.unitCost` | `unitCost` | `Decimal` | `DECIMAL(19,4)` | Cost, `round4` scale (matches `products.md`'s cost fields — confirm there when reviewed; consistent with the master plan's cost-scale rule). |
| `OpeningStockLine.qty` | `qty` | `Decimal` | `DECIMAL(19,4)` | Quantity scale, same convention as every other qty field in the app. |
| `PartyOpeningBalanceInput.asOfDate` | `asOfDate` | `NaiveDate` | `DATE` | Business-local date — this field's exact value drives the `afterGoLive` accounting branch, so it must never be shifted by a timezone conversion. |
| Route field | `link` on the `logActivity('party', ..., /customers/${id}` / `/suppliers/${id})` calls in `postPartyOpeningBalance` | `RouteRef { name, params }` | — | Two more of the 61 path-string links (F7) — owned by `opening.ts`, tracked in §8. |
| Route field | `link: '/accounting/journal'` in `postOpeningEntry`/`closeOpeningBalanceEquity` | `RouteRef { name: 'accounting-journal' }` (no id — links to the list, not a specific entry) | — | Also F7. |
| Route field | `link: '/inventory'` in `postOpeningStockForBranch` | `RouteRef { name: 'inventory' }` | — | Also F7. |

## 3. Validation and errors

| Function | Rule (source: mock check line) | Code | Exact Arabic message |
|---|---|---|---|
| `applyCountryTax` | refused once `isBaseCurrencyLocked()` (any journal entry posted) | `FORBIDDEN` | `لا يمكن تغيير الدولة أو العملة الأساسية بعد أول ترحيل` |
| `applyCoaTemplate` | refused once any journal entry exists | `FORBIDDEN` | `لا يمكن تغيير شجرة الحسابات بعد بدء الترحيل` |
| `applyBranches` (via `setup.ts`) | at least one branch required | `VALIDATION` (default `ApiError` code) | `أضف فرعاً واحداً على الأقل` |
| `applyBranches` (via reused `createBranch`, for every branch after the first) | same rules as `settings.createBranch` (`settings.md` §3: name/code required, code unique) — **except** this function's own duplicate-code branches are silently **skipped**, not thrown (§8) | — | — |
| `reversePartyOpening` | entry must exist | plain `Error` (not `ApiError`) — **contract gap, see §8** | `القيد غير موجود` |
| `postOpeningBalances` / `postOpeningStock` / `postPartyOpening` / `recloseOpeningBalanceEquity` | `assertOpenPeriod` (via `postJournal`) is **bypassed** on every call here (`allowClosedPeriod: true` everywhere in this module) | n/a — deliberately unchecked | n/a | This is intentional (opening entries predate the fiscal-year lock by construction) but means Rust's `shared::ledger::post` must accept an explicit `allow_closed_period` flag that `setup`'s commands always pass `true`, same as the mock — not a validation gap to "fix." |

**No dedicated Zod schema** for this module — the wizard's Vue components (`StepCountryTax.vue`, `StepCompany.vue`, etc.) do their own inline field checks before calling these services (e.g. the VAT-pattern check already noted as shared with `settings.md`'s finding), but none of that client-side validation is re-asserted server-side except where listed above. **Flagged in §8**: several destructive/replace-style writes (`applyFiscalYear`, `applyPaymentMethods`, `applyBranches`'s per-branch skip) have no equivalent of `applyCoaTemplate`'s "refuse after go-live" guard.

## 4. Undo matrix (every function that writes)

| Function | Undoable? | Compensation (existing fn) | Refused when | Period rule (D7) |
|---|---|---|---|---|
| `saveOnboardingProgress` / `markStepDone` / `markStepSkipped` / `finishOnboarding` | No | — | n/a | n/a |
| `applyBusinessTypeDefaults` | No (but self-guarding — no-op once units exist, so it can't clobber) | — | n/a | n/a |
| `applyCountryTax` | No | — | n/a | n/a |
| `applyFiscalYear` | No | — | n/a | n/a — always posts with the wizard's own re-run semantics, not period-gated |
| `applyBranches` | No | — | n/a | n/a |
| `applyCoaTemplate` | No | — | n/a | n/a |
| `applyPaymentMethods` | No | — | n/a | n/a |
| `postOpeningBalances` | **No via the generic undo registry**, but the wizard's own "before first use" flow (docs/v2/05 §3) is the intended compensation path: `reversePartyOpening`-style reversal is described for the party stub, and the opening/closing pair is meant to be **re-postable** by calling `postOpeningBalances` again after a manual reversal — this repo's mock does **not** currently expose a `reverseOpeningBalances` counterpart for the step-8 entry itself (only for the party-form stub). **Contract gap — flagged in §8/§9**: there is no reviewed reversal path for the step-8 opening/closing entries, only for `postPartyOpening`. | none found in the mock | — | Posts with `allowClosedPeriod: true` always |
| `postOpeningStock` | Same gap as above — no reviewed reversal function for opening-stock postings exists in this module (a stock adjustment reversal, if any, would live in `products`/`inventory`'s own module, out of scope here) | none found in this module | — | `allowClosedPeriod: true` always |
| `recloseOpeningBalanceEquity` | No (idempotent — repeated calls after 3900 is already ~0 do nothing, so "undo" is moot for the no-op case; for the posting case, same gap as `postOpeningBalances`) | — | — | `allowClosedPeriod: true` always |
| `postPartyOpening` | **Yes** — `reversePartyOpening` is exactly the existing compensation, matching master plan §3 rule 7's pattern | `reversePartyOpening` | Doc says "allowed until a payment is allocated to it" — **but that check is not enforced in the mock function itself**, only assumed to be done by the calling page (§8 gap) | `allowClosedPeriod: true` on the reversal too |
| `reversePartyOpening` | n/a (it IS the undo action) | — | Entry must exist (`Error`, not `ApiError` — §3/§8) | `allowClosedPeriod: true` |

## 5. Concurrency under D8 (several terminals on one DB)

| Race | Rows | Settled by |
|---|---|---|
| Two terminals run the wizard simultaneously (unlikely in practice — onboarding is a single-operator, first-run flow, but D8 requires stating the row-lock story regardless) | `settings.onboarding`, `fiscalYears`, `accounts`, `paymentMethods`, `branches` | Under D1 ("Main PC hosts MariaDB… the branch works 100% offline"), the wizard only ever runs once, on the Main PC, before other terminals would have anything to connect to — so this module needs **no new locking primitive** beyond what `shared::ledger`/`shared::numbering` already provide for `postOpeningBalances`/`postOpeningStock`/`postPartyOpening`'s journal posts. Flagged as a non-issue rather than skipped, per the template's requirement to state the row/lock story explicitly. |
| `postPartyOpening`/`reversePartyOpening` racing with a payment allocation against the same party (this function is **not** onboarding-exclusive — see §1) | `journalEntries`, party balance | Same mechanism as every other ledger-posting function — `shared::ledger`'s own locking. The **real** gap here isn't concurrency, it's the missing server-side "not yet allocated against" check before allowing a reversal (§4, §8) — a business-rule gap, not a race condition. |
| `applyCoaTemplate` / `applyFiscalYear` / `applyPaymentMethods` each doing a wholesale table replace while another write is mid-flight against the same table | `accounts`, `fiscalYears`, `paymentMethods` | These are onboarding-only, single-operator operations by design (the whole point of the wizard is that nothing else has started yet) — the only real protection needed is `applyCoaTemplate`'s existing "refuse if `journalEntries` non-empty" guard, extended (per §8) to `applyFiscalYear` and `applyPaymentMethods` too, which makes the concurrency question moot: once real activity exists, wholesale replacement is refused outright rather than raced. |

## 6. Events and side effects

- **Activity/audit rows** written for: `applyCountryTax` (via `setBaseCurrency`/`createCurrency`, when currency actually changes/currency added), `applyBranches` (via reused `createBranch`), `postOpeningBalances` (2 rows: `postOpeningEntry`'s "ترحيل القيد الافتتاحي" + `closeOpeningBalanceEquity`'s "إقفال حساب الأرصدة الافتتاحية"), `postOpeningStock` (`activityKind: 'stock'`), `recloseOpeningBalanceEquity` (only if it actually posts), `postPartyOpening` (`activityKind: 'party'`).
- **No audit trail at all**: `saveOnboardingProgress`, `markStepDone`, `markStepSkipped`, `applyBusinessTypeDefaults`, `applyFiscalYear`, `applyCoaTemplate`, `applyPaymentMethods`, `finishOnboarding`, `reversePartyOpening` (the reversal itself has **no** `logActivity`/`logAudit` call, unlike its own compensated action — asymmetric and worth flagging, see §8). This is largely acceptable for onboarding progress bookkeeping (steps 1-3 above), but `applyFiscalYear`/`applyCoaTemplate`/`applyPaymentMethods`/`reversePartyOpening` are structural/financial changes with no trail — flagged in §8.
- **Events emitted:** none of this module's functions call `emit(...)` directly, **except** `createBranch` (reused by `applyBranches`), which emits `ledger:changed` as documented in `settings.md`.
- **Attachments/printing:** none.

## 7. Aggregations (reports / analytics / dashboard / insights only)

Not a reporting module — `getOpeningBalanceEquityNet` is the closest thing to an aggregation, listed here for completeness:

| Output (DTO field) | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `getOpeningBalanceEquityNet` → `number` | `journalEntries` (flattened lines) | lines where `accountId` = the `openingBalanceEquity` system-role account | — (single scalar) | `round2` (`opening.ts:73`) | `opening.ts:65-74` |

## 8. Contract fixes needed in the mock (01.C — resolved 2026-09-27)

Per CLAUDE.md's "Architectural autonomy" rule (added 2026-09-27): the items below were decided by
picking the recommended/strictest option and applied directly, without a per-item confirmation
round — same resolution the user had already given for every item in this list's `settings.md`
counterpart.

- [x] **`persistProgress` reclassified to `drop`.** Overridden in `scripts/contract/config.ts` → `overrides['setup.persistProgress']`, reason recorded there. `bun run contract` re-run — setup is now 20 port / 1 frontend / 1 drop-equivalent (307 total port across the repo, was 308).
- [x] **Duplicate `isBaseCurrencyLocked` logic** (`setup.ts` vs `settings`/`currency.ts`) — **fixed in 01.C**: `setupService.ts`'s `isBaseCurrencyLocked` now delegates to `currency.ts`'s `isBaseCurrencyLocked()` (imported as `currencyIsBaseCurrencyLocked`) instead of re-implementing `db.journalEntries.length > 0` inline. No behavior change; `verify:mocks` confirmed unchanged (128/0).
- [x] **`applyCountryTax`'s hard-coded `tax-vat-out`/`tax-vat-in`** seed-id dependency — **fixed in 01.C**: the tax-row rewrite in `setupService.ts`'s `applyCountryTax` now matches by `t.accountRole === 'vatOutput'`/`'vatInput'` instead of the two hard-coded seed ids, exactly the resolution this section recommended. No behavior change (every seeded tax row already carries the matching `accountRole`); `verify:mocks` confirmed unchanged (128/0).
- [x] **`applyBranches` now throws on a duplicate branch code** (`src/mocks/backend/setup.ts`), matching `createBranch`'s own uniqueness `ApiError` (`رمز الفرع "<code>" مستخدم بالفعل`) instead of silently skipping the branch.
- [x] **"Already past go-live" guards added** to `setFiscalYear` (`FORBIDDEN`, `"لا يمكن تغيير السنة المالية بعد بدء الترحيل"`) and `applyPaymentMethods` (`FORBIDDEN`, `"لا يمكن تغيير طرق الدفع بعد بدء الترحيل"`) in `src/mocks/backend/setup.ts`, matching `applyCoaTemplate`'s existing `journalEntries.length > 0` pattern exactly.
- [x] **`reversePartyOpeningBalance` now throws `ApiError('القيد غير موجود', 'NOT_FOUND')`** instead of a plain `Error`, closing the F5 error-code gap.
- [x] **Server-side allocation guard added to `reversePartyOpeningBalance`** (`src/mocks/backend/opening.ts`): refuses (`FORBIDDEN`, `"لا يمكن التراجع عن رصيد افتتاحي له تخصيص دفعة — أزل التخصيص أولاً"`) when any `payments[].allocations[]` has `targetKind === 'opening'` and `targetId === entryId`. The check no longer relies on the calling Vue page alone.
- [x] **`reversePartyOpeningBalance` now writes a matching `logActivity('party', 'التراجع عن رصيد افتتاحي', ...)` row**, symmetric with `postPartyOpeningBalance`'s own audit entry, linking to the same customer/supplier route.
- [x] **F7 (shared task, done in 01.C):** all path-string links in `opening.ts` (`/accounting/journal` ×2 → `{ name: 'journal' }`, `/inventory` → `{ name: 'movements' }` (judgment call, no route named exactly `/inventory` exists), plus the two `${input.partyId}`-templated customer/supplier links → `{ name: 'customer' | 'supplier', params: { id } }`) converted in the shared 01.C pass — see `01-FRONTEND-ANALYSIS.md`'s status note.
- [ ] **No reviewed reversal path for `postOpeningBalances`/`postOpeningStock` themselves** (only the party-stub has one) — confirmed there is no other reversal primitive in the wizard's Vue components either (`StepOpening.vue` only calls `postOpeningBalances`, nothing resembling a reversal). This is a genuine scope gap, not something to invent here: **carried forward to Part 03** as "no compensating action exists yet for the step-8 opening/closing entries or opening-stock postings; if the 'before first use' edit flow needs one, it must be designed then, using the same reversal pattern as `reversePartyOpeningBalance` (mirror lines + `reversalOfId` + `reversed` flag), not invented ad hoc."

## 9. Open questions (→ decisions in `00-MASTER-PLAN.md`)

All four questions raised during this review were resolved directly (see §8) under CLAUDE.md's
architectural-autonomy rule. One item remains genuinely open, not because it's a multiple-choice
implementation detail but because it's a scope question about a compensating action that doesn't
exist yet anywhere in the codebase to reuse:

- **Reversal path for step-8 opening entries.** `postOpeningBalances`/`postOpeningStock` have no compensating action today (see §8, last bullet). This isn't blocking 01.B — it's a note for whoever implements `setup_post_opening_balances`/`setup_post_opening_stock` in Part 03 to design one, following the same shape `reversePartyOpeningBalance` now uses.

## Gate

- [x] Every inventory function is in §1 with a confirmed disposition (21/21 wrapped functions, plus `ensureEmptyCompanyShell`/`uid` noted as non-endpoints for completeness). `persistProgress` → `drop` override applied.
- [x] Every write function is in §4.
- [x] Every DTO field needing a non-default mapping is in §2.
- [x] `bun run contract:check` is green after the override (`setup.persistProgress` → `drop`, `scripts/contract/config.ts`). Also green: `bun run build`, `bun run check`, `bun run verify:mocks` (128 ok, 0 failed), `bun run memory:check` (0 new seam violations), `bun run diag:check`.
