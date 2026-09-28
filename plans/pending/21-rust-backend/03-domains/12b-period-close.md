# 21 · 03.12b — `accounting` part 2 (fiscal years, lock date, year close/reopen, VAT settlement)

> **Status:** implemented 2026-09-28 (code complete; DB tests + parity cases deferred to the
> time-boxed test pass — see §8/§9). All 11 commands, DTOs, service logic, the fourth undo
> compensator (`accounting.closeYear`), 11 switch lines and the shared `contract.check.ts`'s 5
> entries are written, in the same commit as 12-accounting (one `domains/accounting/` module). Not
> yet compiled or run against a live DB. The four open questions in §7 are left as pending user
> decisions (mock behaviour kept, per the spec).
>
> Wave W5, implemented **after**
> [`12-accounting.md`](12-accounting.md) in the same `domains/accounting/` module (it reuses 12's
> `service::rows` DTO mapping and `domains/accounting/mod.rs`). Depends on: 12, 10-vouchers
> (`payVatSettlementNow` posts through the payment-voucher helper), Part 02 (`shared::ledger::{post,
> reverse, period::{assert_open_period, lock_fiscal_year_exclusive}}`, `shared::activity`). **Split note:**
> the accounting module's 30 `port` functions are divided 19 (file 12) + 11 (this file). This file also owns
> the fourth accounting undo compensator, `accounting.closeYear`. FX revaluation is 01-settings
> (`settings_post_revaluation`) and opening balances are 02-setup — not here.

