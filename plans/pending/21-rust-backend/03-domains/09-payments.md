# 21 · 03.09 — `payments` (customer receipts / supplier payments, sub-ledger allocations, realized FX)

> **Status:** code complete 2026-09-28 (Wave 3 implementer), pending the manager's throttled
> `cargo check`/registration pass. All 7 commands, DTOs, service logic (`common`/`read`/`create`/
> `allocate`) and the 7 frontend switch lines + `contract.check.ts` are written. **Q-P1 decided**
> (architectural-autonomy, CLAUDE.md): `remove_allocation` now **refuses** removing an allocation
> that carries a non-zero `fxGainLoss` with a `VALIDATION` message
> (`لا يمكن إلغاء تخصيص حقق فرق عملة — قم بعكسه عبر قيد تسوية يدوي بدلاً من ذلك`) rather than silently
> leaving the FX-adjustment entry unreversed — the conservative, recommended option per §7's Q-P1,
> same stance as the shift close / card settlements. Tests written in
> `src-tauri/tests/domain_payments.rs` (§8a coverage) but **not run** (per-implementer hard rule: no
> cargo from this agent) — ⏳ deferred to the time-boxed test pass. `domains/mod.rs` still needs
> `pub mod payments;`, the `export_bindings`/`all_ipc_signatures` hook lines, and `generate_handler!`
> registration — manager's job (see "Needs from manager" in the final report).
>
> Wave **W4** in entry file §4 — **recommended W3**
> (08 PG-8: `invoices_get_invoice` needs this domain's `Payment` DTO and `payments_for_invoice`; this domain
> has no code dependency on 07/08, only on entities and PG-1/PG-6). Depends on: 05-parties (party rows,
> rule R-1), Part 02 `shared::{ledger,numbering,balances,activity}`, and the Part 02 gaps PG-1, PG-3,
> PG-5, PG-6, PG-7 (08 §7).

**Goal.** Port `paymentService.ts` and `src/mocks/backend/payments.ts`: a payment posts **once**, for its
full amount, against the receivable/payable control account (plus a realized FX line); `allocations[]` are a
sub-ledger link to open invoices/POs that never touch the GL again, except the small FX-delta entry an
"allocate later" can post (analysis header). Allocation targets and the payment's remaining cash are checked
under row locks (analysis §5). Unallocated credit comes from `shared::balances` (P2-26, one copy).

**Read first.** [`../01-frontend-analysis/payments.md`](../01-frontend-analysis/payments.md) §1–§9 · mock
`src/mocks/backend/payments.ts:29-388`, `src/modules/payments/services/paymentService.ts:133-220` · types
`src/modules/payments/types/index.ts:1-125` · pages `PaymentFormPage.vue`, `PaymentDetailPage.vue`,
`PaymentListPage.vue`, `parties/pages/PartyDetailPage.vue` · Rust `shared/balances.rs:180-302`,
`shared/ledger/{post,accounts}.rs`, `entities/payments/{payments,payment_allocations}.rs`,
`entities/sales/invoices.rs`, `entities/purchases/purchase_orders.rs`, migration `m0010_payments.rs` ·
`docs/v2/02-accounting-review.md` §3 (customer receipt / supplier payment rows), §4 invariants 3 and 6.

## 1. Commands

Reads: `with_read`, actor cloned from `state.session`, `require_any(…, Read)` (08 §1 helper pattern — a
local copy in `domains/payments/service/access.rs`, 3 lines). Areas from the callers: payments pages (area
`payments`), `PartyDetailPage` (area `parties`: `getPayments`, `getOpenDocuments`).

