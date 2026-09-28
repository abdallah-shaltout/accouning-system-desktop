# 21 · 03.11 — `expenses` (expense categories, one-shot expenses, recurring-expense templates)

> **Status:** planned 2026-09-28, not implemented. Wave W2 (entry file §4). Depends on: 00-import
> (demo data), 01-settings (payment methods, taxes, `settings` row), 05-parties (supplier rows +
> the `PartyKind` DTO), Part 02 (`shared::ledger`, `shared::numbering`, `shared::activity`).
> Needs the Part 02 API items in §7 "Needs from manager" before it compiles (`SystemRole::from_str`).

**Goal.** Port the 11 `expenseService` functions (all `port`, analysis §1) as 11 IPC commands that
return exactly the mock's DTOs: category CRUD, the expense list/detail, `createExpense` (one balanced
entry: Dr expense[cost center] + VAT input / Cr payment-method account or `payable[supplier]`), and
the recurring-template due list with its one-click post. Posting goes only through
`shared::ledger::post`; numbers through `shared::numbering`; audit through `shared::activity`.

**Read first:** [`../01-frontend-analysis/expenses.md`](../01-frontend-analysis/expenses.md) (§1
endpoints, §2 DTOs, §3 messages, §4 undo, §5 races, §6 events, §8/§9 open items) · mock
`src/mocks/backend/expenses.ts` (whole file, 186 lines) · `src/modules/expenses/services/expenseService.ts`
(1-89) · `src/modules/expenses/types/index.ts` (1-102) · entities `src-tauri/src/entities/expenses/{expense_categories,expenses,recurring_expenses}.rs`
· migration `src-tauri/migration/src/m0011_expenses.rs` (the `date_key` generated column, `uq_expense_categories_name_live`,
`ck_recurring_expenses_day`) · [`../02-core-and-shared/phase-c-ledger.md`](../02-core-and-shared/phase-c-ledger.md).

## 1. Commands

Area/Access comes from the pages that call each function (`expenses/routes/*.ts`: `expenses`,
`expense-new`, `expenses-recurring` → area `expenses`; `settings-expenses` → area `settings`). Reads that
need no business clock run in `with_read` and check access with
`crate::core::settings::require(tx, actor.as_ref(), area, Access::Read)` (the actor is cloned from
`state.session` before the closure; no session → `UNAUTHORIZED` `سجّل الدخول أولاً`). Reads that need
"today" use `with_tx` and touch nothing.

| Mock service fn (`expenseService.ts`) | Disp. | Rust command | Args → Return DTO | Area / Access | Tx | Events |
|---|---|---|---|---|---|---|
| `getExpenseCategories` (:25) | port | `expenses_get_expense_categories` | `()` → `Vec<ExpenseCategory>` | Expenses / Read | `with_read` | — |
| `saveExpenseCategory` (:30) | port | `expenses_save_expense_category` | `ExpensesSaveExpenseCategoryArgs { input: ExpenseCategoryInput, id: Option<Id> }` → `ExpenseCategory` | Settings / Write | `with_tx` | — |
| `deleteExpenseCategory` (:35) | port | `expenses_delete_expense_category` | `ExpensesDeleteExpenseCategoryArgs { id }` → `()` | Settings / Write | `with_tx` | — |
| `getExpenses` (:44) | port | `expenses_get_expenses` | `ExpensesGetExpensesArgs { filter: Option<ExpenseFilter> }` → `Vec<ExpenseRow>` | Expenses / Read | `with_read` | — |
| `getExpense` (:53) | port | `expenses_get_expense` | `ExpensesGetExpenseArgs { id }` → `ExpenseRow` | Expenses / Read | `with_read` | — |
| `createExpense` (:60) | port | `expenses_create_expense` | `ExpensesCreateExpenseArgs { input: ExpenseInput }` → `Expense` | Expenses / Write | `with_tx` | Ledger (+ Parties on credit) via `ledger::post` |
| `getRecurringExpenses` (:65) | port | `expenses_get_recurring_expenses` | `()` → `Vec<RecurringExpense>` | Expenses / Read | `with_read` | — |
| `saveRecurringExpense` (:70) | port | `expenses_save_recurring_expense` | `ExpensesSaveRecurringExpenseArgs { input: RecurringExpenseInput, id: Option<Id> }` → `RecurringExpense` | Expenses / Write | `with_tx` | — |
| `deleteRecurringExpense` (:75) | port | `expenses_delete_recurring_expense` | `ExpensesDeleteRecurringExpenseArgs { id }` → `()` | Expenses / Write | `with_tx` | — |
| `getDueRecurringExpenses` (:81) | port | `expenses_get_due_recurring_expenses` | `()` → `Vec<RecurringExpense>` | Expenses / Read | `with_tx` (needs `cx.clock.today()`) | — |
| `postDueRecurringExpense` (:86) | port | `expenses_post_due_recurring_expense` | `ExpensesPostDueRecurringExpenseArgs { id }` → `Expense` | Expenses / Write | `with_tx` | Ledger (+ Parties on credit) |