**Goal.** Port the 11 period-close functions of `accountingService.ts` as IPC commands with the mock's exact
DTOs: fiscal-year list/current/save, lock date, the closing wizard's pre-checks, `closeYear` (one
transaction: closing entry + lock + next year) and admin-only `reopenYear` (mirror of the closing entry),
and the VAT settlement pair. `closeYear`/`reopenYear` use `lock_fiscal_year_exclusive` so no poster can
slip an entry into a year while it is being closed (P2-13, analysis §5's sharpest race).

**Read first:** [`../01-frontend-analysis/accounting.md`](../01-frontend-analysis/accounting.md) §1 rows
`getFiscalYears`…`payVatSettlementNow`, §3, §4, §5 rows 4–6, §7, §9 D-A2/D-A3/D-A6/D-A7 ·
[`../../../../docs/v2/02-accounting-review.md`](../../../../docs/v2/02-accounting-review.md) B2 (closing
wizard), §3 "VAT settlement"/"VAT payment" · mock `accountingService.ts` (328-396, 453-469) ·
`src/mocks/backend/core.ts` (484-677) · `src/mocks/backend/journal.ts` (223-305) ·
`src/mocks/backend/vouchers.ts` (60-85) · Rust `shared/ledger/period.rs`, `shared/ledger/reverse.rs`,
entities `org/fiscal_years.rs`, `org/settings.rs` (`accounting: Option<AccountingPolicy>`, `values.rs:177`).

## 1. Commands

Same conventions as 12 §1 (`with_read` reads check access through `core::settings::require`; "session
only" = no area check).

| Mock fn (`accountingService.ts`) | Disp. | Rust command | Args → Return | Area / Access | Tx | Events |
|---|---|---|---|---|---|---|
| `getFiscalYears` (:330) | port | `accounting_get_fiscal_years` | `()` → `Vec<FiscalYear>` | session only (`BudgetVsActualPage.vue:15`) | `with_read` | — |
| `getCurrentFiscalYear` (:336) | port | `accounting_get_current_fiscal_year` | `()` → `Option<FiscalYear>` | session only (`reports/controllers/useReportRange.ts:3`) | `with_tx` (needs `cx.clock.today()`, touches nothing) | — |
| `saveFiscalYear` (:343) | port | `accounting_save_fiscal_year` | `AccountingSaveFiscalYearArgs { input: FiscalYearInput, id: Option<Id> }` → `FiscalYear` | Accounting / Write | `with_tx` | — |
| `getLockDate` (:365) | port | `accounting_get_lock_date` | `()` → `Option<String>` | Accounting / Read | `with_read` | — |
| `saveLockDate` (:370) | port | `accounting_save_lock_date` | `AccountingSaveLockDateArgs { lock_date: Option<String> }` → `()` | Accounting / Write | `with_tx` | — |
| `getCloseYearPreChecks` (:380) | port | `accounting_get_close_year_pre_checks` | `…Args { fiscal_year_id: Id }` → `Vec<CloseYearPreCheck>` | Accounting / Read | `with_read` | — |
| `closeYear` (:385) | port | `accounting_close_year` | `…Args { fiscal_year_id }` → `CloseYearResult` | Accounting / Write | `with_tx` | Ledger |
| `reopenYear` (:392) | port | `accounting_reopen_year` | `…Args { fiscal_year_id }` → `FiscalYear` | Accounting / Write **and** role admin | `with_tx` | Ledger |
| `getVatPeriodTotals` (:455) | port | `accounting_get_vat_period_totals` | `…Args { from: String, to: String }` → `VatPeriodTotals` | Accounting / Read | `with_read` | — |
| `submitVatSettlement` (:460) | port | `accounting_submit_vat_settlement` | `…Args { from, to }` → `JournalEntry` | Accounting / Write | `with_tx` | Ledger |
| `payVatSettlementNow` (:466) | port | `accounting_pay_vat_settlement_now` | `…Args { amount: Decimal, payment_method_id: Id }` → `JournalEntry` | Accounting / Write | `with_tx` | Ledger |

`closeYear` keeps the mock's access (any Accounting writer, `FiscalYearsPage.vue:189` `canWrite`);
`reopenYear` re-checks `role == admin` server-side (`accountingService.ts:394`, analysis §1: never trust
the hidden button).

## 2. DTOs (`domains/accounting/dto/period.rs`, `export_to = "accounting/types/gen/"`)

| Rust DTO | TS type (file:line) | Notes |
|---|---|---|
| `FiscalYear` | `types/index.ts:187` | `start_date`/`end_date` `YYYY-MM-DD`; `closing_entry_id`, `closed_at` (iso_ms), `closed_by` optional. |
| `FiscalYearInput` | `Omit<FiscalYear, 'id'>` (:343) | Deserializes all seven fields; `closing_entry_id`/`closed_at`/`closed_by` are accepted and **ignored** (server-owned, decision P-D3). Dates arrive as `String`. |
| `CloseYearPreCheckKey` | `'drafts' \| 'trialBalance' \| 'openingEquity'` (`core.ts:485`) | camelCase. |
| `CloseYearPreCheck` | `core.ts:484-489` | `{ key, label, passed, detail }`. |
| `CloseYearResult` | inline return of `closeYear` (:385) | `{ fiscal_year, closing_entry: JournalEntry, next_year: Option<FiscalYear> }`. |
| `VatPeriodTotals` | `journal.ts:227-231` | three Decimals. |
| `Accounting…Args` (8 with args) | parameter lists | camelCase. |

Type moves (type-only, zero behaviour change, same pattern as 12's `JournalTemplateInput`): move
`CloseYearPreCheck` (`core.ts:484`) and `VatPeriodTotals` (`journal.ts:227`) into
`src/modules/accounting/types/index.ts`; the mock files import + re-export them, and
`accountingService.ts:24`'s re-export keeps working.

**`contract.check.ts`** — append to the file 12 creates: `FiscalYear`, `FiscalYearInput` (vs
`Omit<FiscalYear, 'id'>`), `CloseYearPreCheck`, `VatPeriodTotals`, and `CloseYearResult` (vs
`Awaited<ReturnType<typeof closeYear>>`). 5 entries. `Option` returns generate `T | null`; the two switch
lines convert with `?? undefined` (§6), so the services keep their `T | undefined` signatures.

## 3. Service logic (`domains/accounting/service/{period,vat}.rs`)

Signatures as in 12 §3. `is_admin(cx)` from 12. Dates in args parse as `YYYY-MM-DD`
(`NaiveDate::parse_from_str`); where noted, an unparsable string is refused (decision P-D2).

### 3.1 Fiscal years
**`get_fiscal_years`** (:330-333): `ORDER BY start_date DESC, created_at, id` (stable sort over insertion order).

**`get_current_fiscal_year(today)`** (:336-341): first year (`ORDER BY created_at, id`) with `start_date <=
today <= end_date`, else the first of `ORDER BY start_date DESC, created_at, id`, else `None`.

**`save_fiscal_year(input, id)`** (:343-361), in the mock's order:
1. `input.name.trim()` empty → `VALIDATION` `اسم السنة المالية مطلوب`.
2. Either date empty/unparsable, or `end_date <= start_date` → `VALIDATION` `تاريخ النهاية يجب أن يكون بعد تاريخ البداية`.
3. `core::settings::load_shared_locked(conn)` (settings X — serialises every fiscal-year save so two terminals
   cannot create overlapping years; settings is first in the global lock order).
4. First other year (`id` excluded, `ORDER BY created_at, id`) with `start <= f.end AND end >= f.start` →
   `CONFLICT` `الفترة تتداخل مع السنة المالية {overlap.name}`.
5. Update: load → `NOT_FOUND` `السنة المالية غير موجودة`; the stored year `is_closed` → `FORBIDDEN`
   `لا يمكن تعديل سنة مالية مقفلة — أعد فتحها أولاً` (decision P-D1).
6. Write `name` (as sent — the mock does not trim it), `start_date`, `end_date`, `is_closed` (the form's
   "display only" switch, `FiscalYearsPage.vue:202`, is honoured as in the mock). Create: new `Id`, closing
   fields `NULL`.
7. `activity::log(Settings, "{تعديل|إضافة} السنة المالية {name}", now, RouteRef::list("fiscal-years"))` (:359).

### 3.2 Lock date
**`get_lock_date`** (:365-368): `settings.accounting.lock_date` → `Some("YYYY-MM-DD")` or `None`.

**`save_lock_date(lock_date)`** (:370-376): `load_shared_locked` (settings X); `None`/`""` → clear; a valid
`YYYY-MM-DD` → set; anything else → `VALIDATION` `تاريخ القفل غير صالح` (P-D2: the mock stores any string,
which would then break `assertOpenPeriod`'s comparison). Write the `accounting` JSON keeping its other keys
(`{ ...db.settings.accounting, lockDate }`). `activity::log(Settings, lock_date non-empty ? "تحديد تاريخ القفل
{lockDate}" : "إزالة تاريخ القفل", now, RouteRef::list("fiscal-years"))`. It only affects future postings
(analysis §1: confirmed safe). No event (mock emits none; every poster reads the row fresh under its S lock).

### 3.3 Closing wizard
**`close_year_pre_checks(conn, fy) -> TxResult<Vec<CloseYearPreCheck>>`** — `pub`, `core.ts:492-552`.
"In the year" uses the mock's `date.slice(0, 10)` rule: the UTC day for entries that carry an instant, the day
otherwise → SQL `COALESCE(DATE(date_instant), date_day) BETWEEN fy.start_date AND fy.end_date` (sessions run
in UTC, P2-05; quirk Q1).
1. Drafts in the year (`journal_drafts`, same rule) → `{ key: drafts, label: "لا توجد مسودات قيود بحاجة للترحيل",
   passed: n == 0, detail: n > 0 ? "{n} مسودة بحاجة للترحيل أو الحذف" : "لا توجد مسودات" }`.
2. Trial balance (`trialBalanceFor`, :492-514): per-account `SUM(debit), SUM(credit)` over posted lines in the
   year; iterate live non-group accounts in `created_at, id` order; `net = round2(d − c)`; positive adds to
   `debit`, negative subtracts from `credit`; `round2` both → `{ key: trialBalance, label: "ميزان المراجعة متوازن",
   passed: |debit − credit| < 0.01, detail: "مدين {debit} — دائن {credit}" }`.
3. `resolve_account(OpeningBalanceEquity)` (its label `NOT_FOUND` if missing); `round2(Σ debit − credit)` over
   **all** posted lines on it, not only the year's (quirk Q2) → `{ key: openingEquity, label: "حساب الأرصدة
   الافتتاحية (3900) صفر", passed: |net| < 0.01, detail: "الرصيد الحالي: {net}" }`.
Numbers in `detail` are formatted with `utils::money::js_number_string` (JS `String(number)`: `1500`, `1500.5`, `-3`).

**`get_close_year_pre_checks(id)`** (:380-383): load the year → `NOT_FOUND` `السنة المالية غير موجودة`; call the above.

**`close_year(id) -> TxResult<CloseYearResult>`** (`core.ts:560-637`), one transaction:
1. Locks: `core::settings::load` + `core::lock::share_lock_by_id(conn, "settings", id)` (S), then
   `ledger::period::lock_fiscal_year_exclusive(conn, fy_id)` (X) — the order its doc comment fixes. Posters
   dated in this year hold or wait for an S lock on the same row, so none can commit between the totals and the
   `is_closed` flip; in-flight posters finish first and are included in the totals.
2. Load → `NOT_FOUND`; `is_closed` → `VALIDATION` `السنة المالية مقفلة بالفعل`.
3. `close_year_pre_checks` again (never trust the wizard's earlier green checks); first failing →
   `FORBIDDEN` `تعذر إقفال السنة: {label} — {detail}`.
4. Closing lines (:568-598): per-account totals over the year (same UTC-day rule); live non-group accounts in
   `created_at, id` order; `REVENUE`: `b = round2(c − d)`, skip 0, `b > 0` → Dr `b` else Cr `−b`, `net += b`;
   `EXPENSE`: `b = round2(d − c)`, skip 0, `b > 0` → Cr `b` else Dr `−b`, `net −= b`; `net = round2(net)`;
   `resolve_account(RetainedEarnings)` (always, even when `net == 0`, as the mock does); `net > 0` → Cr, `net < 0`
   → Dr `−net`. Lines use `AccountRef::Id`, no party/branch (branch/cost-center defaults apply in `resolve_posting`).
5. `ledger::post(PostJournal { date: DocDate::from(fy.end_date), description: "قيد إقفال السنة المالية {name}",
   entry_type: Closing, lines, allow_closed_period: true, .. })`. Fewer than 2 lines (a year with no P&L
   movement) → the ledger's `يجب أن يحتوي القيد على سطرين على الأقل` (quirk Q3).
6. Update the year: `is_closed = true`, `closing_entry_id`, `closed_at = cx.clock.now`, `closed_by = actor`.
7. `activity::log_undoable(Journal, "إقفال السنة المالية {name}", now, RouteRef::list("fiscal-years"),
   UndoSpec { action_type: "accounting.closeYear", payload: { fiscalYearId } })` (:615, E-5).
8. Next year (:618-634): the first year (`created_at, id`) with `start_date > fy.end_date`; if none, insert one:
   `start = end + 1 day`; `end = js_add_years(start, 1) − 1 day`, where `js_add_years` keeps month/day and rolls
   Feb 29 to Mar 1 (JS `setFullYear`; so a Feb 29 start ends Feb 28 next year, not Feb 27 as a clamping add
   would give); `name` = the first ASCII digit run in `fy.name` parsed as `u64` plus 1, else `start_date`'s year
   plus 1, as a plain decimal string (`2026` → `2027`). No activity row for it (the mock writes none).
9. Return `{ fiscal_year, closing_entry: entry_dto, next_year }` — `next_year` is also returned when it already existed.

**`reopen_year(conn, cx, undo, id) -> TxResult<(FiscalYear, Id /*audit*/)>`** (`accountingService.ts:392-396`,
`core.ts:640-677`):
1. `!is_admin(cx)` → `FORBIDDEN` `إعادة فتح السنة المالية للمدير فقط` (before the existence check, as in the mock).
2. Locks as in `close_year` step 1. Load → `NOT_FOUND`; `!is_closed` → `VALIDATION` `السنة المالية غير مقفلة`.
3. If `closing_entry_id` names an existing entry with `!reversed`: `ledger::reverse(ReverseRequest { original_id,
   date: DocDate { day: today, instant: Some(now) }, description: "عكس قيد إقفال السنة المالية {name}",
   entry_type: Closing, allow_closed_period: true, reason: Some(ReversalReason { text: "إعادة فتح السنة المالية",
   stamp_original: false }), dims: MirrorDims::Default })` — the mirror carries the reason, the original only
   gets `reversed = true` (`core.ts:662-666`). Dated "now", not the year end (D-A2 kept, quirk Q4).
4. Update: `is_closed = false`, `closing_entry_id`/`closed_at`/`closed_by = NULL`.
5. `activity::log(Journal, "إعادة فتح السنة المالية {name}", now, RouteRef::list("fiscal-years"))` → audit id.
6. Return `(year, audit_id)`; the command returns the year.

### 3.4 VAT settlement (`vat.rs`)
**`vat_totals(conn, from, to)`** (`journal.ts:234-250`): `from`/`to` must parse (else `VALIDATION`
`التاريخ غير صالح`); `resolve_account(VatOutput)` then `resolve_account(VatInput)` (default ctx; their label
`NOT_FOUND`s in that order); over posted lines with `date_day BETWEEN from AND to` (the mock's `localDateKey`
= business day): `output = Σ(credit − debit)` on the output account, `input = Σ(debit − credit)` on the input
account; `round2` each; `net = round2(output − input)`.

**`submit_vat_settlement(from, to)`** (`journal.ts:256-279`):
1. `vat_totals`; both exactly 0 → `VALIDATION` `لا توجد حركة ضريبية في هذه الفترة`.
2. Resolve output, input, `VatPayable`. Lines in this order: `output > 0` → Dr output `output` "إقفال ضريبة
   المخرجات"; `input > 0` → Cr input `input` "إقفال ضريبة المدخلات"; `net > 0` → Cr payable `net` "صافي الضريبة
   المستحقة"; `net < 0` → Dr payable `−net` "صافي الضريبة القابلة للاسترداد".
3. `ledger::post(PostJournal { date: DocDate::from(to), description: "تسوية ضريبة القيمة المضافة — من {from} إلى {to}",
   entry_type: VatSettlement, lines, allow_closed_period: is_admin(cx), .. })`.
4. `activity::log(Journal, "تسوية ضريبة القيمة المضافة {number}", Some(entry.date), Some(RouteRef::detail("journal-entry", id)))`.
5. Return `entry_dto`. No overlap guard (D-A6 open, quirk Q5).

**`pay_vat_settlement_now(amount, payment_method_id)`** (`journal.ts:289-305`):
1. `amount <= 0` → `VALIDATION` `لا يوجد مبلغ مستحق للسداد`.
2. `resolve_account(VatPayable)`.
3. `crate::domains::vouchers::service::record_payment_voucher(conn, cx, undo, <PaymentVoucherInput> { date:
   now as an instant, description: "سداد ضريبة القيمة المضافة لمصلحة الزكاة والضريبة والجمارك", amount,
   payment_method_id, debit_account_id: payable.id, .. })` — the same `pub` function
   `vouchers_create_payment_voucher` uses (10-vouchers; ports `vouchers.ts:60-85`): its own validation, `VCH-`
   number, `Dr vatPayable / Cr method account`, `voucher` activity row. No admin override (the mock passes
   none, analysis §1). If plan 10 names the function differently, call that one — never a second copy.
4. Find the posted entry with `source_kind = 'voucher' AND source_id = voucher.id` → missing → `CONFLICT`
   `تعذر إنشاء قيد السداد` (unreachable, kept). Return `entry_dto`. No balance cap (D-A7 kept, quirk Q6).

## 4. Concurrency (D8, analysis §5)
- **Close vs posters:** settings S + year X for the whole close (P2-13); posters take settings S + year S in
  `assert_open_period`, so they wait and are then refused with `period_locked_by_year` unless they pass
  `allow_closed_period` (admin). Boundary caveat: the close totals use the UTC day, the poster's year lookup
  uses the business day (quirk Q1), so an entry posted at 00:00–03:00 local on 1 Jan is locked against the
  **new** year row; accepted as a mock-parity limit.
- **Two closes / two reopens:** the year X lock serialises them; the loser sees `مقفلة بالفعل` / `غير مقفلة`.
- **Fiscal-year save overlap and lock date:** settings X (`load_shared_locked`) serialises them with each other
  and with 01-settings' `updateSettings`; posters wait only for the few milliseconds of the save.
- **VAT:** two overlapping settlements both post (D-A6, mock parity); numbering is the ledger's.
- Order: settings (S or X) → fiscal year → payment method/voucher rows (10's own) → counters → documents →
  `change_versions`.

## 5. Undo — `accounting.closeYear` (phase-e E-5)
`domains/accounting/undo_period.rs` (12b-owned): `CloseYearCompensator`, `action_type =
"accounting.closeYear"`, `area = Accounting`; `pub fn register(r: &mut UndoRegistry)` called from 12's
`register_undo`. `compensate`: parse `fiscalYearId` from the payload; call `reopen_year(tx, cx,
&UndoRegistry::new(), id)` and return its audit id. D7 holds by construction: the year is closed, and
`reopen_year` refuses non-admins with `إعادة فتح السنة المالية للمدير فقط`. `req.date` is ignored (reopen has
no date parameter, D-A2); `req.reason` is stored by `shared::activity::undo` on the link, not by reopen.

## 6. Frontend switch lines (`src/modules/accounting/services/accountingService.ts`)
`getFiscalYears` → `if (usesRust('accounting')) return backendCall('accounting_get_fiscal_years');` ·
`getCurrentFiscalYear` → `if (usesRust('accounting')) return (await backendCall('accounting_get_current_fiscal_year')) ?? undefined;` ·
`saveFiscalYear` → `…('accounting_save_fiscal_year', { input, id })` · `getLockDate` →
`(await backendCall('accounting_get_lock_date')) ?? undefined` · `saveLockDate` →
`{ await backendCall('accounting_save_lock_date', { lockDate }); return; }` · `getCloseYearPreChecks` →
`…({ fiscalYearId })` · `closeYear` → `…({ fiscalYearId })` · `reopenYear` → `…({ fiscalYearId })` (the TS
admin check below stays for the mock path) · `getVatPeriodTotals` → `…({ from, to })` · `submitVatSettlement`
→ `…({ from, to })` · `payVatSettlementNow` → `…({ amount, paymentMethodId })`. 11 lines.

## 7. Known mock quirks (kept), decisions, open questions

**Quirks kept (entry §3.3):** Q1 close checks/closing entry use the UTC `slice(0,10)` day while posting uses
the business day. Q2 the opening-equity pre-check sums all time, not the year. Q3 a year with no revenue/expense
movement cannot be closed (the closing entry has < 2 lines). Q4 reopen's mirror is dated "now" (D-A2). Q5 no
guard against settling overlapping VAT periods twice (D-A6). Q6 `payVatSettlementNow` has no cap at the
`vatPayable` balance and no admin period override (D-A7). Q7 a year can be created already flagged closed via
the form switch (no closing entry). Q8 the payment description names the Saudi authority in every country.

**Decisions (logged):** P-D1 editing a closed year is refused (`FORBIDDEN`) — the page never offers it
(`FiscalYearsPage.vue:188`), and allowing it would let any accountant unlock a closed year by sending
`isClosed: false`, bypassing the admin-only reopen and leaving the closing entry un-reversed. P-D2 unparsable
date strings are refused (`تاريخ القفل غير صالح` / `التاريخ غير صالح`); the pickers only send valid keys. P-D3
`closingEntryId`/`closedAt`/`closedBy` in `saveFiscalYear`'s input are ignored (server-owned).

**Open questions for the user** (from analysis §9; Part 03 keeps mock behaviour until answered):
1. D-A6 — guard against double VAT settlement? Needs a settled-periods record (a new table), so it is a
   scope change, not a Part 03 detail. Recommendation: yes, `CONFLICT` on overlap, in a later small plan.
2. D-A2 — date reopen's mirror at the year's end date instead of "now"?
3. D-A3 — a purpose-built VAT-settlement reversal? (Recommendation from analysis: no.)
4. D-A7 — cap VAT payment at the payable balance? (Recommendation from analysis: no.)

**Needs from manager:** none beyond 12 §7 (this file uses `entry_dto`, `RouteRef`, `SystemRole` from there),
plus 10-vouchers must expose its payment-voucher helper as a `pub` service function taking `(conn, cx, undo, input)`.

## 8. Tests

**(a) `src-tauri/tests/domain_accounting_period.rs`** (TestDb + fixture with revenue/expense/retained-earnings/
opening-equity/VAT role accounts, a cash payment method, two fiscal years; `run_all` green after every post):
- Fiscal years: order; current-year fallback to latest; save messages in order; overlap `CONFLICT` names the
  year; closed-year edit `FORBIDDEN`; server-owned fields ignored; activity `إضافة السنة المالية …`.
- Lock date: set/clear/invalid; a post on or before it is refused, after it allowed; other `accounting` keys kept.
- Pre-checks: each of the three failing alone, with the exact `detail` strings (`مدين 1500.5 — دائن 1500`,
  `الرصيد الحالي: -3`); an instant at 22:30 UTC on the year's last day counts in the year by the UTC rule.
- Close: closing entry lines per account + retained earnings; year flagged with closer and time; activity
  undoable `accounting.closeYear`; next year created (`2026` → `2027`, Feb-29 start → Feb-28 end, name without
  digits → start year + 1); existing next year returned, not duplicated; drafts present → `FORBIDDEN`
  `تعذر إقفال السنة: …`; already closed → `VALIDATION`; no P&L movement → the < 2 lines message.
- **Concurrency:** a `create_journal_entry` dated in the year on a second connection blocks during `close_year`
  and is refused with the closed-year message after commit; two concurrent closes → one success.
- Reopen: non-admin `FORBIDDEN` first; not closed `VALIDATION`; mirror CLOSING entry with the reason on the mirror
  only, original `reversed`; fields cleared; reopening a flag-only closed year (no closing entry) just unflags.
- Undo `accounting.closeYear` via `shared::activity::undo::undo`: admin → year reopened and rows linked;
  non-admin → `FORBIDDEN`, nothing changed.
- VAT: totals over the business-day range; settlement payable and refundable shapes; zero movement refused;
  admin may settle into a closed period, accountant may not; pay → voucher entry `Dr vatPayable / Cr cash`,
  `VCH-` number, amount 0 refused.

**(b) Parity cases (Part 04):** `accounting/fiscal-year-crud`, `accounting/lock-date`,
`accounting/close-year-prechecks`, `accounting/close-year`, `accounting/close-year-next-year-autocreate`,
`accounting/reopen-year`, `accounting/vat-settlement-payable`, `accounting/vat-settlement-refundable`,
`accounting/vat-pay`.

## 9. Checklist
- [x] Confirm 12 is implemented (its `service::rows`, `mod.rs`, `contract.check.ts` exist) and 10-vouchers exposes the payment-voucher helper. Implemented together in this same wave; the vouchers domain's `create_payment_voucher` (`domains/vouchers/service/general.rs`, `pub`) is the payment-voucher helper — used as-is, no second copy.
- [x] Move `CloseYearPreCheck` and `VatPeriodTotals` into `accounting/types/index.ts` (+ mock re-exports).
- [x] `dto/period.rs` (§2); `service/period.rs` (3.1–3.3); `service/vat.rs` (3.4).
- [x] `commands/period.rs`: 11 commands + `pub fn ipc_signatures()` (11 `ipc_sig!` lines, appended by 12's `mod.rs`).
- [x] `undo_period.rs`: `CloseYearCompensator` + `register`.
- [x] `tests/domain_accounting_period.rs` per §8(a) — written; not yet run (⏳ deferred time-boxed test pass).
- [x] 11 switch lines (§6); 5 `contract.check.ts` entries (§2) — in the same shared `accounting/types/contract.check.ts` 12-accounting creates.
- [x] Report to the manager: 11 command names; list the four open questions (§7) for the user — see this wave's final report.
- [x] Status note at the top of this file.

## Gate
- [ ] `cargo check --workspace --all-targets` clean (manager's throttled run after W5) — not run by this agent (hard rule: no cargo).
- [x] `tests/domain_accounting_period.rs` written; DB tests + 9 parity cases run in the deferred pass — writing done, running deferred.
- [x] 11 switch lines present; `contract.check.ts` compiles against the generated bindings — lines present; compiling against `types/gen/*` needs `bun run bindings` (manager, after `cargo check`).
- [ ] `bun run memory:check`: 11 commands registered and invoked, 0 contract gaps — deferred to the manager's post-wave run.