| # | Mock fn (`paymentService.ts`) | Disp. | Rust command | Args → Return | Area / Access | Tx | Events |
|---|---|---|---|---|---|---|---|
| 1 | `getPayments` (:144) | port | `payments_get_payments` | `{ filter?: PaymentFilter }` → `Vec<PaymentRow>` | Payments∨Parties / Read | `with_read` | — |
| 2 | `getPaymentsPaged` (:160) | port | `payments_get_payments_paged` | `{ query: PagedQuery<PaymentFilter> }` → `PagedResult<PaymentRow>` | Payments / Read | `with_read` | — |
| 3 | `getPayment` (:193) | port | `payments_get_payment` | `{ id }` → `PaymentRow` | Payments / Read | `with_read` | — |
| 4 | `createPayment` (:200) | port | `payments_create_payment` | `{ input: PaymentInput }` → `Payment` | Payments / Write | `with_tx` | ledger, parties |
| 5 | `allocateExistingPayment` (:206) | port | `payments_allocate_existing_payment` | `{ paymentId, allocations: Vec<PaymentAllocationInput> }` → `Payment` | Payments / Write | `with_tx` | parties (+ ledger when FX) |
| 6 | `removeAllocation` (:211) | port | `payments_remove_allocation` | `{ paymentId, allocationId }` → `Payment` | Payments / Write | `with_tx` | parties |
| 7 | `getOpenDocuments` (:217) | port | `payments_get_open_documents` | `{ targetType: PartyKind, targetId }` → `Vec<OpenDocument>` | Payments∨Parties / Read | `with_read` | — |

Public (non-command) read function for 08: `service::read::payments_for_invoice(conn, invoice_id) ->
TxResult<Vec<Payment>>` (§3.6).

## 2. DTOs (`domains/payments/dto.rs`, `#[ts(export_to = "payments/types/gen/")]`, conventions as 08 §2)

