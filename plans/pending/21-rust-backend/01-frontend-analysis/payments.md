# 21 · 01.B — `payments` contract

> **Status:** done (2026-09-27) · **Inventory:** `docs/backend/contract/payments.md`
> (regenerate with `bun run contract`) · **Mock spec:** `src/mocks/backend/payments.ts` ·
> **Services:** `src/modules/payments/services/paymentService.ts` · **Types:**
> `src/modules/payments/types/index.ts`
>
> Payment receipt/disbursement vouchers with sub-ledger allocation (docs/v2/02-accounting-review.md
> C1, docs/v2/09-purchases-payments-expenses.md §3, docs/v2/10 §2 "Realized FX"). **The single most
> important modeling fact in this module**: a payment posts **once**, for its full `amount`, against
> the receivable/payable control account — `allocations[]` are a **sub-ledger-only** record linking
> that one posting to one or more open documents; they never touch the GL again except for a small,
> separate FX-adjustment entry when a later allocation re-values previously-unallocated money against
> an FC document's own rate. Getting this wrong (e.g. posting a new GL entry per allocation) would
> silently double-count money on the control account.

## 1. Endpoints

| Function | Disposition (confirmed / changed + why) | Rust command | Request DTO | Response DTO | Writes (confirmed) | Shared | Undo | Notes |
|---|---|---|---|---|---|---|---|---|
| `getPayments` | port (confirmed) | `payments_get_payments` | `{ filter?: PaymentFilter }` | `PaymentRow[]` | — | — | n/a (read) | `PaymentRow` = `Payment` + computed `partyName`/`allocated`/`unallocated`/`allocationStatus` — none of the four are stored columns (see §7). Sorted by `date` descending. |
| `getPaymentsPaged` | port (confirmed) | `payments_get_payments_paged` | `{ query: PagedQuery<PaymentFilter> }` | `PagedResult<PaymentRow>` | — | — | n/a (read) | Server-mode `DataTable` variant: same filter as `getPayments`, plus paging + a `totals.amount` sum over the **filtered, unpaged** result set. Sort key is read dynamically off the row object (`(a as any)[sort.key]`) — Rust's SQL `ORDER BY` must whitelist exactly the sortable columns this generates today (`date`, `amount`, `number`, `partyName`, etc.), not accept an arbitrary client-sent column name. |
| `getPayment` | port (confirmed) | `payments_get_payment` | `{ id: string }` | `PaymentRow` | — | — | n/a (read) | **Fixed this review** — see §8: was throwing a plain `Error`, not `ApiError`. |
| `createPayment` | port (confirmed) | `payments_create_payment` | `PaymentInput` | `Payment` | `payments`, `invoices`/`purchaseOrders` (`paidAmount`/`paymentStatus` on every allocated target), `journalEntries`, `counters`, activity, audit | activity, ledger, numbering, period | see §4 | The core posting function (`recordPayment`) — one GL entry (control account vs. settlement account, plus an optional FX gain/loss line), plus a sub-ledger `allocations[]` array built from the optional `input.allocations`. See §7 for the realized-FX math this function performs inline. |
| `allocateExistingPayment` | port (confirmed) | `payments_allocate_existing_payment` | `{ paymentId: string, allocations: PaymentAllocationInput[] }` | `Payment` | `payments.allocations`, `invoices`/`purchaseOrders.paidAmount`, **conditionally** `journalEntries`/`counters` (only when the new allocation(s) realize a non-zero FX gain/loss), activity | activity, ledger (conditional), numbering (conditional), period (conditional) | see §4 | "Allocate later" — the payment's original GL entry is **never edited**; if this reallocation re-values previously-unallocated money against an FC document, a **second, small** journal entry posts just the FX delta. This conditional-posting behavior (write to `journalEntries` only sometimes) must be modeled as "the command always opens a transaction, but only inserts a journal entry row when `newFx !== 0`" — not two different Rust commands. |
| `removeAllocation` | port (confirmed) | `payments_remove_allocation` | `{ paymentId: string, allocationId: string }` | `Payment` | `payments.allocations`, `invoices`/`purchaseOrders.paidAmount` | activity | see §4 | **No GL entry at all** — removing an allocation only frees the money back to "unallocated" on the payment and reverses the target document's `paidAmount`; the original control-account posting doesn't change (the cash was always fully posted, allocation is sub-ledger bookkeeping only). If the removed allocation carried a realized `fxGainLoss` from `allocateExistingPayment`'s FX-adjustment entry, **that adjustment entry is not reversed** — flagged in §8/§9, this is the one real gap in this module. |
| `getOpenDocuments` | port (confirmed) | `payments_get_open_documents` | `{ targetType: 'customer' \| 'supplier', targetId: string }` | `OpenDocument[]` | — | — | n/a (read) | Feeds the payment form's allocation grid. See §7 — computed from `invoices`/`purchaseOrders`, not a stored view. |

