# 21 · 01.B — `vouchers` contract

> **Status:** done (2026-09-27) · **Inventory:** `docs/backend/contract/vouchers.md`
> (regenerate with `bun run contract`) · **Mock spec:** `src/mocks/backend/vouchers.ts` (general
> vouchers), `src/mocks/backend/settlements.ts` (card/wallet settlement) · **Services:**
> `src/modules/vouchers/services/voucherService.ts` · **Types:** `src/modules/vouchers/types/index.ts`
>
> Two things live in this module, both "fast journal" documents that post directly through
> `postJournal` with no sub-ledger allocation step (unlike `payments.md`'s receipt/payment vouchers,
> which allocate against open invoices/POs): **(1) the four general-voucher kinds** — RECEIPT/
> PAYMENT/TRANSFER/OWNER (docs/v2/09-purchases-payments-expenses.md §5, "the reference's fast
> journal for money movements that aren't invoices") — and **(2) card/wallet settlement**
> (docs/v2/09 §2 "Card settlement", docs/v2/02-accounting-review.md C3/F4), which clears the
> `cardClearing`/`walletClearing` account that POS card/wallet tenders post to at sale time
> (`payments.md` and `invoices.md` already documented the receivable/payable control-account side
> of money movement; this module documents the **other** kind of clearing balance — the one
> `sales.ts`'s tender lines create and only a settlement voucher here can clear). Confirmed by
> reading `docs/backend/contract/vouchers.md` first: nothing in this module overlaps `payments.md`
> (no allocation, no control account) or `accounting.md` (manual journal entries) — every function
> here is either a general voucher or a settlement, both new document types the mock invents
> specifically for money movements invoices/purchases/payments don't cover.

## 1. Endpoints

| Function | Disposition (confirmed / changed + why) | Rust command | Request DTO | Response DTO | Writes (confirmed) | Shared | Undo | Notes |
|---|---|---|---|---|---|---|---|---|
| `createReceiptVoucher` | port (confirmed) | `vouchers_create_receipt_voucher` | `ReceiptVoucherInput` | `Voucher` (RECEIPT) | `vouchers`, `journalEntries`, `counters`, activity, audit | activity, ledger, numbering, period | none (see §4) | سند قبض عام — Dr method account / Cr chosen (user-picked) account. |
| `createPaymentVoucher` | port (confirmed) | `vouchers_create_payment_voucher` | `PaymentVoucherInput` | `Voucher` (PAYMENT) | same as above | activity, ledger, numbering, period | none | سند صرف عام — Dr chosen account / Cr method account. |
| `createTransferVoucher` | port (confirmed) | `vouchers_create_transfer_voucher` | `TransferVoucherInput` | `Voucher` (TRANSFER) | same as above | activity, ledger, numbering, period | none | Dr destination / Cr source (+ fee), the mechanism a POS shift's cash-drop uses (`invoices.md`'s `closePosShift` optionally calls this — already documented there as a consumer, not re-specified here). |
| `createOwnerVoucher` | port (confirmed) | `vouchers_create_owner_voucher` | `OwnerVoucherInput` | `Voucher` (OWNER) | same as above | activity, ledger, numbering, period | none | مسحوبات/إضافة رأس مال — drawings vs. contribution flips debit/credit between `drawings` and the chosen cash account. |
| `getVouchers` | port (confirmed) | `vouchers_get_vouchers` | `{ filter?: VoucherFilter }` | `Voucher[]` | — | — | n/a (read) | Filters by `kind`, date range, text search over `number`/`description`/`note`. Sorted by `date` descending. |
| `getVoucher` | port (confirmed) | `vouchers_get_voucher` | `{ id: string }` | `Voucher` | — | — | n/a (read) | `NOT_FOUND` if missing — already correct (`getVoucherById`, `vouchers.ts:149`). |
| `getUnsettledTenderGroups` | port (confirmed) | `vouchers_get_unsettled_tender_groups` | — | `UnsettledTenderGroup[]` | — | — | n/a (read) | The settlement screen's pick list — day×method groups of card/wallet tenders not yet covered by a posted settlement. Computed, not stored (see §7). |
| `estimateSettlementFee` | port (confirmed) | `vouchers_estimate_settlement_fee` | `{ groups: UnsettledTenderGroup[] }` | `number` | — | — | n/a (read, pure calc) | Pre-fills the deposit-amount field from each payment method's `feePct` — a convenience estimate the user can override; the **actual** fee posted is always `grossAmount − depositAmount` as entered on `createCardSettlement`, never this estimate (see §7, don't conflate the two in Rust). |
| `createCardSettlement` | port (confirmed) | `vouchers_create_card_settlement` | `CardSettlementInput` | `CardSettlement` | `cardSettlements`, `journalEntries`, `counters`, activity, audit | activity, ledger, numbering, period | none (see §4) | Dr bank + Dr cardFees (if fee > 0) / Cr cardClearing-or-walletClearing (grouped by role, so a settlement mixing both card and wallet groups gets one credit line per role). |
| `getCardSettlements` | port (confirmed) | `vouchers_get_card_settlements` | — | `CardSettlement[]` | — | — | n/a (read) | Sorted by `date` descending. |
| `getCardSettlement` | port (confirmed) | `vouchers_get_card_settlement` | `{ id: string }` | `CardSettlement` | — | — | n/a (read) | `NOT_FOUND` if missing — already correct (`getCardSettlementById`, `settlements.ts:112`). |

