# 21 · 03.06b — `products` (inventory: adjustments, movements, batches/expiry, stock counts, branch transfers)

> **Status:** code complete 2026-09-28 (§9 checklist done — DTOs, service, commands, frontend switch
> lines incl. `branchStockQty`'s cache branch, `contract.check.ts` lines, DB tests written).
> `record_stock_adjustment` widened from the plan's `pub(crate)` to `pub` (07-purchases' planned
> cross-domain call site still compiles unchanged) so `tests/domain_products.rs`, an external
> integration-test crate, can drive the STOCK_IN/LOSS/STOCKTAKE/draft cases directly — this codebase's
> tests exercise the service layer, never the Tauri `State` layer, so a `pub(crate)` entry point used
> only by another domain would otherwise be untestable from `tests/*.rs`. **Blocked on the manager**
> before this compiles — see 06's status note (same blockers: `domains/mod.rs`/`lib.rs` hooks,
> `AppState.approval_grants`, `SequenceLock::ProductCodes`, migration m0016). A10 (the
> `scripts/contract/config.ts` overrides) is also a manager task, not yet done. Wave **W2** (entry file
> §4). Depends on: 06-products (same `domains/products/` module, same implementer — this file is its
> second half, see 06's split note), 01-settings, 03-users, Part 02 `shared::stock` / `shared::ledger` /
> `shared::numbering` / `shared::activity`, and manager tasks G-P2, G-P3, G-P4, G-P6, G-P8, G-P9, G-P10
> (listed in 06 §7).

**Goal.** Port the 25 inventory `port` functions of `inventoryService.ts` and `transferService.ts` so each
returns the mock's exact DTO and posts the exact journal lines, with every quantity/value/average-cost
change going through `shared::stock` (`lock_products`, `apply_change`, `receive_batch`, `consume_fefo`,
`cost_at_average`) and every journal through `shared::ledger::post`. One command = one transaction, so
the mock's partial-write failure modes disappear (D-I3).

**Read first.** [`../01-frontend-analysis/products.md`](../01-frontend-analysis/products.md) §1 (rows 47–73),
§2 (rows 98–115), §3 (rows 152–187), §4 (rows 202–212), §5 (rows 223–226), §6, §7 (rows 245–248), §8, §9 ·
mock: `src/modules/products/services/inventoryService.ts:35-270`, `transferService.ts:12-38`,
`src/mocks/backend/inventory.ts:18-446`, `src/mocks/backend/transfers.ts:17-236`,
`src/mocks/backend/core.ts:251-297` (`productById`, `applyStockChange`) · types:
`src/modules/products/types/index.ts:187-374` · Rust: `src-tauri/src/shared/stock/{mod,cost,batches}.rs`,
`shared/ledger/{post,accounts,period}.rs`, `shared/numbering.rs` (`DocumentKind::{Adjustment,StockCount,
DebitNoteDraft,StockTransfer}`), `entities/inventory/*.rs`, `entities/catalog/product_batches.rs`,
`core/lock.rs`, `utils/money.rs` (`round2`, `round4`, `js_number_string`), `utils/dates.rs`
(`RawDocDate`, `DocDate`, `format_iso_ms`).

## 1. Commands

Area `inventory` for every command (`products/routes/index.ts:12-23`; `StorekeeperHome.vue` reads the
expiry report — storekeeper has Inventory/Write). Reads: `with_read` + `settings::require(tx, actor,
Inventory, Read)` (06 §1 pattern). Writes: `with_tx` + `cx.require(tx, Inventory, Write)`. Posting
commands pass `state.undo.clone()` into the closure (the registry `shared::activity::log` needs).

| Mock fn (file:line) | Disp. | Rust command | Args → Return | Access | Tx | Events |
|---|---|---|---|---|---|---|
| `adjustmentValue` (`inventoryService.ts:43`) | frontend | — | untouched | — | — | — |
| `getStockAdjustments` (`:47`) | port | `products_get_stock_adjustments` | `{ filter: Option<AdjustmentFilter> }` → `Vec<StockAdjustment>` | Read | read | — |
| `getStockAdjustment` (`:61`) | port | `products_get_stock_adjustment` | `{ id }` → `StockAdjustmentDetail` | Read | read | — |
| `createStockAdjustment` (`:69`) | port | `products_create_stock_adjustment` | `{ input: StockAdjustmentInput, asDraft: Option<bool> }` → `StockAdjustment` | Write | tx | posted: Ledger + Catalog; draft: none |
| `completeAdjustment` (`:74`) | port | `products_complete_adjustment` | `{ id }` → `StockAdjustment` | Write | tx | Ledger + Catalog |
| `deleteDraftAdjustment` (`:79`) | port | `products_delete_draft_adjustment` | `{ id }` → `()` | Write | tx | — |
| `getStockMovements` (`:96`) | port | `products_get_stock_movements` | `{ filter: Option<MovementFilter> }` → `Vec<StockMovementRow>` | Read | read | — |
| `getStockMovementsPaged` (`:112`) | port | `products_get_stock_movements_paged` | `{ query: PagedQuery<MovementFilter> }` → `PagedResult<StockMovementRow>` | Read | read | — |
| `getBatches` (`:164`) | port | `products_get_batches` | `{ productId }` → `Vec<ProductBatch>` | Read | read | — |
| `getExpiryReport` (`:190`) | port | `products_get_expiry_report` | `()` → `Vec<ExpiryRow>` | Read | read | — |
| `batchAlertTone` (`:205`) | **frontend** (was port — D-I1) | — | untouched | — | — | — |
| `writeOffExpiredBatches` (`:211`) | port | `products_write_off_expired_batches` | `{ batchIds: Vec<Id>, note: Option<String> }` → `StockAdjustment` | Write | tx | Ledger + Catalog |
| `returnBatchesToSupplier` (`:217`) | port | `products_return_batches_to_supplier` | `{ supplierId, lines: Vec<DebitNoteDraftLine>, note? }` → `DebitNoteDraft` | Write | tx | — |
| `getDebitNoteDrafts` (`:226`) | port | `products_get_debit_note_drafts` | `()` → `Vec<DebitNoteDraft>` | Read | read | — |
| `getStockCounts` (`:235`) | port | `products_get_stock_counts` | `()` → `Vec<StockCount>` | Read | read | — |
| `getStockCount` (`:240`) | port | `products_get_stock_count` | `{ id }` → `StockCount` | Read | read | — |
| `createStockCount` (`:247`) | port | `products_create_stock_count` | `{ input: StockCountInput }` → `StockCount` | Write | tx | — |
| `updateStockCountLine` (`:252`) | port | `products_update_stock_count_line` | `{ countId, productId, qty: Decimal, delta: Option<bool> }` → `StockCount` | Write | tx | — |
| `submitCountForReview` (`:257`) | port | `products_submit_count_for_review` | `{ countId }` → `StockCount` | Write | tx | — |
| `resumeCounting` (`:262`) | port | `products_resume_counting` | `{ countId }` → `StockCount` | Write | tx | — |
| `completeStockCount` (`:267`) | port | `products_complete_stock_count` | `{ countId }` → `StockAdjustment` | Write | tx | Ledger + Catalog |
| `getTransfers` (`transferService.ts:12`) | port | `products_get_transfers` | `()` → `Vec<StockTransfer>` | Read | read | — |
| `getTransfer` (`:16`) | port | `products_get_transfer` | `{ id }` → `StockTransfer` | Read | read | — |
| `createTransfer` (`:20`) | port | `products_create_transfer` | `{ input: StockTransferInput }` → `StockTransfer` | Write | tx | — |
| `sendTransfer` (`:24`) | port | `products_send_transfer` | `{ id }` → `StockTransfer` | Write | tx | Ledger + Catalog |
| `receiveTransfer` (`:28`) | port | `products_receive_transfer` | `{ id, input: ReceiveTransferInput }` → `StockTransfer` | Write | tx | Ledger + Catalog |
| `rejectTransfer` (`:32`) | port | `products_reject_transfer` | `{ id, reason }` → `StockTransfer` | Write | tx | Ledger + Catalog |
| `branchStockQty` (`:36`) | **frontend** (was port — D-I1) | — | Rust-mode body reads 06's `rememberBranchStock` cache | — | — | — |

25 commands (Args struct names: `Products<FnPascal>Args`). Events come from the shared primitives
(P2-12: `ledger::post` touches Ledger, `apply_change` touches Catalog); the mock emits only
`catalog:changed` (adjustments/counts) or only `ledger:changed` (transfers) — Rust emitting both closes
analysis §8/§9's transfer-event question structurally (C-07, D-I5).

## 2. DTOs (`domains/products/dto/inventory.rs`, `#[ts(export_to = "products/types/gen/")]`)

Conventions as 06 §2. Every document `date`-like field is a `DocDate` serialized as its key string
(`#[ts(type = "string")]`); `DateTime<Utc>` instants (`startedAt`, `approvedAt`) serialize with
`utils::dates::format_iso_ms` (the mock's `toISOString()`).

| Rust DTO | TS type (file:line) | Notes |
|---|---|---|
| `StockAdjustmentType` / `StockInReason` | `types/index.ts:187,198` | `SCREAMING_SNAKE_CASE` / `snake_case`. |
| `StockAdjustmentLine` | `:200-210` | `system_qty`/`counted_qty`/`unit_cost: Option<Decimal>`, `qty_change: Decimal`, `batch_no`, `expiry_date: Option<NaiveDate>` `#[ts(type="string")]`. |
| `StockAdjustment` | `:212-227` | `status` enum `DRAFT`/`COMPLETED`; `approved_by: Option<Id>`; `approved_at: Option<String>` (ISO). |
| `StockAdjustmentDetail` | `StockAdjustment & { journalEntryId?: string }` (`inventoryService.ts:61`) | `#[serde(flatten)]` + `journal_entry_id: Option<Id>`. |
| `StockAdjustmentInput` / `StockAdjustmentLineInput` | `:229-237` | `date: String` (resolved with `RawDocDate::parse(..).resolve(&cx.clock)`; parse failure → `VALIDATION` `تاريخ غير صالح`); line `qty_change`/`counted_qty: Option<Decimal>`. |
| `AdjustmentFilter` / `MovementFilter` | `inventoryService.ts:35-40,88-93` | `status` as the adjustment status enum; `from`/`to: Option<String>` (`YYYY-MM-DD`). |
| `StockMovementReason` | `:239-242` | `snake_case` DTO enum; mapped from/to the `&'static str` `shared::stock` stores. |
| `StockMovement` / `StockMovementRow` | `:244-259`; `inventoryService.ts:109` | Row = `#[serde(flatten)] StockMovement` + `product_name: String` + `ref_link: Option<RouteRef>` `#[ts(optional, type = "import('../../core/types/route').AppRoute")]`. |
| `PagedQuery<MovementFilter>` / `PagedResult<StockMovementRow>` | `core/types/paging.ts` | reuse `crate::core::dto` generics. |
| `ProductBatch` | `:38-51` | `received_date: String` = the batch's `DocDate` key (G-P4d); `supplier_id`/`source_ref_*` optional. Shared with 07 (`getActiveBatches`). |
| `ExpiryBucket` / `ExpiryRow` | `inventoryService.ts:169-177` | Row = flatten `ProductBatch` + `product_name`, `product_sku`, `supplier_name?`, `bucket`, `days_left: Option<i64>` with `#[serialize_always]` (TS `number \| null`, never absent). |
| `DebitNoteDraft` / `DebitNoteDraftLine` | `:312-320` | status one-variant enum `DRAFT`; line `{ product_id, batch_id: Id, qty, unit_cost: Decimal }`. Shared with 07. |
| `StockCountScope` / `StockCountStatus` / `StockCountLine` / `StockCount` / `StockCountInput` | `:265-298` | `started_at: String` (ISO); `started_by: Id`. |
| `StockTransferStatus` / `StockTransferLine` / `StockTransfer` / `StockTransferInput` / `ReceiveTransferInput` | `:328-374` | `unit_factor` `Decimal` (6 dp column); `sent_at`/`received_at`/`rejected_at: Option<String>` DocDate keys; `shortage_value: Option<Decimal>`. Input line struct `StockTransferLineInput`; receive line struct `ReceiveTransferLineInput`. |

`contract.check.ts` (append to 06's file): `StockAdjustmentType`, `StockInReason`, `StockAdjustmentLine`,
`StockAdjustment`, `StockAdjustmentInput`, `StockMovementReason`, `StockMovement`, `ProductBatch`,
`DebitNoteDraft`, `StockCountScope`, `StockCountStatus`, `StockCountLine`, `StockCount`, `StockCountInput`,
`StockTransferStatus`, `StockTransferLine`, `StockTransfer`, `StockTransferInput`, `ReceiveTransferInput`,
and from `../services/inventoryService`: `AdjustmentFilter`, `MovementFilter`, `ExpiryBucket`,
`Flat<ExpiryRow>`, `Flat<StockMovementRow>`, plus `Flat<StockAdjustment & { journalEntryId?: string }>`.

## 3. Service logic (`domains/products/service/{stock_lines,adjustments,movements,batches,counts,transfers}.rs`)

Helpers: `TYPE_LABEL` (`inventory.ts:18`: `إدخال مخزون` / `إتلاف/فقد` / `جرد`) and `STOCK_IN_REASON_LABEL`
(`:20-26`) copied verbatim; `pub(crate) fn to_fixed2(d) = format!("{:.2}", round2(d))` (JS `toFixed(2)`; 07 reuses it); numbers inside
messages use `js_number_string` (JS `String(n)`); `now_date = DocDate { day: cx.clock.today(), instant:
Some(cx.clock.now) }` wherever the mock writes `new Date().toISOString()`.

**S-1 `lock_line_products(conn, ids)`** (`stock_lines.rs`, `pub(crate)`, reused by 07): plain
`SELECT id FROM products WHERE id IN (…)`, then `shared::stock::lock_products(conn, &existing)` (sorted
`FOR UPDATE`). A missing id is simply absent, so callers raise `NOT_FOUND` `المنتج غير موجود` **at that
line's position**, keeping the mock's per-line error order (`productById`, `core.ts:251-255`) while every
check below reads locked rows.

**S-2 `build_lines(ty, lines, locked, snapshot)`** — `buildLines` (`inventory.ts:30-66`), same order:
empty → `أضف صنفاً واحداً على الأقل`; per line: missing product → `NOT_FOUND`; untracked
(`LockedProduct::is_untracked`) → `` "${name}" لا يُتتبع مخزونها ``; dedupe key = `id::batchNo` for a
STOCK_IN of a `track_batches` product else `id`, repeat → `` "${name}"${batchNo ? ` (تشغيلة ${batchNo})` : ''} مكرر في القائمة ``;
`system_qty = snapshot[id] ?? stock_qty`. STOCKTAKE: `counted` missing or `< 0` →
`` أدخل الكمية المعدودة لـ "${name}" ``; `qty_change = round2(counted − system_qty)`, `unit_cost = cost_price`.
Otherwise `qty = qty_change`; missing or `<= 0` → `` أدخل كمية صحيحة لـ "${name}" ``; LOSS and `qty >
stock_qty` → `` كمية الإتلاف لـ "${name}" أكبر من المتوفر (${stock_qty}) ``; STOCK_IN of a tracked product
with blank `batch_no` → `` أدخل رقم التشغيلة لـ "${name}" — هذا الصنف يتتبع التشغيلات وتاريخ الصلاحية ``;
line = `{ system_qty, qty_change: LOSS ? −qty : qty, unit_cost: cost_price, batch_no: trimmed-or-None,
expiry_date }`.

**S-3 `validate_stock_in(input)`** (`:135-145`): only STOCK_IN; no reason → `اختر سبب إدخال المخزون`;
`other` without `offset_account_id` → `اختر الحساب المقابل لسبب "أخرى"`; `accounts::account_by_id`
(`NOT_FOUND` `الحساب غير موجود في شجرة الحسابات`); system role `receivable`/`payable`/`inventory` or
`!allow_manual` → `` لا يمكن اختيار "${name}" كحساب مقابل — اختر حساباً غير رئيسي ``.

**S-4 `assert_approval(ty, lines, approved_by, approval)`** (`:218-226` + D-I2): STOCKTAKE → ok;
`threshold = settings.inventory_approval_threshold`, `None`/`<= 0` → ok; `value = round2(Σ |qty_change| ×
unit_cost)`; `value >= threshold` and (`approved_by` absent **or** not granted by G-P3 **or** the user is not
an active `admin`/`manager`) → `FORBIDDEN` `` قيمة هذه الحركة (${to_fixed2(value)}) تتجاوز حد الاعتماد (${to_fixed2(threshold)}) — يلزم تأكيد المدير ``.
`ApprovalCheck { granted: bool }` is computed in the command before `with_tx` from
`state.approval_grants.is_granted(approved_by)`.

**S-5 `post_adjustment(conn, cx, adj, lines, locked)`** (`:153-209`): reason string `stock_in` / `loss` /
`stocktake`; per line in order: `value = round2(|qty_change| × unit_cost)`; `gains`/`losses` accumulate
`value`; `apply_change(p, qty_change, ±value, reason, StockRef { adj.id, adj.number }, &adj.date, None)`
(default branch — the mock passes none); tracked batches: STOCK_IN with `batch_no` →
`batches::receive_batch(product, qty_change, unit_cost, batch_no, expiry, &adj.date, &ref)`; LOSS →
`batches::consume_fefo(product, |qty_change|, allow_expired = true, cx.clock.today())`. Then
`gains = round2(gains)`, `losses = round2(losses)` (the second rounding pass, analysis §7 row 247); both
`<= 0` → no journal. Lines: STOCK_IN `Dr Role(Inventory) gains` / `Cr` by reason — `opening →
OpeningBalanceEquity`, `owner_contribution → OwnerCurrent`, `gift → OtherIncome`, `found →
InventoryVariance`, `other → AccountRef::Id(offset_account_id)`; LOSS `Dr InventoryWriteOff losses / Cr
Inventory losses`; STOCKTAKE `Dr Inventory / Cr InventoryVariance` (gains > 0) then `Dr InventoryVariance /
Cr Inventory` (losses > 0). `ledger::post(PostJournal { date: adj.date, description:
"{TYPE_LABEL} {number}{note ? ' — ' + note : ''}", entry_type: System, source: SourceRef { kind:
"stockAdjustment", id: adj.id, number }, … })`.

**C-A1 `record_stock_adjustment(conn, cx, reg, input, as_draft, approval)`** (`pub(crate)`, `:228-250`;
also called by 06 C-7 and B-3): 1 S-3 · 2 `lock_line_products` · 3 S-2 (no snapshot) · 4 `!as_draft` → S-4 ·
5 `next_number(Adjustment)` · 6 insert `stock_adjustments` (status `DRAFT`/`COMPLETED`, `approved_at =
approved_by.map(now)`) + lines (`position` = index) · 7 `!as_draft` → S-5 · 8 `activity::log(Stock,
"{TYPE_LABEL}{reason ? ' — ' + REASON_LABEL : ''} {number}{as_draft ? ' (مسودة)' : ''} — {n} صنف",
Some(adj.date), detail("adjustment", id))` · 9 `!as_draft` → `touch(Catalog)` · return DTO.

**C-A2 `complete_adjustment(id)`** (`:259-282`): `lock::for_update_by_id("stock_adjustments", id)`; missing
→ `NOT_FOUND` `التسوية غير موجودة`; not DRAFT → `التسوية مكتملة بالفعل`; snapshot = `{product_id →
system_qty ?? 0}`; lock products; S-2 with the snapshot over `{ productId, countedQty, qtyChange:
|qty_change|, batchNo, expiryDate }` (S-3 and S-4 are **not** re-run — Q-I1); replace the lines; `date =
now_date`, status COMPLETED; S-5; `log(Stock, "اعتماد {TYPE_LABEL} {number}", adj.date, detail("adjustment"))`;
`touch(Catalog)`.

**C-A3 `delete_draft_adjustment(id)`** (`inventoryService.ts:79-86`): lock row; missing → `التسوية غير
موجودة`; not DRAFT → `لا يمكن حذف تسوية معتمدة — أنشئ تسوية عكسية بدلاً من ذلك`; hard-delete lines + row
(a draft is a worklist row, P2-16; `stock_adjustments` is not in the posted-document list);
`log(Stock, "حذف مسودة تسوية {number}", now_date, detail("adjustment", id))`. No event.

**C-A4 reads** (`:47-67`): `get_stock_adjustments` — SQL filters `type`, `status`, `date_day BETWEEN`
(`inDateRange` on the local day = `date_day`); load lines; stable sort by `DocDate::key()` descending
(`b.date.localeCompare(a.date)`, ties keep `created_at, id`). `get_stock_adjustment` — missing → `التسوية غير
موجودة`; `journal_entry_id` = first `journal_entries` (`created_at, id`) with `source_kind =
'stockAdjustment' AND source_id = id`.

**C-M1 `get_stock_movements(filter)`** (`:96-107`): filter `product_id`, `reason`, `date_day` range;
`LEFT JOIN products` for `product_name` (`'—'` when missing); `ref_link` (`:141-158`): `sale → detail
("invoice", ref_id)`; `refund →` the refund's `invoice_id` (absent when the refund row is missing);
`purchase → detail("purchase", ref_id)`; `purchase_return →` its `purchase_order_id`; any other reason →
`detail("adjustment", ref_id)` (Q-I4). Stable sort by date key descending.

**C-M2 `get_stock_movements_paged(query)`** (`:112-139`): same `WHERE`; `total = COUNT(*)`; order: no
`sort` → `date_key DESC, created_at, id`; `sort.key` mapped by `match` (cross-cutting §6): `qtyChange`,
`valueChange`, `balanceAfter` numeric columns; `date → date_key`, `productName → products.name`, `reason`,
`refNumber → ref_number` string columns `COLLATE utf8mb4_unicode_ci`; unknown key → `created_at, id`
(every row compares equal in the mock, so insertion order). Tie-break always `created_at, id`. `LIMIT
pageSize OFFSET (page−1)·pageSize`.

**C-B1 `get_batches(product_id)`** (`:164-167`): `shared::stock::batches::active_batches` → DTO.
**C-B2 `get_expiry_report()`** (`:179-203`): `today = SELECT UTC_DATE()` (the mock uses the UTC
`toISOString().slice(0,10)`, not the business day — D-I8); batches with `qty > 0.0001 AND expiry_date IS
NOT NULL` (`created_at, id`); `days = (expiry − today).num_days()`; bucket `<0 expired`, `<=30`, `<=60`,
`<=90`, else `ok` (dropped); join product name (`'—'`) / sku (`''`) and supplier name (`parties.name`,
absent when no `supplier_id`); stable sort by `days_left` ascending.

**C-B3 `write_off_expired_batches(batch_ids, note)`** (`inventory.ts:397-421`): empty → `اختر تشغيلة واحدة
على الأقل`; read the batches (plain) → their product ids → `lock_line_products` → re-read those batches'
`qty` (now stable: batch writers hold the product lock); group by product in first-seen order, skipping
missing or `qty <= 0`, `qty = round2(acc + batch.qty)`; none → `لا توجد كميات متبقية في التشغيلات
المحددة`; then C-A1 with `{ type: LOSS, date: now_date, note: note ?? "إتلاف — بضاعة منتهية الصلاحية",
lines }`, `as_draft = false`, `ApprovalCheck::none()` (Q-I3: the LOSS consumes FEFO, not the chosen ids).

**C-B4 `return_batches_to_supplier(supplier_id, lines, note)`** (`:432-446`): empty → `اختر تشغيلة واحدة على
الأقل للإرجاع`; `next_number(DebitNoteDraft)`; insert `debit_note_drafts` (`date = now_date`, lines JSON as
sent); `log(Stock, "مسودة إرجاع للمورد {number} — بانتظار مرحلة المشتريات (Phase 8)", now_date,
list("products"))`. No event. The client's `unitCost` is stored but never used for posting (07 recomputes
cost from the PO line — Q-I7).
**C-B5 `get_debit_note_drafts()`** (`pub(crate)`, `:226-229`): all rows, `created_at, id` (07 reuses it).

**C-K1 reads** (`:235-245`): counts ordered `started_at DESC, created_at, id`; lines by `position`;
missing → `NOT_FOUND` `الجرد غير موجود`.
**C-K2 `create_stock_count(input)`** (`:293-327`): in-scope products (`created_at, id`): `type = 'product'
AND COALESCE(stock_mode,'tracked') <> 'none' AND active` plus `category_id <=> ?` (scope `category` — NULL-safe,
the mock's `===` matches uncategorised products when no id is sent, Q-I12) or `COALESCE(shelf_location,'') =
COALESCE(?, '')` (scope `location`); none → `لا توجد أصناف ضمن نطاق الجرد المحدد`; `next_number
(StockCount)`; `started_at = now`, `started_by = actor`; lines snapshot `stock_qty`/`cost_price` (A5, read
at RC without product locks — a snapshot, not a movement); `log(Stock, "بدء جرد {number}{blind ? ' (أعمى)'
: ''} — {n} صنف", now_date, detail("count", id))`. No event.
**C-K3 `update_stock_count_line(count_id, product_id, qty, delta)`** (`:330-340`): `for_update_by_id
("stock_counts")` (serialises scan increments from two devices); missing → `الجرد غير موجود`; not OPEN →
`لا يمكن تعديل جرد غير مفتوح`; no line → `الصنف خارج نطاق هذا الجرد`; `counted = round2(delta ?
(counted ?? 0) + qty : qty)`; return the full count. No log, no event.
**C-K4 `submit_count_for_review`** (`:343-350`): lock; not OPEN → `الجرد ليس في حالة مفتوحة`; any
`counted_qty IS NULL` → `أكمل عدّ جميع الأصناف قبل المتابعة للمراجعة`; → REVIEW.
**C-K5 `resume_counting`** (`:352-358`): lock; not REVIEW → `الجرد ليس قيد المراجعة`; → OPEN.
**C-K6 `complete_stock_count(count_id)`** (`:361-388`): lock; missing / not REVIEW → `يجب إرسال الجرد
للمراجعة أولاً`; snapshot = lines' `system_qty`; lock products; S-2 STOCKTAKE over `{ productId,
countedQty }`; `next_number(Adjustment)` (after S-2 — the mock burns a number first; a rollback makes that
invisible); insert a COMPLETED STOCKTAKE adjustment (`date = now_date`, note `"جرد {count.number}{count.note
? ' — ' + note : ''}"`); count → COMPLETED, `adjustment_id`; S-5; `log(Stock, "اعتماد نتيجة الجرد {number}",
adj.date, detail("adjustment", adj.id))`; `touch(Catalog)`; return the adjustment. No approval
(STOCKTAKE, `:219`).

**C-T1 reads** (`transfers.ts:21-29`): `created_at, id` order (the mock returns the array unsorted);
missing → `NOT_FOUND` `التحويل غير موجود`.
**C-T2 `create_transfer(input)`** (`:37-61`): same branch → `لا يمكن التحويل لنفس الفرع`; either branch
missing → `NOT_FOUND` `فرع غير موجود`; no lines → `أضف صنفاً واحداً على الأقل`; per line: product missing →
`NOT_FOUND`, untracked → `` "${name}" لا يُتتبع مخزونها ``, `qty <= 0` → `الكمية يجب أن تكون أكبر من صفر`;
`number = branch_prefix(from) + next_number(StockTransfer)` (G-P2; one shared counter — D-I7); insert DRAFT
+ lines as sent; `log(Stock, "إنشاء تحويل مخزون {number} من {from.name} إلى {to.name}", date,
list("transfers"))`. No event.
**C-T3 `send_transfer(id)`** (`:68-121`): `for_update_by_id("stock_transfers")`; missing → `التحويل غير
موجود`; not DRAFT → `لا يمكن إرسال تحويل تم إرساله بالفعل`; lock products; per line in order: `qty =
round2(qty × (unit_factor ?? 1))`; `available = shared::stock::branch_stock_qty(conn, pid, from)`; `qty >
available + 0.0001` → `CONFLICT` `` الكمية المطلوب تحويلها من "${name}" أكبر من المتوفر بالفرع (${available}) ``;
`value = cost_at_average(qty, cost_price)`; `transit = round2(transit + value)`. Then `sent_at = now_date`,
SENT, `sent_by`; each line's `unit_cost` = the **first** posting entry for its product: `qty > 0 ?
round4(value / qty) : 0`; `apply_change(p, −qty, −value, "transfer_out", ref, &sent_at, Some(from))` per
entry; post `{ date: sent_at, "إرسال تحويل مخزون {number} — {from.name} → {to.name}", System, source {
"stockAdjustment", transfer.id, number }, [Dr Role(InventoryInTransit) transit, Cr Role(Inventory) transit],
both branch_id = from }`; `log(Stock, "إرسال تحويل {number} بقيمة {to_fixed2(transit)}", sent_at,
list("transfers"))`.
**C-T4 `receive_transfer(id, input)`** (`:129-190`): lock; missing; not SENT → `لا يمكن استلام تحويل لم
يُرسل بعد`; `by_product` = input lines keyed by product (last wins); lock products; per line: `sent =
baseQty`, `uc = unit_cost ?? 0`, `transit += round2(sent × uc)`; `received = min(input ?? sent, sent)`; `<
0` → `الكمية المستلمة لا يمكن أن تكون سالبة`; store `received_qty`; `received_value += round2(received ×
uc)`; `received > 0` → destination entry; `received < sent − 0.0001` → `shortage = round2(shortage +
round2((sent − received) × uc))`. Then RECEIVED, `received_at = now_date`, `received_by`, `shortage_value
= shortage > 0 ? Some : None`; `apply_change(+qty, +value, "transfer_in", ref, &received_at, Some(to))`;
post `{ received_at, "استلام تحويل مخزون {number} في {to.name}", source { "stockAdjustment",
receipt_source_id(transfer.id), number }, [Dr Inventory received_value, Cr InventoryInTransit transit,
(shortage > 0) Dr InventoryVariance shortage description "عجز تحويل {number}"], branch_id = to }`;
`log(Stock, "استلام تحويل {number}{shortage > 0 ? ' — عجز ' + to_fixed2(shortage) : ''}", received_at,
list("transfers"))`.
**C-T5 `reject_transfer(id, reason)`** (`:193-236`): lock; missing; not SENT → `لا يمكن رفض تحويل لم
يُرسل بعد`; `reason.trim()` empty → `سبب الرفض مطلوب`; per line `qty`, `value = round2(qty × (unit_cost
?? 0))`, `transit` accumulates; REJECTED, `rejected_at = now_date`, `rejected_by`, `reject_reason` trimmed;
lock products; `apply_change(+qty, +value, "transfer_in", ref, &rejected_at, Some(from))`; post `{
rejected_at, "رفض تحويل مخزون {number} — {reason}", source { "stockAdjustment",
receipt_source_id(transfer.id), number }, [Dr Inventory transit, Cr InventoryInTransit transit], branch_id
= from }`; `log(Stock, "رفض تحويل {number} — {reason}", rejected_at, list("transfers"))`.

`receipt_source_id(id)` (D-I4, `pub` in `transfers.rs` for the 00 importer) = the transfer id's 16 bytes
with the version nibble set to 8 (`b[6] = (b[6] & 0x0F) | 0x80`) — deterministic, never equal to any v7
id, shared by receive and reject exactly like the mock's `${transfer.id}-recv` (`:182,225`).

## 4. Concurrency (D8, analysis §5 rows 223–226)

Lock order as 06 §4 (document row → products sorted → domain counters → `ledger::post`'s settings S /
FY S / journal counter → `change_versions`).
- **Stock read-modify-write** (row 224): every posting command locks its products through
  `lock_line_products` **before** any stock-dependent check (LOSS vs `stock_qty`, branch availability), so
  checks and `apply_change` see one locked state.
- **Transfer availability** (row 225): `branch_stock_qty` is read after the product lock inside the same
  transaction; `product_branch_stock` is only written under that lock.
- **Status transitions** (send/receive/reject, complete/delete draft, count line/submit/resume/complete):
  `for_update_by_id` on the document row first, then re-read the status — two terminals can't both send,
  both receive, or lose a scan increment.
- **Snapshots** (row 226): completion uses the stored `system_qty`, never current stock.
- **Numbering** (row 223): `next_number` (row X-lock until commit, gapless); the transfer counter is shared by
  all branches (D-I7).

## 5. Undo

Not undoable via the registry (phase-e E-5; analysis §4 rows 202–212): corrections are a new opposite
adjustment, `rejectTransfer` while SENT, or a new reverse transfer. No `UndoSpec`, no compensator.

## 6. Frontend switch lines (dormant)

`src/modules/products/services/inventoryService.ts`: `getStockAdjustments` → `{ filter }`,
`getStockAdjustment` → `{ id }`, `createStockAdjustment` → `{ input, asDraft }`, `completeAdjustment` →
`{ id }`, `deleteDraftAdjustment` → `{ await backendCall('products_delete_draft_adjustment', { id }); return; }`,
`getStockMovements` → `{ filter }`, `getStockMovementsPaged` → `{ query }`, `getBatches` → `{ productId }`,
`getExpiryReport` → `()`, `writeOffExpiredBatches` → `{ batchIds, note }`, `returnBatchesToSupplier` →
`{ supplierId, lines, note }`, `getDebitNoteDrafts` → `()`, `getStockCounts` → `()`, `getStockCount` →
`{ id }`, `createStockCount` → `{ input }`, `updateStockCountLine` → `{ countId, productId, qty, delta }`,
`submitCountForReview`/`resumeCounting`/`completeStockCount` → `{ countId }`.
`transferService.ts`: `getTransfers` → `()`, `getTransfer` → `{ id }`, `createTransfer` → `{ input }`,
`sendTransfer` → `{ id }`, `receiveTransfer` → `{ id, input }`, `rejectTransfer` → `{ id, reason }`;
`branchStockQty` (sync) → `if (usesRust('products')) return branchStockFromCache(productId, branchId);`
(06's `rememberBranchStock` map, `?.[branchId]?.qty ?? 0`). `adjustmentValue` and `batchAlertTone` untouched.

**Part 04 note:** `ExpiryReportPage.vue:136-141` calls `returnBatchesToSupplier` (products) then
`postDebitNoteDraft` (purchases) — and purchases' mock would otherwise move mock stock while products reads
MariaDB. Flip `products` and `purchases` in the same step (G-P10).

## 7. Known mock quirks (kept) · Decisions

**Quirks kept:**
- Q-I1 `completeStockAdjustment` re-runs neither `validateStockIn` nor `assertApproval`
  (`inventory.ts:259-282`), so a large draft completes without a manager PIN. Kept (fixing it needs a UI to
  collect the PIN on completion) → open question O-I1.
- Q-I2 Analysis §3 row 162 says `completeAdjustment`/`completeStockCount` enforce the threshold; the code
  does not (`assertApproval` runs only in `recordStockAdjustment` when not a draft, and returns early for
  STOCKTAKE, `:219,231`). This plan follows the code.
- Q-I3 A write-off consumes FEFO across the product's batches, not necessarily the selected ids (`:418-419`).
- Q-I4 Transfer movements link to `{ adjustment, transferId }` and transfer journals carry source kind
  `stockAdjustment` (`transfers.ts:110,182`) — a dead link on the movements page.
- Q-I5 `sendTransfer` with the same product on two lines checks each against the same pre-send qty and
  stamps the first line's unit cost on both (`:77,92`).
- Q-I6 A transfer whose lines all have zero cost can't be sent (both journal lines drop → "at least two
  lines" refusal).
- Q-I7 `draftReturnToSupplier` validates neither supplier nor batches and stores the client's `unitCost`.
- Q-I8 `ExpiryRow.supplierName` is absent for received batches (`receiveBatch` never sets `supplierId`).
- Q-I9 `active_batches` breaks equal-expiry ties by `received_date, id`; the mock by insertion order.
- Q-I10 String-key sorts in the paged movements list use `utf8mb4_unicode_ci`; the mock uses
  `localeCompare('ar')` — ordering of equal-collation strings may differ (parity tolerance, P-I8).
- Q-I11 The expiry report's "today" is the UTC date, not the business day (`inventoryService.ts:192`).
- Q-I12 Scope `category` with no `categoryId` counts uncategorised products.

**Decisions:**
- D-I1 `batchAlertTone` and `branchStockQty` are called synchronously from templates
  (`ProductDetailPage.vue:143-145`, `StockTransferModals.vue:55`); a Rust command can't serve a sync call
  without editing pages. `batchAlertTone` stays pure frontend (display tone only; the authoritative "don't
  sell expired" rule is server-side `consume_fefo` with the server clock). `branchStockQty` reads the
  per-product `stockByBranch` that the Rust `getProducts`/`getProduct` just returned. Both become
  `frontend` via `scripts/contract/config.ts` overrides (G-P10). 47 of the analysis' 49 `port` functions get
  commands.
- D-I2 The manager-PIN approval is re-verified server-side (analysis §8 "required hardening"): grant from
  `users_verify_manager_pin` (G-P3) + active admin/manager check, same `FORBIDDEN` text as the mock.
  Parity cases that set `approvedBy` call `users_verify_manager_pin` first on the Rust side.
- D-I3 One transaction per command: the mock's partial writes (receive-line mutations before a thrown
  negative qty, `transfers.ts:148`; SENT status and stock before a failed post; a burned count number) roll
  back entirely in Rust.
- D-I4 Receipt/reject journal source id = `receipt_source_id(transfer.id)` (the mock's `-recv` suffix can't
  be a UUID). 00-import maps `<mockId>-recv` with the same function.
- D-I5 Transfers also touch `catalog` and adjustments also touch `ledger` (P2-12, C-07).
- D-I6 No `deleteDraftTransfer` is added (analysis §9 left it open; no caller, no accounting effect).
- D-I7 One shared `stockTransfer` counter across branches, branch-prefixed (parity).
- D-I8 Expiry "today" = `UTC_DATE()` (Q-I11) to match the mock exactly.

**Open questions for the user:** O-I1 require a manager PIN when completing a draft adjustment above the
threshold (needs a PIN dialog on the detail page)? O-I2 per-branch transfer numbering (decide together
with 07's purchase-order question)?

## 8. Tests

**(a) `src-tauri/tests/domain_products.rs`** (inventory part; G-P9 seed incl. `inventory`,
`inventoryInTransit`, `inventoryVariance`, `inventoryWriteOff`, `openingBalanceEquity`, `ownerCurrent`,
`otherIncome` roles, two branches, one open fiscal year; every posting test ends with
`shared::invariants::run_all` all green — invariant 4 is the GL↔stock tie):
- STOCK_IN per reason → the credit role of S-5; `other` with a control account → message; tracked product
  without batch → message; batch row created with the unit cost.
- LOSS above stock → message; LOSS on a tracked product consumes FEFO incl. expired; write-off groups by
  product.
- STOCKTAKE with a gain and a loss → one entry with both pairs, per-line then per-total rounding.
- Draft → no journal/movement, no event; complete uses the snapshot (sell 2 between draft and complete:
  delta unchanged); complete twice → `التسوية مكتملة بالفعل`; delete completed → message; delete draft writes
  an activity row.
- Approval: value ≥ threshold without `approvedBy` → `FORBIDDEN` text; with an ungranted or cashier id →
  same `FORBIDDEN`; with a granted manager → posts; STOCKTAKE exempt.
- Movements: filters, date-desc order, `refLink` per reason; paged sort by `qtyChange` asc/desc, page 2.
- Expiry report buckets and order; batches FEFO order.
- Counts: scope category/location/all; empty scope → message; `delta` scans accumulate; submit with an
  uncounted line → message; complete posts STOCKTAKE and links `adjustmentId`.
- Transfers: create validations; send over branch stock → `CONFLICT`; send posts transit (branch dims =
  from), per-branch rows move; receive with a shortage → variance line, `shortageValue`; reject returns the
  full value; wrong-status messages; receive/reject source id = `receipt_source_id`.
- Concurrency: two parallel LOSS on the last unit → one succeeds, one gets the "أكبر من المتوفر" message; two
  parallel sends exceeding branch stock → one `CONFLICT`; two parallel `delta` scans → both counted.

**(b) Parity cases for Part 04:** P-I1 STOCK_IN each reason; P-I2 LOSS + batch FEFO; P-I3 STOCKTAKE
gain+loss rounding; P-I4 draft → complete with an intervening sale; P-I5 approval threshold (with the
Rust-side grant); P-I6 stock count full cycle; P-I7 transfer send → receive short → reject path; P-I8
movements list + paged numeric sorts (string sorts compared as sets, Q-I10); P-I9 expiry report + write-off;
P-I10 return-to-supplier draft (continued in 07 P-P8).

## 9. Checklist (implementation order — after 06 §9 C1–C9)

- [x] A1 `dto/inventory.rs`: every DTO in §2 + the 25 Args structs.
- [x] A2 `service/stock_lines.rs`: S-1 `lock_line_products`, S-2 `build_lines`, labels, `to_fixed2`.
- [x] A3 `service/adjustments.rs`: S-3, S-4 (`ApprovalCheck`), S-5, C-A1 (widened to `pub` — see status
      note above), C-A2, C-A3, C-A4.
- [x] A4 `service/movements.rs`: C-M1, C-M2 (sort-key `match`).
- [x] A5 `service/batches.rs`: C-B1…C-B5.
- [x] A6 `service/counts.rs`: C-K1…C-K6.
- [x] A7 `service/transfers.rs`: `receipt_source_id`, C-T1…C-T5.
- [x] A8 `commands/inventory.rs` + `commands/transfers.rs`: 25 thin commands (`ApprovalCheck` built from
      `state.approval_grants` before `with_tx`), `ipc_sig!` lines appended to `ipc_signatures()`.
- [x] A9 `contract.check.ts` lines (§2) and the switch lines (§6) incl. `branchStockFromCache`.
- [ ] A10 Ask the manager for the G-P10 `scripts/contract/config.ts` overrides. **Not done — manager task.**
- [x] A11 `tests/domain_products.rs` inventory tests (§8a) — written; run in the deferred DB-backed pass.
- [x] → return to 06 §9 C10 (`create_product` opening stock).

## Gate

`cargo check` clean (manager's wave build); DB tests written; 25 switch lines present (+ the
`branchStockQty` cache branch); `contract.check.ts` entries type-check after `bun run bindings`;
`bun run memory:check` → the 47 products commands (06 + 06b) invoked and registered, 0 contract gaps.
DB tests and parity cases run in the deferred, time-boxed pass.