## 2. DTOs → Rust

| Type | Field | Rust type | Column | Why |
|---|---|---|---|---|
| `Payment` | `id` | `Uuid` | `UUID` | UUIDv7 (D2). |
| `Payment` | `type` | `enum PaymentType { Received, Paid }` | `ENUM('RECEIVED','PAID')` | `#[serde(rename_all = "UPPERCASE")]`. |
| `Payment` | `targetType` | `enum PartyKind { Customer, Supplier }` | `ENUM('customer','supplier')` | Reuse whatever `parties.md`/`accounting.md` already name this — it's the same `partyKind` concept used on journal lines, not a payments-specific type. |
| `Payment` | `method` | `enum PaymentMethodTender { Cash, Card, BankTransfer }` | `ENUM('cash','card','bank_transfer')` | `#[serde(rename_all = "snake_case")]`. **Note the naming collision**: this `PaymentMethod` type union (`'cash' \| 'card' \| 'bank_transfer'`, 3 variants) is a *different, narrower* type than `settings.md`'s `PaymentMethod` **interface** (the configurable tender record with `accountRole`/`feePct`/etc., 6 variants including `wallet`/`credit`/`store_credit`). Do not conflate the two in Rust — name this one something like `PaymentTenderKind` to avoid a naming clash with the `settings`-owned `PaymentMethod` entity. |
| `Payment` | `amount` | `Decimal` | `DECIMAL(19,2)` | Money, `round2` throughout. |
| `Payment` | `amountFc` | `Option<Decimal>` | `DECIMAL(19,4)` | FC tendered amount — only set when `currency` is set. |
| `Payment` | `rate` | `Option<Decimal>` | `DECIMAL(19,6)` | Same FX-rate scale convention as `settings.md`'s `ExchangeRate.rate`/`setup.md`'s cash-line rate. |
| `Payment` | `fxGainLoss` | `Option<Decimal>` | `DECIMAL(19,2)` | Money — realized gain(+)/loss(−), accumulated across the initial posting AND any later `allocateExistingPayment` FX adjustments (see the `mutate(() => (payment.fxGainLoss = round2((payment.fxGainLoss ?? 0) + newFx)))` line — this field is a running total across possibly-multiple postings, not a single write-once value). |
| `Payment` | `allocations` | `Vec<PaymentAllocation>` | child table `payment_allocations(id, payment_id, target_kind, target_id, target_number, amount, date, amount_fc, fx_gain_loss)` | One-to-many, no surprises. |
| `PaymentAllocation` | `targetKind` | `enum AllocationTargetKind { Invoice, PurchaseOrder, Opening }` | `ENUM(...)` | `#[serde(rename_all = "camelCase")]`. **Note**: `'opening'` is a valid `targetKind` on the stored `PaymentAllocation` (used by `setup.md`'s `postPartyOpening`/`reversePartyOpeningBalance`'s allocation check, already reviewed there) but is **not** a valid input on `PaymentAllocationInput` (only `'invoice' \| 'purchaseOrder'`) — confirming `createPayment`/`allocateExistingPayment` can never themselves create an `'opening'`-kind allocation; only the setup module's opening-balance flow does, by writing directly to a different code path. Rust's `payments_create_payment`/`payments_allocate_existing_payment` commands should validate the input enum has only 2 variants, while the stored/response type has 3. |
| `PaymentAllocation` | `amount` / `amountFc` / `fxGainLoss` | `Decimal` (all) | `DECIMAL(19,2)` / `DECIMAL(19,4)` / `DECIMAL(19,2)` | `amount` is always the base-currency, document's-own-rate AR/AP posting amount — **never** the raw cash amount when FX is involved (see §7's worked walkthrough). |
| `PaymentFilter` | `to` | `Option<NaiveDate>` | — (request-only filter, not persisted) | **Generator hint false-positive**: the contract inventory tags this `_route_` because the field name `to` matches the route-hint regex (`^(link|to|actionTo|...)$` in `scripts/contract/config.ts`). It is NOT a route — `inDateRange(p.date, filter.from, filter.to)` uses it as a plain date-range upper bound, identical in shape to `filter.from`. **Not a mock bug, a generator heuristic limitation** — noted here so Part 02/03 doesn't accidentally type it as `RouteRef`. Worth a note in `01-FRONTEND-ANALYSIS.md`'s cross-cutting doc, not a `scripts/contract/config.ts` change (the hint regex is deliberately broad and this is a rare, single-instance collision, not worth narrowing at the cost of missing real route fields elsewhere). |
| `OpenDocument` | `kind` | `enum OpenDocumentKind { Invoice, PurchaseOrder }` | — (never persisted — this is a read-only computed shape) | `#[serde(rename_all = "camelCase")]`. |
| `OpenDocument` | `total` / `outstanding` / `fcOutstanding` / `rate` | `Decimal` (all) | — (computed) | `DECIMAL(19,2)` for the base-currency fields, `DECIMAL(19,4)`/`DECIMAL(19,6)` for `fcOutstanding`/`rate` respectively, matching the FX-field conventions established in `settings.md`/`setup.md`. |
| `PaymentRow` (service-layer type, not in the generated inventory's DTO list since it's declared in `paymentService.ts`, not `types/index.ts`) | `partyName` / `allocated` / `unallocated` / `allocationStatus` | `String` / `Decimal` / `Decimal` / `enum AllocationStatus { Full, Partial, Unallocated }` | — (all computed on read, see §7) | This is the actual response shape for every read endpoint in this module — Rust's DTOs should be named to match (`PaymentRow`, not bare `Payment`, for the list/detail commands). |