No disposition changes; the contract generator's own 11/11 `port` count is confirmed as-is.

## 2. DTOs → Rust

| Type | Field | Rust type | Column | Why |
|---|---|---|---|---|
| `VoucherBase`/`Voucher` | `id` | `Uuid` | `UUID` | UUIDv7 (D2). |
| `VoucherBase` | `kind` | `enum VoucherKind { Receipt, Payment, Transfer, Owner }` | discriminator column (single-table, see below) | `#[serde(rename_all = "UPPERCASE")]` — matches the 4-way TS union `'RECEIPT' \| 'PAYMENT' \| 'TRANSFER' \| 'OWNER'` exactly. |
| `VoucherBase` | `amount` | `Decimal` | `DECIMAL(19,2)` | Money, `round2` throughout (`vouchers.ts` never calls `round4` — every amount here is already a rounded money value from the form, unlike `products.md`'s cost/qty fields). |
| `VoucherBase` | `attachmentIds` | `Vec<Uuid>` | child table or JSON array | Same attachment-id pattern as other modules — no voucher-specific handling. |
| `VoucherBase` | `costCenterId` | `Option<Uuid>` | nullable FK | Falls through to `resolvePosting`'s branch-default cost center when unset on the posting line, same as every other module's `PostingLine.costCenterId` (`core.ts:72`). |
| `Voucher` (discriminated union) | — | **One entity table with nullable kind-specific columns**, not four separate tables | `vouchers(id, number, kind, date, amount, description, note, created_by, ..., payment_method_id NULL, credit_account_id NULL, debit_account_id NULL, source_account_id NULL, destination_account_id NULL, fee_amount NULL, fee_account_id NULL, direction NULL, cash_account_id NULL)` | The TS type is `ReceiptVoucher \| PaymentVoucher \| TransferVoucher \| OwnerVoucher`, each adding 1-3 fields to the same `VoucherBase` shape and sharing one `number` sequence (`nextNumber('voucher')` — one counter, not four) and one list/detail read path (`getVouchers`/`getVoucher` return the union, `VoucherFilter.kind` filters across all four). A single table with kind-specific nullable columns mirrors this 1:1 (same shape SeaORM/serde already needs for the union deserialize-by-`kind` tag) and keeps `getVouchers`'s cross-kind list/filter a single `SELECT`, matching what the mock does today (`db.vouchers` is one flat array). Four separate tables would force `getVouchers` into a `UNION ALL` for no benefit, since no kind-specific column is ever queried across kinds. |
| `ReceiptVoucher` | `paymentMethodId` / `creditAccountId` | `Uuid` / `Uuid` | FK columns | `creditAccountId` is checked against `allowManual` at write time (§3) — Rust must re-run that check, not trust the FK alone. |
| `PaymentVoucher` | `paymentMethodId` / `debitAccountId` | `Uuid` / `Uuid` | FK columns | Same `allowManual` re-check on `debitAccountId`. |
| `TransferVoucher` | `sourceAccountId` / `destinationAccountId` | `Uuid` / `Uuid` | FK columns | Must differ (`VALIDATION` if equal, §3) — no `allowManual` check on transfer accounts (any two distinct accounts are allowed; the reference intends this for drawer↔bank / bank↔bank moves that may legitimately be non-manual-postable-elsewhere accounts). |
| `TransferVoucher` | `feeAmount` | `Option<Decimal>` | `DECIMAL(19,2)` | `round2`'d; stored as `undefined`/`NULL` when the rounded fee is exactly 0 (`vouchers.ts:96`: `feeAmount: fee \|\| undefined`) — Rust should store `NULL` rather than `Some(0)` to match. |
| `TransferVoucher` | `feeAccountId` | `Option<Uuid>` | nullable FK | Required by validation when `feeAmount > 0` (§3), otherwise unset. |
| `OwnerVoucher` | `direction` | `enum OwnerDirection { Drawings, Contribution }` | `ENUM('drawings','contribution')` | `#[serde(rename_all = "snake_case")]` (matches the 2 lowercase TS literals). |
| `OwnerVoucher` | `cashAccountId` | `Uuid` | FK | The user-picked cash/bank account; the other leg is always the fixed `drawings` system-role account (`accountFor('drawings')`, not user-selectable). |
| `VoucherFilter` | `to` | `Option<NaiveDate>` | — (request filter, not persisted) | **Same generator-hint false positive `payments.md` already flagged** (`PaymentFilter.to`) — tagged `_route_` by the naming heuristic, but `inDateRange(v.date, filter.from, filter.to)` uses it as a plain date-range bound. Confirmed, not re-derived; no new config override needed, this is the second and (per the `01-FRONTEND-ANALYSIS.md` note already logged) expected instance of the same known limitation. |
| `CardSettlementInput`/`CardSettlement` | `groups[].date` | `NaiveDate` | — | Local date key (`localDateKey`), the day-grouping key for tenders — see §7. |
| `CardSettlement` | `groups` | `Vec<{ date, paymentMethodId, amount }>` | child table `card_settlement_groups(settlement_id, date, payment_method_id, amount)` | Snapshotted at settlement time (`selected.map(...)`, `settlements.ts:77`) — **not** a live re-query; if a later data change altered what "the total for that day×method" would be, the settlement's own stored `groups[].amount` stays frozen. Rust must persist these as their own rows, not recompute them from `unsettledTenderGroups()` on read. |
| `CardSettlement` | `grossAmount` / `depositAmount` / `feeAmount` | `Decimal` (all three) | `DECIMAL(19,2)` | `round2` throughout; `feeAmount = round2(grossAmount − depositAmount)`, can be **negative** (deposit exceeded gross — see §3's `-0.005` tolerance and the extra `bank` debit line for that rare case, `settlements.ts:94`). |
| `UnsettledTenderGroup` | `accountRole` | `enum ClearingRole { CardClearing, WalletClearing }` | — (never persisted — read-only computed shape, same pattern as `payments.md`'s `OpenDocument`) | Narrower than the full `SystemRole` enum (`accounts.ts`) — only these 2 of ~30 roles are ever the value of this field, since `unsettledTenderGroups()` filters `paymentMethods` down to exactly these two `accountRole` values (`settlements.ts:24`). |
| `UnsettledTenderGroup` | `total` | `Decimal` | `DECIMAL(19,2)` | `round2` (`settlements.ts:40,48`). |
| `UnsettledTenderGroup` | `tenderCount` | `i32` (not `Decimal`) | `INT` | **Generator hint imprecision**: tagged `_decimal_` by the contract inventory because it's a bare `number`, but it's an integer tender count (`existing.tenderCount += 1`, `settlements.ts:41`), never fractional or rounded. Same class of hint limitation as `VoucherFilter.to`'s route false-positive — noted here, not worth narrowing the generator's numeric-field heuristic for one field. |

## 3. Validation and errors

| Function | Rule (source: mock check line) | Code | Exact Arabic message |
|---|---|---|---|
| `createReceiptVoucher`/`createPaymentVoucher`/`createTransferVoucher`/`createOwnerVoucher` | `amount > 0` | `VALIDATION` (default code) | `المبلغ يجب أن يكون أكبر من صفر` |
| `createReceiptVoucher`/`createPaymentVoucher` (via `methodAccount`) | `paymentMethodId` must resolve to an active payment method | `VALIDATION` (default code) | `اختر طريقة الدفع` |
| `createReceiptVoucher` | `creditAccountId` must resolve to a real account (`accountById`) | `NOT_FOUND` | `الحساب غير موجود في شجرة الحسابات` |
| `createReceiptVoucher` | that account must have `allowManual: true` | `VALIDATION` (default code) | `` لا يمكن الترحيل يدوياً إلى حساب "{name}" `` |
| `createPaymentVoucher` | `debitAccountId` must resolve to a real account | `NOT_FOUND` | `الحساب غير موجود في شجرة الحسابات` |
| `createPaymentVoucher` | that account must have `allowManual: true` | `VALIDATION` (default code) | `` لا يمكن الترحيل يدوياً من حساب "{name}" `` |
| `createTransferVoucher` | `sourceAccountId` ≠ `destinationAccountId` | `VALIDATION` (default code) | `اختر حسابين مختلفين للتحويل` |
| `createTransferVoucher` | both accounts must resolve (`accountById` ×2) | `NOT_FOUND` | `الحساب غير موجود في شجرة الحسابات` |
| `createTransferVoucher` | `feeAccountId` required when `round2(feeAmount) > 0` | `VALIDATION` (default code) | `اختر حساب العمولة` |
| `createOwnerVoucher` | `cashAccountId` must resolve to a real account | `NOT_FOUND` | `الحساب غير موجود في شجرة الحسابات` |
| `createOwnerVoucher` | `drawings` system-role account must exist in the CoA (`accountFor('drawings')`) | `NOT_FOUND` | `` لا يوجد حساب في شجرة الحسابات لدور "المسحوبات الشخصية" — أضف حساباً بهذا الدور أولاً `` |
| all 4 general-voucher creators (via `postJournal` → `resolvePosting`) | resolved debits must equal resolved credits (± 0.001) | `VALIDATION` (default code) | `القيد غير متوازن: المدين {x} ≠ الدائن {y}` — should never actually fire given the hand-built 2/3-line posting arrays above; kept as the shared ledger choke-point's own defense, not a voucher-specific rule. |
| all 4 general-voucher creators + `createCardSettlement` (via `postJournal` → `assertOpenPeriod`) | posting date must be in an open, unlocked fiscal period | `FORBIDDEN` | `` لا يمكن الترحيل في تاريخ {date} — الفترة مقفلة حتى {lockDate} `` / `` …السنة المالية "{name}" مقفلة `` — no `allowClosedPeriod` override anywhere in this module (unlike `setup.md`'s opening-balance flow), so vouchers and settlements always respect the period lock, even for admins. |
| `createCardSettlement` | `groups.length >= 1` | `VALIDATION` (default code) | `اختر يوماً واحداً على الأقل للتسوية` |
| `createCardSettlement` | `depositAmount >= 0` | `VALIDATION` (default code) | `أدخل مبلغ الإيداع البنكي` |
| `createCardSettlement` | every requested `{date, paymentMethodId}` group must still be in the live `unsettledTenderGroups()` result (not already settled by a concurrent/prior settlement) | `CONFLICT` | `أحد العناصر المختارة غير متاح للتسوية (ربما تمت تسويته بالفعل)` |
| `createCardSettlement` | `feeAmount` (`grossAmount − depositAmount`) can't be less than `-0.005` (deposit can exceed gross by a hair for a bank correction/bonus, but not meaningfully) | `VALIDATION` (default code) | `مبلغ الإيداع أكبر من إجمالي العمليات المختارة` |
| `getVoucher` | voucher must exist | `NOT_FOUND` | `السند غير موجود` — already correct, no fix needed. |
| `getCardSettlement` | settlement must exist | `NOT_FOUND` | `سند التسوية غير موجود` — already correct, no fix needed. |

No dedicated Zod schema for this module — every rule above is inline in `vouchers.ts`/`settlements.ts`, matching `payments.md`'s finding for the same class of "fast journal" document.

## 4. Undo matrix (every function that writes)

| Function | Undoable? | Compensation (existing fn) | Refused when | Period rule (D7) |
|---|---|---|---|---|
| `createReceiptVoucher` | **No** — no reversal/void function exists for any general voucher kind. Same conclusion `payments.md` reached for `createPayment`: once posted, "undo" means a manual correcting journal entry, not a code path. Confirmed intentional, not a gap — a general voucher is deliberately a thin, one-shot posting with no sub-ledger state to unwind (unlike `payments.md`'s allocations), so there is nothing partial to peel back either. | none | n/a | Standard `assertOpenPeriod`, no override |
| `createPaymentVoucher` | No (same as above) | none | n/a | Standard `assertOpenPeriod`, no override |
| `createTransferVoucher` | No (same as above) | none | n/a | Standard `assertOpenPeriod`, no override |
| `createOwnerVoucher` | No (same as above) | none | n/a | Standard `assertOpenPeriod`, no override |
| `createCardSettlement` | **No** — no reversal function exists. Unwinding one is materially harder than a general voucher: it would need to both reverse the GL entry (bank/cardFees/clearing) AND re-open the settled `{date, paymentMethodId}` tender groups so they reappear in `unsettledTenderGroups()` (currently keyed purely by "does any settlement's `groups[]` already cover this key" — `settledKeys()`, `settlements.ts:16`). A future undo would need to either delete the settlement row outright (breaking the "nothing hard-deleted" rule, master plan §3 rule 7) or add a `voided`/`reversed` flag `settledKeys()` also checks. **Deliberately left as a Part 03 design question, not guessed at here** — see §9. | none today | n/a | Standard `assertOpenPeriod`, no override |

## 5. Concurrency under D8 (several terminals on one DB)

| Race | Rows | Settled by |
|---|---|---|
| Two terminals post a general voucher (any of the 4 kinds) at the same moment | `counters.voucher` (`nextNumber('voucher')`), `journalEntries` | Same "shared counter, prefixed" pattern already documented in `purchases.md`/`products.md` — Rust needs the counter row locked (`SELECT ... FOR UPDATE`) for the duration of allocate-and-increment, or a DB sequence/`AUTO_INCREMENT` shadow column with a unique constraint on the rendered document number. No document-level state is read-then-written here (unlike payments' allocation race), so this is the *only* race for the 4 general-voucher functions — they're pure "validate inputs, resolve accounts, post" with no shared mutable state beyond the number sequence. |
| Two terminals both try to settle overlapping `{date, paymentMethodId}` tender groups at once | `cardSettlements` (existence check via `unsettledTenderGroups()` → `settledKeys()`), `counters.cardSettlement` | **This is this module's sharpest race**, structurally identical to `payments.md`'s "two terminals allocate the same outstanding balance" finding: `recordCardSettlement` computes `unsettledTenderGroups()` (which depends on **every row** of `db.cardSettlements` read so far) at the start of its own transaction, then validates the requested groups against that snapshot. If two terminals both read "day X / method Y is unsettled" before either commits, both can post a settlement for the same tenders, double-crediting the clearing account and leaving a permanent understatement (the mock's own in-process execution serializes this away, but MariaDB with two real connections would not). Rust's `vouchers_create_card_settlement` must take a locking pattern that makes "is this {date, paymentMethodId} pair already settled" atomic with the insert — either a real DB uniqueness constraint on `card_settlement_groups(date, payment_method_id)` (cleanest: the DB itself refuses the second insert, no manual lock needed) or a `SELECT ... FOR UPDATE` over the invoices/tenders being grouped. The **unique constraint** is the recommended approach — it needs no extra locking logic and turns the race into a normal constraint-violation error mapped to the same `CONFLICT` code the mock already returns for this case. |
| Two terminals post a card settlement referencing overlapping-but-not-identical group sets (e.g. terminal A settles {day1,card}, terminal B settles {day1,card,day2,wallet}) while a sale is still being tendered on a third terminal that same day | `invoices.tenders` (read by `unsettledTenderGroups()`), `cardSettlements` | Not a genuine new race beyond the one above — a same-day sale completing concurrently with a settlement just changes what the *next* `unsettledTenderGroups()` call sees; it can't retroactively invalidate a settlement that already locked in its `groups[].amount` snapshot. No additional lock needed beyond the unique constraint above. |

## 6. Events and side effects

- **Activity/audit rows**: every write function in this module calls `logActivity('voucher', ...)` (4 general-voucher creators) or `logActivity('payment', ...)` (`createCardSettlement` — reuses the `payment` activity kind rather than a `voucher`-specific one, since it's conceptually closer to a payment settlement than a general voucher; not a bug, just the kind label choice already made in the mock). `logActivity` is the adapter that also writes the structured `AuditEntry` to `db.audit` (`core.ts:331`), so **every write in this module is fully audited already** — zero gaps found, matching `payments.md`'s and `purchases.md`'s "already correct" finding for this bug class.
- **Events emitted**: all 5 write functions (`createReceiptVoucher`, `createPaymentVoucher`, `createTransferVoucher`, `createOwnerVoucher`, `createCardSettlement`) emit `ledger:changed` — **this module is the exception to the cross-module pattern** `invoices.md`/`purchases.md`/`payments.md` all found (posting functions elsewhere skip `ledger:changed`). Every posting path here correctly signals the ledger changed. No `catalog:changed`/`parties:changed` — correctly so, since nothing here touches products or party balances.
- **Route links**: all 5 `logActivity` calls use path-templated string links (`` `/vouchers/${voucher.id}` `` ×4, `` `/payments/settlements/${settlement.id}` ``) — the standard F7 violation shape (unlike `payments.md`'s query-string variant), tracked in §8 for the shared 01.C pass.
- **Attachments**: the 4 general-voucher creators accept `input.attachmentIds` and pass them straight through to `postJournal`'s `attachmentIds` (stored on the `JournalEntry`, not the voucher itself) — `createCardSettlement` has no attachment support at all (not in `CardSettlementInput`), which is consistent with it being a computed/derived posting rather than a user-attached-evidence document.
- **Printing**: both voucher kinds "print as a PDF" per the type file's header comment (generic voucher print route, since `pdfService` only has the invoice template) — not reviewed here since `templateService`/`pdfService` are `templates.md`'s territory, not this module's.

## 7. Aggregations (reports / analytics / dashboard / insights only)

This module isn't a reports/analytics module, but two read endpoints are meaningfully computed
rather than stored, worth documenting for Part 03:

| Output (DTO field) | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `UnsettledTenderGroup[]` | `invoices` (their embedded `tenders[]`), `paymentMethods`, `cardSettlements` (to exclude already-settled keys) | `invoice.status === 'COMPLETED'`, `paymentMethod.accountRole ∈ {cardClearing, walletClearing}`, key not in `settledKeys()` | `(localDateKey(invoice.date), paymentMethodId)` | `round2` on the running `total` sum (`settlements.ts:40`) | `settlements.ts:23-55` — Part 03's SQL equivalent is a `GROUP BY DATE(invoice.date), payment_method_id` over tender line items with a `NOT EXISTS` (or `LEFT JOIN ... IS NULL`) against `card_settlement_groups`, not a full in-memory scan. |
| `estimateSettlementFee` | `paymentMethods` (`feePct` per group) | — (pure function over its `groups` argument, no DB filter of its own) | Σ per group, `total × feePct / 100` | `round2` on the final sum (`settlements.ts:118`) | `settlements.ts:117-124` — a pure calculation Rust can port as-is; **not** the same number as the posted `feeAmount` (§2), which is always `grossAmount − depositAmount` as entered, never this estimate. Keep the two clearly named/separated in Rust (e.g. `estimate_settlement_fee` vs. the settlement's own stored `fee_amount`) so a future reader doesn't conflate an estimate with a posted fact. |

## 8. Contract fixes needed in the mock (01.C)

Per CLAUDE.md's architectural-autonomy rule: this module's baseline `bun run verify:mocks` was
**128 ok, 0 failed** before any review and **no mock-code edit was made**, so it remains
**128 ok, 0 failed** unchanged. This is the third module this session (after `purchases.md` and
`invoices.md`) with zero fixes needed — every named bug class was checked and found already correct:

- **Saudi-only VAT regex**: not applicable — this module never touches tax numbers or VAT registration.
- **Hard-delete/cancel with no reference/state check**: not applicable — there is no delete or cancel
  function anywhere in this module (no `deleteVoucher`, `cancelCardSettlement`, etc.). Every function
  is create-or-read only, so there's nothing to hard-delete a reference check could have prevented.
- **Writes with no audit trail**: checked and found **already correct** — every one of the 5 write
  functions calls `logActivity` (see §6), which itself writes both the activity feed row and the
  structured `AuditEntry`. Zero gaps.
- **Plain `Error` instead of `ApiError`**: checked (`grep` over `vouchers.ts`/`settlements.ts`/
  `voucherService.ts`) — every throw site already uses `ApiError`, including the two `getXById`
  lookups (`النوع غير موجود` messages, `NOT_FOUND` code) that other modules sometimes got wrong.
  Zero gaps.
- **Silent-skip-instead-of-throw**: checked — `createCardSettlement`'s loop over `input.groups`
  throws `CONFLICT` the moment any requested group isn't in the live unsettled set (`settlements.ts:66`),
  it doesn't silently drop the invalid group and proceed with a partial settlement. No silent-skip
  pattern found anywhere in the module.

Remaining items are tracked, not fixed here, because they are either shared cross-module work
already scheduled in 01.C or genuine design questions (§9):

- [ ] **F7 (shared task, still pending)**: 5 path-templated string links in this module
      (`/vouchers/${id}` ×4 in `vouchers.ts`, `/payments/settlements/${id}` ×1 in `settlements.ts`)
      — the standard path-string shape (unlike `payments.md`'s query-string variant), for the shared
      01.C pass to turn into `RouteRef` objects.
- [ ] **`VoucherFilter.to`'s route-hint false positive** (§2) — not a mock bug, the same generator-hint
      limitation already logged against `payments.md`'s `PaymentFilter.to`; no action needed beyond
      the note already carried in `01-FRONTEND-ANALYSIS.md`'s cross-cutting doc.
- [ ] **No reversal path for `createCardSettlement`** — see §4/§9. Deliberately not implemented as a
      mock fix: it isn't a bug (nothing today is wrong or silently incorrect), it's a missing
      capability that needs a design decision on how "unsettling" should interact with
      `settledKeys()`'s existence-based lookup.

## 9. Open questions (→ decisions in `00-MASTER-PLAN.md`)

- **Should card/wallet settlements ever be reversible, and if so, how does un-settling interact with
  `settledKeys()`'s lookup?** Today `unsettledTenderGroups()` treats "does any settlement's `groups[]`
  contain this `{date, paymentMethodId}` key" as a permanent fact once a settlement posts (§4) — there
  is no `voided` state. Two honest implementation paths if a future phase wants this: (a) add a
  `status: 'POSTED' | 'VOIDED'` column to settlements and have `settledKeys()` skip voided ones (so
  the tender groups reappear for re-settlement), paired with a real reversing journal entry for the
  bank/fees/clearing lines; or (b) refuse reversal entirely and require a manual correcting journal
  entry, the same conservative stance `payments.md` and this module's own general vouchers already
  take. **This is a genuine scope/design decision, not an implementation detail** — it changes what
  "undo" means for a settlement and touches the clearing-balance invariant (`verify:mocks`'s
  "card clearing = unsettled card tenders" check, confirmed still green in the baseline run above)
  that already depends on `settledKeys()`'s current permanent-once-settled semantics. No accounting
  bug exists today (nothing exercises an unsettle path), so this doesn't block the gate; it's a note
  for whoever implements `vouchers_create_card_settlement`'s full lifecycle in Part 03. Recommendation
  if forced to pick now: (b), for consistency with every other document type in this module and with
  `payments.md`'s identical conclusion for `createPayment` — but this is flagged as a decision for
  the user/Part-03 implementer, not silently assumed.

## Gate

- [x] Every inventory function is in §1 with a confirmed disposition (11/11, no dispositions changed).
- [x] Every write function is in §4 (5/5: 4 general-voucher creators + `createCardSettlement`).
- [x] Every DTO field needing a non-default mapping is in §2.
- [x] `bun run contract:check` — no override applied, contract unchanged (11/11 port). Also green:
      `bun run build`, `bun run check`, `bun run verify:mocks` (128 ok, 0 failed, unchanged —
      no mock edit was made this review), `bun run memory` + `bun run memory:check` (0 new seam
      violations), `bun run diag:check`.
