# 21 · 01.B — `accounting` contract

> **Status:** done (2026-09-27) · **Inventory:** `docs/backend/contract/accounting.md`
> (regenerate with `bun run contract`) · **Mock spec:** `src/mocks/backend/core.ts` (`resolvePosting`,
> `assertOpenPeriod`, `postJournal`, `draftJournal`, `updateDraftJournal`, `deleteDraftJournal`,
> `postDraftJournal`, `logActivity`/`logAudit`/`diffFields`, `closeYearPreChecks`/`closeFiscalYear`/
> `reopenFiscalYear`), `src/mocks/backend/journal.ts` (`recordManualJournal`, `editDraftJournal`,
> `reverseJournal`, journal templates + recurrence, VAT settlement), `src/mocks/backend/accounts.ts`
> (`accountFor`/`accountById`/`revenueAccountFor`/`cogsAccountFor`/`purchaseAccountFor` — system-role
> resolution) · **Services:** `src/modules/accounting/services/accountingService.ts` · **Types:**
> `src/modules/accounting/types/index.ts`
>
> **This is the posting engine every other reviewed module (`purchases`, `invoices`, `payments`,
> `vouchers`, `expenses`, `settings`'s `postRevaluation`) already assumed exists and calls through
> `postJournal`/`resolvePosting`/`assertOpenPeriod`.** Reviewed against `docs/v2/02-accounting-review.md`
> in full (the authoritative posting-rules doc for exactly this module — B1 control-account rules,
> B2 period-lock/closing-wizard rules, B3 reversal rules) and against `src/mocks/backend/invariants.ts`
> (the 14 invariants `bun run verify:mocks` and the diagnostics tab share — §4.1, §4.2, §4.7, §4.8,
> §4.9 are this module's own invariants; §4.3–§4.6/§4.10/§4.11 depend on it but are owned by the
> modules that post through it). **Baseline `bun run verify:mocks`: 128 ok, 0 failed** (confirmed
> fresh before any edit, on both SA and EG seeds — matches every prior module's baseline this
> session). One mock fix applied (journal-template audit trail — see §8); baseline **unchanged
> after the fix: 128 ok, 0 failed.** Per the task's explicit mandate for this module: everything
> touching posting logic, balance validation, period-close/reopen, or the reversal mechanism itself
> was **deliberately left untouched** even where a gap was found, and is written up precisely in §9
> instead of guessed at. This module has more open questions than any other reviewed this session —
> that is the correct, expected outcome for the accounting core, not a sign of a less careful review.

## 1. Endpoints

| Function | Disposition (confirmed / changed + why) | Rust command | Request DTO | Response DTO | Writes (confirmed) | Shared | Undo | Notes |
|---|---|---|---|---|---|---|---|---|
| `signedBalance` | **frontend** (confirmed) | — | `{ account: Pick<Account,'normalSide'>, debit, credit }` | `number` | — | — | n/a | Pure arithmetic (`round2(normalSide==='DEBIT' ? debit-credit : credit-debit)`) — no data access. Used both here and by pages; safe to keep client-side, Rust's own DTOs already carry a computed `balance` field so nothing is lost by not porting this as a command. |
| `accountPath` | **frontend** (confirmed) | — | `{ account: Pick<Account,'parentId'>, all: Account[] }` | `string` | — | — | n/a | Pure tree-walk over an already-fetched `Account[]` — no reason to round-trip to Rust per call. |
| `rolledBalance` | **frontend** (confirmed) | — | `{ account: AccountWithBalance, all: AccountWithBalance[] }` | `number` | — | — | n/a | Recursive pure computation over an already-fetched list (parent + descendants). If the CoA tree ever gets large enough that this recursion becomes a perf concern, Part 03 can add a server-computed rolled-balance field to `getAccounts`'s response — not needed today, no measurement asks for it. |
| `getAccounts` | port (confirmed) | `accounting_get_accounts` | `{ range?: { from?: string; to?: string } }` | `AccountWithBalance[]` | — | — | n/a (read) | Computes `debitTotal`/`creditTotal`/`balance`/`hasPostings` per account by scanning `journalEntries` in `range` — the same "balance is never a stored column" pattern `parties.md` already established for customer/supplier balances. Sorted by `code`. |
| `saveAccount` | port (confirmed) | `accounting_save_account` | `{ input: AccountInput, id?: string }` | `Account` | `accounts`, activity, audit | activity | not undoable | Create-or-update. **The single most important validation gate for the whole chart of accounts** (§3): code format/uniqueness/parent-prefix, parent must be a group of the same `kind`, a group can't `allowManual`, and — critically — a **system account** (`canDelete: false`) can never have its `code`/`kind`/`isGroup` changed, and a leaf↔group flip is blocked once the account has children or postings. Every one of these must port exactly; this is what keeps `accountFor(role)` resolution and the invariants meaningful under a real multi-terminal DB. |
| `deleteAccount` | port (confirmed) | `accounting_delete_account` | `{ id: string }` | — | `accounts` | — | not undoable (hard delete of master data — but see §5/§9, this one needs a lock, not just a check) | Refuses a system account (`canDelete: false`), an account with children, or an account with any posted journal line — already a full reference check, no gap. |
| `reparentAccount` | port (confirmed) | `accounting_reparent_account` | `{ id: string, newParentId: string \| null }` | `Account` | `accounts`, activity, audit | activity | not undoable | Drag-to-reparent on the CoA tree. Same `kind`-match rule as `saveAccount`, plus an explicit cycle check (new parent can't be a descendant of the account being moved) — walks the parent chain, so Rust's version needs the same tree-walk (a recursive CTE or an application-level walk over an already-fetched tree), not just a naive FK check. |
| `getJournalEntries` | port (confirmed) | `accounting_get_journal_entries` | `{ filter?: JournalFilter }` | `JournalRow[]` | — (generator hint says `accounts` — see §8, a false positive) | — | n/a (read) | Reads **both** `journalEntries` (posted) and `journalDrafts` (unposted) via `allEntriesAndDrafts()` — the journal list page shows drafts inline with posted entries, distinguished by `status`. `JournalRow` adds computed `createdByName`/`sourceLink`/`sourceLabel`/`attachmentCount` — none stored. Sorted by `date` then `number`, both descending. |
| `getJournalEntriesForSource` | port (confirmed) | `accounting_get_journal_entries_for_source` | `{ sourceKind: string, sourceId: string }` | `LinkedJournalEntry[]` | — | — | n/a (read) | Posted-only (`db.journalEntries`, not drafts) — feeds a source document's "قيود مرتبطة" card (an expense/voucher/etc. showing its own journal entry). |
| `getJournalEntriesPaged` | port (confirmed) | `accounting_get_journal_entries_paged` | `{ query: PagedQuery<JournalFilter> }` | `PagedResult<JournalRow>` | — (same false-positive `accounts` write hint) | — | n/a (read) | Server-mode `DataTable` variant. `totals.totalDebit`/`totalCredit` are summed over the **filtered, unpaged** set. Sort key is read dynamically off the row (`(a as any)[sort.key]`) — same caution as `payments.md`'s `getPaymentsPaged`: Rust's `ORDER BY` must whitelist exactly the columns this generates, not accept an arbitrary client-sent column. |
| `getJournalEntry` | port (confirmed) | `accounting_get_journal_entry` | `{ id: string }` | `JournalRow & { reversedById?, reversedByNumber?, related: JournalRow[] }` | — | — | n/a (read) | Looks in both posted and draft tables. `related` = every other **posted** entry sharing the same `sourceRef.kind`+`id` (e.g. a sale's SYSTEM entry and, if ever exercised, another entry against the same source) — this is the "one active entry per source" invariant's read-side view, not itself a new rule. |
| `createJournalEntry` | port (confirmed) | `accounting_create_journal_entry` | `JournalEntryInput` | `JournalEntry` | `journalDrafts` **or** `journalEntries`, `counters`, activity, audit | activity, ledger, numbering, period | none (see §4) | The manual-entry choke point (`recordManualJournal`). Branches on `input.asDraft`: draft path skips `assertOpenPeriod` entirely (see §9 — deliberately not touched); post path goes through the full `postJournal`→`assertOpenPeriod`→`resolvePosting` pipeline with `allowClosedPeriod: isAdmin`. **This is where B1's control-account rules are enforced** (`validateManualLines`, §3) — the only place in the whole app a human picks arbitrary accounts to post to. |
| `updateJournalDraft` | port (confirmed) | `accounting_update_journal_draft` | `{ id: string, input: JournalEntryInput }` | `JournalEntry` | `journalDrafts` | — | n/a (drafts have no GL effect to undo) | Re-runs `validateManualLines` (same B1 rules as a fresh entry) via `editDraftJournal`, then `updateDraftJournal` replaces the draft's lines/date/description/attachments in place. **No `logActivity` call** — see §8 (a genuine gap, but deliberately not fixed, see reasoning there). |
| `postJournalDraft` | port (confirmed) | `accounting_post_journal_draft` | `{ id: string }` | `JournalEntry` | `journalDrafts`, `journalEntries` | ledger, period | none (see §4) | **Not touched — see §9, D-A1.** Moves a draft to `journalEntries` after only an `assertOpenPeriod` check; it does **not** re-run `validateManualLines`/`resolvePosting`'s account-state checks (active/group/allowManual/requiresParty) against the accounts as they stand *now*, only as they stood when the draft was saved/edited. Also has **no `logActivity` call** at all, unlike every other posting function in this module. Both are genuine, precise findings that touch the posting choke point's validation guarantee — flagged, not fixed. |
| `deleteJournalDraft` | port (confirmed) | `accounting_delete_journal_draft` | `{ id: string }` | — | `journalDrafts` | — | n/a (drafts have no GL effect) | Existence check only, hard delete. **No `logActivity` call** — same class of gap as `updateJournalDraft`, not fixed (see §8). Not itself posting-adjacent (nothing is reversed, since a draft never touched the ledger), so this one *could* have been fixed under the "safe master-data audit gap" pattern were it not for the fact that all three draft-CRUD gaps (`updateJournalDraft`/`deleteJournalDraft`/`postJournalDraft`) read as one coherent design gap in "drafts have no activity trail at all" rather than three independent bugs — see §8's reasoning for treating them as one carried-forward item, matching `expenses.md`'s precedent for not piecemeal-fixing a repeating cross-function policy gap. |
| `reverseJournalEntry` | port (confirmed) | `accounting_reverse_journal_entry` | `{ id: string, date: string, reason: string }` | `JournalEntry` | `journalEntries`, `counters`, activity, audit | activity, ledger, numbering, period | **is itself** the undo mechanism for a `MANUAL` entry (see §4 — this is the "compensation," not a thing that itself needs one) | `reverseJournal` — refuses a non-`MANUAL` type ("القيود الآلية تُعكس من المستند المصدر"), refuses an already-reversed entry, requires a non-empty reason. Posts a full mirror (debit↔credit swapped) at the **caller-chosen date**, through the normal `assertOpenPeriod` with the same `allowClosedPeriod: isAdmin` the original posting used (B3). |
| `getFiscalYears` | port (confirmed) | `accounting_get_fiscal_years` | — | `FiscalYear[]` | — | — | n/a (read) | Sorted by `startDate` descending. |
| `getCurrentFiscalYear` | port (confirmed) | `accounting_get_current_fiscal_year` | — | `FiscalYear \| undefined` | — | — | n/a (read) | The year containing today, falling back to the latest year if none contains it — reports default to this. |
| `saveFiscalYear` | port (confirmed) | `accounting_save_fiscal_year` | `{ input: Omit<FiscalYear,'id'>, id?: string }` | `FiscalYear` | `fiscalYears`, activity, audit | activity | not undoable | `endDate > startDate`, and **no date-range overlap with any other fiscal year** (`CONFLICT` if it overlaps) — this is a real accounting-safety guard (two open years covering the same date would make `assertOpenPeriod`'s lookup ambiguous), already correctly enforced. **Nothing stops editing a year that is already `isClosed`** (e.g. shrinking its `startDate`/`endDate` after close) — flagged in §9, not fixed, since changing a closed year's boundaries after the fact is exactly the kind of "touches period rules" change this review must not guess at. |
| `getLockDate` | port (confirmed) | `accounting_get_lock_date` | — | `string \| undefined` | — | — | n/a (read) | `db.settings.accounting?.lockDate`. |
| `saveLockDate` | port (confirmed) | `accounting_save_lock_date` | `{ lockDate: string \| undefined }` | — | `settings.accounting.lockDate`, activity, audit | activity | not undoable | **No validation at all** — any string is accepted as the new lock date (not even a date-format check), and there is **no check that the new lock date doesn't sit inside a period that already has activity dated after it in a way that would suddenly need reposting** (moving the lock date only ever affects *future* postings via `assertOpenPeriod`, so this isn't actually unsafe — a later lock date just blocks more future dates, an earlier one unblocks some; nothing retroactively invalidates already-posted entries). Confirmed **not a bug** — `assertOpenPeriod` only ever gates *new* postings against the *current* lock date at posting time, so changing the stored value has no effect on entries already posted. No fix needed. |
| `getCloseYearPreChecks` | port (confirmed) | `accounting_get_close_year_pre_checks` | `{ fiscalYearId: string }` | `CloseYearPreCheck[]` | — | — | n/a (read) | Three checks: no drafts dated inside the year, trial balance balanced (±0.01), `openingBalanceEquity` (3900-role) net ≈ 0. Pure read, no mutation — safe to call repeatedly from the closing wizard's UI before the user confirms. |
| `closeYear` | port (confirmed) | `accounting_close_year` | `{ fiscalYearId: string }` | `{ fiscalYear: FiscalYear; closingEntry: JournalEntry; nextYear?: FiscalYear }` | `journalEntries`, `counters`, `fiscalYears`, activity, audit | activity, ledger, numbering, period | **not touched — see §9, D-A2** (reopen exists but is a distinct, admin-gated action, not an "undo" in the registry sense) | **The single highest-blast-radius write in this module.** Re-runs `closeYearPreChecks` server-side and refuses if any fails (never trusts a client that already saw green checks — good). Computes every REVENUE/EXPENSE account's net movement for the year, posts one closing entry (revenue/expense → `retainedEarnings`) with `allowClosedPeriod: true` (it posts *into* the year being closed, which is about to become locked), marks the year `isClosed`, and **auto-creates the next fiscal year if it doesn't exist yet** (a fourth side effect bundled into one call). All of this must commit as a single transaction — a partial commit (e.g. the closing entry posts but the year doesn't flip to closed) would leave the books in a materially wrong state. See §9 for what's deliberately not touched here. |
| `reopenYear` | port (confirmed) | `accounting_reopen_year` | `{ fiscalYearId: string }` | `FiscalYear` | `journalEntries`, `counters`, `fiscalYears`, activity, audit | activity, ledger, numbering, period | **is itself** the compensation for `closeYear` (posts the exact inverse of the closing entry, unlocks the year) | **Admin-only**, checked in the service layer (`useAuthStore().user?.role !== 'admin'` → `FORBIDDEN`) before calling `reopenFiscalYear` — this coarse role check is the *only* authorization gate in this entire module beyond the general "must be logged in" assumption; Rust must re-implement the same role check server-side (never trust a client-sent "I am admin"), not just hide the button in the UI. Reverses the closing entry (debit↔credit swapped, dated "now", `allowClosedPeriod: true`) only if it exists and isn't already reversed (idempotent against a partially-broken prior close), then unmarks `isClosed` and clears `closingEntryId`/`closedAt`/`closedBy`. |
| `getJournalTemplates` | port (confirmed) | `accounting_get_journal_templates` | — | `JournalTemplate[]` | — | — | n/a (read) | Sorted by `name` (Arabic collation). |
| `getJournalTemplate` | port (confirmed) | `accounting_get_journal_template` | `{ id: string }` | `JournalTemplate` | — | — | n/a (read) | `NOT_FOUND` if missing — already correct. |
| `createOrUpdateJournalTemplate` | port (confirmed) | `accounting_create_or_update_journal_template` | `{ input: JournalTemplateInput, id?: string }` | `JournalTemplate` | `journalTemplates`, activity, audit | activity | not undoable | **Fixed this review** — see §8: now writes an activity/audit row (`saveJournalTemplate`), matching the master-data-audit-gap pattern already fixed for `settings.md`/`parties.md`/similar functions. Template lines get the same "resolves to a real account" + "AR/AP line needs a party" checks as a manual entry (`validateTemplateLines`), but **not** the full `validateManualLines` set — no `allowManual`/`active`/`isGroup` check on a template's lines at save time (only re-checked when the template is actually posted, via `postRecurringTemplate`→`recordManualJournal`→`validateManualLines`). This is intentional, not a gap: a template can reference an account that's since been deactivated and simply fail loudly at post time, which is the same lifecycle every other "saved-for-later" document in the app has (a draft, a held sale). |
| `removeJournalTemplate` | port (confirmed) | `accounting_remove_journal_template` | `{ id: string }` | — | `journalTemplates`, activity, audit | activity | not undoable (hard delete of master data) | **Fixed this review** — see §8: now writes an activity/audit row and takes a `userId` param (`deleteJournalTemplate`, `src/mocks/backend/journal.ts`), same pattern. **No reference check against `JournalEntry.templateId`** — confirmed **not a gap**, same reasoning `expenses.md` already established for `Expense.recurringTemplateId`: `templateId` is a historical breadcrumb no query anywhere reads back (confirmed by grep — only `postJournal`/`draftJournal` ever *write* it, nothing joins on it), so deleting a template can never orphan a live reference the way deleting an account referenced by a posted line would. |
| `loadTemplateIntoEntry` | port (confirmed) | `accounting_load_template_into_entry` | `{ id: string }` | `JournalTemplate` | — | — | n/a (read) | Thin wrapper — literally calls `getJournalTemplate(id)` under a different name for the entry form's "load a template" action. Rust can expose this as an alias of the same command rather than a second implementation, if the frontend's two call sites are ever worth collapsing — not required for parity, just a note. |
| `postRecurringTemplate` | port (confirmed) | `accounting_post_recurring_template` | `{ id: string }` | `JournalEntry` | `journalDrafts` **or** `journalEntries`, `counters`, `journalTemplates`, activity, audit | activity, ledger, numbering, period | none (posts through the same `recordManualJournal` as `createJournalEntry` — same undo story: none, a manual correcting entry only) | Posts a template's `nextDate` entry via `recordManualJournal` (full `validateManualLines` re-check happens **here**, at post time — see the `createOrUpdateJournalTemplate` note above), then calls `advanceRecurrence` to roll `recurrence.nextDate` forward by `every` (month/quarter/year). **Two writes that must commit together** (post the entry AND advance the date) — see §5, this module's sharpest concurrency finding lives here. |
| `getVatPeriodTotals` | port (confirmed) | `accounting_get_vat_period_totals` | `{ from: string, to: string }` | `VatPeriodTotals` | — | — | n/a (read) | `vatTotalsForPeriod` — output/input VAT summed straight from the GL's `vatOutput`/`vatInput` role accounts for the date range, not from documents (this is deliberately the ledger's own view, so it always reconciles with what `submitVatSettlement` is about to post — see §7). |
| `submitVatSettlement` | port (confirmed) | `accounting_submit_vat_settlement` | `{ from: string, to: string }` | `JournalEntry` | `journalEntries`, `counters`, activity, audit | activity, ledger, numbering, period | **not touched — see §9, D-A3** (no reversal function exists for a VAT settlement) | Refuses if both `outputVat` and `inputVat` are exactly 0 for the period (no VAT activity to settle). Posts Dr `vatOutput` (closes it) / Cr `vatInput` (closes it) / the net difference to `vatPayable` (credit if payable, debit if refundable) — a clean 2-or-3-line closing entry, dated `to`. **Nothing stops calling this twice for overlapping periods** (see §9 — would double-close the same VAT movement into `vatPayable`), a genuine gap that touches VAT math directly, so it's flagged, not guessed at. |
| `payVatSettlementNow` | port (confirmed) | `accounting_pay_vat_settlement_now` | `{ amount: number, paymentMethodId: string }` | `JournalEntry` | `vouchers`, `journalEntries`, `counters`, activity, audit | activity, ledger, numbering, period | none (routes through `vouchers.md`'s `createPaymentVoucher`-equivalent internals — same "no reversal, manual correction only" story as every voucher) | `payVatSettlement` — Dr `vatPayable` / Cr the chosen payment method's settlement account, via `recordPaymentVoucher` (the same shared voucher-posting helper `vouchers.md` already documented, reused here rather than hand-rolled). **No `amount <= vatPayable`'s current balance check** — a user could "pay" more than is actually owed, posting `vatPayable` into an unexpected debit balance with nothing to stop it. Not fixed (touches VAT-payable balance validation directly) — flagged in §9. **Also note**: unlike every other posting function in this module, this one has **no `allowClosedPeriod`/admin-override parameter at all** — it always posts through `recordPaymentVoucher`'s own period check with no admin bypass, an inconsistency worth a design decision (§9), not silently "fixed" to match the others. |

## 2. DTOs → Rust

| Type | Field | Rust type | Column | Why |
|---|---|---|---|---|
| `Account` | `id` | `Uuid` | `UUID` | UUIDv7 (D2). |
| `Account` | `code` | `String` | `VARCHAR(8)` with a `UNIQUE` index | `/^\d{1,8}$/` — digits only, up to 8. The **business key** every posting-adjacent doc (`docs/v2/03-chart-of-accounts.md`) refers to; never used for lookups by posting code (that's `accountFor(role)`'s job) but must stay unique and format-checked. |
| `Account` | `kind` | `enum AccountKind { Asset, Liability, Equity, Revenue, Expense }` | `ENUM('ASSET','LIABILITY','EQUITY','REVENUE','EXPENSE')` | `#[serde(rename_all = "UPPERCASE")]`. |
| `Account` | `subtype` | `enum AccountSubtype { Cash, Bank, Clearing, Receivable, Payable, Inventory, Tax, Prepaid, OtherCurrentAsset, FixedAsset, AccumulatedDepreciation, CurrentLiability, LongTermLiability, Equity, Revenue, OtherIncome, CostOfSales, OperatingExpense, OtherExpense, ZakatTax }` | `ENUM(...)` | `#[serde(rename_all = "camelCase")]` — 20-variant statement-classification enum, purely a reporting dimension (drives which financial-statement section an account rolls into), never read by any posting rule reviewed in this module. |
| `Account` | `normalSide` | `enum NormalSide { Debit, Credit }` | `ENUM('DEBIT','CREDIT')` | `#[serde(rename_all = "UPPERCASE")]` — drives `signedBalance`'s sign flip; this is a genuinely load-bearing field for every balance computation in every module reviewed this session (`parties.md`'s customer/supplier balances, this module's own trial balance / balance sheet identity). |
| `Account` | `systemRole` | `Option<enum SystemRole { ... 36 variants ... }>` | `ENUM(...)` nullable, plus a **partial unique index** (`WHERE system_role IS NOT NULL`) scoped by whatever `ctx` dimension Part 02/09 ends up keying on (today: effectively one row per role, `accountFor` picks the first branch-less/currency-less match) | `#[serde(rename_all = "camelCase")]`. **The single most load-bearing enum in the whole backend** — every posting function across every module reviewed this session resolves its accounts through `accountFor(role)`, never a hard-coded id or code (F3 in `02-accounting-review.md`, already fixed in v2). Rust's `shared::ledger::resolve_account(role, ctx)` **must** be the equivalent one-and-only lookup path — do not let any `domains/*/service.rs` query `accounts` by `code` or a hard-coded id, ever, mirroring the mock's own file-header comment in `accounts.ts` ("this is the ONLY place... that should know account codes"). |
| `Account` | `currency` | `Option<String>` | `CHAR(3)` nullable | ISO 4217 code — inert until Phase 9 per the type's own doc comment (a foreign-currency-denominated cash/bank account); `accountFor`'s `ctx.currency` matching already exists in the mock, so this is "ready but not exercised by any seeded data yet," not a missing feature. |
| `Account` | `branchId` | `Option<Uuid>` | nullable FK → `branches` | Inert until Phase 9 (branch-scoped cash drawers) per the same doc comment — `settings.md`'s `createBranch` already creates one cash account per branch with this set, so it's not entirely unused, just not yet exercised by more than one branch's worth of accounts sharing a role. |
| `Account` | `requiresParty` | `bool` | `BOOLEAN` | B1's control-account rule: true on AR/AP-type accounts, enforced by `validateManualLines` (a manual line on a `requiresParty` account needs `partyId`) and implicitly by every posting function that resolves `receivable`/`payable` and always attaches a party. |
| `Account` | `allowManual` | `bool` | `BOOLEAN` | B1's other control-account rule: **false** for inventory, VAT input, VAT output (and any other system-only account) — blocks `validateManualLines` from ever letting a human post there directly. This is arguably the single most important boolean in the accounting core: get it wrong and a manual journal entry can silently break the "GL(inventory) = Σ stockValue" / VAT invariants that every other module's postings keep intact. |
| `Account` | `requiresCostCenter` | `Option<bool>` | `BOOLEAN` nullable, default `false` | Inert until `db.settings.features?.costCenters` is on (v2 phase 9) — only enforced by `validateManualLines` when that feature flag is set; ties into `settings.md`'s cost-center CRUD (owned there) and `payments.md`/`vouchers.md`/`expenses.md`'s already-documented `costCenterId` fallback-to-branch-default behavior. |
| `Account` | `canDelete` | `bool` | `BOOLEAN` | **Server-computed, never client-set** on create (`saveAccount`: new accounts get `canDelete: true` unconditionally; `false` only via direct seed insertion for system-role accounts) — Rust must reproduce this asymmetry: the create path always sets `true`, and there is no service-layer path that ever flips an existing account's `canDelete` to `false` after creation. `saveAccount`'s own guard (code/kind/isGroup immutable when `canDelete: false`) is what actually protects seeded system accounts, not a separate permission check. |
| `AccountWithBalance` (service-layer type, computed) | `balance` / `debitTotal` / `creditTotal` | `Decimal` (all three) | **not stored** — computed per-request from `journal_lines` grouped by `account_id`, exactly like `parties.md`'s `Customer.balance` | `DECIMAL(19,2)`, `round2` (`signedBalance`, and `round2(t.d)`/`round2(t.c)` in `getAccounts`). |
| `AccountWithBalance` | `hasPostings` | `bool` | — (computed: `totals.has(a.id)`) | Distinguishes "zero balance because nothing posted yet" from "zero balance because debits and credits cancelled" — used by the CoA page to grey out never-used accounts. |
| `JournalEntry` | `id` | `Uuid` | `UUID` | UUIDv7 (D2). |
| `JournalEntry` | `number` | `String` | `VARCHAR`, unique per branch (or globally — confirm against `nextNumber('journal')`'s actual scope in `mocks/db.ts`, not re-derived here since numbering scope is a `cross-cutting.md` concern) | Sequential document number, same "shared counter, prefixed" pattern every other module's posting functions use. |
| `JournalEntry` | `type` | `enum JournalEntryType { System, Manual, Opening, Closing, VatSettlement }` | `ENUM('SYSTEM','MANUAL','OPENING','CLOSING','VAT_SETTLEMENT')` | `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` (or explicit per-variant `#[serde(rename = "...")]` for the 2-word variants) — **this field is a UI label only**, per the type's own doc comment: every value posts through the identical `postJournal` choke point, `type` never gates any validation branch differently (confirmed by reading every call site — `reverseJournal` checks `original.type !== 'MANUAL'` to *refuse* reversing a non-manual entry, which is the one place `type` is read for a decision, not to change posting behavior). |
| `JournalEntry` | `status` | `enum JournalEntryStatus { Draft, Posted }` | discriminates which **table** the row lives in (`journal_entries` vs `journal_drafts`), not a column value within one table — see the entity-design note below | `#[serde(rename_all = "UPPERCASE")]`. **Important modeling decision for Part 02-B**: the mock keeps drafts and posted entries in two entirely separate arrays (`db.journalEntries` vs `db.journalDrafts`) specifically so every GL/balance/invariant reader that scans `journalEntries` never has to filter out drafts (`invariants.ts`'s own `checkDraftsIsolated` exists purely to guard this separation). Rust should very likely keep this as **two tables**, not one `journal_entries` table with a `status` column that every single GL query then has to remember to filter on — a single shared table is a real, easy-to-reintroduce accounting bug (one un-filtered report query and a draft silently counts toward a balance). Flagged as a firm recommendation, not left ambiguous, precisely because getting it wrong here breaks the invariant every other invariant depends on. |
| `JournalEntry` | `sourceRef` | `Option<{ kind: JournalSourceKind, id: Uuid, number: Option<String> }>` | `source_kind ENUM(...) NULL`, `source_id UUID NULL`, `source_number VARCHAR NULL` | Only set on `type: 'SYSTEM'` entries posted by other modules (`postJournal({ sourceRef: ... })`); `MANUAL`/`OPENING`/`CLOSING`/`VAT_SETTLEMENT` entries posted **from this module** never set it (confirmed: no call site in `journal.ts`/`accountingService.ts` passes `sourceRef`) — so a Rust `NULL` here should be read as "this entry originated from a human or from this module's own wizards," not as missing data. |
| `JournalSourceKind` | (enum) | `enum JournalSourceKind { Invoice, Refund, PurchaseOrder, PurchaseReturn, Payment, StockAdjustment, Expense, Voucher, Settlement, Shift, FxReval, Opening }` | `ENUM(...)` | `#[serde(rename_all = "camelCase")]`. **Cross-module reuse note** (flagged by `parties.md`'s own §9): `PartyStatementRow.kind` should share this exact enum rather than being redeclared — now confirmed here as the canonical definition, since this module owns `JournalSourceKind`. |
| `JournalEntry` | `lines` | `Vec<JournalLine>` | child table `journal_lines(id, journal_entry_id, account_id, description, debit, credit, party_kind NULL, party_id NULL, branch_id, cost_center_id NULL, currency, amount_fc NULL, rate NULL)` | One-to-many, standard. |
| `JournalLine` | `debit` / `credit` | `Decimal` (both) | `DECIMAL(19,2)` | `round2`'d in `resolvePosting` (`round2(l.debit ?? 0)`, `round2(l.credit ?? 0)`) — **exactly one of the two is non-zero per line**, enforced by `resolvePosting` dropping zero-lines and by `validateManualLines`'s explicit "can't be both debit and credit" check (invariant §4.1's `no-both-sided-lines` check is the runtime guard for this). |
| `JournalLine` | `partyKind` | `Option<enum PartyKind { Customer, Supplier }>` | `ENUM('customer','supplier')` nullable | Reuse whatever `parties.md` names this (same concept, not accounting-specific). |
| `JournalLine` | `amountFc` / `rate` | `Option<Decimal>` (both) | `DECIMAL(19,4)` / `DECIMAL(19,6)` | Same FX-field scale convention `settings.md`/`payments.md` already established. `invariants.ts`'s `checkFxConversion` (not one of the 11 numbered invariants, but part of `runAllInvariants()`) checks `base = round2(amountFc × rate)` within 0.01 for every line that sets both — Rust's FX-line insertion must satisfy this exactly, and this module doesn't itself create any FX lines (no call site in `journal.ts`/`accountingService.ts` sets `amountFc`/`rate`) — it only needs to **carry** them correctly for entries other modules post. |
| `JournalEntry` | `reversed` / `reversalOfId` / `reversalReason` | `bool` (default `false`) / `Option<Uuid>` / `Option<String>` | `BOOLEAN NOT NULL DEFAULT FALSE`, nullable FK → `journal_entries.id`, `TEXT` nullable | B3's fix: reversal is a **new mirrored entry**; the original is flagged `reversed: true` and keeps its own copy of the reason (both entries store `reversalReason` — confirmed by `reverseJournal`'s `mutate()` block setting it on both `reversal` and `original`). `reversalOfId` only ever points from the reversal **to** the original, never the reverse — the original has no `reversedById` column, that's a **computed** lookup (`getJournalEntry`'s `reversal = db.journalEntries.find(e => e.reversalOfId === id)`), so Rust doesn't need a `reversed_by_id` column, just the one-directional FK plus a reverse lookup at read time. |
| `JournalEntry` | `templateId` | `Option<Uuid>` | nullable FK → `journal_templates.id`, **no `ON DELETE` cascade/restrict needed** (§8/§1 — confirmed a pure historical breadcrumb, same as `expenses.md`'s `recurringTemplateId`) | Set when an entry was posted from a template (manual or recurring) — never read back by any query. |
| `JournalEntry` | `attachmentIds` | `Vec<Uuid>` | child table or JSON array | Same pattern as every other module's `attachmentIds`. |
| `FiscalYear` | `id` | `Uuid` | `UUID` | UUIDv7. |
| `FiscalYear` | `startDate` / `endDate` | `NaiveDate` (both) | `DATE` | Business-local dates, not instants — `assertOpenPeriod`'s range check (`fy.startDate <= key && fy.endDate >= key`) is a plain string/date comparison on `localDateKey` output, so Rust's equivalent must use the same local-date semantics `cross-cutting.md` (01.D) settles once for the whole app, not a UTC-instant comparison. |
| `FiscalYear` | `isClosed` | `bool` | `BOOLEAN` | The period-lock gate `assertOpenPeriod` checks on every single posting in the entire application — this single boolean, plus `settings.accounting.lockDate`, is the **entire enforcement mechanism** for B2's "nothing stops posting into a closed period" fix. Get the transactional semantics of flipping this bit wrong (§5) and every module's `assertOpenPeriod` check becomes racy. |
| `FiscalYear` | `closingEntryId` / `closedAt` / `closedBy` | `Option<Uuid>` / `Option<DateTime<Utc>>` / `Option<Uuid → users.id>` | nullable FK / `DATETIME(3)` / nullable FK | `closedAt` is an instant (`new Date().toISOString()`), not a business date — UTC `DATETIME(3)` per the cross-cutting rule. |
| `JournalFilter` | `to` | `Option<NaiveDate>` | — (request-only filter, not persisted) | **Fourth confirmed instance** of the generator-hint false positive already logged against `payments.md`'s `PaymentFilter.to`, `vouchers.md`'s `VoucherFilter.to`, and `expenses.md`'s `ExpenseFilter.to` — `inDateRange(e.date, filter.from, filter.to)` uses it as a plain date-range bound, not a route. No action needed beyond the note already carried in the cross-cutting doc. |
| `JournalFilter` | `sourceKind` | `Option<JournalSourceKind>` | — (request filter) | Reuses the same enum as `JournalEntry.sourceRef.kind`. |
| `JournalFilter` | `minAmount` / `maxAmount` | `Option<Decimal>` (both) | — (request filter) | Filters on `totalDebit` only (`e.totalDebit >= filter.minAmount`), **not** `Math.max(totalDebit, totalCredit)` — since every entry is balanced by construction (`totalDebit === totalCredit` always, enforced by `resolvePosting`), this is a distinction without a difference, not a bug. |
| `JournalTemplate` | `id` | `Uuid` | `UUID` | UUIDv7. |
| `JournalTemplate` | `lines` | `Vec<{ accountId, description?, debit, credit, partyKind?, partyId? }>` | child table `journal_template_lines(...)` | **Narrower than `JournalLine`** — no `branchId`/`costCenterId`/`currency`/`amountFc`/`rate` fields (confirmed against the type definition, `types/index.ts:203-210`), since a template is filled in at post time via `recordManualJournal`'s normal line-resolution, which supplies branch/cost-center defaults then. |
| `JournalTemplate.recurrence` | `every` | `enum RecurrenceInterval { Month, Quarter, Year }` | `ENUM('month','quarter','year')` | `#[serde(rename_all = "lowercase")]`. |
| `JournalTemplate.recurrence` | `day` | `i32` | `TINYINT` | **Unlike `expenses.md`'s `RecurringExpense.day`, this one is never validated against a `[1,28]` range anywhere in this module** — `advanceRecurrence` just does `next.setMonth(next.getMonth()+1)` (or `+3`/`+1 year`) on the stored `nextDate` directly, with **no day-clamping at all**. This means a template recurring `every: 'month'` with `nextDate` on the 31st will silently skip or shift in JS `Date`'s native month-rollover behavior (e.g. Jan 31 + 1 month → Mar 3, not Feb 28) — a genuine, different-shaped bug from `expenses.md`'s (which deliberately caps at 28 to avoid exactly this). **Flagged in §8/§9, not fixed**: this is arguably accounting-adjacent (a recurring depreciation/rent entry silently drifting its posting date by a few days over several cycles is a real-world nuisance, though not a GL-correctness bug — the entry that does post is still fully valid and balanced), and the "right" fix (clamp to 28, like expenses; or use last-day-of-month semantics; or a real `chrono` `checked_add_months`) is a product decision about what "day of month" should mean for a template, not a narrow implementation bug to silently pick. |
| `JournalTemplate.recurrence` | `nextDate` | `NaiveDate` | `DATE` | Advanced by `advanceRecurrence` — see the `day` note above for the exact non-clamped `setMonth`/`setFullYear` arithmetic Rust must reproduce (or fix, per a product decision). |
| `JournalTemplate.recurrence` | `autoPost` | `bool` | `BOOLEAN` | **Same "persisted but never read" pattern `expenses.md` already found and flagged as D-open** for `RecurringExpense.autoPost` — confirmed by grep: nothing in `journal.ts`/`accountingService.ts` ever reads `template.recurrence.autoPost` to decide whether to post automatically; `postRecurringTemplate` is the only way a due template's entry ever posts, and it's always an explicit user/insight-engine-triggered call. This is the **same open product question** `expenses.md`'s §9 already raised for the identical field shape on a different entity — not re-litigated here, just cross-referenced (see §9). |
| `CloseYearPreCheck` | `key` | `enum CloseYearPreCheckKey { Drafts, TrialBalance, OpeningEquity }` | — (never persisted, a read-only computed shape) | `#[serde(rename_all = "camelCase")]`. |
| Route field | `link` on every `logActivity` call in this module (`/accounting/accounts` ×2, `/accounting/journal/${id}` ×3, `/accounting/fiscal-years` ×3, plus the two newly-added `/accounting/journal-templates` calls from this review's fix — see §8) | `RouteRef { name, params }` | — | 10 of the 61 (now +2 = up to whatever the current running count is after this review's fix — the shared F7 pass in 01.C should re-count via `bun run contract`) path-string links — tracked in §8 for the shared 01.C pass, not fixed individually here (matches every prior module's treatment of F7). |

## 3. Validation and errors

| Function | Rule (source: mock check line) | Code | Exact Arabic message |
|---|---|---|---|
| `saveAccount` (`validateAccount`) | `code` matches `/^\d{1,8}$/` | `VALIDATION` (default code) | `رمز الحساب أرقام فقط (حتى 8 أرقام)` |
| `saveAccount` | `name` non-empty after trim | `VALIDATION` (default code) | `اسم الحساب مطلوب` |
| `saveAccount` | `code` unique among other accounts | `CONFLICT` | `رمز الحساب مستخدم من قبل` |
| `saveAccount` | `parentId`, if set, ≠ the account's own id | `VALIDATION` (default code) | `لا يمكن أن يكون الحساب أباً لنفسه` |
| `saveAccount` | parent must exist | `VALIDATION` (default code) | `الحساب الأب غير موجود` |
| `saveAccount` | parent must be a group (`isGroup: true`) | `VALIDATION` (default code) | `الحساب الأب يجب أن يكون حساباً رئيسياً (تجميعياً)` |
| `saveAccount` | parent's `kind` must match the input's `kind` | `VALIDATION` (default code) | `` الحساب الأب من نوع مختلف ({parent.kind}) `` |
| `saveAccount` | `code` must start with the parent's `code` | `VALIDATION` (default code) | `` رمز الحساب يجب أن يبدأ برمز الحساب الأب ({parent.code}) `` |
| `saveAccount` | a group account can't have `allowManual: true` | `VALIDATION` (default code) | `الحسابات الرئيسية (التجميعية) لا تقبل الترحيل المباشر` |
| `saveAccount` (update path) | account must exist | `NOT_FOUND` | `الحساب غير موجود` |
| `saveAccount` (update path) | a system account (`!canDelete`) can't change `code`/`kind`/`isGroup` | `VALIDATION` (default code) | `لا يمكن تغيير رمز أو نوع أو شكل (رئيسي/فرعي) حساب أساسي في النظام` |
| `saveAccount` (update path, leaf→group flip) | refused if the account already has children | `VALIDATION` (default code) | `للحساب حسابات فرعية — لا يمكن جعله فرعياً (postable)` — **note: this Arabic message is inverted from what the check does** (it fires when trying to make a *group* into a *leaf* while it has children, but the message says "can't make it leaf/postable" — actually reads correctly on closer inspection: the message IS about not being able to make it a leaf ("postable") because it has children; not a bug, just worth flagging that the two flip-direction messages are easy to misread side-by-side — see the next row for the other direction). |
| `saveAccount` (update path, group→leaf flip) | refused if the account has posted journal lines | `VALIDATION` (default code) | `للحساب قيود مسجلة — لا يمكن جعله رئيسياً (تجميعياً)` |
| `deleteAccount` | account must exist | `NOT_FOUND` | `الحساب غير موجود` |
| `deleteAccount` | system account (`!canDelete`) can't be deleted | `VALIDATION` (default code — **note**: this is the same "protected record" refusal class `settings.md`/`expenses.md` already fixed to `FORBIDDEN` for their equivalents; this one is **left as the default `VALIDATION` code**, a real but narrow error-code-precision gap. Not fixed in this review — see §8 for why.) | `حساب أساسي في النظام ولا يمكن حذفه` |
| `deleteAccount` | account must have no children | `CONFLICT` | `للحساب حسابات فرعية — احذفها أولاً` |
| `deleteAccount` | account must have no posted journal lines | `CONFLICT` | `للحساب قيود مسجلة — يمكنك إيقافه بدلاً من حذفه` |
| `reparentAccount` | account must exist | `NOT_FOUND` | `الحساب غير موجود` |
| `reparentAccount` | `newParentId` ≠ the account's own id | `VALIDATION` (default code) | `لا يمكن أن يكون الحساب أباً لنفسه` |
| `reparentAccount` | new parent, if given, must exist | `VALIDATION` (default code) | `الحساب الأب غير موجود` |
| `reparentAccount` | new parent must be a group | `VALIDATION` (default code) | `الحساب الأب يجب أن يكون حساباً رئيسياً (تجميعياً)` |
| `reparentAccount` | new parent's `kind` must match | `VALIDATION` (default code) | `` لا يمكن نقل الحساب إلى مجموعة من نوع مختلف ({cursor.kind}) `` |
| `reparentAccount` | no cycles (new parent can't be a descendant of the account) | `VALIDATION` (default code) | `لا يمكن نقل الحساب إلى أحد فروعه` |
| `createJournalEntry`/`updateJournalDraft`/`postRecurringTemplate` (via `validateManualLines`) | `description` non-empty after trim | `VALIDATION` (default code) | `أدخل بيان القيد` |
| same (`validateManualLines`) | ≥ 2 lines | `VALIDATION` (default code) | `يجب أن يحتوي القيد على سطرين على الأقل` |
| same | every line's `accountId` must resolve | `VALIDATION` (default code) | `اختر الحساب لكل سطر` |
| same | the account must be `active` | `VALIDATION` (default code) | `` الحساب "{name}" غير نشط `` |
| same | the account must not be a group | `VALIDATION` (default code) | `` "{name}" حساب رئيسي (تجميعي) ولا يقبل الترحيل المباشر `` |
| same | `debit`/`credit` can't be negative | `VALIDATION` (default code) | `المبالغ لا يمكن أن تكون سالبة` |
| same | a line can't have both `debit > 0` and `credit > 0` | `VALIDATION` (default code) | `السطر الواحد إما مدين أو دائن` |
| same | **B1**: a non-zero line on an `allowManual: false` account is refused | `FORBIDDEN` | `` لا يمكن الترحيل يدوياً على حساب "{name}" — استخدم تسوية المخزون أو تسوية ضريبة القيمة المضافة `` |
| same | **B1**: a non-zero line on a `requiresParty` account needs `partyId` | `VALIDATION` | `` السطر على حساب "{name}" يتطلب اختيار عميل أو مورد `` |
| same | a non-zero line on a `requiresCostCenter` account (only when `settings.features.costCenters` is on) needs `costCenterId` | `VALIDATION` | `` السطر على حساب "{name}" يتطلب اختيار مركز تكلفة `` |
| all posting functions (via `resolvePosting`) | Σdebit = Σcredit within 0.001 | `VALIDATION` (default code) | `` القيد غير متوازن: المدين {x} ≠ الدائن {y} `` — the shared ledger choke-point's own defense; should never actually fire given every caller's hand-built or pre-validated lines. |
| `postJournal` | `resolvePosting`'s kept (non-zero) lines must number ≥ 2 | `VALIDATION` (default code) | `يجب أن يحتوي القيد على سطرين على الأقل` — a **second**, later check than `validateManualLines`'s own ≥2 check, catching the case where zero-amount lines were dropped and fewer than 2 non-zero lines remain. |
| all posting functions (via `assertOpenPeriod`) | date inside an open, unlocked fiscal period | `FORBIDDEN` | `` لا يمكن الترحيل في تاريخ {date} — الفترة مقفلة حتى {lockDate} `` / `` …السنة المالية "{name}" مقفلة `` |
| `updateDraftJournal` | draft must exist | `NOT_FOUND` | `المسودة غير موجودة` |
| `deleteJournalDraft` | draft must exist | `NOT_FOUND` | `المسودة غير موجودة` |
| `postJournalDraft` | draft must exist | `NOT_FOUND` | `المسودة غير موجودة` |
| `reverseJournalEntry` | entry must exist | `NOT_FOUND` | `القيد غير موجود` |
| `reverseJournalEntry` | entry's `type` must be `'MANUAL'` | `VALIDATION` (default code) | `القيود الآلية تُعكس من المستند المصدر (مرتجع/إلغاء)` |
| `reverseJournalEntry` | entry must not already be reversed | `VALIDATION` (default code) | `هذا القيد معكوس بالفعل` |
| `reverseJournalEntry` | `reason` non-empty after trim | `VALIDATION` (default code) | `سبب العكس مطلوب` |
| `saveFiscalYear` | `name` non-empty after trim | `VALIDATION` (default code) | `اسم السنة المالية مطلوب` |
| `saveFiscalYear` | `endDate > startDate` (both required) | `VALIDATION` (default code) | `تاريخ النهاية يجب أن يكون بعد تاريخ البداية` |
| `saveFiscalYear` | no date-range overlap with another fiscal year | `CONFLICT` | `` الفترة تتداخل مع السنة المالية {overlap.name} `` |
| `saveFiscalYear` (update path) | fiscal year must exist | `NOT_FOUND` | `السنة المالية غير موجودة` |
| `closeYearPreChecks`/`closeYear` | fiscal year must exist | `NOT_FOUND` | `السنة المالية غير موجودة` |
| `closeYear` | year must not already be closed | `VALIDATION` (default code) | `السنة المالية مقفلة بالفعل` |
| `closeYear` | all 3 pre-checks must pass (server-side re-check, not trusting a client that already saw green) | `FORBIDDEN` | `` تعذر إقفال السنة: {failed.label} — {failed.detail} `` |
| `reopenYear` | caller must be `role === 'admin'` (checked in the service layer, before calling `reopenFiscalYear`) | `FORBIDDEN` | `إعادة فتح السنة المالية للمدير فقط` |
| `reopenYear` | fiscal year must exist | `NOT_FOUND` | `السنة المالية غير موجودة` |
| `reopenYear` | year must currently be closed | `VALIDATION` (default code) | `السنة المالية غير مقفلة` |
| `saveJournalTemplate` | `name` non-empty after trim | `VALIDATION` (default code) | `اسم القالب مطلوب` |
| `saveJournalTemplate` (`validateTemplateLines`) | ≥ 2 lines | `VALIDATION` (default code) | `يجب أن يحتوي القالب على سطرين على الأقل` |
| same | every line's `accountId` must resolve | `VALIDATION` (default code) | `اختر الحساب لكل سطر` |
| same | a non-zero line on a `requiresParty` account needs `partyId` | `VALIDATION` | `` السطر على حساب "{name}" يتطلب اختيار عميل أو مورد `` |
| `saveJournalTemplate` (update path) | template must exist | `NOT_FOUND` | `القالب غير موجود` |
| `removeJournalTemplate`/`getJournalTemplate`/`loadTemplateIntoEntry` | template must exist | `NOT_FOUND` | `القالب غير موجود` |
| `postRecurringTemplate` | template must exist | `NOT_FOUND` | `القالب غير موجود` |
| `postRecurringTemplate` | template must actually have a `recurrence` block | `VALIDATION` (default code) | `القالب ليس متكرراً` |
| `getJournalEntry` | entry (posted or draft) must exist | `NOT_FOUND` | `القيد غير موجود` |
| `submitVatSettlement` | at least one of `outputVat`/`inputVat` must be non-zero for the period | `VALIDATION` (default code) | `لا توجد حركة ضريبية في هذه الفترة` |
| `payVatSettlementNow` | `amount > 0` | `VALIDATION` (default code) | `لا يوجد مبلغ مستحق للسداد` — **note: this only checks `amount > 0`, not `amount <= currentVatPayableBalance`** (see §1's note on this function and §9). |

No dedicated Zod schema for this module — every rule above is inline `ApiError` checks in
`accountingService.ts`/`journal.ts`/`core.ts`, matching every "fast journal"/posting module
reviewed this session (`payments.md`, `vouchers.md`, `expenses.md`).

## 4. Undo matrix (every function that writes)

| Function | Undoable? | Compensation (existing fn) | Refused when | Period rule (D7) |
|---|---|---|---|---|
| `saveAccount` | No (master-data edit) | — | n/a | n/a — not itself period-scoped |
| `deleteAccount` | No (hard delete, master data — already fully reference-checked: no children, no postings) | — | n/a | n/a |
| `reparentAccount` | No | — | n/a | n/a |
| `createJournalEntry` (post path) | **Yes, via `reverseJournalEntry`** — but **only** for `type: 'MANUAL'` entries (every entry this function itself creates when `!asDraft` IS `type: 'MANUAL'`, so this function's own output is always reversible). | `reverseJournalEntry` | n/a to create itself | Standard `assertOpenPeriod` at post time, `allowClosedPeriod: isAdmin` |
| `createJournalEntry` (draft path) | n/a — a draft has no GL effect; `deleteJournalDraft`/`updateJournalDraft` are its own lifecycle, not an "undo" of a posting | `deleteJournalDraft` (delete the draft) or `updateJournalDraft` (edit it) | n/a | **No period check at all on the draft path** — see §9, D-A4 |
| `updateJournalDraft` | n/a (no GL effect to undo) | — | n/a | n/a |
| `postJournalDraft` | **Yes, via `reverseJournalEntry`** — the posted result is `type: 'MANUAL'` (inherited from the draft, which is always created as `type: 'MANUAL'` per `draftJournal`'s hard-coded `type: 'MANUAL'`), so it's reversible the same way `createJournalEntry`'s post path is. | `reverseJournalEntry` | n/a | Standard `assertOpenPeriod` at post time — **but see §9, D-A1: no re-validation of the lines' account state at post time**, only the period check. |
| `deleteJournalDraft` | n/a (no GL effect) | — | n/a | n/a |
| `reverseJournalEntry` | **This function IS the undo action for a `MANUAL` entry — it has no undo of its own** (reversing a reversal isn't a thing the mock supports; the reversal itself is a fresh `MANUAL` entry that could in principle be reversed again, but nothing in the codebase does this and it isn't a modeled workflow). Matches `payments.md`'s and `vouchers.md`'s conclusion that the compensating action itself needs no further undo. | n/a | Refuses on a non-`MANUAL` original, an already-reversed original, or a missing reason | Standard `assertOpenPeriod` on the reversal's own (caller-chosen) date, `allowClosedPeriod: isAdmin` |
| `saveFiscalYear` | No | — | n/a | n/a — but see §9 for the "editing an already-closed year's dates" gap |
| `saveLockDate` | No (but has no accounting *effect* to undo either — see §1, confirmed safe) | — | n/a | n/a |
| `closeYear` | **Yes, exactly via `reopenYear`** — this is the one pair in the whole module (and one of the very few in the whole app, alongside `settings.md`'s `deactivateBranch`/`reactivateBranch`) where the compensating action is a **named, purpose-built function**, not the generic reversal registry. `reopenYear` reverses the exact closing entry and unlocks the year — this is about as close to "real undo" as anything in the mock gets. | `reopenYear` (admin-only) | Refuses if any pre-check fails (drafts exist, trial balance unbalanced, opening-equity nonzero) | Closing entry posts with `allowClosedPeriod: true` (posts into the year being closed, on its own end date) |
| `reopenYear` | **This function IS the undo for `closeYear`.** Reopening again after a reopen would just be calling `closeYear` again — a fresh close, not an "undo of the undo." | n/a | Refuses if not admin, year doesn't exist, or year isn't currently closed | Reversal posts with `allowClosedPeriod: true`, dated "now" (not the original close date — see §9, D-A2's date-choice question) |
| `createOrUpdateJournalTemplate` | No (master-data edit — a template itself never touches the GL; only *posting* it does, and that's `postRecurringTemplate`'s job, already covered above via `createJournalEntry`'s logic) | — | n/a | n/a |
| `removeJournalTemplate` | No (hard delete, master data — confirmed not a live-reference violation, §1/§8) | — | n/a | n/a |
| `postRecurringTemplate` | **Yes, via `reverseJournalEntry`** for the posted entry itself (same `type: 'MANUAL'` reasoning as `createJournalEntry`) — **but `advanceRecurrence`'s `nextDate` roll-forward is never rolled back** if the posted entry is later reversed. If an accountant reverses a wrongly-posted recurring entry, the template's `nextDate` stays advanced by one period, so the *next* due date is now wrong relative to what actually got posted vs. reversed. This compound-undo gap is the **exact same shape** `expenses.md`'s §4 flagged for `postDueRecurringExpense`/`nextDate` — not a new bug class, the second confirmed instance of it, and equally "not a gap in the mock today" since nothing claims this compound scenario is handled — a Part 03 design note if a real undo feature is ever built for recurring postings, not an implementation detail to guess at now. | `reverseJournalEntry` (for the entry only — not the `nextDate` advance) | n/a | Standard `assertOpenPeriod`, `allowClosedPeriod: isAdmin` |
| `submitVatSettlement` | **No — no reversal function exists for a VAT settlement entry.** Same "once posted, undo means a manual correcting journal entry" conclusion `payments.md` reached for `createPayment` and `vouchers.md` reached for card settlements — except here it's sharper, because a VAT settlement's own posted entry is `type: 'VAT_SETTLEMENT'`, not `'MANUAL'`, so **`reverseJournalEntry` would refuse it outright** even if someone tried (the "القيود الآلية تُعكس من المستند المصدر" refusal fires, but there is no "source document" for a VAT settlement to reverse *from* — it's a dead end, not a working alternate path). This is worth stating precisely because it's a slightly different flavor of "no undo" than every other module's finding: those all have *some* theoretical path (however unimplemented); this one is actively blocked by the type check with nowhere else to go. Flagged in §9, not fixed (implementing a VAT-settlement-specific reversal is a scope decision, not a narrow bug). | none | n/a | Standard `assertOpenPeriod`, `allowClosedPeriod: isAdmin` |
| `payVatSettlementNow` | No (routes through the voucher-posting helper, same "no reversal, manual correction only" story as every voucher in `vouchers.md`) | none | n/a | Standard period check inside `recordPaymentVoucher`, **no admin override parameter exposed here at all** — see §9. |

## 5. Concurrency under D8 (several terminals on one DB)

| Race | Rows | Settled by |
|---|---|---|
| Two terminals create/edit an account with the same `code` at once | `accounts.code` | `UNIQUE` constraint on `accounts.code`, same pattern as every other module's code-uniqueness field (`branches.code`, `costCenters.code`, `taxes`… per `settings.md`). |
| Two terminals both try to `deleteAccount`/flip `isGroup` on the same account while a third terminal is mid-post to it | `accounts`, `journal_lines` | **A genuinely sharp race, sharper than most master-data deletes reviewed so far**, because the thing being protected against (a posted line referencing a since-deleted or since-grouped account) is a direct accounting-integrity violation, not just a UX inconvenience. Rust's `accounting_delete_account`/`accounting_save_account` (the group⇄leaf flip path) must take a locking read (`SELECT ... FOR UPDATE`) on the account row AND re-run the "no posted lines reference it" check **inside the same transaction** as any concurrent posting function's own account-resolution step — a naive "check then delete" without a lock could let a posting to that account commit in the gap. |
| Two terminals post manual journal entries (or anything else) at the same moment | `counters.journal` (`nextNumber('journal')`), `journal_entries` | Same "shared counter, prefixed" pattern as every posting module reviewed this session — the counter row needs `SELECT ... FOR UPDATE` for the duration of allocate-and-increment. |
| Two terminals post while a **third** terminal is mid-`closeYear` on the same fiscal year | `fiscal_years.is_closed`, `journal_entries`, `counters.journal` | **This module's sharpest concurrency finding, and arguably the sharpest in the whole plan so far.** `closeYear` computes account totals, decides the closing entry's lines, then posts (with `allowClosedPeriod: true`) and only *afterward* flips `isClosed`. If another terminal posts an ordinary entry into the same year **between** the closing-entry computation and the `isClosed` flip, that entry (a) was never included in the totals the closing entry just computed (so the closing entry is now wrong — revenue/expense accounts have a balance the retained-earnings sweep didn't account for) and (b) is now sitting inside a year that's about to become locked, with no error to anyone. Rust's `accounting_close_year` **must** hold an exclusive lock for its entire duration that blocks every other posting function's `assert_open_period` check against this fiscal year — not just a row lock on the `fiscal_years` row (which wouldn't stop a concurrent `postJournal` that only reads `is_closed`, not writes it), but something that makes "compute totals → post closing entry → flip is_closed" atomic with respect to every other poster. The cleanest mechanism: `assert_open_period` takes a locking read (`SELECT ... FOR UPDATE`) on the `fiscal_years` row for **every** posting, not just `close_year`'s own read — so a `close_year` transaction holding that row's lock for its full duration genuinely blocks concurrent posters from proceeding until it commits or rolls back. This is more invasive than any other lock recommended in this plan so far (it turns the fiscal-year row into a serialization point for every single posting in the whole app during a close), and is worth flagging to whoever designs `shared::ledger` in Part 02-C as a deliberate, load-bearing design constraint, not an incidental detail. |
| Two terminals call `reopenYear` on the same year at once | `fiscal_years.is_closed`, `journal_entries` | The second call's `!fy.isClosed` check (post-lock, inside the transaction) naturally refuses once the first has completed — needs the same row lock as above, but a much smaller blast radius (only matters for the instant of the reopen itself, not an ongoing posting stream). |
| Two terminals both call `postRecurringTemplate` on the same due template at nearly the same moment | `journal_templates.recurrence.next_date`, a doubled `journal_entries`/`counters` write | **The same shape of race `expenses.md`'s §5 already found for `postDueRecurringExpense`** — nothing in `postRecurringTemplate` locks or re-checks the template's `nextDate`/due status between whatever UI list showed it as due and the actual post call; the function only checks the template still *exists* and *has* a `recurrence` block, not that it's still due "now." Two concurrent calls would post the entry twice and the `advanceRecurrence` roll-forward would only advance by one period net (same double-post-single-advance shape `expenses.md` documented). Rust's `accounting_post_recurring_template` needs the identical fix: a locking read on the template row, re-verifying `nextDate <= today` inside the same transaction that posts the entry and advances the date. |
| Two terminals edit `db.settings.accounting.lockDate` at the same moment | `settings` (single row, shared per D8) | Same generic "settings is one shared row" concurrency note `settings.md`'s §5 already flagged for `updateSettings`'s shallow-merge-without-lock behavior — not a new finding, this module's `saveLockDate` writes into the same settings row and inherits that same open concurrency gap (tracked once in `settings.md`, not re-flagged as a second bug here). |

## 6. Events and side effects

- **Activity/audit rows** written for: `saveAccount`, `reparentAccount` (both `activityKind: 'journal'`),
  `createJournalEntry`'s post path (via `recordManualJournal`), `reverseJournalEntry` (via `reverseJournal`),
  `saveFiscalYear` (`activityKind: 'settings'` — a fiscal year is settings-adjacent master data, not
  itself a journal entry), `saveLockDate` (`activityKind: 'settings'`), `closeYear`/`reopenYear` (both
  `activityKind: 'journal'`), `submitVatSettlement`/`payVatSettlementNow` (both `activityKind: 'journal'`).
  **Fixed this review**: `createOrUpdateJournalTemplate`/`removeJournalTemplate` now also write an
  activity/audit row (see §8) — previously silent.
- **Still silent (deliberately not fixed — see §8)**: `updateJournalDraft`, `deleteJournalDraft`,
  `postJournalDraft` — none of the three draft-lifecycle functions writes an activity/audit row,
  even though `postJournalDraft` is the one function in this entire module that moves money onto the
  GL with **zero** audit trail (every other posting function in the app logs something). This is a
  genuine, precise finding — flagged in §8/§9, not silently folded into the generic "master-data
  writes lack audit trails" bucket, because `postJournalDraft` specifically is a *posting* action,
  not a master-data edit, and posting with no audit trail is a materially different severity than
  a silent category rename.
- **Events emitted**: `postJournal` (called by `createJournalEntry`'s post path, `reverseJournalEntry`,
  `closeYear`, `reopenYear`, `submitVatSettlement`, `payVatSettlementNow`'s underlying voucher call,
  and `postRecurringTemplate`) emits `ledger:changed` every time — this module is, like `vouchers.md`
  and `expenses.md`, **not** part of the `ledger:changed`-skipping pattern `invoices.md`/`purchases.md`/
  `payments.md`'s posting functions showed; every real posting here correctly signals the change.
  `postJournalDraft` **also** emits `ledger:changed` (confirmed: `core.ts`'s `postDraftJournal` has its
  own `emit('ledger:changed')` call, separate from `postJournal`'s) — so the event fires correctly even
  though the audit-trail call doesn't, an inconsistency worth naming precisely: the event bus knows
  something posted, but the audit log doesn't record who or why. `createJournalEntry`'s draft-save path
  (`draftJournal`) does **not** emit any event — correct, since nothing changed the GL. `saveAccount`/
  `reparentAccount`/`deleteAccount` emit **no** `catalog:changed`-equivalent event for the chart of
  accounts itself (there is no `accounts:changed`/`coa:changed` event anywhere in the app) — every page
  that shows account pickers must be re-fetching or relying on `ledger:changed` as a catch-all; not
  flagged as a bug since no other module's account-picker staleness has surfaced as an issue, but worth
  a note for Part 02-F's event-bridge design (a fourth event name may be warranted once multi-terminal
  makes staleness observable in ways the single-process mock never could).
- **Route links**: 10 (now 12, after this review's 2 additions) path-templated string links across
  this module — tracked in §8/§2 for the shared 01.C F7 pass.
- **Attachments**: `createJournalEntry`/`updateJournalDraft` accept `input.attachmentIds` and pass them
  straight to `draftJournal`/`postJournal`/`updateDraftJournal` — same pattern as every other posting
  module. Journal templates and fiscal years have no attachment support (not in their input types).
- **Printing**: none — journal entries have no dedicated print template (the closest is the general
  voucher print route `vouchers.md` already documented, not this module's).

## 7. Aggregations (reports / analytics / dashboard / insights only)

Not primarily a reports module, but three read endpoints compute real aggregations worth documenting
precisely for Part 03's SQL:

| Output (DTO field) | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `AccountWithBalance[]` (`getAccounts`) | `journalEntries` (flattened lines), `accounts` | lines where `date` in the optional `range` | by `accountId` | `round2` on `debitTotal`/`creditTotal`; `signedBalance` applies `round2` again to the net | `accountingService.ts:40-58` |
| `CloseYearPreCheck[]`'s trial-balance check (`trialBalanceFor`) | `journalEntries` (flattened lines), `accounts` | lines dated inside `[fy.startDate, fy.endDate]` (string-sliced date comparison, not `inDateRange` — same effective semantics but worth noting the different helper used, `core.ts:474` vs. the `inDateRange` helper every other module's date filters use) | by `accountId`, then summed into `debit`/`credit` per the account's net-debit-or-net-credit sign | `round2` per account, then `round2` on the two totals | `core.ts:471-493` |
| `VatPeriodTotals` (`getVatPeriodTotals`/`vatTotalsForPeriod`) | `journalEntries` (flattened lines), `accountFor('vatOutput')`/`accountFor('vatInput')` | lines dated `[from, to]` via `localDateKey` string comparison (again, not `inDateRange` — same effective result for same-format date strings) | — (two running sums, not grouped) | `round2` on `outputVat`, `inputVat`, and the derived `net` | `journal.ts:232-248` — **this is the ledger's own view of VAT for the period, deliberately not the document-level view** `invariants.ts`'s `checkVatControl` (§4.5) computes from `invoices`/`refunds`/`purchaseOrders`/`expenses` directly; the two should reconcile once nothing is mid-flight, but they are genuinely two different computations over two different table sets arriving at (hopefully) the same number — worth stating explicitly so Part 03 doesn't assume one can be derived from the other's SQL, they're independent queries that happen to agree. |
| `closeFiscalYear`'s own revenue/expense sweep (not a read endpoint, but the same aggregation shape) | `journalEntries` (flattened lines), `accounts` (REVENUE/EXPENSE kind, non-group) | lines dated inside `[fy.startDate, fy.endDate]`, same string-slice comparison as the pre-check | by `accountId`, net per account, then summed into one `net` figure | `round2` per account's net, `round2` on the aggregate `net` | `core.ts:547-575` |

## 8. Contract fixes needed in the mock (01.C)

Per CLAUDE.md's architectural-autonomy rule and this module's explicit "maximally conservative"
mandate: **one safe fix applied** (a master-data audit-trail gap, the same class already fixed in
`settings.md`/`parties.md`/`expenses.md`'s equivalents), confirmed **not** to touch posting, balance,
period, or reversal logic. `bun run verify:mocks` baseline was **128 ok, 0 failed** before this
review and remains **128 ok, 0 failed** after the fix.

- [x] **`createOrUpdateJournalTemplate`/`removeJournalTemplate` now write an activity/audit row.**
      `saveJournalTemplate` (`src/mocks/backend/journal.ts`) now calls `logActivity('journal', ...)`
      after create/update; `deleteJournalTemplate` now takes a `userId` parameter and calls
      `logActivity('journal', ...)` on delete, matching the pattern `settings.md` established for
      `deleteCostCenter`/`setBaseCurrency` (functions that needed a `userId` added to write an audit
      row). The caller (`accountingService.ts`'s `removeJournalTemplate`) now passes `session.userId`.
      This is a pure master-data audit-trail addition — a journal *template* has no GL effect on its
      own (only *posting* it, via `postRecurringTemplate`→`recordManualJournal`, does, and that path
      was already fully audited before this fix), so this carries zero accounting risk. Confirmed by
      the unchanged `verify:mocks` count.
- [ ] **`postJournalDraft` has no `logActivity` call at all** — the one function in this entire module
      (and, by inspection, one of very few in the whole app) that moves an entry onto the real GL with
      **zero** audit trail. **Deliberately not fixed.** This sits squarely inside "anything touching
      posting logic" per the task's explicit mandate: `postJournalDraft` IS the posting choke point for
      the draft-to-post flow, and adding a `logActivity` call there means deciding *what* to log (the
      original draft's `createdBy`? the poster's `userId`? does it matter that `editDraftJournal` may
      have silently changed the lines since the draft was first saved?) — small-looking questions that
      are nonetheless decisions about what the posting audit trail should say, not a mechanical
      addition like the template fix above. Flagged precisely in §9 (D-A5) for a human to decide with
      full context, not guessed at.
- [ ] **`updateJournalDraft`/`deleteJournalDraft` also have no `logActivity` call.** Considered fixing
      these two as "safe" (they have zero GL effect, so on the surface this looks like the same class
      of harmless master-data audit gap as the template fix above) — but decided **not to**, because
      they are inseparable from `postJournalDraft`'s identical gap in the same three-function draft
      lifecycle: fixing 2 of 3 draft functions while deliberately leaving the third (the posting one)
      unfixed would produce an inconsistent, half-audited draft trail that's arguably worse than the
      current uniformly-silent one, and the "what to log" question for `updateJournalDraft` (does an
      edit's diff matter here the way `settings.md`'s `updateSettings` diffs do?) has the same shape of
      open design question as `postJournalDraft`'s. Tracked as one item in §9 (D-A5), not two separate
      "safe" fixes plus one deferred — the three belong together.
- [ ] **`deleteAccount`'s "system account, can't delete" refusal uses the default `VALIDATION` code,
      not `FORBIDDEN`** (§3) — the same class of error-code-precision gap `settings.md` fixed for
      `deletePaymentMethod`/`deleteTax` and `expenses.md` fixed for `deleteExpenseCategory`.
      **Deliberately not fixed here**, unlike those precedents, because `deleteAccount` sits one level
      closer to the posting engine than a payment method or expense category — an account's `canDelete`
      flag is load-bearing for `accountFor(role)` resolution across every module reviewed this session,
      and while changing only the error *code* (not the refusal logic) is very likely just as safe as
      the precedent fixes, this review's mandate is to apply zero fixes adjacent to the accounting core
      unless unambiguous, and "is an account-protection error code adjacent enough to count" is close
      enough to the line that it's worth a human's explicit yes rather than this review's own judgment
      call. Flagged in §9 as a low-risk, easy candidate for a human to wave through, not silently
      applied.
- [ ] **`JournalTemplate.recurrence.day` has no `[1,28]` validation and `advanceRecurrence` doesn't
      clamp the day when rolling `nextDate` forward** (§2) — unlike `expenses.md`'s `RecurringExpense`,
      which validates `day` at save time and never hits a month-length edge case as a result. A
      template with `nextDate` on the 29th–31st will drift under `Date`'s native month-rollover
      behavior. **Deliberately not fixed**: this touches the exact date a recurring entry posts on,
      which is accounting-adjacent even though it doesn't corrupt any posted entry's own correctness —
      flagged precisely in §9, not guessed at, since the right fix (clamp like expenses, or a different
      "day of month" semantics) is a product decision, not a narrow bug fix.
- [ ] **`submitVatSettlement` has no guard against being called twice for overlapping/identical
      periods** (§1/§4) — would double-close the same output/input VAT movement into `vatPayable`,
      a genuine VAT-math risk. **Deliberately not fixed** — this is squarely "anything touching...
      balance validation" per the task mandate. Flagged in §9 (D-A6), not guessed at.
- [ ] **`payVatSettlementNow` has no check that `amount` doesn't exceed the current `vatPayable`
      balance** (§1/§3) — could post `vatPayable` into an unexpected debit position with nothing to
      stop it. **Deliberately not fixed** — same reasoning as the settlement-twice gap above, this is
      balance validation on a control account. Flagged in §9 (D-A7).
- [ ] **`postJournalDraft` doesn't re-run `validateManualLines`'s account-state checks at post time**
      (§1/§4/§9) — an account could have been deactivated, turned into a group, or lost `allowManual`
      between when the draft was saved and when it's posted, and the draft would post anyway (only
      `assertOpenPeriod` is checked). **Deliberately not fixed** — this is the posting choke point's
      validation guarantee, the single most explicitly-named "do not touch" category in the task brief.
      Flagged precisely in §9 (D-A1).
- [ ] **F7 (shared task, still pending)**: this module owns 10 (now 12, after this review's 2
      additions) of the 61-and-growing path-templated string links (`/accounting/accounts` ×2,
      `/accounting/journal/${id}` ×3, `/accounting/fiscal-years` ×3, `/accounting/journal-templates`
      ×2 new) — for the shared 01.C pass to turn into `RouteRef` objects, no module-specific behavior
      change needed.
- [ ] **`JournalFilter.to`'s route-hint false positive** (§2) — not a mock bug, the fourth confirmed
      instance of the generator-hint limitation already logged against `payments.md`'s `PaymentFilter.to`,
      `vouchers.md`'s `VoucherFilter.to`, and `expenses.md`'s `ExpenseFilter.to`. No action needed.
- [ ] **`getJournalEntries`/`getJournalEntriesPaged`'s generator-hint "writes: accounts" is a false
      positive** (§1) — both are pure read functions; the transitive-write tracker almost certainly
      picked up `accountIsDescendantOf`'s `db.accounts.find(...)` (a read, via `.find`, which the
      generator's own documented heuristic — "references obtained via `find`/`at`" — can conflate with
      a write when the returned reference is later touched in *some* code path it also scans) as a
      write. Not a mock bug — a generator lower-bound artifact, per `01-FRONTEND-ANALYSIS.md` §2's own
      stated caveat ("writes are a lower bound... reviewers confirm them in 01.B"). Confirmed here: no
      `mutate()` call anywhere in `getJournalEntries`/`getJournalEntriesPaged`/`toRow`/
      `matchesJournalFilter`/`accountIsDescendantOf`. No config change needed — this is a single-module,
      already-explained false positive, not worth tuning the generator's heuristic over.

## 9. Open questions (→ decisions in `00-MASTER-PLAN.md`)

This module has the most open questions of any reviewed this session, by design — every one below is
something the task brief explicitly said to flag rather than fix, because it touches posting logic,
balance validation, period-close/reopen, or the reversal mechanism itself.

- **D-A1 (`postJournalDraft` doesn't re-validate account state at post time).** A draft's lines are
  validated once, at save/edit time (`validateManualLines`, full B1 rule set). Between then and
  `postJournalDraft`, an admin could deactivate the account, turn it into a group, or flip
  `allowManual` off — and the draft would still post, since `postJournalDraft` only calls
  `assertOpenPeriod`, never `resolvePosting`'s or `validateManualLines`'s account-state checks again.
  **Two honest fixes**: (a) re-run `validateManualLines`-equivalent checks against the *current* account
  state inside `postJournalDraft`'s own transaction, refusing with the same messages a fresh entry
  would get if the accounts have since become invalid; or (b) treat a draft's accounts as pinned at
  save/edit time (accept the drift as a known, narrow edge case, on the theory that an admin who just
  deactivated an account mid-flight through someone else's draft is an unusual enough scenario not to
  engineer around). **Recommendation if forced to pick**: (a), since it costs nothing extra a fresh
  entry doesn't already pay and closes a real (if narrow) gap in the posting-integrity guarantee every
  other path in the app enjoys — but this is exactly the kind of "changes what posting means" call the
  task brief says not to make unilaterally. Also decide whether `postJournalDraft` should gain a
  `logActivity` call at the same time (D-A5 below) — likely yes, same design pass.
- **D-A2 (should `closeYear`/`reopenYear` support a caller-chosen date, like `reverseJournalEntry`
  does for B3?).** `reopenFiscalYear`'s reversal is always dated "now" (`new Date().toISOString()`),
  exactly the bug B3 already fixed for the general-purpose `reverseJournal` — `reopenFiscalYear` never
  picked up that fix. Unlike a manual entry's reversal, a closing-entry reversal is `allowClosedPeriod:
  true` regardless of date, so "now" being outside the (about-to-reopen) year's own dates isn't a period
  violation — but it does mean the reversal doesn't land inside the year being reopened, which could be
  confusing on a statement filtered to that year. **Needs a decision**: should `reopenYear` take an
  optional date parameter the way `reverseJournalEntry` does, defaulting to the fiscal year's own
  `endDate` (symmetric with how the closing entry itself was dated) rather than "now"? No accounting
  bug exists today (the reversal is still fully correct, balanced, and reverses the right entry — it's
  a filing/presentation nuance, not a wrong number), so this doesn't block the gate.
- **D-A3 (no reversal path exists for `submitVatSettlement`, and `reverseJournalEntry` actively
  refuses to try, since the posted entry's `type` is `'VAT_SETTLEMENT'`, not `'MANUAL'`).** If a VAT
  settlement is posted for the wrong period or with a stale `vatTotalsForPeriod` snapshot, the only
  fix today is a manual correcting journal entry an accountant builds by hand — there's no "undo the
  settlement" button and no code path that could become one without a design decision. **Two options**:
  (a) accept this as intentional, matching every other module's identical "posted is posted" stance
  (`payments.md`'s `createPayment`, `vouchers.md`'s general vouchers and card settlements,
  `expenses.md`'s `createExpense` all reach the same conclusion); or (b) add a real
  `reverseVatSettlement` that mirrors the entry (same "MANUAL-only" restriction doesn't have to apply
  to it, since it's a purpose-built compensation, the same shape as `closeYear`/`reopenYear`'s pair).
  **Recommendation if forced to pick**: (a), for consistency with the "posted documents get compensated
  by a manual entry, not undone" pattern already established everywhere else this session, and because
  nothing indicates VAT settlements are re-run often enough to need a first-class undo button. No
  accounting bug exists today (the posted entry is correct for the snapshot it was computed from), so
  this doesn't block the gate — flagged for Part 03.
- **D-A4 (drafts skip `assertOpenPeriod` entirely).** `createJournalEntry`'s `asDraft: true` path calls
  `draftJournal` directly, which never calls `assertOpenPeriod` — a draft can be saved with any date,
  including one inside a locked period, and the period check only fires later, at `postJournalDraft`
  time. This is very likely **intentional and correct** (a draft has zero GL effect, so there's nothing
  for the period lock to protect yet — the check firing at post time, when it actually matters, is the
  right place for it), matching how `payments.md`/`vouchers.md`/`expenses.md`'s only-check-at-actual-
  posting-time pattern works everywhere else. Recorded here as a confirmed-correct behavior worth
  stating explicitly (so a future reviewer doesn't mistake "drafts don't check the period" for a gap)
  rather than a genuine open question — no decision needed, no fix needed.
- **D-A5 (should `postJournalDraft`/`updateJournalDraft`/`deleteJournalDraft` write an activity/audit
  row, and if so, what should the message/entity say?).** See §8 — all three are currently silent.
  `postJournalDraft` is the sharper case (it's a posting action with zero audit trail, unlike the other
  two which are pure master-data-with-no-GL-effect edits). **Needs a decision** on: (a) whether to add
  `logActivity` to all three uniformly (simplest, keeps the trio consistent), and (b) for
  `postJournalDraft` specifically, what `userId`/message to log — the draft's original `createdBy`, or
  the user who clicked "post," and whether the message should note it was posted *from a draft*
  (distinguishing it from a fresh `createJournalEntry` post in the activity feed) — a11 UI-and-copy
  question, not a mechanical fix. No accounting bug exists today (the GL is correct either way; only
  the audit trail is incomplete), so this doesn't block the gate.
- **D-A6 (`submitVatSettlement` can be called twice for overlapping periods with no guard).** See §8.
  Two calls with overlapping `[from, to]` ranges would both compute a real (non-zero) `outputVat`/
  `inputVat` from the same underlying GL movements (since `vatTotalsForPeriod` doesn't exclude
  already-settled periods) and both would post, double-closing the same VAT into `vatPayable`.
  **Needs a decision** on the guard's shape: track "already-settled" date ranges and refuse an
  overlapping request (`CONFLICT`, matching `vouchers.md`'s `createCardSettlement`'s "already settled"
  check for the identical *kind* of problem — a tender/VAT-movement being claimed by two settlements),
  or something coarser (one settlement per calendar month/quarter, enforced by a unique constraint).
  Since `submitVatSettlement` posts a `type: 'VAT_SETTLEMENT'` entry with a `date: to` but no
  `sourceRef`, there's no existing "settled periods" table to check against today — building the guard
  means deciding what that tracking mechanism looks like, which is genuinely a Part 03 design question,
  not a one-line validation add. No `verify:mocks` invariant catches this today (nothing exercises a
  double-settlement in the seed data), so it's a real but currently-latent gap, not a proven live bug.
- **D-A7 (`payVatSettlementNow` doesn't check `amount` against the current `vatPayable` balance).**
  See §8. Overpaying would leave `vatPayable` in an unexpected debit balance, which nothing downstream
  currently flags (no invariant checks `vatPayable`'s sign or magnitude directly — the closest is
  `checkVatControl`'s §4.5, which reconciles `vatOutput`/`vatInput` against documents, not
  `vatPayable`'s own balance against payments made against it). **Needs a decision**: should this throw
  a `VALIDATION` error when `amount` exceeds the current net-payable balance (computed the same way
  `vatTotalsForPeriod`/`closeYearPreChecks`-style balance reads do elsewhere in this module), or is
  overpayment a legitimate real-world scenario (paying the authority extra as a buffer, or correcting a
  prior underpayment) that shouldn't be blocked? **Recommendation if forced to pick**: don't block it —
  a business can legitimately choose to pay more than currently accrued (e.g. settling a dispute, or a
  voluntary early payment) and the resulting debit balance in `vatPayable` is not itself wrong, just
  unusual — but this is a judgment call about real-world VAT-authority interactions this review isn't
  positioned to make unilaterally, so it's left open rather than silently deciding either way.
- **Should `deleteAccount`'s "system account" refusal be upgraded to `'FORBIDDEN'` (from the default
  `VALIDATION`)?** See §8 — flagged as a low-risk candidate a human could likely wave through in one
  line, listed here rather than auto-applied purely because this module's mandate was to err
  maximally conservative on anything touching the accounts/posting core, and an error-code change is
  a smaller but still real behavior change (any e2e/UI code branching on the error code would need to
  match `FORBIDDEN` instead of the default).
- **Should `JournalTemplate.recurrence.day` gain the same `[1,28]` validation `RecurringExpense.day`
  already has** (see §2/§8), and should `advanceRecurrence` clamp the day the same way
  `nextMonthDate` does for expenses? This is the cleanest, most mechanical of this module's open
  questions — the fix pattern already exists verbatim in `expenses.ts`'s `nextMonthDate` — but it's
  still a behavior change to a recurrence's exact posting date, which sits close enough to the
  accounting core (a mis-dated recurring depreciation/rent entry lands in the wrong month, which can
  matter for month-end reporting even though the entry itself is balanced and correct) that this
  review leaves it as a recommendation rather than an applied fix.

## Gate

- [x] Every inventory function is in §1 with a confirmed disposition (33/33, no dispositions changed —
      3 `frontend` pure-computation functions, 30 `port`).
- [x] Every write function is in §4.
- [x] Every DTO field needing a non-default mapping is in §2.
- [x] `bun run contract:check` — no override applied, contract unchanged in disposition terms (this
      module's 33/33 split is unaffected by the internal `deleteJournalTemplate` signature change,
      confirmed by re-running `bun run contract`: 341 service fns / 306 port, same as immediately
      before the fix — the +2 vs. the master plan's original 339/309 baseline is plan 22's
      in-progress, unrelated work already noted in `settings.md`, not this review). Also green:
      `bun run build`, `bun run check`, `bun run verify:mocks` (**128 ok, 0 failed — confirmed
      identical before and after the one fix**), `bun run memory` + `bun run memory:check` (0 new
      seam violations), `bun run diag:check` (`docs/diagnostics/ISSUES.md` up to date, 21 issues,
      none newly opened by this review).
