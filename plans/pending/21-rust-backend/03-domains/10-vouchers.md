# 21 · 03.10 — `vouchers` (general receipt/payment/transfer/owner vouchers, card & wallet settlements)

> **Status:** 2026-09-28 — code complete (`domains/vouchers/{mod,dto,commands}.rs`,
> `service/{general,settlements,read}.rs`, all 11 commands + `ipc_sig!` lines, frontend switch lines,
> `types/contract.check.ts`, `tests/domain_vouchers.rs` with 22 tests). Written blind per the
> per-implementer hard rules (no cargo/bun run from this agent) — **not yet compiled, `cargo check`
> and the DB-backed test run are deferred to the manager's throttled pass** (⏳ deferred time-boxed
> test pass). `domains/mod.rs` (the `pub mod vouchers;` line, `all_ipc_signatures()`,
> `export_bindings()` hook, and `generate_handler!` registration) is manager-owned — **needs the
> manager** to wire this domain in; see "Needs from manager" in the wave's final report.

**Goal.** Port `voucherService.ts`, `src/mocks/backend/vouchers.ts` and `src/mocks/backend/settlements.ts`:
four "fast journal" vouchers that post one balanced entry each, and the card/wallet settlement that clears
`cardClearing`/`walletClearing` (docs/v2/02 C3, §3 "Card settlement") with a DB-enforced
one-settlement-per-(day, method) rule (analysis §5, `uq_card_settlement_groups_date_method`). Also expose
`record_transfer_voucher` as the cash-drop engine 08b calls.

**Read first.** [`../01-frontend-analysis/vouchers.md`](../01-frontend-analysis/vouchers.md) §1–§9 · mock
`src/mocks/backend/vouchers.ts:21-151`, `src/mocks/backend/settlements.ts:16-127`,
`src/modules/vouchers/services/voucherService.ts:135-194` · types `src/modules/vouchers/types/index.ts:1-106`
· pages `VoucherFormPage.vue`, `VoucherListPage.vue`, `VoucherDetailPage.vue`, `CardSettlementPage.vue` ·
Rust `entities/payments/{vouchers,card_settlements,card_settlement_groups}.rs`, `entities/sales/{invoices,
invoice_tenders}.rs`, `entities/org/{accounts,payment_methods}.rs`, migration `m0010_payments.rs:88-190`,
`shared/ledger/{post,accounts}.rs`, `utils/text.rs` (`like_contains`, `normalize_arabic`).

## 1. Commands

Every route of this module is area `payments` (`vouchers/routes`: `vouchers`, `voucher-new` write,
`voucher-detail`; `card-settlements` write); `pdfService` prints vouchers from the same area. Reads:
`with_read` + `core::settings::require(tx, actor, Payments, Read)` (actor cloned from `state.session`).