| Rust DTO | TS type (file:line) | Field notes |
|---|---|---|
| `PaymentAllocation` | `types/index.ts:9-26` | `target_kind` camelCase (`invoice`/`purchaseOrder`/`opening` — 3 variants stored); `date` = DocDate key (the payment's date); `amount` money; `amount_fc: Option` (`DECIMAL(19,4)`), `fx_gain_loss: Option` present only when non-zero (`payments.ts:182`). |
| `Payment` | `:28-63` | `type` UPPERCASE; `target_type` = `customer`/`supplier` (the payments entity enum); `method` = `PaymentTenderKind` snake_case (`cash`/`card`/`bank_transfer` — named to avoid the settings `PaymentMethod` clash, analysis §2); `target_ref`/`target_ref_number` optional; `allocations` by `position`; `currency`/`amount_fc`/`rate` only when FX; `fx_gain_loss` only when non-zero. |
| `PaymentRow` | `paymentService.ts:133` | `#[serde(flatten)]` `Payment` + `party_name`, `allocated`, `unallocated`, `allocation_status` (`full`/`partial`/`unallocated`, lowercase). |
| `PaymentAllocationInput` | `:65-69` | request-only; `target_kind` **2 variants** (`invoice`/`purchaseOrder` — an `opening` input fails deserialization, analysis §2). |
| `PaymentInput` | `:71-86` | request-only; `date: RawDocDate`; `amount`/`amount_fc`/`rate` numbers. |
| `PaymentFilter` | `:88-97` | `from`/`to` plain date strings (not routes — the generator false positive, analysis §2). |
| `OpenDocument` | `:100-116` | `kind` camelCase; `date` = DocDate key; `due_date` (invoices only); `total`/`outstanding` base money; `currency`, `fc_outstanding` (only when FX), `rate` (the document's `exchange_rate`, when set). |
| `AllocationStatus` + `allocation_status_for` | `:118-125` | pure fn port: `≤ 0.005 → unallocated`, `≥ amount − 0.005 → full`, else `partial`. |

`contract.check.ts` (new `src/modules/payments/types/contract.check.ts`): `Payment`, `PaymentAllocation`,
`PaymentAllocationInput`, `PaymentInput`, `PaymentFilter`, `OpenDocument`, `AllocationStatus`, and
`PaymentRow` (type-only import from `../services/paymentService`).

## 3. Service logic (`service/{access,read,create,allocate,common}.rs`)

### 3.1 Common pieces

- **`payment_dto(conn, model)`** — allocations by `position`; `payment_dtos(conn, ids)` batch variant.
- **`party_name(target_type, target_id)`** — live party name of that kind, else `—` (`paymentService.ts:135-137`).
- **`to_row`** (:139-142): `allocated = round2(Σ allocation.amount)` (`allocatedTotal`, `payments.ts:81-83`);
  `unallocated = shared::balances::unallocated_amount(payment)` (FX-aware cash consumed, `:94-97`);
  `allocation_status = allocation_status_for(amount, allocated)`.
- **`open_documents(target_type, target_id)`** (`payments.ts:29-74`) = the full `shared::balances`
  open-document list (PG-6): customer → invoices `COMPLETED` with `invoice_outstanding > 0`; supplier → POs
  `RECEIVED` with `purchase_outstanding > 0`; order `date_key, created_at, id`. Per doc: `fc_outstanding =
  outstanding in the doc's own currency`; `outstanding = currency && rate ? round2(fc × rate) : fc`;
  `total = currency && rate ? round2((grand − refunded|returned) × rate) : round2(grand − refunded|returned)`.
- **`apply_allocation(conn, alloc, sign)`** (`payments.ts:108-128`): `settled = amount_fc ?? amount`;
  invoice → missing `NOT_FOUND` `الفاتورة غير موجودة`, else `paid_amount = round2(paid + sign × settled)`,
  `payment_status = payment_status_for(grand − refunded, paid)` (PG-1); purchase order → missing
  `NOT_FOUND` `أمر الشراء غير موجود`, same with `returned_amount`; `opening` → nothing. The target row is
  already locked by the caller.
- **`validate_allocations(conn, payment, inputs, already)`** (`payments.ts:139-192`) — exact order:
  `open_docs = open_documents(payment.target)`; `remaining = round2(payment.amount − already)` (`already` =
  Σ existing allocation `amount`, the AR-side total — quirk Q-P2); for each input, in order: skip
  `!(amount > 0)`; doc = the open doc with that id **and** kind, else
  `المستند غير موجود ضمن المستندات المفتوحة لهذا الطرف`; `cash = round2(amount)`; `cash > remaining +
  0.005` → `إجمالي التخصيص أكبر من مبلغ السند`.
  - **FC document** (`currency && rate && fc_outstanding`): `fc_settled = payment.rate ? round2(cash /
    payment.rate) : (round2(cash − doc.outstanding) != 0 → ``التخصيص الجزئي بالعملة الأساسية على مستند
    بعملة {currency} غير مدعوم — خصص المبلغ كاملاً أو استخدم دفعة بنفس العملة`` else doc.fc_outstanding)`;
    `fc_settled > fc_outstanding + 0.005` → ``الكمية المخصصة لـ {number} أكبر من المتبقي عليه
    ({fc_outstanding:.2} {currency})``; `ar = round2(fc_settled × doc.rate)`; `ar > outstanding + 0.01` →
    ``المبلغ المخصص لـ {number} أكبر من المتبقي عليه ({outstanding:.2})``; `fx = round2(cash − ar)`;
    `remaining = round2(remaining − cash)`; row `{ amount: ar, amount_fc: fc_settled, fx_gain_loss: fx if
    ≠ 0, date: payment.date, target_number: doc.number }`.
  - **Base document**: `cash > outstanding + 0.005` → ``المبلغ المخصص لـ {number} أكبر من المتبقي عليه
    ({outstanding:.2})``; `remaining -= cash`; row `{ amount: cash }`.
  All messages are `VALIDATION`.
- **`highlight_link(payment_id)`** = `RouteRef { name: "payments", query: { highlight: id } }` (PG-5).

### 3.2 `create_payment(conn, cx, reg, input)` (`payments.ts:195-307`)

1. `amount = round2(input.amount)`; `!(amount > 0)` → `المبلغ يجب أن يكون أكبر من صفر`.
2. `branch = input.branch_id ?? settings.default_branch_id`; `currency = input.currency` unless base.
3. Party **by `input.type`** (the mock ignores `input.targetType`, :203-211): `RECEIVED` → live party kind
   customer (active or not) else `اختر العميل`; `PAID` → kind supplier else `اختر المورد`.
4. **Locks** (`core/lock.rs` order): target documents of inputs with `amount > 0` — invoices / purchase
   orders `FOR UPDATE` sorted by id (analysis §5 row 1) → the party `share_lock_by_id` (R-1).
5. `allocations = validate_allocations(payment{amount, rate: currency ? input.rate : None, target}, inputs,
   already = 0)` (after the locks, so `outstanding` is current).
6. `target_ref`/`target_ref_number` = the single allocation's, when exactly one (:232-235).
7. Posting math (:241-253): `allocated_control = round2(Σ a.amount)`; `cash_consumed = round2(Σ (a.amount_fc
   && payment.rate ? round2(a.amount_fc × rate) : a.amount))`; `unallocated = round2(amount −
   cash_consumed)`; `control = round2(allocated_control + unallocated)`; `fx = round2(Σ a.fx_gain_loss)`
   (stored only when ≠ 0); `control_fc = round2(Σ a.amount_fc)`, `None` when 0.
8. `fx_line`: `fx > 0` → `Role(FxGain)` credit `fx`; `fx < 0` → `Role(FxLoss)` debit `−fx`; description
   `فرق عملة محقق`, `branch_id` (:256-261).
9. Settlement account = `settlement_account_for(method, AccountCtx { branch_id: Some(branch), currency })`
   → `AccountRef::Id` (:266). `control_fc_tag` = `control_fc && currency` → `{ currency, amount_fc:
   control_fc, rate: round2(allocated_control / control_fc) }` (rate rounded to 2 dp — Q-P3).
10. Lines (:268-279): RECEIVED → settlement Dr `amount` (+ `{currency, amount_fc: input.amount_fc, rate:
    input.rate}` when FX) · `Role(Receivable)` Cr `control`, party customer, tag · fx line. PAID →
    `Role(Payable)` Dr `control`, party supplier, tag · settlement Cr `amount` (+ FX tag) · fx line. All
    with `branch_id`.
11. `assert_open_period(date.day)`; `number = next_number(Payment)` (`PAY-…`).
12. Insert `payments { number, date, type, target_type (derived from type), target_id, target_ref*, amount,
    method, note, branch_id, currency, amount_fc/rate only when FX, fx_gain_loss }` + allocation rows
    (`position = i`, `date = payment date`); `apply_allocation(+1)` for each (:281-284).
13. `ledger::post` (date = `input.date`, description ``سند قبض {number} من {party}{ — targetRefNumber}`` or
    ``سند صرف {number} إلى {party}{ — targetRefNumber}``, SYSTEM, source `{ "payment", id, number }`) (:286-296).
14. `log(Payment, "{تحصيل|سداد} {amount:.2} {من|إلى} {party}", date, highlight_link)` (:298-304);
    `cx.touch(Parties)` (:305). Return `payment_dto`.

### 3.3 `allocate_existing_payment(conn, cx, reg, payment_id, inputs)` (`payments.ts:314-369`)

1. Lock the payment `FOR UPDATE` (analysis §5 row 2); missing → `NOT_FOUND` `السند غير موجود`.
2. Lock the input target documents (sorted), then `share_lock_by_id("parties", target)` (R-1).
3. `rows = validate_allocations(payment, inputs, already = Σ existing amounts)`; empty → `لم يتم إدخال أي تخصيص`.
4. Insert rows at `position = max + 1…`; `apply_allocation(+1)`; if the payment now has exactly one
   allocation → set `target_ref`/`target_ref_number` (:322-325).
5. `new_fx = round2(Σ rows.fx_gain_loss)`; if ≠ 0 (:332-357): `payment.fx_gain_loss = round2((fx ?? 0) +
   new_fx)`; `new_fc = round2(Σ rows.amount_fc)` or `None`; tag = `new_fc && payment.currency` →
   `{ currency, amount_fc: new_fc, rate: payment.rate }`; control line RECEIVED → `Role(Receivable)` debit
   `max(new_fx, 0)` / credit `max(−new_fx, 0)`, party customer; PAID → `Role(Payable)` credit
   `max(new_fx, 0)` / debit `max(−new_fx, 0)`, party supplier; both `branch_id = payment.branch_id` + tag;
   fx line `FxGain` credit `new_fx` or `FxLoss` debit `−new_fx`, description `فرق عملة محقق (تخصيص لاحق)`;
   `ledger::post` dated **now** (`:350`), description `فرق عملة محقق — تخصيص لاحق على سند {number}`,
   source `{ "payment", id, number }` (the period check and journal number come from `post`).
6. `log(Payment, "تخصيص {round2(Σ rows.amount):.2} من سند {number} ({party}) على {numbers joined '، '}",
   now, highlight_link)`; `cx.touch(Parties)`. Return the full `payment_dto`.

### 3.4 `remove_allocation(conn, cx, reg, payment_id, allocation_id)` (`payments.ts:372-388`)

1. Lock the payment; missing → `NOT_FOUND` `السند غير موجود`.
2. Allocation of that payment by id; missing → `NOT_FOUND` `التخصيص غير موجود`.
3. Lock its target document (invoice / PO; `opening` has none).
4. `apply_allocation(−1)`; **hard-delete** the `payment_allocations` row (a sub-ledger link, not a posted
   document — not in the architecture test's posted list; the mock filters it out, D-P4).
5. If `payment.target_ref == alloc.target_id` → `target_ref`/`target_ref_number` = the first remaining
   allocation's (by `position`) or `None`.
6. No GL entry (a prior FX-delta entry is left as is — Q-P1).
7. `log(Payment, "إلغاء تخصيص {alloc.amount:.2} من سند {number} عن {alloc.target_number}", now,
   highlight_link)`; `cx.touch(Parties)`. Return `payment_dto`.

### 3.5 Reads (`paymentService.ts:144-220`)

- **`get_payments` / `get_payments_paged`**: SQL `WHERE` `type`, `method`, `target_id`, `date_day` between
  `from`/`to`; batch-load allocations of the narrowed set and build rows; then in Rust: `unallocated_only`
  → `unallocated > 0.005`, and `search` → `matches_search([number, party_name, target_ref_number, note], q)`
  (joined-name haystack decided per Part 02 handoff §9: filter in Rust after SQL narrows, D-P2). Default
  order `date_key DESC, created_at, id`. Paged: `total`, `totals { amount: Σ amount }` over the filtered,
  unpaged set; `sort.key` whitelisted over `PaymentRow` fields (numbers numeric; text via collation, 08
  Q-7) then `created_at, id`; unknown key → insertion order; `LIMIT/OFFSET`.
- **`get_payment`**: `NOT_FOUND` `السند غير موجود` (the 01.C fix, analysis §8); returns the row.
- **`get_open_documents`**: `open_documents(target_type, target_id)` → DTOs (POs carry no `due_date`).

### 3.6 `payments_for_invoice(conn, invoice_id)` (for 08 `get_invoice`)

`type = RECEIVED` payments having an allocation `target_kind = 'invoice' AND target_id = invoice_id`,
ordered `created_at, id`, as full `Payment` DTOs (`invoiceService.ts:123`). Signature fixed here so the W3
`cargo check` of 08 compiles (08 PG-8).

## 4. Concurrency (D8, analysis §5)

| Race | Settled by |
|---|---|
| Two terminals allocate to the same invoice/PO | target rows `FOR UPDATE` (sorted) **before** `open_documents` is read; the second re-reads the reduced `outstanding` and gets the "أكبر من المتبقي" message. |
| Two allocations of one payment's remaining cash | payment row `FOR UPDATE` before `already` is summed. |
| Two `removeAllocation` of one allocation | payment row lock; the second gets `التخصيص غير موجود`. |
| Allocation vs. a refund of the same invoice | both lock the invoice row (08 §3.5 step 1). |
| Party deactivated while a payment posts | `share_lock_by_id("parties")` (R-1, 05 §4). |
| Payment numbers | `next_number(Payment)` + `uq_payments_number`. |

Lock order: payment → target documents (invoices, then POs, each sorted) → party (S) → settings (S) →
fiscal year (S) → counters (`core/lock.rs`, PG-7).

## 5. Undo

None registered (phase-e E-5 lists no payments action). A payment's posting is permanent; `removeAllocation`
is the only (sub-ledger) inverse, and it is itself a normal audited write (analysis §4).

## 6. Frontend switch lines (`src/modules/payments/services/paymentService.ts`)

Add `import { backendCall, usesRust } from '@/modules/core/services/backend';` and, first inside each
`wrap`: `if (usesRust('payments')) return backendCall('<cmd>', { … });` for `getPayments` `{ filter }`,
`getPaymentsPaged` `{ query }`, `getPayment` `{ id }`, `createPayment` `{ input }`,
`allocateExistingPayment` `{ paymentId, allocations }`, `removeAllocation` `{ paymentId, allocationId }`,
`getOpenDocuments` `{ targetType, targetId }`.

## 7. Known mock quirks (kept) and decisions

**Quirks kept:**
- Q-P1 `removeAllocation` doesn't reverse a realized-FX entry the allocation produced (analysis §8/§9) — see
  the open question below.
- Q-P2 The remaining-cash check uses `amount − Σ allocation.amount` (AR side), while "unallocated" uses cash
  consumed (`amountFc × payment.rate`) — they differ for FX allocations (`payments.ts:142` vs `:95`).
- Q-P3 The control line's FC rate is `round2(allocated_control / control_fc)` (2 dp, not the 6 dp rate scale).
- Q-P4 `input.targetType` is ignored; `amount = round2(amountFc × rate)` isn't checked for FX input
  (`types/index.ts:82` states it, the mock never enforces it).
- Q-P5 An `opening` allocation can be removed with no document update and no guard (`payments.ts:108-128`).

**Decisions:**
- D-P1 Recommended wave W3 (08 PG-8).
- D-P2 `partyName` search: Rust post-filter; no stored joined name (payments has no `search_normalized`, P2-38).
- D-P3 Allocation targets and the payment row are locked before any outstanding/remaining read (analysis §5).
- D-P4 `removeAllocation` hard-deletes the link row (sub-ledger, not a posted document); the audit row keeps
  the message.
- D-P5 `getOpenDocuments` and allocation validation use the one `shared::balances` open-document list (PG-6),
  not a domain copy (P2-26 "one copy").

**Open question for the user (scope/design, analysis §9 — kept as-is until answered):**
- **Q-P1 — DECIDED (2026-09-28, architectural autonomy):** when an allocation carrying a realized
  `fxGainLoss` is removed, Rust now (a) **refuses** the removal with `VALIDATION`
  (`لا يمكن إلغاء تخصيص حقق فرق عملة — قم بعكسه عبر قيد تسوية يدوي بدلاً من ذلك`), rather than (b)
  posting an inverse FX entry. This is the conservative, recommended option (same stance as the shift
  close and card settlements) per CLAUDE.md's architectural-autonomy rule — auto-picked rather than
  left blocking, since it strictly favors data integrity (no orphaned FX-adjustment entry) over
  convenience. Implemented in `service::allocate::remove_allocation` (`domains/payments/service/
  allocate.rs`). The mock (`src/mocks/backend/payments.ts`'s `unallocatePayment`) still has the old,
  silent-no-op behavior — **not changed in this pass** (out of this implementer's file scope; a
  follow-up should port the same refusal into the mock so `verify:mocks`/e2e parity holds once this
  domain flips to Rust).

## 8. Tests

**(a) `src-tauri/tests/domain_payments.rs`** (posting tests end with `shared::invariants::run_all` — invariants
3 and 6 in particular):
- receipt, no allocation: Dr cash / Cr receivable[customer] full amount; `unallocated = amount`; row status
  `unallocated`; `parties` + `ledger` bumped; activity link has `query.highlight`.
- receipt allocated to two invoices: invoices' `paid_amount`/`payment_status` updated; one GL entry only;
  `target_ref` unset (2 allocations); single allocation → `target_ref` set.
- supplier payment against a received PO (mirror).
- FX: USD invoice at 3.75, USD receipt at 3.80 → FX gain line, AR at the invoice rate, party FC balance nets
  to 0; base payment settling an FC invoice partially → the "غير مدعوم" message; full → OK.
- allocate later: base receipt then allocation to an FC invoice → second small entry (control ± / fxGain),
  `payment.fx_gain_loss` accumulated; allocation with zero FX posts nothing.
- remove allocation: document paid amount restored, row gone, no GL change; unknown ids → both `NOT_FOUND`s.
- every §3.1/§3.2 validation message byte-exact, in order (e.g. unknown doc before over-allocation).
- concurrency: two connections allocate 500 each to one invoice with 500 outstanding → one succeeds, one gets
  the "أكبر من المتبقي" message; two `allocate_existing_payment` on one payment's last 100 → one refused.
- list: filters, `unallocatedOnly`, Arabic search on party name, paged totals over the filtered set.
- `payments_for_invoice` returns only RECEIVED payments allocated to that invoice.
- period locked → `FORBIDDEN`, nothing written.

**(b) Parity cases for Part 04:** `payment-receipt-unallocated`, `payment-receipt-multi-invoice`,
`payment-supplier-po`, `payment-fx-gain-worked-example` (docs/v2/10 §2), `payment-allocate-later-fx`,
`payment-remove-allocation`, `payment-validation-each`, `payment-list-filters-paged`.

## 9. Checklist (implementation order)

- [ ] PG-1, PG-3, PG-5, PG-6 in place; wave decided (D-P1) (manager) — **needs manager confirmation**;
      this implementer found PG-1 (`shared::totals::payment_status_for`) and PG-6
      (`shared::balances::{OpenDocument,open_invoices_for,open_purchase_orders_for,allocated_total,
      unallocated_amount,unallocated_credit_for}`) already landed in Part 02, so built directly on them.
- [x] `domains/payments/{mod,dto,commands}.rs`, `service/{mod,common,read,create,allocate}.rs` (no
      separate `access.rs` needed — `TxCtx::require`/`require_any` already cover it in one line each).
- [x] DTOs (§2) incl. `allocation_status_for`; `payment_dto`/`payment_dtos`; `payments_for_invoice` first
      (08 depends on it).
- [x] `open_documents`, `apply_allocation`, `validate_allocations` (§3.1) with mock line comments.
- [x] `create_payment` (§3.2) → `allocate_existing_payment` (§3.3) → `remove_allocation` (§3.4) → reads (§3.5).
- [x] `commands.rs`: 7 commands + `ipc_sig!` lines; **manager still needs to** register them in
      `generate_handler!` and add `pub mod payments;` + the hook lines in `domains/mod.rs`.
- [x] Switch lines (§6); `src/modules/payments/types/contract.check.ts` (§2).
- [x] `tests/domain_payments.rs` (§8a) written; ⏳ deferred time-boxed test pass (not run — no cargo
      from this agent); parity list to Part 04 stays as specified in §8b (not separately produced —
      the fixtures in `tests/domain_payments.rs` cover the same scenarios).
- [x] Status note at the top of this file.

## Gate

`cargo check` clean (manager's throttled run) · tests written · 7 switch lines · `contract.check.ts`
compiles · `memory:check` 0 contract gaps for the 7 commands. DB tests and parity cases run in the deferred,
time-boxed test pass.