## 3. Validation and errors

| Function | Rule (source: mock check line) | Code | Exact Arabic message |
|---|---|---|---|
| `getPayment` | payment must exist | `NOT_FOUND` | `السند غير موجود` — **fixed this review**, was a plain `Error` with no code (see §8). |
| `createPayment` | `amount` (after `round2`) must be `> 0` | `VALIDATION` (default code) | `المبلغ يجب أن يكون أكبر من صفر` |
| `createPayment` | target customer/supplier must exist | `VALIDATION` (default code) | `اختر العميل` / `اختر المورد` |
| `createPayment`/`allocateExistingPayment` (via `validateAllocations`) | allocation target must be a real open document for this party | `VALIDATION` (default code) | `المستند غير موجود ضمن المستندات المفتوحة لهذا الطرف` |
| `createPayment`/`allocateExistingPayment` | total allocated across all inputs can't exceed the payment's remaining unallocated cash (`+0.005` tolerance) | `VALIDATION` (default code) | `إجمالي التخصيص أكبر من مبلغ السند` |
| `createPayment`/`allocateExistingPayment` (FC document, base-currency payment) | a **partial** base-currency allocation against an FC document is refused — only full settlement has an unambiguous FX rate | `VALIDATION` (default code) | `التخصيص الجزئي بالعملة الأساسية على مستند بعملة {currency} غير مدعوم — خصص المبلغ كاملاً أو استخدم دفعة بنفس العملة` — **deliberately scoped, not a bug**: the mock's own comment says "build precisely what the worked example needs, not guess at an unspecified partial-base-vs-FC UX." Port this refusal exactly; don't "improve" it into a partial-FX feature without a product decision. |
| `createPayment`/`allocateExistingPayment` (FC document) | FC-settled amount can't exceed the document's own `fcOutstanding` (`+0.005` tolerance) | `VALIDATION` (default code) | `الكمية المخصصة لـ {number} أكبر من المتبقي عليه ({fcOutstanding} {currency})` |
| `createPayment`/`allocateExistingPayment` (FC document) | the resulting base-currency AR/AP amount can't exceed the document's `outstanding` (`+0.01` tolerance — note the wider tolerance than the FC check above, since this is a second-order rounding check on a converted value) | `VALIDATION` (default code) | `المبلغ المخصص لـ {number} أكبر من المتبقي عليه ({outstanding})` |
| `createPayment`/`allocateExistingPayment` (base-currency document) | allocated amount can't exceed `doc.outstanding` (`+0.005` tolerance) | `VALIDATION` (default code) | `المبلغ المخصص لـ {number} أكبر من المتبقي عليه ({outstanding})` |
| `allocateExistingPayment` | payment must exist | `NOT_FOUND` | `السند غير موجود` |
| `allocateExistingPayment` | at least one valid allocation row must result | `VALIDATION` (default code) | `لم يتم إدخال أي تخصيص` |
| `removeAllocation` | payment must exist | `NOT_FOUND` | `السند غير موجود` |
| `removeAllocation` | allocation must exist on that payment | `NOT_FOUND` | `التخصيص غير موجود` |