| # | Mock fn (`voucherService.ts`) | Disp. | Rust command | Args → Return | Area / Access | Tx | Events |
|---|---|---|---|---|---|---|---|
| 1 | `createReceiptVoucher` (:135) | port | `vouchers_create_receipt_voucher` | `{ input: ReceiptVoucherInput }` → `Voucher` | Payments / Write | `with_tx` | ledger |
| 2 | `createPaymentVoucher` (:140) | port | `vouchers_create_payment_voucher` | `{ input: PaymentVoucherInput }` → `Voucher` | Payments / Write | `with_tx` | ledger |
| 3 | `createTransferVoucher` (:145) | port | `vouchers_create_transfer_voucher` | `{ input: TransferVoucherInput }` → `Voucher` | Payments / Write | `with_tx` | ledger |
| 4 | `createOwnerVoucher` (:150) | port | `vouchers_create_owner_voucher` | `{ input: OwnerVoucherInput }` → `Voucher` | Payments / Write | `with_tx` | ledger |
| 5 | `getVouchers` (:155) | port | `vouchers_get_vouchers` | `{ filter?: VoucherFilter }` → `Vec<Voucher>` | Payments / Read | `with_read` | — |
| 6 | `getVoucher` (:164) | port | `vouchers_get_voucher` | `{ id }` → `Voucher` | Payments / Read | `with_read` | — |
| 7 | `getUnsettledTenderGroups` (:171) | port | `vouchers_get_unsettled_tender_groups` | `()` → `Vec<UnsettledTenderGroup>` | Payments / Read | `with_read` | — |
| 8 | `estimateSettlementFee` (:176) | port | `vouchers_estimate_settlement_fee` | `{ groups: Vec<UnsettledTenderGroup> }` → `FeeEstimate` | Payments / Read | `with_read` | — |
| 9 | `createCardSettlement` (:181) | port | `vouchers_create_card_settlement` | `{ input: CardSettlementInput }` → `CardSettlement` | Payments / Write | `with_tx` | ledger |
| 10 | `getCardSettlements` (:186) | port | `vouchers_get_card_settlements` | `()` → `Vec<CardSettlement>` | Payments / Read | `with_read` | — |
| 11 | `getCardSettlement` (:191) | port | `vouchers_get_card_settlement` | `{ id }` → `CardSettlement` | Payments / Read | `with_read` | — |

Public (non-command) service function for 08b: `service::general::record_transfer_voucher(conn, cx, reg,
input: TransferVoucherInput, number: Option<String>) -> TxResult<Voucher>` (§3.3).

## 2. DTOs (`domains/vouchers/dto.rs`, `#[ts(export_to = "vouchers/types/gen/")]`, conventions as 08 §2)

| Rust DTO | TS type (file:line) | Field notes |
|---|---|---|
| `VoucherKind`, `OwnerDirection` | `types/index.ts:6-9` | UPPERCASE / snake_case. |
| `Voucher` | `:11-58` (`ReceiptVoucher \| PaymentVoucher \| TransferVoucher \| OwnerVoucher`) | `#[serde(tag = "kind")]` enum with variants `RECEIPT`/`PAYMENT`/`TRANSFER`/`OWNER`, each a struct = the `VoucherBase` fields (`id`, `number`, `date` = DocDate key, `amount`, `description`, `note?`, `attachment_ids?`, `cost_center_id?`, `created_by`) + its own fields (`payment_method_id` + `credit_account_id` / `debit_account_id`; `source_account_id`, `destination_account_id`, `fee_amount?`, `fee_account_id?`; `direction`, `cash_account_id`). One row shape (`vouchers` table, nullable kind columns — analysis §2) mapped by `kind`. |
| `ReceiptVoucherInput` … `OwnerVoucherInput` | `:60-63` | request-only (`Omit<…, 'id' \| 'number' \| 'createdBy' \| 'kind'>`); `date: RawDocDate`. |
| `VoucherFilter` | `:65-70` | `from`/`to` plain dates (the route-hint false positive, analysis §2). |
| `UnsettledTenderGroup` | `:78-85` | `date` `YYYY-MM-DD`; `account_role` = `ClearingRole` (`cardClearing`/`walletClearing`, camelCase); `total` money; `tender_count: i32` (not a decimal, analysis §2). |
| `CardSettlementInput`, `CardSettlementGroupRef` | `:87-94` | request-only. |
| `CardSettlement`, `CardSettlementGroup` | `:96-106` | `groups` by `position` (the frozen snapshot rows, analysis §2); `fee_amount` may be negative. |
| `FeeEstimate` | `Promise<number>` (`voucherService.ts:176`) | newtype `pub struct FeeEstimate(#[serde(with = serde_number)] #[ts(type = "number")] pub Decimal)` with `#[serde(transparent)]` → TS `number`; no floats (master rule 5). |

`contract.check.ts` (new `src/modules/vouchers/types/contract.check.ts`): `Voucher` (if the tagged-enum
binding renders as a union of intersections that `Equals` rejects, check each variant against its TS
interface and add `// contract-ok: tagged union checked per variant`), the four inputs, `VoucherFilter`,
`UnsettledTenderGroup`, `CardSettlementInput`, `CardSettlement`.

