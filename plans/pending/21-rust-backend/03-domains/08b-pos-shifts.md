# 21 · 03.08b — `invoices` part 2 (POS shifts, cash in/out, X/Z report, held sales)

> **Status:** code complete, not yet compiled/tested (2026-09-28, W3). `service/shifts.rs` (open,
> close incl. variance + optional cash-drop transfer voucher via
> `vouchers::service::general::record_transfer_voucher`, force-close, cash in/out, reads,
> `record_shift_movement`/`shift_summary`/`open_shift_for_terminal`/`lock_open_shift_of_terminal`
> shared helpers 08's sale/refund call) and `service/held.rs` (hold/resume/discard) written; all 12
> commands wired in `domains/invoices/commands.rs`/`mod.rs`. Switch lines (§6) and
> `contract.check.ts` entries (§2) are in, in the same files 08 added them to. Shift/held-sale
> tests are in `tests/domain_invoices.rs` (§8a) — ⏳ deferred time-boxed run; parity list (§8b)
> handed to Part 04. Needs from manager: same hooks as 08 (one domain, 25 commands total).
>
> Same implementer as [`08-invoices.md`](08-invoices.md) (one domain, `domains/invoices/**`; this
> file is the size-rule split). Depends on: 03-users (session), 10-vouchers
> (`record_transfer_voucher`, 10 §3.3 — see 08 PG-8),
> Part 02 `shared::{ledger,numbering,activity}`, `core::lock`, the Part 02 gaps PG-3/PG-7/PG-8 (08 §7), and
> the `uq_shifts_open_key` generated-column unique from m0008 (C-15).

**Goal.** Port the shift and held-sale half of `invoiceService.ts` and all of `src/mocks/backend/shifts.ts`:
one open shift per terminal (the `open_key` unique), an append-only movement log feeding the X/Z report and
expected cash, a close that posts the cash over/short variance (docs/v2/02 §3 "Shift close", invariant 11)
and an optional cash-drop transfer voucher, and per-terminal parked carts. The terminal is **always** this
process's `cx.terminal_id` (cross-cutting §2), never a client string.

**Read first.** [`../01-frontend-analysis/invoices.md`](../01-frontend-analysis/invoices.md) §1 (shift and
held-sale rows), §4 ("Why `closePosShift` cannot simply be reopened"), §5 rows 4–7, §7 (variance and
cash-drop rows) · [`../01-frontend-analysis/cross-cutting.md`](../01-frontend-analysis/cross-cutting.md) §2 ·
mock `src/mocks/backend/shifts.ts:20-166`, `src/modules/invoices/services/invoiceService.ts:249-355` · types
`src/modules/invoices/types/index.ts:155-240` · pages `PosPage.vue:104-166`, `ShiftsManagerPage.vue:20-41`,
`CashierHome.vue:27`, `controllers/usePosStore.ts:50` (`terminalId = 'pos-1'`) · Rust
`entities/sales/{shifts,shift_movements,held_sales}.rs`, `migration/src/m0008_sales.rs:299-380`.

## 1. Commands

Same read/write conventions as 08 §1 (`with_read` + `require_any`, `with_tx` for writes and clock reads).
Areas from the callers: `PosPage` (route `pos`, area `pos`), `ShiftsManagerPage`/`ShiftReportPage` (area
`pos`), `CashierHome` (dashboard), `pdfService` (Z-report PDF).

| # | Mock fn (`invoiceService.ts`) | Disp. | Rust command | Args → Return | Area / Access | Tx | Events |
|---|---|---|---|---|---|---|---|
| 1 | `getCurrentShift` (:265) | port | `invoices_get_current_shift` | `()` → `Option<ShiftRow>` | Pos∨Dashboard / Read | `with_read` | — |
| 2 | `getShifts` (:271) | port | `invoices_get_shifts` | `{ filter?: ShiftFilter }` → `Vec<ShiftRow>` | Pos / Read | `with_read` | — |
| 3 | `getShift` (:279) | port | `invoices_get_shift` | `{ id }` → `ShiftRow` | Pos / Read | `with_read` | — |
| 4 | `openPosShift` (:286) | port | `invoices_open_pos_shift` | `{ input: OpenShiftInput }` → `Shift` | Pos / Write | `with_tx` | — |
| 5 | `getXReport` (:292) | port | `invoices_get_x_report` | `{ shiftId }` → `ShiftRow` | Pos / Read | `with_read` | — |
| 6 | `closePosShift` (:297) | port | `invoices_close_pos_shift` | `{ shiftId, input: CloseShiftInput }` → `Shift` | Pos / Write | `with_tx` | ledger |
| 7 | `forceClosePosShift` (:303) | port | `invoices_force_close_pos_shift` | `{ shiftId, countedCash? }` → `Shift` | Pos / Write | `with_tx` | ledger |
| 8 | `recordCashInOut` (:319) | port | `invoices_record_cash_in_out` | `{ kind: CashMovementKind, amount, note? }` → `()` | Pos / Write | `with_tx` | ledger (mock parity) |
| 9 | `getHeldSales` (:332) | port | `invoices_get_held_sales` | `()` → `Vec<HeldSale>` | Pos / Read | `with_read` | — |
| 10 | `holdSale` (:337) | port | `invoices_hold_sale` | `{ input: HeldSaleInput }` → `HeldSale` | Pos / Write | `with_tx` | — |
| 11 | `resumeHeldSale` (:344) | port | `invoices_resume_held_sale` | `{ id }` → `HeldSale` | Pos / Write | `with_tx` | — |
| 12 | `discardHeldSale` (:352) | port | `invoices_discard_held_sale` | `{ id }` → `()` | Pos / Write | `with_tx` | — |

`terminalId` parameters of #1, #8, #9 and the `terminalId` field of `OpenShiftInput`/`HeldSale` input are
**not sent/ignored** — the Rust side uses `cx.terminal_id` / the session's terminal (D-S1). `with_read`
commands read the terminal from `state.terminal.terminal_id` before the closure.

## 2. DTOs (`domains/invoices/dto.rs`, `#[ts(export_to = "invoices/types/gen/")]`, conventions as 08 §2)

| Rust DTO | TS type (file:line) | Field notes |
|---|---|---|
| `DenominationCount` | `types/index.ts:186-189` | `value` money, `count: i32`. Stored as the JSON columns `opening_denominations`/`closing_denominations`. |
| `ShiftMovement` | `:194-203` | `kind` SCREAMING_SNAKE; `amount` money (always positive); `at` = DocDate key; `ref_id`/`ref_number`/`note` optional. |
| `Shift` | `:205-226` | `terminal_id: String` (UUID text of the owning terminal); `opened_at`/`closed_at` = DocDate keys; `movements` by `position`; `handover_mode` UPPERCASE; money `serde_number`. |
| `ShiftSummary` + `ShiftRow` | `invoiceService.ts:253` (`Shift & ReturnType<typeof shiftSummary> & {…}`), `shifts.ts:69-91` | `ShiftRow` = `#[serde(flatten)]` `Shift` + flatten `ShiftSummary { cash_sales, cash_refunds, pay_ins, pay_outs, bank_drops, expected_cash, sales_by_method: Vec<{ label, amount }>, sales_total, invoice_count: i32 }` + `opened_by_name`, `closed_by_name?`. |
| `OpenShiftInput` | `:228-233` | request-only; `terminal_id: Option<String>` accepted and ignored. `// contract-ok: terminalId is server-owned (cross-cutting §2)` if `Equals` needs it optional. |
| `CloseShiftInput` | `:235-240` | request-only. |
| `HeldSale`, `HeldSaleLine` | `:160-182` | `terminal_id: String`; `held_at` = DocDate key; `lines` = the JSON `cart` column (`values::HeldSaleCart`), numbers as JSON numbers; `discount_rate` `serde_number`. |
| `HeldSaleInput` | `Omit<HeldSale, 'id' \| 'heldAt' \| 'heldBy'>` (`invoiceService.ts:337`) | request-only, `terminal_id` ignored. |
| `ShiftFilter`, `CashMovementKind` | `invoiceService.ts:271,319` | `CashMovementKind` = `PAY_IN \| PAY_OUT \| BANK_DROP` (SCREAMING_SNAKE). |

`contract.check.ts` (same file as 08): `Shift`, `ShiftMovement`, `DenominationCount`, `HeldSale`,
`OpenShiftInput`, `CloseShiftInput`, and `ShiftRow` (type-only import from `../services/invoiceService`).

## 3. Service logic (`service/shifts.rs`, `service/held.rs`)

### 3.1 Shared helpers (also called by 08 §3.4 step 1/12 and §3.5 step 8/15)

- **`open_shift_for_terminal(conn, terminal_id, lock: bool) -> Option<shifts::Model>`** — `WHERE terminal_id
  = ? AND status = 'OPEN'` (`FOR UPDATE` when `lock`) = `currentOpenShift` (`shifts.ts:20-22`).
- **`lock_open_shift_of_terminal(conn, shift_id) -> Option<shifts::Model>`** — reads that shift's
  `terminal_id` (no lock), then `open_shift_for_terminal(…, lock = true)` (the sale's movement target,
  `sales.ts:376`).
- **`record_shift_movement(conn, cx, shift, kind, amount, note, ref_id, ref_number, at: DocDate)`**
  (`shifts.ts:45-66`): `amount == 0` → no-op; else insert `shift_movements { position = COALESCE(MAX(position)
  + 1, 0) for that shift, kind, amount: round2(amount), note, ref_id, ref_number, at, by: actor }`. The caller
  must already hold the shift row lock (every caller does), which also serializes `position`.
- **`shift_summary(conn, shift) -> ShiftSummary`** (`shifts.ts:69-91`): `cash_sales`/`cash_refunds`/
  `pay_ins`/`pay_outs`/`bank_drops` = `round2(SUM(amount))` per kind (`sum()` rounds, `utils.ts:70-72`);
  `expected_cash = round2(opening_float + cash_sales − cash_refunds + pay_ins − pay_outs − bank_drops)`;
  invoices with `shift_id = shift.id` (any status) by `(created_at, id)`; `sales_by_method`: walk their
  tenders by `(invoice created_at, invoice id, tender position)` into an insertion-ordered list keyed by
  label = live payment-method name, else the method id text, `amount = round2(acc + t.amount)`;
  `sales_total = round2(Σ grand_total)`; `invoice_count`. Lists (`getShifts`) compute summaries with three
  grouped queries (movements by shift×kind, tenders by shift, invoice totals by shift), not N+1.
- **`to_shift_row`** (`invoiceService.ts:255-262`): shift DTO + summary + `opened_by_name` (live user name
  else `—`) + `closed_by_name` (only when `closed_by` is set; absent if the user isn't found).

### 3.2 Reads

- `get_current_shift` (:265-269): `open_shift_for_terminal(terminal, false)` → row or `None`.
- `get_shifts` (:271-277): optional `status` filter; order `opened_at_instant DESC, opened_at_day DESC,
  created_at, id` (`shifts` has no generated `_key` column; `openedAt` is always an ISO instant, `shifts.ts:24`).
- `get_shift` / `get_x_report` (:279-295): one function; `NOT_FOUND` `الوردية غير موجودة`. `get_x_report`
  calls it (analysis §1: "don't duplicate the summary computation").

### 3.3 `open_pos_shift` (`shifts.ts:24-42`)

1. `open_shift_for_terminal(cx.terminal_id, lock = true)` exists → `CONFLICT`
   `توجد وردية مفتوحة بالفعل على هذا الجهاز`.
2. `opening_float < 0` → `VALIDATION` `رصيد الافتتاح لا يمكن أن يكون سالباً`.
3. `number = next_number(DocumentKind::Shift)` (`SH-…`).
4. Insert `{ terminal_id: cx.terminal_id, branch_id: input.branch_id, status OPEN, opened_by: actor,
   opened_at: now, opening_float: round2, opening_denominations }`. A `TxError::Db` whose
   `duplicate_key_name` (PG-3) is `uq_shifts_open_key` → the same `CONFLICT` message (the race loser, analysis §5).
5. `log(Shift, "فتح وردية {number} — رصيد افتتاحي {float:.2}", now, RouteRef::list("pos-shifts"))`.
6. Return the `Shift` (no movements). No events (the mock emits none).

### 3.4 `close_shift(conn, cx, reg, shift_id, input, forced_by: Option<Id>)` (`shifts.ts:93-156`)

1. Lock the shift `FOR UPDATE`; missing → `NOT_FOUND` `الوردية غير موجودة`; `status != OPEN` →
   `VALIDATION` `الوردية مغلقة بالفعل`.
2. `expected = shift_summary(shift).expected_cash` — read under the lock, so a concurrent sale/refund/pay-in
   either appended before (included) or waits and then finds the shift closed (analysis §5 row 5).
3. `counted = round2(input.counted_cash)`; `variance = round2(counted − expected)`; `posts_variance =
   |variance| > 0.005`; `drop = input.handover_mode == DROP && counted > 0`.
4. If `posts_variance || drop`: `assert_open_period(today, false)`; if `drop`: **pre-allocate** `voucher_no
   = next_number(Voucher)` now, so counters are always taken voucher → journal (a plain transfer voucher
   takes voucher → journal too; allocating after the variance entry's journal number would invert that and
   invite a deadlock). Numbers equal the mock's (separate counters; journal order unchanged).
5. `posts_variance` → `ledger::post` (date now, `تسوية عجز/زيادة الصندوق — وردية {number}`, SYSTEM, source
   `{ "shift", id, number }`): over → `Role(Cash)` Dr `variance` / `Role(CashOver)` Cr `variance`; short →
   `Role(CashShort)` Dr `−variance` / `Role(Cash)` Cr `−variance` (no dims, like the mock).
6. `drop` → `domains::vouchers::service::record_transfer_voucher(conn, cx, reg, TransferVoucherInput {
   date: now, amount: counted, description: "إيداع نقدية الوردية {number} إلى الخزينة/البنك",
   source_account_id: resolve_account(Cash, default).id, destination_account_id: resolve_account(Bank,
   default).id, fee_amount: None, fee_account_id: None, note: None, attachment_ids: None, cost_center_id:
   None }, Some(voucher_no))` — the voucher writes its own journal entry and `voucher` activity row (10 §3.3).
7. Update the shift: `status CLOSED`, `closed_by: actor`, `closed_at: now`, `counted_cash`,
   `closing_denominations`, `expected_cash`, `variance`, `handover_mode = input ?? HANDOVER`, `note`,
   `force_closed_by = forced_by` (folded into the same update, analysis §1).
8. `log(Shift, "إغلاق وردية {number} — المتوقع {expected:.2}، المعدود {counted:.2}، الفرق {variance:.2}",
   now, RouteRef::list("pos-shifts"))`.
9. `cx.touch(Ledger)` unconditionally (`shifts.ts:154`). Return the `Shift`.

- **`close_pos_shift`** = `close_shift(shift_id, input, None)`.
- **`force_close_pos_shift`** (`shifts.ts:159-166`): lock the shift; missing → `NOT_FOUND`
  `الوردية غير موجودة`; `expected` from the summary; `close_shift(id, { counted_cash: counted_cash ??
  expected, handover_mode: HANDOVER, note: "إغلاق إجباري من المدير" }, Some(actor))` (so a force-close can
  never drop, analysis §1). The status check happens inside `close_shift`, as in the mock.

### 3.5 `record_cash_in_out` (`invoiceService.ts:319-326`)

1. `open_shift_for_terminal(cx.terminal_id, lock = true)` none → `CONFLICT` `لا توجد وردية مفتوحة`.
2. `!(amount > 0)` → `VALIDATION` `المبلغ يجب أن يكون أكبر من صفر`.
3. `record_shift_movement(kind, amount, note, None, None, now)`.
4. `cx.touch(Ledger)` (the mock emits `ledger:changed` though nothing posts — Q-S2). No audit row (Q-S1).

### 3.6 Held sales (`invoiceService.ts:332-355`)

- `get_held_sales`: `WHERE terminal_id = this terminal ORDER BY held_at_instant DESC, held_at_day DESC,
  created_at, id` (`heldAt` is always an instant, `invoiceService.ts:339`).
- `hold_sale`: insert `{ label, terminal_id: cx.terminal_id, held_at: now, held_by: actor, customer_id,
  discount_rate, discount_is_pct, note, cart: lines }` — no validation (analysis §2 "unvalidated draft
  cart"), no audit, no events. Return the DTO.
- `resume_held_sale`: `SELECT … FOR UPDATE` by id; none → `NOT_FOUND` `لا يوجد بيع معلّق بهذا المعرف`;
  delete it (held sales are hard-deleted, P2-16); return the DTO read before the delete.
- `discard_held_sale`: `DELETE WHERE id = ?`; zero rows is not an error (idempotent, analysis §1).

## 4. Concurrency (D8, analysis §5)

| Race | Settled by |
|---|---|
| Two opens on one terminal | pre-check under `FOR UPDATE` + `uq_shifts_open_key` (generated `open_key = terminal_id` while OPEN); the loser's 1062 maps to the mock's `CONFLICT` (PG-3). |
| Two closes of one shift | shift row `FOR UPDATE`; the second sees `CLOSED` → `الوردية مغلقة بالفعل`. |
| Sale / refund / pay-in appending while the shift closes | every appender locks the shift row **first** (document step of `core/lock.rs`) — 08 §3.4 step 1, §3.5 step 8, §3.5 above — so the summary read at close is atomic with the status flip. |
| Close (journal counter) vs. a transfer voucher (voucher counter) | voucher number pre-allocated before the variance posting (§3.4 step 4). |
| Two resumes of one held sale | row lock; the second gets `NOT_FOUND`. |
| Shift numbers | `next_number(Shift)` + `uq_shifts_number`. |

## 5. Undo

None registered. A shift is never reopened (analysis §4: the variance and drop postings are separate
documents and a new shift may already be open); a wrong count is corrected by a manual journal entry; a
wrong pay-in/out by an opposite movement; held-sale actions are their own inverse.

## 6. Frontend switch lines (`src/modules/invoices/services/invoiceService.ts`)

Same import as 08 §6. Lines (first statement inside each `wrap`):
- `getCurrentShift`: `if (usesRust('invoices')) return (await backendCall('invoices_get_current_shift')) ?? undefined;`
- `getShifts` `{ filter }`, `getShift` `{ id }`, `openPosShift` `{ input }`, `getXReport` `{ shiftId }`,
  `closePosShift` `{ shiftId, input }`, `forceClosePosShift` `{ shiftId, countedCash }`, `holdSale`
  `{ input }`, `resumeHeldSale` `{ id }`: `if (usesRust('invoices')) return backendCall('<cmd>', { … });`
- `getHeldSales`: `if (usesRust('invoices')) return backendCall('invoices_get_held_sales');`
- `recordCashInOut`: `if (usesRust('invoices')) { await backendCall('invoices_record_cash_in_out', { kind, amount, note }); return; }`
- `discardHeldSale`: `if (usesRust('invoices')) { await backendCall('invoices_discard_held_sale', { id }); return; }`

(`Option` comes back as `null` and a unit return as `null`; the two forms above keep the TS signatures
`ShiftRow | undefined` and `void`.)

## 7. Known mock quirks (kept) and decisions

**Quirks kept:**
- Q-S1 No audit/activity row for `recordCashInOut` (real drawer cash) or held-sale CRUD (analysis §6/§9 —
  the analysis recommends one for `recordCashInOut`; later fix, mock and Rust together).
- Q-S2 `recordCashInOut` and every close bump `ledger` even when nothing posted (harmless refresh).
- Q-S3 Any user with Pos:Write (cashiers too) can force-close another till's shift; no role guard in the mock.
- Q-S4 A `PAY_OUT` doesn't create an expense voucher (`invoiceService.ts:308-317`, documented follow-up).
- Q-S5 A sale whose `shiftId` points at a closed shift records its cash on that terminal's **current**
  open shift, if any (`sales.ts:376`).

**Decisions:**
- D-S1 Terminal identity is server-owned: shift and held-sale commands use `cx.terminal_id`
  (`terminal.json`, cross-cutting §2); the client's `'pos-1'` (`usePosStore.ts:50`) is ignored. The D10
  importer maps each legacy terminal string to one new `Id` (Part 02 handoff §9); parity maps ids.
- D-S2 The cash-drop voucher number is pre-allocated before the variance entry (deadlock-free counter order,
  identical numbers).
- D-S3 `record_shift_movement` requires the caller to hold the shift lock; that lock also makes `position`
  (`uq_shift_movements_shift_id_position`) race-free.
- D-S4 `resumeHeldSale` isn't restricted to this terminal's carts (the mock resumes any id); only the list is
  filtered, as today.

## 8. Tests

**(a) `src-tauri/tests/domain_invoices.rs`** (shift section; posting tests end with `run_all`, invariant 11):
- open → `SH-000001`, opening float `round2`; second open on the same terminal → `CONFLICT` message; two
  connections opening at once → exactly one row, the loser gets the same message (unique path).
- negative float → message; `get_current_shift` returns the row for this terminal only.
- summary: sale cash tender, cash refund, pay-in, pay-out, bank drop → each sum and `expected_cash`;
  `sales_by_method` labels in first-appearance order; `sales_total`/`invoice_count`.
- close over (+5) → Dr cash / Cr cashOver 5; short (−3) → Dr cashShort / Cr cash 3; exact → no entry, still
  `ledger` bumped; invariant 11 holds.
- close with `DROP` and counted 500 → a `TRANSFER` voucher Dr bank / Cr cash 500 with its own number and
  activity row; variance entry number precedes the voucher's journal number.
- close twice → `الوردية مغلقة بالفعل`; unknown id → `NOT_FOUND`; force-close defaults counted to expected
  (0 variance), stamps `force_closed_by`, never drops.
- concurrency: a sale appending a cash movement while another connection closes the shift → either included
  in `expected_cash` or recorded nowhere, never split; no deadlock with a concurrent transfer voucher.
- `record_cash_in_out`: no open shift → `CONFLICT`; amount 0 → message; movement appended.
- held sales: hold → list (newest first, this terminal only) → resume (row gone) → resume again →
  `NOT_FOUND`; discard of a missing id → `Ok`.

**(b) Parity cases for Part 04:** `shift-open-close-exact`, `shift-close-over`, `shift-close-short`,
`shift-close-drop`, `shift-force-close`, `shift-cash-in-out`, `shift-xreport-with-sales-refunds`,
`held-sale-hold-resume-discard`.

## 9. Checklist (implementation order)

- [x] PG-3 and the 10-vouchers `record_transfer_voucher` signature are available (confirmed: both
      `fixed`/wired in `_part02-gaps.md` and `domains::vouchers::service::general`).
- [x] `service/shifts.rs`: §3.1 helpers first (08's sale/refund call them), then reads, open, close,
      force-close, cash in/out.
- [x] `service/held.rs`: §3.6.
- [x] DTOs (§2) in the shared `dto.rs`; 12 commands in `commands.rs` with their `ipc_sig!` lines.
- [x] Switch lines (§6); `contract.check.ts` entries (§2).
- [x] Shift/held-sale tests in `tests/domain_invoices.rs` (§8a, highest-value scenarios); parity list
      to Part 04.
- [x] Status note at the top of this file.

## Gate

`cargo check` clean (manager's throttled run) · tests written · 12 switch lines · `contract.check.ts`
compiles · `memory:check` 0 contract gaps for these commands. DB tests and parity cases run in the deferred,
time-boxed test pass.