Reason for Settings/Write on category writes: the only caller is `ExpenseCategoriesSettingsPage.vue`
(route `settings-expenses`, area `settings`); `getExpenseCategories` stays Expenses/Read because the
expense form and list call it too (`ExpenseFormPage.vue:53`, `ExpenseListPage.vue:50`). No
`stay`/`drop` functions in this module (analysis §1: 11/11 port).

## 2. DTOs (`domains/expenses/dto.rs`, `#[ts(export_to = "expenses/types/gen/")]`)

All structs `#[serde(rename_all = "camelCase")]` + `#[skip_serializing_none]`; optional TS fields get
`#[ts(optional)]`; `Id` fields `#[ts(type = "string")]`; `Decimal` fields
`#[serde(with = "crate::utils::money::serde_number")]` + `#[ts(type = "number")]` (P2-33); `DocDate`
fields serialize as the mock's exact string (`DocDate::key()`, P2-09) with `#[ts(type = "string")]`.

| Rust DTO | TS type (file:line) | Notes |
|---|---|---|
| `ExpenseCategory` | `ExpenseCategory` (`types/index.ts:7`) | `icon`, `default_tax_id`, `default_cost_center_id` optional; `can_delete` server-set. |
| `ExpenseCategoryInput` | `ExpenseCategoryInput` (`types/index.ts:20`) | `Omit<…,'id'|'canDelete'>` written out flat (Deserialize + TS). |
| `ExpensePaidFrom` | `ExpensePaidFrom` (`types/index.ts:23`) | `#[serde(tag = "kind", rename_all = "camelCase")] enum { Method { #[serde(rename = "paymentMethodId")] payment_method_id: Id }, Credit { #[serde(rename = "supplierId")] supplier_id: Id } }` — per-field renames (not `rename_all_fields`) so ts-rs emits the exact union. Maps to `paid_from_kind` + the two nullable columns (entity `expenses.rs`). |
| `Expense` | `Expense` (`types/index.ts:25`) | `date: DocDate`; `amount`/`net_amount`/`tax_amount` Decimal; `attachment_ids: Option<Vec<String>>` (kept `Some(vec![])` when the input sent `[]`, analysis §2); `branch_id` always `None` (inert, analysis §2). |
| `ExpenseRow` | `ExpenseRow` (`services/expenseService.ts:23`) | Flat struct: every `Expense` field + `category_name: String` (no `#[serde(flatten)]`, so the generated type is a plain object that `Equals` can match). |
| `ExpenseInput` | `ExpenseInput` (`types/index.ts:50`) | `date: String` parsed with `RawDocDate::parse` → `resolve(&cx.clock)`; parse failure → `VALIDATION` `التاريخ غير صالح` (decision E-D5). `amount: Decimal`. |
| `ExpenseFilter` | `ExpenseFilter` (`types/index.ts:66`) | all four fields optional strings (`to` is a date bound, analysis §2 false-positive note). |
| `RecurringExpense` | `RecurringExpense` (`types/index.ts:74`) | `day: i32` (entity `i8`), `next_date: NaiveDate` serialized `YYYY-MM-DD` (`#[ts(type = "string")]`). |
| `RecurringExpenseInput` | `RecurringExpenseInput` (`types/index.ts:90`) | independent struct (the TS type does not use `Omit`, analysis §2). |
| `Expenses…Args` (11) | the service parameter lists | one per command, camelCase, `Deserialize + TS`. |

**`contract.check.ts`** — new file `src/modules/expenses/types/contract.check.ts` (pattern:
`core/types/contract.check.ts`): one `Expect<Equals<Gen.X, X>>` each for `ExpenseCategory`,
`ExpenseCategoryInput`, `ExpensePaidFrom`, `Expense`, `ExpenseInput`, `ExpenseFilter`,
`RecurringExpense`, `RecurringExpenseInput`, and `ExpenseRow` (imported with `import type { ExpenseRow }
from '../services/expenseService'`). 9 entries.