## 3. Service logic (`service/{general,settlements,read}.rs`)

### 3.1 Common pieces

- **`method_role(payment_method_id)`** (`vouchers.ts:21-25`): live **active** payment method by id, else
  `VALIDATION` `اختر طريقة الدفع`; returns `SystemRole::from_str(account_role)` (PG-4) + name.
- **`manual_account(id, verb)`**: `shared::ledger::accounts::account_by_id` (`NOT_FOUND`
  `الحساب غير موجود في شجرة الحسابات`); `!allow_manual` → ``لا يمكن الترحيل يدوياً {إلى|من} حساب "{name}"``.
- **`insert_voucher(conn, cx, kind, input fields, number)`**: `number` = the given one or
  `next_number(DocumentKind::Voucher)` (`VCH-…`), after `assert_open_period(date.day)` (lock order PG-7);
  `amount` stored `round2` (D-V3); `created_by = actor`; kind columns per variant; the entity's
  `before_save` fills `search_normalized` from `[number, description, note]` (P2-38, already in code).
- **`post_voucher(…)`**: `ledger::post(PostJournal { date: input.date, description: "{KIND_LABEL} {number}
  — {suffix}", SYSTEM, source { "voucher", id, number }, lines, attachment_ids: input.attachment_ids })`;
  `KIND_LABEL` = `سند قبض عام`/`سند صرف عام`/`تحويل بين الحسابات`/`سند مالك` (`vouchers.ts:27-29`).