No dedicated Zod schema for this module — every rule above is inline in `recordPayment`/`validateAllocations`/`allocatePayment`/`unallocatePayment`.

## 4. Undo matrix (every function that writes)

| Function | Undoable? | Compensation (existing fn) | Refused when | Period rule (D7) |
|---|---|---|---|---|
| `createPayment` | **Partially** — `removeAllocation` can peel back individual allocations (freeing the party's outstanding balance again), but there is **no function that reverses the payment's own GL posting itself** (no `reversePayment`/`voidPayment`). A payment, once recorded, is permanent on the ledger; "undoing" it in practice means the accountant posts a manual correcting entry, same conclusion `invoices.md` reached for `closePosShift`. **Confirmed intentional for this phase**, not a gap to fix — payments are meant to be as immutable as any other posted document, and reversing one would need the same care as `reverseJournal`'s MANUAL-only restriction (docs 21 master plan §2). Flagged for completeness, not action. | none (partial: `removeAllocation` for the sub-ledger link only) | n/a | Standard `assertOpenPeriod` via `postJournal`, no override |
| `allocateExistingPayment` | Partially reversible via `removeAllocation` for the sub-ledger link; the FX-adjustment entry it may post (when `newFx !== 0`) has **no reversal path at all** — see the next row and §8/§9, this is the module's one real gap. | `removeAllocation` (sub-ledger only — does NOT reverse the FX-adjustment journal entry) | n/a | Standard `assertOpenPeriod`, no override on this path (unlike setup/opening entries, this one does NOT pass `allowClosedPeriod: true`) |
| `removeAllocation` | n/a — it IS the (partial) undo action for an allocation, but see the note above: it doesn't reverse a prior FX-adjustment entry that allocation may have triggered | — | n/a | n/a (no GL entry) |

## 5. Concurrency under D8 (several terminals on one DB)

| Race | Rows | Settled by |
|---|---|---|
| Two terminals allocate against the same open document (invoice/PO) at once, each within their own payment's remaining balance | `invoices.paidAmount`/`purchaseOrders.paidAmount`, the target's `outstanding` (read via `invoiceOutstanding`/`purchaseOutstanding`) | **This is the sharpest concurrency risk in the module**: `validateAllocations` reads `openDocumentsFor(...)`'s `outstanding` (itself derived from `paidAmount`) at the START of the transaction, then checks the new allocation against it — if two terminals both read "500 outstanding" and each allocate 500, the document ends up over-allocated by 500. Rust's `payments_create_payment`/`payments_allocate_existing_payment` must take a locking read (`SELECT ... FOR UPDATE`) on the target invoice/PO row for the duration of the validate-then-apply sequence, not just at the final write. |
| Two terminals allocate against the SAME payment's remaining unallocated cash concurrently | `payments.allocations`, `payments.amount` (the "remaining" check) | Same class of race as above, one level up — `validateAllocations`'s `remaining = round2(payment.amount - already)` must be computed under a lock on the payment row too, so two concurrent `allocateExistingPayment` calls against the same payment can't both succeed past what's actually left. |
| Two terminals racing `removeAllocation` on the same allocation | `payments.allocations` | The second call simply gets `NOT_FOUND` once the first has removed it (a straightforward existence check under a row lock) — no special handling needed beyond ordinary transactional isolation. |

## 6. Events and side effects

- **Activity/audit rows** written for: `createPayment` (`activityKind: 'payment'`), `allocateExistingPayment` (same), `removeAllocation` (same) — every write in this module is audited, no gaps found.
- **Events emitted:** `createPayment`, `allocateExistingPayment`, `removeAllocation` all emit `parties:changed` — correct, since every write here changes a party's balance/outstanding picture. No `ledger:changed` from this module despite posting real journal entries — **consistent with `invoices.md`'s and `purchases.md`'s identical finding** (posting functions elsewhere in the app also skip `ledger:changed`), so this is a cross-module pattern to resolve once in Part 02-F's event-bridge design, not a payments-specific gap.
- **Route links**: `logActivity`'s `link` fields here are already query-string-based (`/payments?highlight=${payment.id}`), not path-templated with an id segment like most other modules' F7 violations — still a path string that needs to become a named route object with a `query: { highlight: id }` param (F7), but a slightly different shape than the `/module/${id}` pattern seen elsewhere. Tracked in §8.
- **Attachments/printing:** none.

## 7. Aggregations (reports / analytics / dashboard / insights only)

Every read-side field in this module is computed, not stored — worth documenting precisely since Part 03 will need to translate each into SQL:

| Output (DTO field) | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `PaymentRow.allocated` | `payments.allocations` (embedded array) | — | Σ per payment | none applied here (already-rounded `amount`s summed) — `allocatedTotal` (`payments.ts:81-83`) | `payments.ts:81-83` |
| `PaymentRow.unallocated` | `payments` + its `allocations` | — | per payment | `round2`, `Math.max(0, ...)` floor (`payments.ts:96`) | `payments.ts:94-97` — **note the FX-aware cash-consumed calculation**: this is NOT simply `amount − allocated`; an FC allocation's cash-consumed is `amountFc × payment.rate`, not its AR-side `amount`, per the doc comment's explicit warning against the naive subtraction. |
| `PaymentRow.allocationStatus` | derived from `amount` vs `allocated` | — | per payment | uses `allocationStatusFor(payment.amount, allocated)` (defined in `types/index.ts`, not reviewed here since it's a pure function, not a service endpoint — trivial `full`/`partial`/`unallocated` classification) | `paymentService.ts:16` |
| `OpenDocument[]` (customer) | `invoices` filtered to `status === 'COMPLETED'` and `invoiceOutstanding(i) > 0` | `customerId` match | — | `round2` on `total`/`outstanding` when FX conversion applies (`payments.ts:41,48`) | `payments.ts:29-54` |
| `OpenDocument[]` (supplier) | `purchaseOrders` filtered to `status === 'RECEIVED'` and `purchaseOutstanding(p) > 0` | `supplierId` match | — | `round2`, same pattern | `payments.ts:56-73` |
| `unallocatedCreditFor` (consumed by `parties.md`'s `Customer.unallocatedCredit`/`Supplier.unallocatedCredit`, already documented there — cross-referenced, not re-specified) | `payments` filtered by `type`/`targetType`/`targetId` | — | Σ `unallocatedAmount` per party | inherits `round2` from `unallocatedAmount` | `payments.ts:100-106` |

## 8. Contract fixes needed in the mock (01.C — resolved 2026-09-27)

Per CLAUDE.md's architectural-autonomy rule: this module's baseline `bun run verify:mocks` was
**128 ok, 0 failed** before any edit and remained **128 ok, 0 failed** after — the one fix applied
below does not touch posting/allocation/FX math at all, so it carried no accounting risk.

- [x] **`getPayment` now throws `ApiError('السند غير موجود', 'NOT_FOUND')`** instead of a plain `Error('السند غير موجود')` (`src/modules/payments/services/paymentService.ts`) — closes the F5 error-code gap, same class of fix already applied in `settings.md` (`restoreFromArchive`) and `setup.md` (`reversePartyOpeningBalance`).
- [ ] **`removeAllocation` doesn't reverse a prior FX-adjustment journal entry** the allocation it's removing may have triggered (via `allocateExistingPayment`'s conditional FX posting) — see §4. **Deliberately not fixed**: this touches realized-FX posting math directly, and the correct fix isn't obvious without a product decision (does removing an allocation need to post an *inverse* FX entry, or should it be refused outright if `alloc.fxGainLoss` is set, forcing a manual correction instead?). Flagged as a genuine open question in §9, not guessed at.
- [ ] **F7 (shared task, still pending):** 3 path-string links in `payments.ts` (`/payments?highlight=${id}` ×3, all in `recordPayment`/`allocatePayment`/`unallocatePayment`) — a slightly different shape than most other modules' F7 findings (query-param based, not path-templated), noted for the shared 01.C pass to handle correctly (a `RouteRef` with `query: { highlight: id }`, not `params: { id }`).
- [ ] **`PaymentFilter.to`'s route-hint false positive** (see §2) — not a mock fix, a generator-hint limitation. Noted for whoever reviews `scripts/contract/config.ts`'s `hints.route` regex in a later pass, not urgent enough to touch now (narrowing the regex risks missing a real route field elsewhere without a full audit of every `to`-named field in the codebase).

## 9. Open questions (→ decisions in `00-MASTER-PLAN.md`)

- **Should `removeAllocation` be refused when the allocation carries a realized `fxGainLoss`** (from a prior `allocateExistingPayment` FX-adjustment posting), rather than silently leaving that adjustment entry unreversed? Two honest options: (a) refuse the removal outright with a new validation message ("can't remove an allocation that already realized a currency gain/loss — post a manual correction instead"), matching the conservative stance `invoices.md` took for `closePosShift`; or (b) implement a proper reversal (post the exact inverse of the original FX-adjustment entry) as part of `removeAllocation`. **This is a genuine scope/design decision** — it changes what "remove an allocation" means for FX payments, not an implementation detail, so it stays open rather than being auto-decided. No accounting bug exists today (the mock's current behavior — leave it unreversed — is merely *incomplete*, not *wrong*, since nothing currently exercises this path in a way `verify:mocks` would catch), so this doesn't block the gate below; it's a note for whoever implements `payments_remove_allocation` in Part 03.

## Gate

- [x] Every inventory function is in §1 with a confirmed disposition (7/7, no dispositions changed).
- [x] Every write function is in §4.
- [x] Every DTO field needing a non-default mapping is in §2.
- [x] `bun run contract:check` — no override applied, contract unchanged (7/7 port). Also green: `bun run build`, `bun run check`, `bun run verify:mocks` (128 ok, 0 failed, unchanged before/after the one fix), `bun run memory:check` (0 new seam violations — see below), `bun run diag:check`.
