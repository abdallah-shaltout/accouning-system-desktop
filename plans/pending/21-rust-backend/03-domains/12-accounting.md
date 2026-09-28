# 21 · 03.12 — `accounting` (chart of accounts, manual journal, drafts, reversal, templates/recurring, undo)

> **Status:** implemented 2026-09-28 (code complete; DB tests + parity cases deferred to the
> time-boxed test pass — see §8/§9). All 19 commands, DTOs, service logic, 3 undo compensators, 19
> switch lines and `contract.check.ts` (12 entries, shared with 12b) are written. Not yet compiled
> (`cargo check` is the manager's throttled run after W5) or run against a live DB. Gaps §7 items
> 1–5 and 9 were already resolved before this wave started (see the status note's "Needs from
> manager" below for what's confirmed vs still open).
>
> Wave W5 (entry file §4). Depends on: every writer
> domain (01–11; the journal list shows entries they post), 05-parties (`PartyKind` DTO), Part 02
> (`shared::ledger::{post, reverse, save_draft, update_draft, delete_draft, post_draft}`,
> `shared::activity`, `shared::numbering`). **Split:** this file covers 19 of the module's 30 `port`
> functions (accounts, journal, drafts, reversal, templates/recurring) and 3 of its 4 undo
> compensators. The period-close half — fiscal years, lock date, close/reopen, VAT settlement and the
> `accounting.closeYear` compensator — is [`12b-period-close.md`](12b-period-close.md). Both files build
> the one `domains/accounting/` module; file ownership is split in §9 so they never edit the same file.
> FX revaluation and opening balances are **not** in this domain: they are `settings.postRevaluation`
> (01-settings, analysis `settings.md` §1) and `setup.*Opening*` (02-setup, analysis `setup.md` §1).

**Goal.** Port the accounting core's CRUD and journal functions as IPC commands that return the mock's
exact DTOs: chart-of-accounts CRUD with the system-account and group-account guards, the journal list/
detail (posted + drafts), manual entries with the B1 control-account rules, the draft lifecycle,
MANUAL-only reversal with a required reason, and journal templates with recurring posting. Register the
three reversal-based undo compensators from phase-e E-5 with `allow_closed_period = actor is admin` (D7).

**Read first:** [`../01-frontend-analysis/accounting.md`](../01-frontend-analysis/accounting.md) (§1–§6,
§8, §9 D-A1/D-A4/D-A5) · [`../../../../docs/v2/02-accounting-review.md`](../../../../docs/v2/02-accounting-review.md)
B1 (control accounts), B3 (reversal), §4 invariants · mock `src/modules/accounting/services/accountingService.ts`
(1-326, 398-451) · `src/mocks/backend/journal.ts` (1-221) · `src/mocks/backend/core.ts` (54-245) ·
`src/modules/accounting/types/index.ts` (1-220) · Rust `src-tauri/src/shared/ledger/{post,reverse,period,accounts}.rs`,
`shared/activity/{record,undo}.rs`, entities `journal/*`, `org/accounts.rs`.

## 1. Commands

Reads with no clock need → `with_read` + `crate::core::settings::require(tx, actor.as_ref(), area, Read)`
(actor cloned from `state.session` first). "Session only" = no area check, only `UNAUTHORIZED`
`سجّل الدخول أولاً` without a session: used for reference data other modules' pages read (the mock has no
access check at all; an area check would break those pages for roles without Accounting access).

| Mock fn (`accountingService.ts`) | Disp. | Rust command | Args → Return | Area / Access | Tx | Events |
|---|---|---|---|---|---|---|
| `signedBalance` (:29), `accountPath` (:64), `rolledBalance` (:75) | **frontend** (analysis §1) | — | — | — | — | — |
| `getAccounts` (:40) | port | `accounting_get_accounts` | `AccountingGetAccountsArgs { range: Option<DateRange> }` → `Vec<AccountWithBalance>` | session only (read by `pdfService.ts:30`, `InvoiceFormPage`, products, vouchers, setup, expenses pages) | `with_read` | — |
| `saveAccount` (:96) | port | `accounting_save_account` | `…SaveAccountArgs { input: AccountInput, id: Option<Id> }` → `Account` | Accounting / Write | `with_tx` | — |
| `deleteAccount` (:121) | port | `accounting_delete_account` | `…DeleteAccountArgs { id }` → `()` | Accounting / Write | `with_tx` | — |
| `reparentAccount` (:136) | port | `accounting_reparent_account` | `…ReparentAccountArgs { id, new_parent_id: Option<Id> }` → `Account` | Accounting / Write | `with_tx` | — |
| `getJournalEntries` (:211) | port | `accounting_get_journal_entries` | `…GetJournalEntriesArgs { filter: Option<JournalFilter> }` → `Vec<JournalRow>` | Accounting / Read | `with_read` | — |
| `getJournalEntriesForSource` (:226) | port | `accounting_get_journal_entries_for_source` | `…ForSourceArgs { source_kind: String, source_id: String }` → `Vec<LinkedJournalEntry>` | session only (`ExpenseDetailPage.vue:12`, `VoucherDetailPage.vue:14`) | `with_read` | — |
| `getJournalEntriesPaged` (:234) | port | `accounting_get_journal_entries_paged` | `…PagedArgs { query: PagedQuery<JournalFilter> }` → `PagedResult<JournalRow>` | Accounting / Read | `with_read` | — |
| `getJournalEntry` (:260) | port | `accounting_get_journal_entry` | `…GetJournalEntryArgs { id }` → `JournalEntryDetail` | Accounting / Read | `with_read` | — |
| `createJournalEntry` (:277) | port | `accounting_create_journal_entry` | `…CreateJournalEntryArgs { input: JournalEntryInput }` → `JournalEntry` | Accounting / Write | `with_tx` | Ledger (+Parties) on the post path; none for a draft |
| `updateJournalDraft` (:283) | port | `accounting_update_journal_draft` | `…UpdateJournalDraftArgs { id, input }` → `JournalEntry` | Accounting / Write | `with_tx` | — |
| `postJournalDraft` (:289) | port | `accounting_post_journal_draft` | `…PostJournalDraftArgs { id }` → `JournalEntry` | Accounting / Write | `with_tx` | Ledger |
| `deleteJournalDraft` (:294) | port | `accounting_delete_journal_draft` | `…DeleteJournalDraftArgs { id }` → `()` | Accounting / Write | `with_tx` | — |
| `reverseJournalEntry` (:299) | port | `accounting_reverse_journal_entry` | `…ReverseJournalEntryArgs { id, date: String, reason: String }` → `JournalEntry` | Accounting / Write | `with_tx` | Ledger (+Parties) |
| `getJournalTemplates` (:400) | port | `accounting_get_journal_templates` | `()` → `Vec<JournalTemplate>` | Accounting / Read | `with_read` | — |
| `getJournalTemplate` (:405) | port | `accounting_get_journal_template` | `…GetJournalTemplateArgs { id }` → `JournalTemplate` | Accounting / Read | `with_read` | — |
| `createOrUpdateJournalTemplate` (:412) | port | `accounting_create_or_update_journal_template` | `…Args { input: JournalTemplateInput, id: Option<Id> }` → `JournalTemplate` | Accounting / Write | `with_tx` | — |
| `removeJournalTemplate` (:417) | port | `accounting_remove_journal_template` | `…RemoveJournalTemplateArgs { id }` → `()` | Accounting / Write | `with_tx` | — |
| `loadTemplateIntoEntry` (:423) | port | `accounting_load_template_into_entry` | `…LoadTemplateIntoEntryArgs { id }` → `JournalTemplate` | Accounting / Read | `with_read` | — (same service fn as `get_journal_template`, analysis §1) |
| `postRecurringTemplate` (:434) | port | `accounting_post_recurring_template` | `…PostRecurringTemplateArgs { id }` → `JournalEntry` | Accounting / Write | `with_tx` | Ledger (+Parties) |

The other 11 `port` functions are in 12b §1. Args struct names are `Accounting<FnPascal>Args` in full
(abbreviated above).

## 2. DTOs (`domains/accounting/dto/{mod,accounts,journal,templates}.rs`, `#[ts(export_to = "accounting/types/gen/")]`)

Conventions as in 11-expenses §2 (camelCase, `skip_serializing_none`, `#[ts(optional)]`, `Id`/`Decimal`/
`DocDate` wire types). Instants (`createdAt`, `postedAt`) serialize with `utils::dates::iso_ms`.

| Rust DTO | TS type (file:line) | Notes |
|---|---|---|
| `AccountKind`, `NormalSide` | `types/index.ts:1-3` | `rename_all = "UPPERCASE"`; entity stores `String` → parse, unknown → `AppError::internal`. |
| `AccountSubtype` | `types/index.ts:6-15` | 20 variants, `rename_all = "camelCase"`. |
| `SystemRole` | `types/index.ts:23-29` | **Reuse `shared::ledger::accounts::SystemRole`** (Part 02 C-1: "no second enum") once it gains `Serialize/Deserialize/TS` + `FromStr` (§7 "Needs from manager"). |
| `Account` | `Account` (`types/index.ts:37`) | `parent_id: Option<Id>` with `#[serialize_always]` + `#[ts(type = "string | null")]` (TS `string \| null`, not optional); `system_role`, `currency`, `branch_id`, `requires_party`, `requires_cost_center`, `name_en` optional. |
| `AccountInput` | `types/index.ts:65` | `parent_id: Option<Id>` optional. |
| `AccountWithBalance` | `accountingService.ts:26` | Flat: every `Account` field + `balance`, `debit_total`, `credit_total` (Decimal), `has_postings: bool`. |
| `DateRange` | inline `{ from?: string; to?: string }` (:40) | both optional strings. |
| `JournalEntryType` | `types/index.ts:125` | explicit renames `SYSTEM`/`MANUAL`/`OPENING`/`CLOSING`/`VAT_SETTLEMENT`. |
| `JournalEntryStatus` | `types/index.ts:126` | `DRAFT`/`POSTED`. |
| `JournalSourceKind` | `types/index.ts:79-98` | 12 variants, camelCase; the canonical enum other domains reuse (analysis §2). Entity stores `String`. |
| `JournalSourceRef` | inline (`types/index.ts:135`) | `{ kind, id, number? }`. |
| `JournalLine` | `types/index.ts:100` | `party_kind: Option<crate::domains::parties::dto::PartyKind>`; `amount_fc`/`rate` optional Decimal. |
| `JournalEntry` | `types/index.ts:128` | `date: DocDate`; `reversed: Option<bool>` emitted only when `true` (the mock leaves it `undefined` otherwise); `posted_by`/`posted_at` absent on drafts; `attachment_ids` optional `Vec<String>`. |
| `JournalEntryInput` (+ `JournalEntryInputLine`) | `types/index.ts:151` | `date: String` (→ `RawDocDate`), `reference` optional and ignored (as in the mock), `as_draft: Option<bool>`, `template_id`. |
| `JournalFilter` | `types/index.ts:171` | all optional; `min_amount`/`max_amount` Decimal. |
| `JournalRow` | `accountingService.ts:158` | Flat: `JournalEntry` fields + `created_by_name`, `source_link` (`#[ts(optional, type = "import('../../core/types/route').AppRoute")]`, the `ActivityEntry.link` pattern in `core/dto.rs`), `source_label`, `attachment_count: u32`. |
| `JournalEntryDetail` | inline return of `getJournalEntry` (:260) | `JournalRow` fields + `reversed_by_id?`, `reversed_by_number?`, `related: Vec<JournalRow>`. |
| `LinkedJournalEntry` | `accountingService.ts:219` | `{ id, number, description }`. |
| `JournalTemplateLine`, `JournalTemplateRecurrence`, `RecurrenceEvery`, `JournalTemplate` | `types/index.ts:199-220` | `every` lowercase; `day: i32`; `next_date` `YYYY-MM-DD`; `created_at` iso_ms. |
| `JournalTemplateInput` | `mocks/backend/journal.ts:168` | Move the interface into `src/modules/accounting/types/index.ts` and have `journal.ts` import + re-export it (type-only move, zero behaviour change; the service already imports it from the mock, `accountingService.ts:13`). |
| `PagedQuery<JournalFilter>`, `PagedResult<JournalRow>` | `core/types/paging.ts` | from `core::dto`. |

**`src/modules/accounting/types/contract.check.ts`** (new): `Account`, `AccountInput`, `JournalEntry`,
`JournalLine`, `JournalEntryInput`, `JournalFilter`, `JournalTemplate`, `JournalTemplateInput`,
`AccountWithBalance`, `JournalRow`, `LinkedJournalEntry` (the last three via `import type` from
`../services/accountingService`), and `JournalEntryDetail` checked as
`Equals<Gen.JournalEntryDetail, Awaited<ReturnType<typeof getJournalEntry>>>`. 12 entries (12b adds its own).

## 3. Service logic (`domains/accounting/service/{accounts,journal,journal_reads,templates,rows}.rs`)

Write functions: `pub async fn f<C: ConnectionTrait>(conn: &C, cx: &TxCtx, undo: &UndoRegistry, …) -> TxResult<T>`.
`is_admin(cx) = cx.actor.role == Role::Admin` replaces `canPostToClosedPeriod()` (`accountingService.ts:35-37`).
Account lookups use `find_live()` (P2-16) so soft-deleted rows behave like the mock's hard-deleted ones.

### 3.1 `rows.rs` — shared mapping (also used by 12b)
- `entry_dto(model, lines) -> JournalEntry`, `draft_dto(model, draft_lines, base_currency) -> JournalEntry`
  (draft lines have no currency column; manual lines always resolved to the base currency in
  `resolvePosting`, `core.ts:74`, so the DTO fills `settings.currency`), `load_entries(conn, ids)` loading
  lines `ORDER BY position`.
- `to_row(entry, ctx) -> JournalRow` (`accountingService.ts:196-204`): `created_by_name` = `users.name` else
  `—`; `source_label` from the 6-entry `SOURCE_LABEL` map (:160-167, other kinds → none); `source_link`
  (:305-326): `invoice` → `invoice{id}`, `refund` → `invoice{refunds.invoice_id}` or none, `purchaseOrder` →
  `purchase{id}`, `purchaseReturn` → `purchase{purchase_returns.purchase_order_id}` or none, `payment` →
  `{ name: 'payments', query: { highlight: id } }` (needs `RouteRef.query`, §7), `stockAdjustment` →
  `adjustment{id}`, anything else → none; `attachment_count` = list length. Batch the user/refund/return
  lookups per call (one `IN (...)` query each).

### 3.2 Accounts (`accounts.rs`)
**`get_accounts(range)`** (:40-58): live accounts `ORDER BY code` (utf8mb4_bin = the mock's
`localeCompare` on digit strings). One aggregate: `SELECT jl.account_id, SUM(debit), SUM(credit) FROM
journal_lines jl JOIN journal_entries je … WHERE date_day >= from AND date_day <= to` (each bound only when
non-empty; `inDateRange` on the business day) `GROUP BY jl.account_id` — posted entries only (drafts are a
separate table, P2-14). Per account: `debit_total = round2(d)`, `credit_total = round2(c)`, `balance =
round2(DEBIT ? d − c : c − d)` (`signedBalance`, :29-31), `has_postings` = the account has a group row.

**`save_account(input, id)`** (:81-119), in this order:
1. `validateAccount` (:81-94): `code` matches `^[0-9]{1,8}$` → `رمز الحساب أرقام فقط (حتى 8 أرقام)`; name trim
   empty → `اسم الحساب مطلوب`; another live account with this code → `CONFLICT` `رمز الحساب مستخدم من قبل`;
   when `parent_id` is set: equals `id` → `لا يمكن أن يكون الحساب أباً لنفسه`, missing → `الحساب الأب غير موجود`,
   not a group → `الحساب الأب يجب أن يكون حساباً رئيسياً (تجميعياً)`, other kind → `الحساب الأب من نوع مختلف ({parent.kind})`
   (raw `ASSET`…), code not prefixed → `رمز الحساب يجب أن يبدأ برمز الحساب الأب ({parent.code})`; `is_group &&
   allow_manual` → `الحسابات الرئيسية (التجميعية) لا تقبل الترحيل المباشر`. All `VALIDATION` except the code clash.
2. Update: `core::lock::for_update_by_id(conn, "accounts", id)`, load live → `NOT_FOUND` `الحساب غير موجود`;
   `!can_delete` and code/kind/isGroup changed → `VALIDATION` `لا يمكن تغيير رمز أو نوع أو شكل (رئيسي/فرعي) حساب أساسي في النظام`;
   `is_group` flips: live children → `للحساب حسابات فرعية — لا يمكن جعله فرعياً (postable)`; any `journal_lines`
   row on it → `للحساب قيود مسجلة — لا يمكن جعله رئيسياً (تجميعياً)` (both `VALIDATION`, :107-110).
3. Write (:111-116): `code`, `name = trim`, `parent_id = input.parent_id` (absent → `NULL`: the mock sets
   `parentId: input.parentId || null` explicitly), `is_group`, `kind`, `subtype`, `normal_side`, `allow_manual`,
   `active`; `name_en`/`requires_party` only when present (decision A-D2); `system_role`, `currency`,
   `branch_id`, `requires_cost_center`, `can_delete` untouched. Create: new `Id`, `can_delete = true`, the
   not-in-input fields `NULL`.
4. `activity::log(Journal, "{تعديل|إضافة} الحساب {code} — {name}", now, RouteRef::list("accounts"))` (:117).
   Return the `Account`.

**`delete_account(id)`** (:121-129): lock `FOR UPDATE`; live row → `NOT_FOUND` `الحساب غير موجود`;
`!can_delete` → `VALIDATION` `حساب أساسي في النظام ولا يمكن حذفه` (default code kept, analysis §8/§9);
live children → `CONFLICT` `للحساب حسابات فرعية — احذفها أولاً`; any `journal_lines` row → `CONFLICT`
`للحساب قيود مسجلة — يمكنك إيقافه بدلاً من حذفه`; soft delete. No audit row (mock writes none, quirk Q4).

**`reparent_account(id, new_parent_id)`** (:136-153): load live → `NOT_FOUND`; `new_parent_id == id` →
`لا يمكن أن يكون الحساب أباً لنفسه`; new parent missing → `الحساب الأب غير موجود`; not a group →
`الحساب الأب يجب أن يكون حساباً رئيسياً (تجميعياً)`; other kind → `لا يمكن نقل الحساب إلى مجموعة من نوع مختلف ({kind})`;
cycle (walk the new parent's ancestor chain) → `لا يمكن نقل الحساب إلى أحد فروعه`. Locking for the cycle
race: compute `{id} ∪ {new_parent} ∪ ancestors(new_parent)`, lock them with
`core::lock::for_update_many_sorted`, re-walk the chain on fresh reads; if it reaches an id not in the locked
set, lock that too and repeat (bounded by tree depth). Then set `parent_id`, `activity::log(Journal, "نقل الحساب
{code} — {name}", now, RouteRef::list("accounts"))`. The code-prefix rule is **not** re-checked (mock
behaviour, quirk Q5). Return the `Account`.

### 3.3 Manual journal (`journal.rs`)
**`validate_manual_lines(conn, input)`** — `pub`, ports `journal.ts:17-40` in order: description trim empty
→ `أدخل بيان القيد`; `lines.len() < 2` → `يجب أن يحتوي القيد على سطرين على الأقل`; per line in input order:
live account → else `اختر الحساب لكل سطر`; `!active` → `الحساب "{name}" غير نشط`; group → `"{name}" حساب
رئيسي (تجميعي) ولا يقبل الترحيل المباشر`; negative → `المبالغ لا يمكن أن تكون سالبة`; both sides → `السطر
الواحد إما مدين أو دائن`; non-zero on `!allow_manual` → **`FORBIDDEN`** `لا يمكن الترحيل يدوياً على حساب "{name}" —
استخدم تسوية المخزون أو تسوية ضريبة القيمة المضافة` (B1); non-zero on `requires_party` without `party_id` →
`السطر على حساب "{name}" يتطلب اختيار عميل أو مورد`; non-zero on `requires_cost_center` without `cost_center_id`
while `settings.features.cost_centers` is on → `السطر على حساب "{name}" يتطلب اختيار مركز تكلفة`.

**`to_posting_lines(input)`** (:76-87): `AccountRef::Id(account_id)`, description, debit, credit, `party =
PartyRef { kind, id }` when `party_id` is set (kind from the line; if the line omits `partyKind`, read it from
the `parties` row — decision A-D6), `branch_id`, `cost_center_id`; never currency/FC.

**`record_manual_journal(conn, cx, undo, input, action_type: &'static str) -> TxResult<JournalEntry>`**
(`journal.ts:89-113`): `validate_manual_lines`; date = `RawDocDate::parse(input.date)` → `resolve(&cx.clock)`
(parse failure → `VALIDATION` `التاريخ غير صالح`, decision A-D5).
- `as_draft == Some(true)`: `ledger::save_draft(conn, cx, date, description.trim(), lines, attachment_ids,
  template_id)` — no period check (D-A4 confirmed correct), no activity row (mock writes none), no event;
  return `draft_dto`.
- Post: `ledger::post(PostJournal { date, description: trim, entry_type: Manual, source: None, lines,
  allow_closed_period: is_admin(cx), attachment_ids, template_id })`, then
  `activity::log_undoable(Journal, "قيد يدوي {number} — {description}", Some(entry.date), Some(RouteRef::detail("journal-entry", id)),
  UndoSpec { action_type, payload: json!({ "journalEntryId": id }) })` (:111). Return `entry_dto`.

**`create_journal_entry(input)`** = `record_manual_journal(…, "accounting.createJournalEntry")` (:277-280).

**`update_journal_draft(id, input)`** (`journal.ts:116-124`, `core.ts:208-224`): `validate_manual_lines`
first (the mock validates before the existence check); `core::lock::for_update_by_id(conn, "journal_drafts", id)`;
`ledger::update_draft(conn, id, date, description.trim(), lines, input.attachment_ids)` (→ `NOT_FOUND`
`المسودة غير موجودة`; `None` attachments keep the old list, as `core.ts:221`). No activity row (quirk Q2).
Return `draft_dto`.

**`post_journal_draft(id)`** (`core.ts:233-245`): lock the draft row; `ledger::post_draft(conn, cx, id,
is_admin(cx))` (→ `NOT_FOUND`; period check; same id and number). Then — required by phase-e E-5, since this
action is undoable — `activity::log_undoable(Journal, "قيد يدوي {number} — {description}", Some(entry.date),
Some(RouteRef::detail("journal-entry", id)), UndoSpec { action_type: "accounting.postJournalDraft", payload:
{ journalEntryId } })` (decision A-D1: the mock writes no row here, D-A5). **No re-validation of the lines
against current account state** (D-A1 kept as the mock behaves, quirk Q1). Return `entry_dto`.

**`delete_journal_draft(id)`** (`core.ts:226-230`): lock; `ledger::delete_draft` (→ `NOT_FOUND`). No audit (Q2).

**`reverse_journal_entry(conn, cx, undo, id, date: DocDate, reason: &str) -> TxResult<(JournalEntry, Id /*audit*/)>`**
— `journal.ts:130-162`, the MANUAL-only rule and the reason live here (phase-c C-6):
1. `core::lock::for_update_by_id(conn, "journal_entries", id)`; load posted entry → `NOT_FOUND` `القيد غير موجود`.
2. `type != Manual` → `VALIDATION` `القيود الآلية تُعكس من المستند المصدر (مرتجع/إلغاء)`.
3. `reversed || reversal_of_id.is_some()` → `VALIDATION` `هذا القيد معكوس بالفعل` (checked here so it comes
   before the reason check, as in the mock; `ledger::reverse` re-checks it harmlessly).
4. `reason.trim()` empty → `VALIDATION` `سبب العكس مطلوب`.
5. `ledger::reverse(ReverseRequest { original_id: id, date, description: "عكس القيد {number} — {description}",
   entry_type: Manual, allow_closed_period: is_admin(cx), reason: Some(ReversalReason { text: trim,
   stamp_original: true }), dims: MirrorDims::Keep })` — mirror keeps account, description, party, branch,
   cost center; stores the reason on both entries (B3).
6. `activity::log(Journal, "عكس القيد {original.number}", Some(reversal.date), Some(RouteRef::detail("journal-entry", reversal.id)))`.
7. Return `(reversal, audit_id)`; the command returns the entry DTO, the compensators return the audit id.

The command parses `date` with `RawDocDate` (the detail page sends `new Date(key).toISOString()`,
`JournalDetailPage.vue:137`).

### 3.4 Journal reads (`journal_reads.rs`)
**`load_journal(conn, filter)`** — `matchesJournalFilter` (:169-184) over posted entries (`ORDER BY
created_at, id`) followed by drafts (`ORDER BY created_at, id`), the mock's `allEntriesAndDrafts()` order
(:207-209). SQL narrows by: `status` (picks the table), `type`, `created_by = userId`, `source_kind`,
`reversed` (drafts count as `false`), `total_debit >= min / <= max`, `date_day` range. Rust applies the
rest exactly: `accountId` = the account **or any descendant** (`accountIsDescendantOf`, :187-194 — build the
descendant set once from the live tree), `partyId` on any line, `hasAttachments`, and
`matches_search(&[number, description, source_number], search)` (search haystack decision A-D3).

**`get_journal_entries(filter)`** (:211-217): `load_journal`, sort by `DocDate::key()` descending then
`number` descending (string compare), map `to_row`.

**`get_journal_entries_paged(query)`** (:234-258): `load_journal(filters)` → rows; `total`; `totals =
{ totalDebit: Σ total_debit, totalCredit: Σ total_credit }` over the filtered set (exact `Decimal` sums);
sort: with `query.sort`, a stable sort starting from the unsorted concatenation, by the whitelisted key —
numeric keys `totalDebit`/`totalCredit`/`attachmentCount`; string keys `id`, `number`, `date` (the key
string), `description`, `type`, `status`, `createdBy`, `createdByName`, `createdAt`, `postedBy`, `postedAt`,
`sourceLabel`, `reversalOfId`, `reversalReason`, `templateId`, `reversed` (`"true"`/`""`); any other key
compares equal (the mock's `String(undefined)` case, decision A-D4). Without `sort`, the default order of
`get_journal_entries`. Slice `[(page−1)·size, +size)` (a start below 0 counts from the end, as JS `slice`).
`totals` goes through the `f64`-map helper requested in §7.

**`get_journal_entry(id)`** (:260-275): posted or draft → `NOT_FOUND` `القيد غير موجود`; `reversed_by` = the
posted entry with `reversal_of_id = id`; `related` = posted entries with the same `(source_kind, source_id)`
excluding `id` (`ORDER BY created_at, id`), only when the entry has a source; all mapped with `to_row`.

**`get_journal_entries_for_source(kind, id)`** (:226-231): posted entries with `source_kind = kind AND
source_id = id`, `ORDER BY created_at, id` → `{ id, number, description }`. A `source_id` that is not a
valid `Id` returns `[]` (the mock simply finds nothing).

### 3.5 Templates and recurring (`templates.rs`)
**`get_journal_templates`** (:400-403): live rows `ORDER BY name COLLATE utf8mb4_unicode_ci, created_at, id`
(the mock's `localeCompare(…, 'ar')`, quirk Q6). **`get_journal_template(id)`** (:405-410) → `NOT_FOUND`
`القالب غير موجود`; `load_template_into_entry` calls the same function.

**`save_journal_template(input, id)`** (`journal.ts:175-201`): name trim empty → `اسم القالب مطلوب`;
`validateTemplateLines`: fewer than 2 lines → `يجب أن يحتوي القالب على سطرين على الأقل`, a line's live account
missing → `اختر الحساب لكل سطر`, non-zero on `requires_party` without `party_id` → `السطر على حساب "{name}" يتطلب
اختيار عميل أو مورد`; update → lock + load live → `NOT_FOUND` `القالب غير موجود`; duplicate live name →
`CONFLICT` `اسم القالب مستخدم من قبل` (DB `uq_journal_templates_name_live`, decision A-D7; `map_unique` for the
race). Write `name` (trimmed), `description` (verbatim), `lines` (JSON, verbatim, no rounding), recurrence
columns from `input.recurrence` (absent → all four `NULL`: the mock assigns `recurrence: input.recurrence`).
Create: `created_at = now`, `created_by = actor`. `activity::log(Journal, "{تعديل|إضافة} قالب القيد \"{name}\"",
now, RouteRef::list("journal-templates"))` (:199). Return the DTO.

**`remove_journal_template(id)`** (:203-208): lock + load live → `NOT_FOUND`; soft delete; `activity::log(Journal,
"حذف قالب القيد \"{name}\"", now, RouteRef::list("journal-templates"))`.

**`post_recurring_template(id)`** (`accountingService.ts:434-451`, `journal.ts:210-221`):
1. Read the template (live, unlocked) → `NOT_FOUND` `القالب غير موجود`; no recurrence → `VALIDATION`
   `القالب ليس متكرراً`.
2. Build the input: `date = next_date` (day only), `description = template.description` if non-empty else
   `name`, the template lines, `template_id = id`, not a draft; run `validate_manual_lines` (full B1 set at post
   time, analysis §1).
3. Locks in the global order: `ledger::period::assert_open_period(conn, &next_date, is_admin(cx))` (settings S
   + year S, same refusal the post would give), then `FOR UPDATE` the template and re-read it. If `next_date`
   changed (another terminal just posted it), redo steps 2–3 with the new date. No "is due" check: the page
   offers "ترحيل الآن" for every recurring template, due or not (`JournalTemplatesPage.vue`, the button's
   `v-if="row.recurrence"`), so posting ahead is a real flow (refines analysis §5, decision A-D8).
4. `record_manual_journal(…, "accounting.postRecurringTemplate")` with payload `{ journalEntryId, templateId }`.
5. `advance_recurrence` (:211-221), JavaScript `Date` semantics on purpose (quirk Q3): `month` → +1 month,
   `quarter` → +3, `year` → +12, keeping the day; when the day does not exist in the target month it rolls
   into the next month (Jan 31 + 1 → Mar 3, or Mar 2 in a leap year; Feb 29 + 1 year → Mar 1). Helper
   `js_add_months(date, n)`: `first_of(target_month) + (day − 1) days`. Update `recurrence_next_date`.
6. Return the entry DTO.

**`due_recurring_templates(conn, today)`** — `pub`, for 14-analytics' `recurring-journal-due` insight
(`insightRules.ts:446`): live templates with a recurrence and `next_date <= today`, `ORDER BY created_at, id`.

### 3.6 Posting traces
Every post here goes through `ledger::post`, which queues a posting trace that `with_tx` pushes into
`AppState.traces` after commit (phase-c C-7). The "explain this number"/posting-trace reads are
`diagnostics` dev-only commands owned by 16-diagnostics (analysis `diagnostics.md` §1); nothing to add here.

## 4. Concurrency (D8, analysis §5)
- **Account code:** there is **no** unique index on `accounts.code` (m0003 has only `ix_accounts_parent_id`/
  `ix_accounts_system_role`); the pre-check alone races. Requested: m0016 `code_live` + `uq_accounts_code_live`
  (§7); `save_account` maps it with `map_unique` to `رمز الحساب مستخدم من قبل`.
- **Group flip / delete vs a concurrent poster:** the domain X-locks the account row, but `resolve_posting`
  reads accounts without a lock, so an uncommitted posting line is invisible to the check. Requested:
  `resolve_posting` takes `LOCK IN SHARE MODE` on each resolved account (§7). With it, the flip/delete waits
  for the poster to commit and then sees its lines.
- **Numbering:** `next_number(Journal)` inside `post`/`save_draft` (row X lock, gapless).
- **Drafts:** update/post/delete X-lock the draft row first; a post racing a delete ends in `NOT_FOUND` for
  the loser. A draft consumes its journal number at save time (`core.ts:190`) and keeps it on post.
- **Reversal:** the domain's `FOR UPDATE` + `ledger::reverse`'s own lock serialise two reversals; the second
  gets `هذا القيد معكوس بالفعل`.
- **Recurring template:** 3.5 step 3 — two terminals post consecutive periods, never the same date twice.
- **Template name:** `uq_journal_templates_name_live` + `map_unique`.
- **Reparent cycle:** 3.2's sorted multi-row lock.
- Order: settings S → fiscal year S → accounts/templates/drafts X → counters → documents → `change_versions`.

## 5. Undo (phase-e E-5 — three of the four accounting compensators; the fourth is 12b §5)

`domains/accounting/undo.rs` (12-owned) defines `ReverseEntryCompensator { action_type: &'static str }`,
registered three times by `pub fn register_undo(r: &mut UndoRegistry)` (which also calls 12b's
`undo_period::register`):

| `action_type` | Recorded by | Payload | Compensation |
|---|---|---|---|
| `accounting.createJournalEntry` | `create_journal_entry`, post path only | `{ journalEntryId }` | `reverse_journal_entry` |
| `accounting.postJournalDraft` | `post_journal_draft` | `{ journalEntryId }` | `reverse_journal_entry` |
| `accounting.postRecurringTemplate` | `post_recurring_template` | `{ journalEntryId, templateId }` | `reverse_journal_entry`; the template's `next_date` stays advanced (documented gap, E-5) |

`area()` = `Area::Accounting`. `compensate`: parse `journalEntryId` (malformed → `AppError::internal`);
`date = req.date.resolve(&cx.clock)` or `DocDate { day: today, instant: Some(now) }`; call
`reverse_journal_entry(tx, cx, &UndoRegistry::new(), id, date, &req.reason)` — the reversal's
`allow_closed_period` is `is_admin(cx)`, which is exactly D7; the empty registry is valid because the reversal
records no `UndoSpec` (see §7 gap); return its audit id. An entry already reversed by hand → `هذا القيد معكوس بالفعل`.

## 6. Frontend switch lines (`src/modules/accounting/services/accountingService.ts`)

Import `usesRust`/`backendCall` from `@/modules/core/services/backend`; first line inside each `wrap` body:
`getAccounts` → `if (usesRust('accounting')) return backendCall('accounting_get_accounts', { range });` ·
`saveAccount` → `…('accounting_save_account', { input, id })` · `deleteAccount` →
`{ await backendCall('accounting_delete_account', { id }); return; }` · `reparentAccount` →
`…('accounting_reparent_account', { id, newParentId })` · `getJournalEntries` → `…({ filter })` ·
`getJournalEntriesForSource` → `…({ sourceKind, sourceId })` · `getJournalEntriesPaged` → `…({ query })` ·
`getJournalEntry` → `…({ id })` · `createJournalEntry` → `…({ input })` · `updateJournalDraft` → `…({ id, input })` ·
`postJournalDraft` → `…({ id })` · `deleteJournalDraft` → await + return · `reverseJournalEntry` →
`…({ id, date, reason })` · `getJournalTemplates` → no args · `getJournalTemplate` → `…({ id })` ·
`createOrUpdateJournalTemplate` → `…({ input, id })` · `removeJournalTemplate` → await + return ·
`loadTemplateIntoEntry` → `…('accounting_load_template_into_entry', { id })` · `postRecurringTemplate` → `…({ id })`.
19 lines; `signedBalance`/`accountPath`/`rolledBalance` untouched (frontend).

## 7. Known mock quirks (kept) and decisions

**Quirks kept (entry §3.3):** Q1 `postJournalDraft` does not re-validate lines against current account state
(D-A1). Q2 `updateJournalDraft`/`deleteJournalDraft` write no audit row (D-A5; only `postJournalDraft` gains
one, A-D1). Q3 `advanceRecurrence` uses JS month rollover with no day clamp and never reads `recurrence.day`
(analysis §2/§9). Q4 account/category delete writes no audit row. Q5 `reparentAccount` does not re-check the
code prefix. Q6 Arabic-text sort: the mock uses ICU `localeCompare(…,'ar')`; Rust uses `utf8mb4_unicode_ci`
(templates) or code-point order (paged string keys) — same order for digit/ASCII keys, possibly different for
Arabic text; Part 04 compares template lists as sets plus a spot order check. Q7 `recurrence.autoPost` inert
(analysis §2). Q8 `deleteAccount`'s system-account refusal keeps the default `VALIDATION` code.

**Decisions (logged):** A-D1 `post_journal_draft` writes a `قيد يدوي …` activity row carrying the undo spec
(E-5 requires an audit row; same copy as a direct post, so no new UI text). A-D2 absent optional input fields
keep the stored value on update (the only caller, `AccountFormModal.vue:139-152`, never sends `nameEn`).
A-D3 journal search = Rust `matches_search` after SQL narrowing (the list returns every filtered row anyway;
`search_normalized` is never written by `ledger::post`, and drafts have no such column). A-D4 unknown paged
sort keys keep the input order (no page uses the paged variant yet, `JournalListPage.vue:68`). A-D5
unparsable dates → `VALIDATION` `التاريخ غير صالح`. A-D6 a line with `partyId` but no `partyKind` takes the
kind from the `parties` row. A-D7 keep `uq_journal_templates_name_live` with the Arabic `CONFLICT` (mock allows
duplicates). A-D8 recurring post locks + re-reads instead of the analysis's "still due" check (early posting is
a UI flow).

**Needs from manager (Part 02 gaps):**
1. `shared::ledger::accounts::SystemRole`: derive `Serialize`, `Deserialize`, `TS` (camelCase,
   `export_to = "accounting/types/gen/"`) and `impl FromStr`.
2. `utils::route::RouteRef` has no `query` field; `sourceLink` for payments is `{ name: 'payments', query:
   { highlight } }` (`accountingService.ts:322`). Add `query: Option<BTreeMap<String, String>>` (skip if none).
3. `core::dto::PagedResult.totals` is `BTreeMap<String, f64>`, but `architecture_rules` forbids `f64` under
   `domains/`. Add a core constructor, e.g. `PagedResult::with_totals(rows, total, &[(&str, Decimal)])`.
4. `ledger::save_draft`/`update_draft` take `date: NaiveDate` and set `date_instant = NULL`; the journal form
   sends an ISO instant (`JournalEntryFormPage.vue:341` `dateKeyToIso`), which the mock stores verbatim. Take
   `DocDate` instead.
5. `ledger::post_draft` sets `posted_by = draft.created_by` (mock: the poster, `core.ts:238`), writes posted lines
   with `currency = NULL` (mock: the base currency) and never touches `Parties` for party lines (P2-12).
6. `ledger::resolve_posting` should `LOCK IN SHARE MODE` each resolved account row (§4).
7. m0016: `accounts.code_live` + `uq_accounts_code_live` (§4).
8. `ledger::post` never fills `journal_entries.search_normalized` (P2-38); either fill it or drop the column (A-D3 does not need it).
9. `Compensator::compensate` receives no `&UndoRegistry`; compensators pass an empty one (valid only because
   compensating writes carry no `UndoSpec`). Consider passing the registry through.
10. `ledger::post` stores an empty `attachment_ids` list as `NULL`, the mock keeps `[]`; Part 04 should treat
    `[]` and absent as equal, or `post` should keep `Some(vec![])`.

## 8. Tests

**(a) `src-tauri/tests/domain_accounting.rs`** (TestDb + the demo fixture or a local
`seed_accounting_fixture`; each posting test ends with `shared::invariants::run_all` green):
- `get_accounts`: order by code, range filter, `has_postings`, signed balances for DEBIT/CREDIT accounts, drafts excluded.
- `save_account`: each `validateAccount` message in order; create sets `can_delete`; system account code/kind/shape
  change refused; group⇄leaf flips refused with children / with postings; `parent_id` absent clears it; activity row written.
- `delete_account`: system → `VALIDATION`; children / postings → `CONFLICT`; success soft-deletes.
- `reparent_account`: kind mismatch, self, cycle messages; success logs `نقل الحساب …`.
- Manual entry: every `validate_manual_lines` message incl. B1 `FORBIDDEN` and the cost-center rule on/off; post
  → `JE-` number, MANUAL, activity `قيد يدوي …` with `is_undoable` + `accounting.createJournalEntry`; admin posts
  into a closed year, accountant gets the period `FORBIDDEN`.
- Draft: save (no period check, consumes a number, no event), update (keeps attachments when absent), post
  (same id/number, audit row with `accounting.postJournalDraft`), delete; missing → `المسودة غير موجودة`.
- Reverse: SYSTEM entry refused; double reversal refused; empty reason refused (in that order); mirror keeps
  branch/cost center; reason on both entries.
- Journal reads: filter by account incl. descendants, party, status, type, reversed, attachments, amount range,
  Arabic search; order (date key desc, number desc); `to_row` fields incl. `sourceLink` for refund/purchase
  return; detail `reversedBy*` and `related`; paged totals over the full filtered set, unknown sort key keeps order.
- Templates: save validation messages; duplicate name `CONFLICT`; recurrence cleared when absent; remove logs.
- Recurring: posts `next_date` entry with `templateId`, advances by JS rules (Jan 31 → Mar 3 / Mar 2; Feb 29 +
  1y → Mar 1; quarter); two concurrent posts → two consecutive periods, no duplicate date.
- Undo (registry from `domains::register_undo`): each of the three action types → `shared::activity::undo::undo`
  reverses the entry and links `undo_of`/`undone_by`; in a closed year a non-admin gets `FORBIDDEN`, an admin succeeds.
- Concurrency: group flip vs a concurrent post (needs gap 6); duplicate account code race (needs gap 7).

**(b) Parity cases (Part 04):** `accounting/coa-crud`, `accounting/coa-refusals`, `accounting/reparent`,
`accounting/manual-entry-post`, `accounting/manual-entry-b1-refusals`, `accounting/draft-lifecycle`,
`accounting/reverse-manual`, `accounting/reverse-refusals`, `accounting/journal-list-filters`,
`accounting/journal-detail-related`, `accounting/template-crud`, `accounting/recurring-post-advance`
(`draft-lifecycle` allowlists the added `postJournalDraft` audit row, A-D1).

## 9. Checklist

- [x] Confirm gaps 1–5 and 9 from §7 are resolved by the manager (they block compilation or parity); 6–8/10 may land later. Found already resolved on inspection: gap 1 (`SystemRole` has `Serialize/Deserialize/TS/FromStr`), gap 2 (`RouteRef.query`), gap 3 (`PagedResult::totals` is `BTreeMap<String, Decimal>` with the JSON-number serde helper), gap 4 (`ledger::save_draft`/`update_draft` take `DocDate`), gap 5's schema part (`journal_draft_lines` has `currency`/`amount_fc`/`rate` columns; `post_draft` still sets posted lines' `currency = NULL` rather than the base currency — see "Needs from manager" below), gap 7 (`accounts.code_live` + `uq_accounts_code_live` exist, m0016). Gap 9 (compensators receive `&UndoRegistry`) was already the case going in.
- [x] Move `JournalTemplateInput` into `accounting/types/index.ts` (+ re-export from `mocks/backend/journal.ts`).
- [x] `domains/accounting/mod.rs` (12-owned): `pub mod commands; pub mod service; pub mod dto; pub mod undo; pub mod undo_period;`, `ipc_signatures()` (19 lines here + `commands::period::ipc_signatures()` from 12b), `register_undo`.
- [x] `dto/{mod,accounts,journal,templates}.rs` per §2.
- [x] `service/rows.rs` (3.1), `service/accounts.rs` (3.2), `service/journal.rs` (3.3), `service/journal_reads.rs` (3.4), `service/templates.rs` (3.5).
- [x] `commands/{mod,accounts,journal,templates}.rs`: 19 commands per §1.
- [x] `undo.rs`: `ReverseEntryCompensator` × 3 (§5).
- [x] `tests/domain_accounting.rs` per §8(a) — written; not yet run (⏳ deferred time-boxed test pass, needs `EQUAL_TEST_DATABASE_URL` and a `cargo test` this agent is not allowed to run).
- [x] 19 switch lines (§6); `accounting/types/contract.check.ts` with the 12 entries of §2 (plus 12b's 5, in the same shared file).
- [x] Report to the manager: 19 command names, `pub mod accounting;`, `accounting::register_undo` hook line, gaps — see this wave's final report.
- [x] Status note at the top of this file.

## Gate

- [ ] `cargo check --workspace --all-targets` clean (manager's throttled run after W5) — not run by this agent (hard rule: no cargo).
- [x] `tests/domain_accounting.rs` written; DB tests and the 12 parity cases run in the deferred pass — writing done, running deferred.
- [x] 19 switch lines present; `contract.check.ts` compiles against the generated bindings — lines present; compiling against `types/gen/*` needs `bun run bindings` (manager, after `cargo check`).
- [ ] `bun run memory:check`: 19 commands registered and invoked, 0 contract gaps — deferred to the manager's post-wave run.