- **`log_voucher`**: `log(Voucher, "{KIND_LABEL} {number} بقيمة {amount:.2}", date,
  RouteRef::detail("voucher-detail", id))`. `ledger::post` touches `ledger` (the mock's `emit('ledger:changed')`).

### 3.2 Receipt / payment / owner (`vouchers.ts:32-85`, `:120-145`)

- **Receipt**: `!(amount > 0)` → `المبلغ يجب أن يكون أكبر من صفر`; `method_role`; `manual_account(credit_account_id,
  "إلى")`; `insert_voucher`; lines `Role(method)` Dr `amount` · `Id(credit)` Cr `amount` with
  `cost_center_id`; suffix = `description`; `log_voucher`.
- **Payment**: same order with `manual_account(debit_account_id, "من")`; lines `Id(debit)` Dr `amount` with
  `cost_center_id` · `Role(method)` Cr `amount`.
- **Owner**: amount check; `account_by_id(cash_account_id)` (`NOT_FOUND` as above); `resolve_account(Drawings,
  default)` (`NOT_FOUND` ``لا يوجد حساب في شجرة الحسابات لدور "المسحوبات الشخصية" — أضف حساباً بهذا الدور أولاً``);
  insert; lines `drawings` → `Id(drawings)` Dr / `Id(cash)` Cr, `contribution` → reverse; suffix
  `مسحوبات شخصية` / `إضافة رأس مال`.

### 3.3 `record_transfer_voucher(conn, cx, reg, input, number)` (`vouchers.ts:88-117`)

1. `!(amount > 0)` → `المبلغ يجب أن يكون أكبر من صفر`.
2. `source_account_id == destination_account_id` (compared before any lookup) → `اختر حسابين مختلفين للتحويل`.
3. `account_by_id(source)`, then `account_by_id(destination)` (`NOT_FOUND`; no `allow_manual` check —
   analysis §2).
4. `fee = round2(fee_amount ?? 0)`; `fee > 0 && fee_account_id None` → `اختر حساب العمولة`.
5. `insert_voucher(…, fee_amount = fee > 0 ? Some(fee) : None, fee_account_id as sent, number)` — when
   `number` is `Some` (the shift close's pre-allocated number, 08b §3.4 step 4) it is used as is; otherwise
   it is allocated here.
6. Lines: `Id(destination)` Dr `amount` · `Id(source)` Cr `round2(amount + fee)` · (`fee > 0`)
   `Id(fee_account)` Dr `fee` (`account_by_id` inside `post` → `NOT_FOUND` if missing, as the mock's
   `resolvePosting`); suffix = `description`; `log_voucher`. Return the DTO.
`vouchers_create_transfer_voucher` calls it with `number = None`.

### 3.4 Reads

- **`get_vouchers`** (`voucherService.ts:155-162`): `kind`, `date_day` between `from`/`to`, and
  `search` → `search_normalized LIKE like_contains(normalize_arabic(q))` (a real SQL filter — the haystack
  is the row's own fields, P2-38); empty normalized query → no search filter; order `date_key DESC,
  created_at, id`.
- **`get_voucher`**: `NOT_FOUND` `السند غير موجود` (`vouchers.ts:147-151`).
- **`get_card_settlements`**: all, `date_key DESC, created_at, id`, groups by `position`.
- **`get_card_settlement`**: `NOT_FOUND` `سند التسوية غير موجود` (`settlements.ts:113-117`).

### 3.5 `unsettled_tender_groups(conn)` (`settlements.ts:16-55`)

One grouped query:

```sql
SELECT i.date_day, t.payment_method_id, SUM(t.amount) AS total, COUNT(*) AS tender_count
FROM invoice_tenders t
JOIN invoices i        ON i.id = t.invoice_id AND i.status = 'COMPLETED' AND i.deleted_at IS NULL
JOIN payment_methods m ON m.id = t.payment_method_id AND m.deleted_at IS NULL
                      AND m.account_role IN ('cardClearing', 'walletClearing')
WHERE NOT EXISTS (SELECT 1 FROM card_settlement_groups g
                  WHERE g.date_day = i.date_day AND g.payment_method_id = t.payment_method_id)
GROUP BY i.date_day, t.payment_method_id
```

(`date_day` is the business day of the sale instant = the mock's `localDateKey(inv.date)`.) Map to
`{ date, payment_method_id, payment_method_name: m.name, account_role, total: round2(total), tender_count }`;
sort `date DESC`, then `payment_method_name` ascending (`settlements.ts:54`, D-V4). No clearing methods at
all → `[]`.

### 3.6 `create_card_settlement(conn, cx, reg, input)` (`settlements.ts:58-111`)

1. `groups` empty → `VALIDATION` `اختر يوماً واحداً على الأقل للتسوية`.
2. `!(deposit_amount >= 0)` → `أدخل مبلغ الإيداع البنكي`.
3. `available = unsettled_tender_groups()`; for each input group in order: not in `available` →
   `CONFLICT` `أحد العناصر المختارة غير متاح للتسوية (ربما تمت تسويته بالفعل)`; collect `{ date, method,
   amount: found.total, role: found.account_role }`.
4. `gross = round2(Σ amount)`; `fee = round2(gross − deposit_amount)`; `fee < −0.005` →
   `مبلغ الإيداع أكبر من إجمالي العمليات المختارة`.
5. `assert_open_period(date.day)`; `number = next_number(CardSettlement)` (`STL-…`).
6. Insert `card_settlements { number, date, gross_amount, deposit_amount: round2(deposit), fee_amount: fee,
   note, created_by }` + `card_settlement_groups` rows (`position = i`, `date_day`, `payment_method_id`,
   `amount`). A `TxError::Db` with `duplicate_key_name == "uq_card_settlement_groups_date_method"` (PG-3)
   → the step-3 `CONFLICT` message (the race loser, analysis §5 recommendation).
7. Lines (:91-94): `Role(Bank)` Dr `deposit_amount` (rounded) · (`fee > 0`) `Role(CardFees)` Dr `fee` ·
   one `Role(CardClearing|WalletClearing)` Cr per role, `round2` sums, in first-appearance order · (`fee <
   0`) `Role(Bank)` Dr `−fee`.
8. `ledger::post` (date `input.date`, `تسوية بطاقات/محافظ {number} — {selected.len()} مجموعة`, SYSTEM,
   source `{ "settlement", id, number }`).
9. `log(Payment, "تسوية بطاقات {number} — إيداع {deposit:.2} وعمولة {fee:.2}", date,
   RouteRef::list("card-settlements"))` (`:108`). Return the DTO.

### 3.7 `estimate_settlement_fee(conn, groups)` (`settlements.ts:120-127`)

`round2(Σ group.total × method.fee_pct / 100)` with `fee_pct` of the live payment method (0 when not
found), summed exactly then rounded once. Pure estimate — never the posted `fee_amount` (analysis §7).

## 4. Concurrency (D8, analysis §5)

| Race | Settled by |
|---|---|
| Voucher / settlement numbers | `next_number(Voucher|CardSettlement)` + `uq_vouchers_number` / `uq_card_settlements_number`. |
| Two terminals settle an overlapping (day, method) | `uq_card_settlement_groups_date_method(date_day, payment_method_id)`: the loser blocks on the winner's index entry, then gets errno 1062 → the mock's `CONFLICT` (PG-3). No row lock needed. |
| A sale tendered on a day being settled | the settlement freezes its `groups[].amount`; the later tender's (day, method) key is then "settled" (Q-V2). |
| Shift close drop vs. a manual transfer voucher | counters in voucher → journal order in both paths (08b §3.4 step 4). |

Lock order (`core/lock.rs`): settings (S) → fiscal year (S) → voucher/settlement counter → journal counter.

## 5. Undo

None registered (phase-e E-5 lists no vouchers action). Vouchers and settlements have no reversal in the
mock; corrections are manual journal entries (analysis §4). Settlement reversal stays a later design
question (analysis §9, recommendation (b) = refuse and correct manually) — nothing to port now.

## 6. Frontend switch lines (`src/modules/vouchers/services/voucherService.ts`)

Add `import { backendCall, usesRust } from '@/modules/core/services/backend';` and, first inside each
`wrap`: `if (usesRust('vouchers')) return backendCall('<cmd>', { … });` for `createReceiptVoucher`
`{ input }`, `createPaymentVoucher` `{ input }`, `createTransferVoucher` `{ input }`, `createOwnerVoucher`
`{ input }`, `getVouchers` `{ filter }`, `getVoucher` `{ id }`, `estimateSettlementFee` `{ groups }`,
`createCardSettlement` `{ input }`, `getCardSettlement` `{ id }`; and the no-args form
`if (usesRust('vouchers')) return backendCall('<cmd>');` for `getUnsettledTenderGroups` and
`getCardSettlements`.

## 7. Known mock quirks (kept) and decisions

**Quirks kept:**
- Q-V1 A voucher stores its input as given (`...input`, `vouchers.ts:38`): a transfer keeps `feeAccountId`
  even when the fee rounds to 0; transfer/owner vouchers keep `costCenterId` but don't use it on any line.
- Q-V2 Invoices that became `REFUNDED` drop out of the unsettled list (`settlements.ts:31`) although a card
  refund credits `bank`, not the clearing account; and a tender added to an already-settled (day, method) is
  never offered again. Both can leave clearing ≠ unsettled tenders (invariant 10) — flagged for a later
  mock+Rust fix, not changed here.
- Q-V3 The settlement's bank line uses the rounded deposit while `fee` uses the raw input deposit
  (`settlements.ts:70,79`).
- Q-V4 `estimateSettlementFee` trusts the client-sent group totals (it's a display estimate only).

**Decisions:**
- D-V1 Recommended wave W3 (08 PG-8).
- D-V2 The (day, method) uniqueness is enforced by the DB constraint (analysis §5's recommended option);
  duplicate (day, method) pairs **within one request** also fail with the `CONFLICT` message (the mock
  would double-count them — malformed input only).
- D-V3 Voucher `amount` stored `round2`ed (DECIMAL(19,2)); postings round in `ledger::post` anyway.
- D-V4 Unsettled groups sort by `date DESC, name` with Rust `str` ordering; JS `localeCompare` (ICU root)
  agrees for the Latin-vs-Arabic and plain Arabic names in use; parity cases use distinct dates.
- D-V5 `record_transfer_voucher` takes an optional pre-allocated number (for the shift close); everything
  else about it is the mock's function.

## 8. Tests

**(a) `src-tauri/tests/domain_vouchers.rs`** (posting tests end with `shared::invariants::run_all`,
invariant 10 in the settlement ones):
- receipt: Dr cash-role account / Cr the picked manual account with its cost center; `VCH-000001`; activity
  row links `voucher-detail`; `ledger` bumped; attachments reach the journal entry.
- payment, transfer with fee (source credited `amount + fee`), transfer without fee (`fee_amount` NULL),
  owner drawings and contribution.
- every §3 message byte-exact: amount ≤ 0, inactive/unknown method, unknown account, non-manual account
  (both wordings), same transfer accounts, fee without account, missing drawings role.
- `get_vouchers`: kind, date range, Arabic search on description (`أحمد`/`احمد`), order.
- unsettled groups: mada + STC Pay tenders across two days → 4 groups with totals/counts; cash tenders and
  REFUNDED invoices excluded; after a settlement its groups disappear.
- settlement: fee > 0 (Dr bank + Dr cardFees / Cr cardClearing), deposit > gross by 0.003 (allowed), by 1
  (message), mixed card + wallet (two credit lines), `CONFLICT` for an already-settled group; two
  connections settling the same group → exactly one settlement, the other `CONFLICT` (constraint path).
- `record_transfer_voucher` with a pre-allocated number uses it and allocates nothing.
- `estimate_settlement_fee` = `round2(Σ total × feePct / 100)`.
- period locked → `FORBIDDEN`, nothing written.

**(b) Parity cases for Part 04:** `voucher-receipt`, `voucher-payment`, `voucher-transfer-fee`,
`voucher-owner-both-directions`, `voucher-validation-each`, `voucher-list-search`,
`settlement-card-with-fee`, `settlement-mixed-card-wallet`, `settlement-already-settled-conflict`.

## 9. Checklist (implementation order)

- [x] PG-3, PG-4 in place (confirmed: `uq_card_settlement_groups_date_method` in `migration/src/m0010_payments.rs`,
      `SystemRole::from_str` in `shared/ledger/accounts.rs`); wave decided (D-V1) (manager).
- [x] `domains/vouchers/{mod,dto,commands}.rs`, `service/{mod,general,settlements,read}.rs`.
- [x] DTOs (§2) incl. the tagged `Voucher` enum and `FeeEstimate`.
- [x] `service/general.rs`: §3.1 helpers, `record_transfer_voucher` first (08b depends on it), then receipt,
      payment, owner.
- [x] `service/settlements.rs`: `unsettled_tender_groups`, `create_card_settlement`, `estimate_settlement_fee`.
- [x] `service/read.rs`: §3.4.
- [x] `commands.rs`: 11 commands + `ipc_sig!` lines; **manager still needs to** register them and add
      `pub mod vouchers;` + the `export_bindings`/`all_ipc_signatures` hook lines in `domains/mod.rs`.
- [x] Switch lines (§6); `src/modules/vouchers/types/contract.check.ts` (§2).
- [x] `tests/domain_vouchers.rs` (§8a, 22 tests) — ⏳ deferred time-boxed test pass (not run by this
      agent); parity list to Part 04 unchanged from §8b (not yet built as `scripts/verify/cases/*.json`
      bundles — that's the deferred pass too).
- [x] Status note at the top of this file.

## Gate

`cargo check` clean (manager's throttled run) · tests written · 11 switch lines · `contract.check.ts`
compiles · `memory:check` 0 contract gaps for the 11 commands. DB tests and parity cases run in the deferred,
time-boxed test pass.