## 3. Service logic (`domains/expenses/service.rs`; split into `service/{categories,expenses,recurring}.rs` if it passes ~400 lines)

Every function: `pub async fn f<C: ConnectionTrait>(conn: &C, cx: &TxCtx, undo: &UndoRegistry, …) -> TxResult<T>`
for writes (the registry comes from `state.undo`; nothing here records an `UndoSpec`), and
`f(conn, …)` for `with_read` reads. Lookups of master data use `find_live()` (P2-16) so a soft-deleted
row behaves like the mock's hard-deleted one.

**3.1 `get_expense_categories`** — `expense_categories` live rows `ORDER BY created_at, id` (mock array
order, analysis §1 "unsorted"). Map to `ExpenseCategory`.

**3.2 `save_expense_category(input, id)`** — ports `expenses.ts:18-30`, in this order:
1. `input.name.trim()` empty → `VALIDATION` `الاسم مطلوب` (:19).
2. `shared::ledger::accounts::account_by_id(conn, input.account_id)` → its `NOT_FOUND`
   `الحساب غير موجود في شجرة الحسابات` (:20; runs **before** the existence check, as in the mock).
3. Update (`id` set): `core::lock::for_update_by_id(conn, "expense_categories", id)`; load live row →
   missing → `NOT_FOUND` `التصنيف غير موجود` (:23).
4. Name uniqueness among live rows other than `id` (the DB has `uq_expense_categories_name_live`, which the
   mock does not — decision E-D1): duplicate → `CONFLICT` `اسم التصنيف مستخدم من قبل`; also map a racing
   insert's unique violation with `AppError::map_unique("uq_expense_categories_name_live", …)` to the same text.
5. Update: set `name = trim`, `account_id`, `active`; `icon`/`default_tax_id`/`default_cost_center_id` are
   set only when present in the input (JSON-absent = the mock's `Object.assign` leaving the key alone,
   :24 — decision E-D2). Create: new `Id`, `can_delete = true` (:27), absent optionals → `NULL`.
6. No audit/activity row (mock writes none, analysis §6/§8 — kept, quirk Q3). Return the saved row.

**3.3 `delete_expense_category(id)`** — `expenses.ts:32-38`: lock row `FOR UPDATE`; missing →
`NOT_FOUND` `التصنيف غير موجود`; `!can_delete` → `FORBIDDEN` `لا يمكن حذف تصنيف أساسي — يمكن إلغاء تفعيله فقط`
(:35, fixed to FORBIDDEN in 01.C); `EXISTS (SELECT 1 FROM expenses WHERE category_id = ?)` → `CONFLICT`
`لا يمكن حذف تصنيف له مصروفات مسجلة — قم بإلغاء تفعيله بدلاً من ذلك` (:36); then soft delete
(`deleted_at = cx.clock.now`, P2-16). No audit row (Q3).

**3.4 `get_expenses(filter)`** — `expenseService.ts:44-51` (search haystack decision E-D3):
1. SQL: `expenses` (+ `LEFT JOIN` live `expense_categories` for the name) with `category_id = ?` when
   `filter.categoryId` is non-empty, `date_day >= from` / `date_day <= to` when set and non-empty
   (`inDateRange` = business day, `utils::dates::in_date_range`), `ORDER BY date_key DESC, created_at ASC, id ASC`
   (mock: stable `b.date.localeCompare(a.date)` on the raw string; `date_key` is the generated mock string
   from m0011, ties keep insertion order).
2. `category_name` = the joined name, else `—` (:41).
3. Rust-side search: keep rows where `utils::text::matches_search(&[number, category_name, description,
   supplier_invoice_no], filter.search)` (:49) — exact `includesText` parity.

**3.5 `get_expense(id)`** — load by id → missing → `NOT_FOUND` `المصروف غير موجود` (:56); same
`ExpenseRow` mapping.

**3.6 `record_expense(conn, cx, undo, input, recurring: Option<Id>) -> TxResult<Expense>`** — the shared
body of `create_expense` and `post_due_recurring_expense`, porting `recordExpense` (`expenses.ts:66-117`):
1. `validateExpenseInput` (:52-63), in order: live category by `input.category_id` → missing →
   `VALIDATION` `اختر تصنيف المصروف`; `!(amount > 0)` → `VALIDATION` `المبلغ يجب أن يكون أكبر من صفر`;
   `Method`: live **and** `active` payment method → else `VALIDATION` `اختر طريقة الدفع`; `Credit`: live party
   with `kind = supplier` → else `VALIDATION` `اختر المورد`.
2. `splitTax` (:44-50): not a tax invoice → `net = round2(amount)`, `vat = 0`; else rate = the live **active**
   tax's `rate` when `tax_id` resolves, otherwise `0` (silent fallback kept, quirk Q2); `net = round2(amount /
   (1 + rate/100))` on the **raw** input amount, `vat = round2(amount − net)` (two-step order, analysis §2).
3. Period locks first (global lock order, entry §3.3 rule 4): `shared::ledger::period::assert_open_period(conn,
   &date.day, false)` — takes settings S + fiscal-year S and raises the mock's `FORBIDDEN` period messages.
   Observationally identical to the mock (between validation and `postJournal`'s own check the mock only
   allocates a number and pushes the row, neither of which can fail), and it keeps settings → year →
   counters ordering. Expenses never pass `allow_closed_period` (analysis §3: no admin override here).
   Then `core::lock::share_lock_by_id(conn, "expense_categories", category_id)` and re-read the category
   with `find_live` → gone → `VALIDATION` `اختر تصنيف المصروف` (closes the delete-vs-create race, §4).
4. `number = shared::numbering::next_number(conn, DocumentKind::Expense)` (`EXP-000001`, :72).
5. Insert `expenses` (:70-90): `amount = round2(input.amount)`, `net_amount`/`tax_amount` from step 2,
   `date` via `entities::doc_date::write`, `paid_from_*` from the enum, `repeat_monthly = input.repeat_monthly
   .unwrap_or(false)` (:86), `recurring_template_id`, `created_by = cx.actor.id`, `branch_id = NULL`.
6. Posting lines (:92-102): `Dr AccountRef::Id(category.account_id)` = `net`, `cost_center_id =
   input.cost_center_id`; if `vat > 0`: `Dr AccountRef::Role(SystemRole::VatInput)` = `vat`; credit leg:
   `Method` → `AccountRef::Role(SystemRole::from_str(&method.account_role)?)` = `amount`, no branch/currency ctx
   (the mock passes none); `Credit` → `AccountRef::Role(SystemRole::Payable)` = `amount` with
   `party = PartyRef { kind: Supplier, id }`.
7. `shared::ledger::post(conn, cx, PostJournal { date, description: "مصروف {number} — {category.name}" +
   (": {description}" when `description` is non-empty), entry_type: System, source: Some(SourceRef { kind:
   "expense", id, number }), lines, allow_closed_period: false, attachment_ids, template_id: None })`
   (:104-112). `post` refuses a group account and an unbalanced set with the mock's messages, touches
   `Ledger`, and `Parties` for the credit leg (P2-12).
8. `shared::activity::record::log(conn, cx, undo, ActivityKind::Expense, "مصروف {number} — {category.name}
   بقيمة {amount:.2}", Some(date), Some(RouteRef::detail("expense-detail", id)))` (:114). No `UndoSpec`
   (analysis §4: not undoable; corrections are manual entries).
9. Return the `Expense` DTO built from the inserted row (never the input).

**3.7 `create_expense(input)`** = `record_expense(…, input, None)` (`expenseService.ts:62`).

**3.8 `get_recurring_expenses`** — live rows `ORDER BY created_at, id` (:67).

**3.9 `save_recurring_expense(input, id)`** — `expenses.ts:136-149`, in order: name trim empty →
`VALIDATION` `الاسم مطلوب`; live category → else `VALIDATION` `اختر تصنيف المصروف`; `1 <= day <= 28` → else
`VALIDATION` `يوم الاستحقاق بين 1 و 28` (the DB `ck_recurring_expenses_day` backs it); `next_date` parses as
`YYYY-MM-DD` → else `VALIDATION` `التاريخ غير صالح` (E-D5); update: lock + load live → `NOT_FOUND`
`القالب غير موجود`; set every present field (`name` trimmed; optional `tax_id`/`description` only when
present, E-D2); **`amount` is stored as sent, no `round2`** (analysis §2; the `DECIMAL(19,2)` column scale is
the only rounding — quirk Q5). Create: new `Id`. No audit row (Q3).

**3.10 `delete_recurring_expense(id)`** — lock + load live → `NOT_FOUND` `القالب غير موجود`; soft delete.
No reference check against `expenses.recurring_template_id` (analysis §8: breadcrumb only). No audit (Q3).

**3.11 `due_recurring_expenses(conn, today: NaiveDate) -> TxResult<Vec<RecurringExpense>>`** — `pub`
(`expenses.ts:158-160`): live rows with `active AND next_date <= today`, `ORDER BY created_at, id`. The
command passes `cx.clock.today()`. **The dashboard insight rule (14-analytics, `recurring-expense-due`,
`insightRules.ts:463`) must call this function**, not re-derive the filter (analysis §7).

**3.12 `post_due_recurring_expense(id)`** — `expenses.ts:163-182` plus the D8 fix from analysis §5:
1. Read the template (live, no lock yet) → missing → `NOT_FOUND` `القالب غير موجود` (:165).
2. Build the `ExpenseInput` from the template's **current** values (:166-178): `date = DocDate { day:
   cx.clock.today(), instant: Some(cx.clock.now) }` (the mock passes `new Date().toISOString()`),
   `category_id`, `amount`, `is_tax_invoice`, `tax_id`, `paid_from`, `description`, `repeat_monthly = true`,
   `recurring_template_id = id`; no cost center, VAT number, supplier invoice no or attachments.
3. Take the period locks (`assert_open_period(date.day, false)`), then `FOR UPDATE` the template row and
   re-read it. If it is gone, `!active`, or `next_date > today` → `CONFLICT`
   `المصروف المتكرر لم يعد مستحقاً — ربما سُجّل من جهاز آخر` (decision E-D4; the due panel only ever offers
   due, active templates, `ExpenseListPage.vue:51-59`, so this fires only on a double post).
4. `record_expense(…)` (3.6) with the re-read values.
5. `next_date = next_month_date(template.day, template.next_date)` (:130-134): year/month of `next_date`
   plus one month (December rolls to January of the next year), day `min(day, 28)` — always a valid date,
   no clamping branch needed (analysis §2). Update the row.
6. Return the `Expense`.

## 4. Concurrency (D8, analysis §5)

- **Numbering:** `next_number(Expense)` then `post`'s `next_number(Journal)` — both counter rows are X-locked
  until commit; gapless (a rollback releases both).
- **Double "record now":** 3.12 step 3 (period S locks → template X lock → re-verify due inside the same
  transaction). The second terminal waits on the row lock, then sees the advanced `next_date` and gets
  `CONFLICT`; it can never post the same due date twice or advance twice.
- **Category delete vs a concurrent `create_expense`:** the delete holds the category's X lock; the
  expense path takes an S lock on the same row (3.6 step 3) and re-reads it. Whoever locks first wins: the
  delete commits first → the expense re-read sees `deleted_at` and refuses with `اختر تصنيف المصروف`; the
  expense commits first → the delete's `EXISTS` check sees it and refuses with `CONFLICT`. A soft delete
  keeps the row, so the FK alone would not have caught this.
- **Category name race:** `uq_expense_categories_name_live` + `map_unique` (3.2 step 4).
- Lock order in every write: settings S → fiscal year S → template/category X → counters → documents →
  `change_versions` (bumped last by `with_tx`).

## 5. Undo

Not undoable via the registry (phase-e E-5 lists no `expenses` action; analysis §4). No `register_undo`
in `domains/expenses/mod.rs`. A posted expense is corrected by a manual journal entry.

## 6. Frontend switch lines (`src/modules/expenses/services/expenseService.ts`)

Import `usesRust`, `backendCall` from `@/modules/core/services/backend`. First line inside each `wrap`
body, before `await delay()`; the mock body below stays unchanged:

| Fn (line) | Line to add |
|---|---|
| `getExpenseCategories` (:25) | `if (usesRust('expenses')) return backendCall('expenses_get_expense_categories');` |
| `saveExpenseCategory` (:30) | `if (usesRust('expenses')) return backendCall('expenses_save_expense_category', { input, id });` |
| `deleteExpenseCategory` (:35) | `if (usesRust('expenses')) { await backendCall('expenses_delete_expense_category', { id }); return; }` |
| `getExpenses` (:44) | `if (usesRust('expenses')) return backendCall('expenses_get_expenses', { filter });` |
| `getExpense` (:53) | `if (usesRust('expenses')) return backendCall('expenses_get_expense', { id });` |
| `createExpense` (:60) | `if (usesRust('expenses')) return backendCall('expenses_create_expense', { input });` |
| `getRecurringExpenses` (:65) | `if (usesRust('expenses')) return backendCall('expenses_get_recurring_expenses');` |
| `saveRecurringExpense` (:70) | `if (usesRust('expenses')) return backendCall('expenses_save_recurring_expense', { input, id });` |
| `deleteRecurringExpense` (:75) | `if (usesRust('expenses')) { await backendCall('expenses_delete_recurring_expense', { id }); return; }` |
| `getDueRecurringExpenses` (:81) | `if (usesRust('expenses')) return backendCall('expenses_get_due_recurring_expenses');` |
| `postDueRecurringExpense` (:86) | `if (usesRust('expenses')) return backendCall('expenses_post_due_recurring_expense', { id });` |

The two `void` functions await and return, because a `()` command's generated return type is `null`,
which is not assignable to `Promise<void>`.

## 7. Known mock quirks (kept) and decisions

**Quirks kept (behaviour-exact, entry §3.3) — each is a later user decision, not fixed here:**
- Q1 — `recordExpense` pushes the `Expense` row before `postJournal`; a period refusal leaves an orphan
  expense and a burnt number in the mock. Rust is atomic (rule 4): refusal → no row, no number. Part 04's
  parity case for the refusal compares the error only.
- Q2 — `splitTax` silently uses rate 0 for a missing/inactive tax on a tax invoice (analysis §3/§9).
- Q3 — the four master-data writes (category save/delete, recurring save/delete) write no audit row
  (analysis §6/§8; same cross-module policy question as products.md).
- Q4 — `RecurringExpense.autoPost` is stored and returned but read by nothing (analysis §9, option (a)).
- Q5 — `RecurringExpense.amount` is not rounded on save; Rust stores it at the column's 2-dp scale, so a
  3-dp input (not producible by the form's money input) would read back rounded.
- Q6 — `Expense.branchId` and `ExpenseCategory.defaultTaxId/defaultCostCenterId` are inert (analysis §2/§9).
- Q7 — a credit expense posts to `payable[supplier]`; Rust also touches `Parties` because `ledger::post`
  derives it from the party line (P2-12). This closes the analysis §6 event gap structurally; no parity impact.

**Decisions (strictest option, logged):**
- E-D1 — keep Part 02's `uq_expense_categories_name_live` (the mock allows duplicate names) and refuse a
  duplicate with `CONFLICT` `اسم التصنيف مستخدم من قبل`. Consequence for 00-import: a snapshot with two
  live categories of the same name must be handled there (see "Needs from manager").
- E-D2 — on update, an optional input field that is absent in the JSON keeps the stored value (the mock's
  `Object.assign` with the key absent); every caller sends the keys it means to change
  (`ExpenseCategoriesSettingsPage.vue:65`, `RecurringExpensesPage.vue:78-90`).
- E-D3 — search haystack with a joined name (Part 02 handoff §9): **filter in Rust after SQL narrows the
  rows**, no stored joined name. Reasons: `expenses` has no `search_normalized` column (P2-38), a category
  rename would make a stored joined name stale, and `getExpenses` is unpaged (it already returns the full
  filtered set), so Rust-side filtering costs nothing extra.
- E-D4 — `post_due_recurring_expense` re-verifies "active and due" under the row lock and refuses with the
  `CONFLICT` above (analysis §5 fix). Unreachable from the UI outside a race.
- E-D5 — an unparsable date string is refused with `VALIDATION` `التاريخ غير صالح` (the mock would store any
  string); the forms always send `dateKeyToIso(...)` or a `YYYY-MM-DD` key.
- E-D6 — `autoPost` stays inert (analysis §9 recommendation (a)); no scheduler in Part 03.

**Needs from manager (Part 02 API gaps found while planning):**
- `shared::ledger::accounts::SystemRole` needs `impl FromStr` (the payment method's `account_role` string →
  role; `payment_methods.account_role` is a `String`). Unknown string → `AppError::internal`.
- 00-import: define how duplicate live `expense_categories` / `journal_templates` names in a snapshot are
  imported under the `*_name_live` unique keys (recommend: suffix ` (2)`, ` (3)`… and record it in the import report).

## 8. Tests

**(a) `src-tauri/tests/domain_expenses.rs`** (uses `tests/support::TestDb`; seeds a settings row, the
role accounts `vatInput`/`payable`/`cash`, one expense account, a 15% active tax, a cash payment method, one
supplier and one open fiscal year — the 00-import demo fixture once it exists, else a local
`seed_expenses_fixture` modelled on `tests/shared_ledger.rs:55`). Every posting test ends with
`shared::invariants::run_all` all green.
- Category save: create sets `can_delete = true`; update keeps `icon` when absent; empty name → `الاسم مطلوب`;
  unknown account → `الحساب غير موجود في شجرة الحسابات`; unknown id → `التصنيف غير موجود`; duplicate name →
  `اسم التصنيف مستخدم من قبل`.
- Category delete: protected → `FORBIDDEN`; with an expense → `CONFLICT`; success soft-deletes and the list omits it.
- Create (method, no VAT): `EXP-000001`, entry `Dr expense / Cr cash`, source `expense`, activity row
  `مصروف EXP-000001 — … بقيمة 100.00`, link `expense-detail`.
- Create (tax invoice 115 @15%): `net 100`, `tax 15`, three lines incl. `vatInput`; `10.005`-style amount rounds via `round2`.
- Create (credit): credit leg `payable` with the supplier party; `Parties` touched.
- Each validation message in 3.6 step 1, in order (first failing rule wins).
- Period: lock-date and closed-year refusals give the mock's `FORBIDDEN` text; nothing inserted, counter unchanged.
- List: category/date filters, Arabic-normalized search hits `categoryName` (`أ`/`ا` variants), order
  `date_key DESC` with ties in insertion order; `get_expense` on an unknown id → `المصروف غير موجود`.
- Race: a category delete and an expense create on two connections — exactly one succeeds, the other gets
  `CONFLICT` (delete) or `اختر تصنيف المصروف` (create).
- Recurring save: day 0/29 → message; update keeps absent `description`; amount stored as sent.
- Due list: only `active && next_date <= today`.
- Post due: posts with the template's current values, `repeat_monthly = true`, `recurring_template_id` set,
  `next_date` → next month day `min(day,28)` (Dec → Jan rollover).
- **Concurrency (two connections):** two `post_due_recurring_expense` on the same template → exactly one
  `Expense`, one advance, the other `CONFLICT`; 20 concurrent `create_expense` → 20 distinct gapless numbers.

**(b) Parity cases for Part 04** (replayed on mock and Rust, DTOs + GL diffed):
`expenses/create-cash-no-vat`, `expenses/create-tax-invoice-15`, `expenses/create-credit-supplier`,
`expenses/create-period-locked` (error only, Q1), `expenses/category-crud`, `expenses/category-delete-refusals`,
`expenses/list-filter-search`, `expenses/recurring-crud`, `expenses/post-due-and-advance`.

## 9. Checklist

- [ ] Confirm the manager has added `SystemRole: FromStr` (§7); if not, stop and request it.
- [ ] `domains/expenses/mod.rs` (`pub mod commands; pub mod service; pub mod dto;` + `ipc_signatures()` with 11 `ipc_sig!` lines).
- [ ] `dto.rs`: the 9 DTOs + 11 `…Args` structs of §2, with ts-rs attributes.
- [ ] `service` 3.1–3.3 (categories), 3.4–3.7 (expenses, incl. `record_expense`), 3.8–3.12 (recurring, `due_recurring_expenses` `pub`).
- [ ] `commands.rs`: 11 commands per §1 (Area/Access, `with_read`/`with_tx`, `ApiErrorPayload` mapping).
- [ ] `tests/domain_expenses.rs`: every §8(a) bullet.
- [ ] Switch lines in `expenseService.ts` (§6, 11 lines + the import).
- [ ] `src/modules/expenses/types/contract.check.ts` (9 entries, §2).
- [ ] Report to the manager: the 11 command names for `generate_handler!`, `pub mod expenses;` for `domains/mod.rs`, the 00-import duplicate-name note.
- [ ] Status note at the top of this file.

## Gate

- [ ] `cargo check --workspace --all-targets` clean (manager's throttled run after W2).
- [ ] `tests/domain_expenses.rs` written (run in the deferred time-boxed pass).
- [ ] 11 switch lines present; `contract.check.ts` compiles against the generated types (`bun run bindings` by the manager).
- [ ] `bun run memory:check`: 11 new commands registered and invoked, 0 contract gaps.
- [ ] DB tests + the 9 parity cases run in the deferred pass (Part 04 gates the flip on them).
