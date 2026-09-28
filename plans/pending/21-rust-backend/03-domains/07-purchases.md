# 21 · 03.07 — `purchases` (purchase orders, receiving at cost, landed costs, returns / debit notes)

> **Status (2026-09-28):** code complete, not yet compiled/tested. All 12 commands, service logic
> (§3), DTOs (§2) and the 12 frontend switch lines (§6) are written per this spec. Tests (§8a) are
> now written in `tests/domain_purchases.rs`; they run in the deferred time-boxed test pass
> (entry §3.6) along with the manager's W3 `cargo check`, which together cover compilation/DB
> verification.
> `PurchaseOrder.paymentStatus` uses a **new, locally-defined** `purchases::dto::PaymentStatus` enum
> (3-way, `SCREAMING_SNAKE_CASE`) rather than importing one from `payments::dto` — the payments
> domain (built in this same wave, in parallel) has no `PaymentStatus` DTO of its own (it only deals
> in `Payment`/`PaymentAllocation`); see §2's note and this wave's final report ("Needs from
> manager"). `PurchaseDetail.payments` is assembled via
> `domains::payments::service::common::payment_dtos` (a `pub` cross-domain service call, the same
> pattern `payments::service::read::payments_for_invoice` establishes for 08-invoices), and
> `PurchaseDetail.supplier` via `domains::parties::service::read::get_supplier`. Wave **W3** (entry
> file §4), in parallel with 08-invoices. Depends on: 05-parties (`Supplier` DTO + reader),
> 06/06b-products (`lock_line_products`, `ProductBatch`, `DebitNoteDraft`, `get_debit_note_drafts`),
> 01-settings, Part 02 `shared::stock` / `shared::ledger` / `shared::numbering` / `shared::activity`,
> and manager tasks **G-P1** (`shared::totals`), **G-P2** (`shared::org`), **G-P4d**
> (`purchase_orders.received_date` instant), **G-P5** (architecture needle), **G-P7**
> (`PaymentStatus` + `Payment` DTOs) — listed in [`06-products.md`](06-products.md) §7.

**Goal.** Port the 12 `purchaseService.ts` functions to `domains/purchases/` so each returns the mock's
exact DTO and posts the exact journal: order save/send/cancel, receiving at the order price with landed
costs folded into the stock value (E4), the non-VAT-supplier branch (E3), short-delivery backorders, and
purchase returns with the three refund methods (E1) and the inventory-variance guard
(`shared::stock::cost::cost_out_at_price`). Totals, VAT, costs and AP amounts are recomputed in Rust from
ids and quantities; the client never supplies a money figure that gets posted.

**Read first.** [`../01-frontend-analysis/purchases.md`](../01-frontend-analysis/purchases.md) §1–§9 (the
primary input) · mock: `src/modules/purchases/services/purchaseService.ts:22-161`,
`src/mocks/backend/purchases.ts:22-575`, `src/mocks/backend/inventory.ts:73-109` (`activeBatchesFor`,
`receiveBatch`), `core.ts:268-297,468-471`, `branches.ts:205-209,264-268`,
`src/modules/invoices/helpers/totals.ts:87-237` · types: `src/modules/purchases/types/index.ts:1-194` ·
Rust: `src-tauri/src/shared/stock/{mod,cost,batches}.rs`, `shared/ledger/{post,accounts}.rs`
(`purchase_account_for`, `settlement_account_for`), `shared/numbering.rs` (`PurchaseOrder`,
`PurchaseReturn`), `entities/purchases/*.rs`, `entities/payments/{payments,payment_allocations}.rs`,
`entities/parties/parties.rs`, `migration/src/m0009_purchases.rs`, `core/lock.rs`.

## 1. Commands

Area `purchases` (`purchases/routes/index.ts:6-11`). `postDebitNoteDraft` is called from
`ExpiryReportPage.vue:141` (inventory screen) but posts AP and a journal → Purchases/Write (storekeeper and
manager have it; accountant/cashier don't). Reads: `with_read` + `settings::require(tx, actor, Purchases,
Read)` (06 §1 pattern). Writes: `with_tx` + `cx.require(tx, Purchases, Write)`; posting commands move
`state.undo.clone()` into the closure.

| Mock fn (`purchaseService.ts:line`) | Disp. | Rust command | Args → Return | Access | Tx | Events (from shared primitives) |
|---|---|---|---|---|---|---|
| `computePurchaseTotals` (`:25`, plain re-export) | frontend (sync preview, not `wrap`ped) | — | untouched | — | — | — |
| `getPurchaseOrders` (`:59`) | port | `purchases_get_purchase_orders` | `{ filter: Option<PurchaseListFilter> }` → `Vec<PurchaseRow>` | Read | read | — |
| `getPurchaseOrder` (`:74`) | port | `purchases_get_purchase_order` | `{ id }` → `PurchaseDetail` | Read | read | — |
| `savePurchaseOrder` (`:102`) | port | `purchases_save_purchase_order` | `{ input: PurchaseOrderInput, id: Option<Id> }` → `PurchaseOrder` | Write | tx | draft: none · `confirm`: Ledger + Parties + Catalog |
| `sendPurchaseOrderToSupplier` (`:107`) | port | `purchases_send_purchase_order_to_supplier` | `{ id }` → `PurchaseOrder` | Write | tx | — |
| `receivePurchaseOrder` (`:113`) | port | `purchases_receive_purchase_order` | `{ id, input: ReceivePurchaseInput }` → `PurchaseOrder` | Write | tx | Ledger + Parties + Catalog |
| `confirmPurchaseOrder` (`:119`) | port | `purchases_confirm_purchase_order` | `{ id }` → `PurchaseOrder` | Write | tx | Ledger + Parties + Catalog |
| `cancelPurchaseOrder` (`:126`) | port | `purchases_cancel_purchase_order` | `{ id }` → `PurchaseOrder` | Write | tx | — |
| `createPurchaseReturn` (`:131`) | port | `purchases_create_purchase_return` | `{ input: PurchaseReturnInput }` → `PurchaseReturn` | Write | tx | Ledger + Parties + Catalog |
| `getPurchaseReturn` (`:139`) | port | `purchases_get_purchase_return` | `{ id }` → `PurchaseReturn` | Read | read | — |
| `getActiveBatches` (`:147`) | port | `purchases_get_active_batches` | `{ productId }` → `Vec<ProductBatch>` | Read | read | — |
| `getDebitNoteDrafts` (`:153`) | port | `purchases_get_debit_note_drafts` | `()` → `Vec<DebitNoteDraft>` | Read | read | — |
| `postDebitNoteDraft` (`:158`) | port | `purchases_post_debit_note_draft` | `{ draftId, refundMethod: Option<RefundMethod> }` → `PurchaseReturn` | Write | tx | Ledger + Parties + Catalog |

12 commands. The mock emits only `parties:changed` on receive/return (analysis §6); `ledger::post`
(Ledger + Parties via the supplier line) and `apply_change` (Catalog) emit all three (P2-12, C-07 — D-U3).

## 2. DTOs (`domains/purchases/dto.rs`, `#[ts(export_to = "purchases/types/gen/")]`)

Conventions as 06 §2 (camelCase, `skip_serializing_none` + `ts(optional)`, `serde_number` Decimals,
`Id` as string, `DocDate` as its key string, `sent_at` via `format_iso_ms`).

| Rust DTO | TS type (file:line) | Notes |
|---|---|---|
| `PurchaseStatus` | `types/index.ts:4` | `SCREAMING_SNAKE_CASE`; maps the entity's `PurchaseStatus`. |
| `PurchaseLine` | `:9-31` | `qty`/`cost_price` `Decimal`; `unit_factor`, `discount`, `received_qty`, `landed_cost_share: Option<Decimal>`; `discount_is_pct: Option<bool>`; `unit_id`/`tax_id: Option<Id>`; `expiry_date: Option<NaiveDate>` `#[ts(type="string")]`. From `purchase_order_lines` ordered by `position`. |
| `LandedCostSpread` / `LandedCostLine` | `:34-43` | `lowercase`; line `id: String` (stored in the `landed_costs` JSON as a new `Id` text); `amount` `Decimal`. |
| `InvoiceDiscount` | inline `{ pct?; amount? }` (`:65`) | both `Option<Decimal>`; from `invoice_discount_pct/amount` (absent object when both NULL). |
| `PurchaseOrder` | `:45-87` | `date: String` (DocDate key); `received_date: Option<String>` (DocDate key, G-P4d); `sent_at: Option<String>`; `supplier_invoice_date: Option<NaiveDate>`; `payment_status: PaymentStatus` (G-P7, imported type); `attachment_ids: Option<Vec<String>>`; `currency`/`exchange_rate` stored as sent (D-U2). |
| `PurchaseLineInput` / `LandedCostLineInput` / `PurchaseOrderInput` | `:89-123` | `date: String` resolved with `RawDocDate`; `confirm: bool` (required). |
| `ReceiveBatchInput` / `ReceiveLineInput` / `ReceivePurchaseInput` | `:131-148` | batch `{ batch_no, expiry_date?, qty }`; `vat_not_recoverable`/`create_backorder: Option<bool>`. |
| `PurchaseListFilter` | `PurchaseFilter & { from?; to? }` (`:150-155`, `purchaseService.ts:59`) | one flat struct (all optional). |
| `RefundMethod` | `:158` | `snake_case` (`bank_transfer`). |
| `DebitNoteLine` / `PurchaseReturn` / `PurchaseReturnInput` | `:160-194` | `PurchaseReturn.date` DocDate key; `cost_price` `Decimal`; the mock object's extra `taxRate` is **not** in the TS type and is not emitted (Q-U8). |
| `PurchaseRow` | `purchaseService.ts:36` | `#[serde(flatten)] PurchaseOrder` + `supplier_name: String`, `outstanding: Decimal`, `missing_supplier_invoice: bool`. |
| `PurchaseProductInfo` | inline (`:41`) | `{ name, sku, stock_qty: Decimal, type: ProductType (06 DTO), track_batches: Option<bool> }`. |
| `PurchaseJournalRef` | inline `{ id; number; description }` (`:44`) | |
| `PurchaseDetail` | `:38-48` | flatten `PurchaseRow` + `supplier: Option<Supplier>` (05's DTO), `products: BTreeMap<String, PurchaseProductInfo>`, `returns: Vec<PurchaseReturn>`, `payments: Vec<Payment>` (G-P7), `journal_entries: Vec<PurchaseJournalRef>`, `returned_qty: BTreeMap<String, Decimal>` (`#[ts(type = "Record<string, number>")]`), `duplicate_invoice_warning: Option<String>`. |
| reused | `ProductBatch`, `DebitNoteDraft` (06b `dto/inventory.rs`), `ProductType` (06) | not redeclared. |

`src/modules/purchases/types/contract.check.ts` (new): `PurchaseStatus`, `PurchaseLine`,
`LandedCostSpread`, `LandedCostLine`, `PurchaseOrder`, `PurchaseLineInput`, `LandedCostLineInput`,
`PurchaseOrderInput`, `ReceiveLineInput`, `ReceivePurchaseInput`, `RefundMethod`, `DebitNoteLine`,
`PurchaseReturn`, `PurchaseReturnInput`, `Flat<PurchaseFilter & { from?: string; to?: string }>`, and from
`../services/purchaseService`: `Flat<PurchaseRow>`, `Flat<PurchaseDetail>` (file-local
`type Flat<T> = { [K in keyof T]: T[K] }`, as in 06).

## 3. Service logic (`domains/purchases/service/{orders,receive,returns,read}.rs`)

**Helpers** (`service/mod.rs`): `supplier(conn, id)` = the `parties` row with `kind = 'supplier'`;
`purchase_outstanding(po) = max(0, round2(grand − returned − paid))` (`purchases.ts:74-76`);
`missing_supplier_invoice(po)` (`:79-81`: RECEIVED and blank number or no date); `base_qty(l) =
round2(qty × (unit_factor ?? 1))` (`:105-107`); `base_unit_cost(l) = factor > 0 ? cost / factor : cost`
(unrounded, `:110-113`); `line_remaining(l) = round2(base_qty − (received ?? 0))` (`:215-217`);
`returned_qty_by_product(po_id)` = `SUM(qty)` of the PO's `purchase_return_lines` per product (`:387-393`);
`purchase_line_account(product)` = `accounts::purchase_account_for(ProductAccountOverrides { product:
product.purchase_account_id, category: live category's purchase_account_id },
settings.accounting.default_purchase_account_id)` (`:27-34`, product → category → settings → `freightIn`);
`compute_purchase_totals(lines, discount, default_rate)` (`:47-72`) = G-P1
`shared::totals::compute_invoice_totals(lines, discount, prices_include_tax = false)` where each line's tax
= the **active** live tax with `tax_id` (rate, category) else `(default_rate, 'S')`, the invoice discount is
`pct` when non-zero else `amount` when non-zero else none, and `tax_rate = lines empty ? default_rate :
round2(vat / (net == 0 ? 1 : net) × 100)`. Message formatting reuses 06b's `to_fixed2` (JS `toFixed(2)`) and
`utils::money::js_number_string` (JS `String(n)`).

**U-1 `get_purchase_orders(filter)`** (`purchaseService.ts:59-72`): SQL narrows by `status`,
`payment_status`, `supplier_id`, `date_day BETWEEN from AND to` (`inDateRange`), joins the supplier name
(`'—'` when missing), loads lines with one `IN` query; `to_row` (`:50-57`: `outstanding` =
RECEIVED ? `purchase_outstanding` : 0); **then** keeps rows where `utils::text::matches_search(&[number,
supplier_name, note], search)` (the joined-name haystack decision, Part 02 handoff §9 — D-U1); stable sort by
date key descending (`b.date.localeCompare(a.date)`; ties keep `created_at, id`).

**U-2 `get_purchase_order(id)`** (`:74-100`): missing → `NOT_FOUND` `أمر الشراء غير موجود`; `returns` =
the PO's returns (`created_at, id`) with lines; `payments` = payments with `type = 'PAID'` that have an
allocation `target_kind = purchaseOrder AND target_id = id` (G-P7 reader, `created_at, id`); `source_ids =
{id} ∪ return ids ∪ payment ids`; `journal_entries` = entries with `source_id IN source_ids`
(`created_at, id`) mapped to `{ id, number, description }`; `products` = for each PO line's product that
exists, its **current** `{ name, sku, stock_qty, type, track_batches }` (never snapshotted); `supplier` =
05's DTO (absent when missing); `returned_qty` from the helper; `duplicate_invoice_warning` when
`supplier_invoice_no` and the supplier exist and another RECEIVED PO of the same supplier (first by
`created_at, id`) has the same trimmed number → `` رقم الفاتورة مستخدم من قبل في أمر الشراء ${dup.number} من نفس المورد `` (`:84-90,87,98`).

**U-3 `validate_lines(input)`** (`purchases.ts:92-102`), exact order and codes (analysis §3 note):
supplier missing → `VALIDATION` `اختر المورد`; inactive → `المورد غير نشط`; no lines → `أضف صنفاً واحداً على
الأقل`; per line: product missing → `NOT_FOUND` `المنتج غير موجود`; `qty <= 0` → `الكمية يجب أن تكون أكبر من
صفر`; `cost_price < 0` → `سعر التكلفة لا يمكن أن يكون سالباً`.

**U-4 `save_purchase_order(input, id)`** (`:115-171`): 1 require · 2 U-3 · 3 `tax_rate =
shared::org::purchase_tax_rate(conn)` (G-P2); totals via the helper · 4a update (`id` set):
`for_update_by_id("purchase_orders", id)`; missing → `NOT_FOUND` `أمر الشراء غير موجود`; not DRAFT →
`لا يمكن تعديل أمر شراء تم إرساله أو استلامه أو إلغاؤه`; set `supplier_id`, `date`, lines (delete + insert,
`position` = index), `note`, invoice discount, `landed_costs` (fresh ids), supplier invoice no/date,
`attachment_ids`, `cost_center_id = default_cost_center_for(found.branch_id, input.cost_center_id)`,
`sub_total`/`tax_amount`/`grand_total`/`tax_rate` (branch, currency, exchange rate, payment fields untouched
— Q-U6) · 4b create: `branch = input.branch_id ?? settings.default_branch_id`; `number =
branch_prefix(branch) + next_number(PurchaseOrder)` (one shared counter, D-U4); insert DRAFT, `UNPAID`,
paid 0, returned 0, currency/exchange rate as sent, cost center as above · 5 `confirm` → U-6 with `{ date:
input.date, lines: po.lines → { productId, receivedQty: base_qty(l) } }` (`:168`), else
`log(Purchase, "حفظ أمر الشراء {number} كمسودة", Some(date), detail("purchase", id))` · 6 return the PO
DTO (post-receipt state when confirmed). The create/update insert body is `insert_or_update_po(...)`
(`pub(crate)`) so U-6's backorder reuses it.

**U-5 `send_purchase_order_to_supplier(id)`** (`:174-184`): lock row; missing → `أمر الشراء غير موجود`;
not DRAFT → `لا يمكن إرسال إلا مسودة`; ORDERED, `sent_at = now`; `log(Purchase, "إرسال أمر الشراء
{number} للمورد", now_date, detail("purchase", id))`.

**U-6 `receive_purchase(conn, cx, reg, id, input)`** (`pub(crate)`, `:228-369`) — posting walkthrough,
analysis §7 rows 189–191; rounding at every named point:
1. `for_update_by_id("purchase_orders", id)` (the PO lock, analysis §5 row 144); reload; missing →
   `NOT_FOUND` `أمر الشراء غير موجود`; not DRAFT/ORDERED → `تم استلام أمر الشراء هذا بالفعل أو تم إلغاؤه`;
   no input lines → `لا توجد كميات للاستلام`.
2. `products::service::stock_lines::lock_line_products(conn, po product ids)` (06b S-1).
3. `by_product` = input lines keyed by product, JS-`Map` semantics (first-occurrence order, last value).
   For **every** input line in order: PO line = the **first** PO line with that product (Q-U3), none →
   `صنف غير موجود في أمر الشراء`; `received_qty < 0` → `الكمية المستلمة لا يمكن أن تكون سالبة`; `>
   line_remaining + 0.0001` → `` الكمية المستلمة لـ "${name}" أكبر من المتبقي في الأمر ``. No
   `by_product` value > 0 → `أدخل كمية استلام واحدة على الأقل`.
4. `landed` = input landed costs (fresh ids) when non-empty, else the PO's (`:245`).
5. `received_valued` = `by_product` values with qty > 0 → `{ pid, qty, value: round2(qty ×
   base_unit_cost(po_line)), is_service: type == 'service' }` (`:248-255`).
6. `allocate_landed_costs(po.supplier_id, landed, non-service rows)` (`:187-213`): stock rows = qty > 0;
   `total_value = round2(Σ value)`; `total_qty = round2(Σ qty)`; per landed line with `amount > 0`:
   own (no supplier or = PO supplier) → `own = round2(own + amount)`, else push `{ supplier,
   round2(amount) }`; skip the spread when no stock rows or `base <= 0`; per stock row `share =
   round2(amount × weight / base)`, `share_by_product[pid] = round2(prev + share)`.
7. `date` = resolved `input.date`; per `received_valued` row: service → `service_by_account[
   purchase_line_account] = round2(prev + value)`, PO line `received_qty = round2((rq ?? 0) + qty)`;
   else `posted = round2(value + share)`, `inventory_value += posted`, `received_qty` as above,
   `landed_cost_share = round2((lcs ?? 0) + share)`, `stock::apply_change(p, qty, posted, "purchase",
   StockRef { po.id, po.number }, &date, Some(po.branch_id ?? default))`; tracked product: requested
   batches with qty > 0 → `batches::receive_batch(pid, b.qty, round2(posted / qty), b.batch_no.trim() or
   "RCV-{now_ms}", b.expiry, &date, &ref)` each, else one batch `(qty, …, "RCV-{po.number}", None)`.
   `inventory_value = round2(inventory_value)`.
8. `all_received` = every PO line `line_remaining <= 0.0001`; status RECEIVED; `received_date = date`.
9. `vat_not_recoverable = input.vat_not_recoverable ?? supplier.vat_number is blank`;
   `received_sum = round2(Σ value)`; `ratio = po.grand_total > 0 ? round2(received_sum / (po.sub_total == 0 ?
   1 : po.sub_total)) : 1`; `vat = round2(po.tax_amount × min(1, ratio))` (`:299-303`).
10. Lines (dim = branch `po.branch_id ?? default`, cost center `po.cost_center_id`): `Dr Role(Inventory)
    inventory_value`; `Dr AccountRef::Id(acc)` per service account (insertion order); not recoverable →
    (vat > 0) `Dr Role(FreightIn) vat` description `ضريبة مدخلات غير مستردة (مورد بدون رقم ضريبي) —
    أُضيفت إلى التكلفة`, else `Dr Role(VatInput) vat`; `ap = round2(received_sum + vat + own)` →
    `Cr Role(Payable) ap` party supplier; `Cr Payable amount` party other-supplier per other line.
11. PO update: `sub_total = received_sum`, `tax_amount = vat`, `grand_total = ap`, `vat_not_recoverable`,
    supplier invoice no/date **only when present** in the input, `landed_costs = landed` when non-empty and
    the PO had none; persist the changed PO lines.
12. Backorder prep (D-U6): when `create_backorder && !all_received`, short lines = PO lines with
    `line_remaining > 0.0001` → `{ product, qty: round2(remaining / (unit_factor ?? 1)), cost_price,
    unit_id, unit_factor, tax_id }` (`:358-360`); allocate its number now:
    `branch_prefix(settings.default_branch_id) + next_number(PurchaseOrder)` (the mock passes no branch —
    Q-U5).
13. `ledger::post(PostJournal { date, "استلام أمر شراء {number}", System, source { "purchaseOrder", po.id,
    number }, lines })` — an unbalanced landed-cost spread is refused here exactly as in the mock (Q-U1).
14. `log(Purchase, "استلام أمر الشراء {number} من {supplier.name ?? ''} بقيمة {to_fixed2(ap)}", date,
    detail("purchase", po.id))`.
15. Backorder: `insert_or_update_po` with `{ supplier, date, note: "أمر متبقٍ من {number}", confirm:
    false, lines }` and the pre-allocated number (runs U-3 — an inactive supplier fails the whole receipt,
    Q-U5), its draft-save log, `backorder_of_id = po.id`, then `log(Purchase, "إنشاء أمر متبقٍ
    {bo.number} من {number}", date, detail("purchase", bo.id))`.
16. Return the PO DTO.

**U-7 `confirm_purchase_order(id)`** (`purchaseService.ts:119-124`, the service version, not
`purchases.ts:372`): lock + load (missing → `أمر الشراء غير موجود`); U-6 with `{ date: now_date, lines:
po.lines → { productId, receivedQty: qty × (unit_factor ?? 1) − (received_qty ?? 0) } }` — unrounded, as
the service does (Q-U13).

**U-8 `cancel_purchase_order(id)`** (`purchases.ts:378-385`): lock; missing; not DRAFT/ORDERED →
`يمكن إلغاء المسودات والأوامر المرسلة فقط — استخدم مرتجع المشتريات للأوامر المستلمة`; CANCELED;
`log(Purchase, "إلغاء أمر الشراء {number}", now_date, detail("purchase", id))`.

**U-9 `record_purchase_return(conn, cx, reg, input, date)`** (`pub(crate)`, `:396-521`; analysis §7 rows
192–194):
1. Lock the PO row (analysis §5 row 145); missing → `NOT_FOUND` `أمر الشراء غير موجود`; not RECEIVED →
   `يمكن الإرجاع من أوامر الشراء المستلمة فقط`; `reason.trim()` empty → `سبب الإرجاع مطلوب`; lines with
   `qty > 0` empty → `اختر صنفاً واحداً على الأقل للإرجاع`.
2. `returned = returned_qty_by_product(po.id)` (under the PO lock); `lock_line_products(requested ids)`.
3. Per requested line in order: PO line (first) missing → `الصنف غير موجود في أمر الشراء`; product
   missing → `NOT_FOUND`; `remaining = round2(base_qty − returned[pid] ?? 0)`; `qty > remaining` →
   `` لا يمكن إرجاع أكثر من ${remaining} من "${name}" ``; `type == 'product' && qty > stock_qty` →
   `CONFLICT` `` المخزون الحالي من "${name}" (${stockQty}) أقل من كمية الإرجاع ``; `cost_price =
   round2(base_unit_cost + (lcs > 0 && base_qty > 0 ? lcs / base_qty : 0))` (the return-time snapshot).
   Numbers in messages via `js_number_string`.
4. `totals = compute_purchase_totals(lines, None, po.tax_rate)`; `settled = min(grand,
   purchase_outstanding(po))`; `cash_back = round2(grand − settled)`; `refund = input.refund_method ??
   credit` (E1).
5. `number = next_number(PurchaseReturn)` (no branch prefix, D-U4); insert the return + lines
   (`from_draft_id` as a plain id — the draft is deleted after posting, analysis §2 row 62); PO
   `returned_amount = round2(returned + grand)`, `payment_status = payment_status_for(po.grand_total −
   new_returned, po.paid_amount)` (G-P1).
6. Per line: service → `service_by_account[acc] = round2(prev + round2(qty × cost_price))`; else
   `(value_out, v) = shared::stock::cost::cost_out_at_price(stock_qty, stock_value, qty, cost_price)`,
   `variance = round2(variance + v)`, `inventory_value += value_out`, `apply_change(p, −qty, −value_out,
   "purchase_return", StockRef { ret.id, ret.number }, &date, Some(po.branch_id ?? default))`; tracked
   product: `batch_id` → `batches::draw_batch(pid, batch_id, qty)` (`:474-476`), else
   `batches::consume_fefo(pid, qty, allow_expired = true, today)` (the mock's walk skips nothing,
   `:478-484`). `inventory_value = round2(inventory_value)`.
7. Lines (ret dim = branch + cost center): `Dr Role(Payable) settled` party; `Dr
   AccountRef::Id(settlement_account_for(refund, AccountCtx::default()))` amount `refund == credit ? 0 :
   cash_back` (resolved even for `credit`, Q-U7); `Dr Payable (refund == credit ? cash_back : 0)` party;
   `Cr Inventory inventory_value`; `Cr Id(acc)` per service account; VAT **without dims** (Q-U9):
   `po.vat_not_recoverable` → (tax > 0) `Cr FreightIn tax`, else `Cr VatInput tax`; variance > 0 → `Dr
   InventoryVariance`, < 0 → `Cr InventoryVariance −variance` (no dims).
8. `post({ date, "مرتجع مشتريات {ret.number} على أمر الشراء {po.number} — {input.reason}", System,
   source { "purchaseReturn", ret.id, ret.number } })`.
9. `log(PurchaseReturn, "مرتجع مشتريات {number} بقيمة {to_fixed2(grand)}", date, detail("purchase",
   po.id))`; return the DTO.
`create_purchase_return(input)` = require + U-9 with `date = now_date` (`:131-134`).

**U-10 reads** — `get_purchase_return(id)`: missing → `NOT_FOUND` `إشعار المدين غير موجود`.
`get_active_batches(product_id)` → 06b C-B1 (`active_batches`, no duplicate logic, analysis row 34).
`get_debit_note_drafts()` → 06b C-B5.

**U-11 `post_debit_note_draft(draft_id, refund)`** (`purchases.ts:547-575`): `for_update_by_id
("debit_note_drafts", draft_id)`; missing → `NOT_FOUND` `مسودة الإرجاع غير موجودة`; no lines → `لا توجد
أصناف في المسودة`; per line: its batch's `source_ref_id` must name a RECEIVED PO, else `CONFLICT` `إحدى
التشغيلات ليست من أمر شراء مستلم — استخدم الإتلاف بدلاً من الإرجاع لهذه التشغيلة`; more than one PO →
`CONFLICT` `تشغيلات المسودة من أوامر شراء مختلفة — أنشئ مرتجعاً منفصلاً لكل أمر شراء`; U-9 with `{
purchaseOrderId, reason: draft.note non-empty or "بضاعة منتهية/قاربت على انتهاء الصلاحية", refund, lines:
{ productId, qty, batchId }, fromDraftId }` and `now_date`; **then** hard-delete the draft (a worklist
row; only after the return succeeded, analysis row 36).

## 4. Concurrency (D8, analysis §5 rows 142–146)

Lock order as 06 §4: document rows (draft → PO) → products (sorted, `lock_line_products`) → domain
counters (return / backorder numbers) → `ledger::post` (settings S, FY S, journal counter) →
`change_versions`.
- **Same PO received twice / over-received** (row 144): every receive, confirm, save-update, send, cancel
  and return takes `FOR UPDATE` on the PO row first and re-reads status and `received_qty` under it.
- **Returnable qty** (row 145): `returned_qty_by_product` is summed after the PO lock, so two concurrent
  returns serialise.
- **Stock / average cost** (row 143): `apply_change` under `lock_line_products`; the CONFLICT stock check
  reads the locked `stock_qty`.
- **Numbering** (row 142): shared branch-prefixed PO counter, unprefixed PR counter, both via
  `next_number` (gapless, X-lock to commit). The backorder number is taken **before** the receipt's journal
  number so every command takes counters in the same order (domain counter → journal), D-U6.

## 5. Undo

Not undoable via the registry (phase-e E-5; analysis §4): a receipt is corrected by a purchase return, a
return by a new purchase; `cancelPurchaseOrder` is a user action, not a compensator. No `UndoSpec`.

## 6. Frontend switch lines (dormant)

`src/modules/purchases/services/purchaseService.ts`, first statement inside each `wrap`:
`getPurchaseOrders` → `return backendCall('purchases_get_purchase_orders', { filter })`;
`getPurchaseOrder` → `{ id }`; `savePurchaseOrder` → `{ input, id }`; `sendPurchaseOrderToSupplier` →
`{ id }`; `receivePurchaseOrder` → `{ id, input }`; `confirmPurchaseOrder` → `{ id }`;
`cancelPurchaseOrder` → `{ id }`; `createPurchaseReturn` → `{ input }`; `getPurchaseReturn` → `{ id }`;
`getActiveBatches` → `{ productId }`; `getDebitNoteDrafts` → `backendCall('purchases_get_debit_note_drafts')`;
`postDebitNoteDraft` → `{ draftId, refundMethod }`. `computePurchaseTotals` is untouched (sync form
preview; it reads the mock's `db.taxes`, so once `settings` runs on Rust the preview may differ from the
posted totals until a later task moves it onto `settingsService` data — the server total is authoritative).
**Part 04:** flip together with `products` (06b §6).

## 7. Known mock quirks (kept) · Decisions

**Quirks kept (behaviour-exact; the first two are accounting defects — report, don't fix here):**
- Q-U1 **Landed-cost spread can unbalance the receipt.** Each share is `round2`'d with no remainder
  distribution (`purchases.ts:207-209`), so 100.00 over three equal lines gives 99.99 on the debit side and
  100.00 in AP → `ledger::post` refuses the receipt (unbalanced). Kept; open question O-U2.
- Q-U2 **Non-stock items (`type 'product'`, `stockMode 'none'`) debit `inventory` on receipt** (only
  `type === 'service'` takes the expense path, `:254,266`) while `applyStockChange` ignores them
  (`core.ts:277`) → `GL(inventory) ≠ Σ stockValue` (invariant 4). Kept; open question O-U3. DB tests avoid
  this product kind in "invariants green" scenarios and pin the drift in one explicit test.
- Q-U3 A PO with the same product on two lines only ever receives/returns against the first line.
- Q-U4 Receiving with its own landed costs spreads those, but the PO keeps its original set (`:339`).
- Q-U5 The backorder goes to the default branch (no `branchId` passed, `:362`), drops line discounts, and
  its validation failure (e.g. supplier deactivated) fails the whole receipt in Rust (the mock persisted
  the receipt, then threw).
- Q-U6 Editing a draft never updates `branchId`/`currency`/`exchangeRate`; U-3 runs before the existence
  check (`:116-123`).
- Q-U7 Return lines of one product are each checked against the full remaining qty; a non-stock item can
  never be returned (stock 0 → CONFLICT); the settlement (bank) account is resolved even for `credit`.
- Q-U8 The mock's `PurchaseReturn` object carries an extra `taxRate`; the TS type and Rust DTO don't.
  Parity compares typed fields only.
- Q-U9 The return's VAT and variance lines carry no branch/cost-center dimension (default branch).
- Q-U10 Partial-receipt VAT uses a `round2`'d ratio (`:302`).
- Q-U11 `base_unit_cost` divides without rounding; `Decimal` is exact where JS float isn't — a diff at a
  half-cent boundary is a float artifact of the mock (parity tolerance P-P9).
- Q-U12 `currency`/`exchangeRate` are stored but never applied (analysis §2/§8).
- Q-U13 `confirmPurchaseOrder` computes the outstanding qty without `round2` (`purchaseService.ts:123`).

**Decisions (strictest option, logged):**
- D-U1 Supplier-name search: SQL narrows by the row's own fields, Rust applies `matches_search` over
  `[number, supplierName, note]` after the join (Part 02 handoff §9). No stored joined name: the list is
  unpaged, and a renamed supplier must match immediately.
- D-U2 FX stays inert: `currency`/`exchange_rate` stored as sent, no `shared::currency::to_base` call — the
  mock converts nothing (analysis §9); wiring FC purchases is a new accounting feature (O-U1).
- D-U3 Events come from the shared primitives (Ledger + Parties + Catalog), closing analysis §6/§9's event
  gap structurally (C-07).
- D-U4 Numbering kept as the mock: PO = branch prefix + one shared counter; PR unprefixed.
- D-U5 PO row lock for every status/quantity guard (analysis §5).
- D-U6 Backorder number allocated before the receipt's journal (counter order); number values unchanged.
- D-U7 Received batches keep `supplier_id` NULL (mock parity; 06b Q-I8).

**Open questions for the user:** O-U1 multi-currency purchase orders in scope (real `to_base` on
receipt/return)? O-U2 fix Q-U1 with the largest-remainder spread (`shared::totals`'s
`spread_proportionally`) — an accounting change needing an `ACC-` ledger issue + a
`scripts/verify/cases/*.json` regression case? O-U3 fix Q-U2 by routing non-stock items like service lines
(same process)? O-U4 per-branch numbering for PO/PR/transfers (decide once, with 06b O-I2)?

## 8. Tests

**(a) `src-tauri/tests/domain_purchases.rs`** (G-P9 seed + a VAT supplier, a supplier without VAT number, a
shipping supplier, a default 15% INPUT tax, `cash`/`bank`/`payable`/`vatInput`/`freightIn`/`inventory`/
`inventoryVariance` roles; every posting test ends with `shared::invariants::run_all` green, except the one
Q-U2 test):
- save: each U-3 message and code (`اختر المورد` is `VALIDATION`, product is `NOT_FOUND`); draft number
  `PO-000001` (prefix when >1 branch); update refused when not DRAFT; totals with a line discount, an invoice
  discount and a zero-rated tax.
- send/cancel: status guards and messages; cancel of RECEIVED refused with the return hint.
- receive full: journal `Dr inventory / Dr vatInput / Cr payable`, stock qty/value/avg cost, PO totals.
- receive short + backorder: PO totals shrink to the received part, VAT by ratio, DRAFT backorder linked
  with `backorderOfId`, three activity rows in order.
- landed costs: own + other supplier, by value and by qty (divisible amounts), shares folded into stock
  value; the 100/3 case → unbalanced refusal (pins Q-U1).
- non-VAT supplier: VAT to `freightIn`, not in stock value; the later return credits `freightIn`.
- tracked product: requested batches created at `round2(posted / qty)`; default `RCV-{number}` batch.
- confirm: receives everything outstanding at `now`.
- return: over-return and stock `CONFLICT` messages; refund `credit` / `cash` / `bank_transfer` lines;
  variance guard (return more value than on hand → variance line); batch drawn by id and by FEFO;
  `returnedAmount`/`paymentStatus` updated.
- debit-note draft: batch from an adjustment → `CONFLICT`; two POs → `CONFLICT`; success deletes the draft;
  a failed post leaves it.
- Q-U2 pin: receiving a non-stock item leaves invariant 4 failing by exactly its value.
- concurrency: two parallel receives of the same PO → one succeeds, the other gets `تم استلام أمر الشراء
  هذا بالفعل أو تم إلغاؤه`; two parallel returns of the last returnable unit → one gets the "لا يمكن إرجاع
  أكثر من" message; 20 parallel draft saves → 20 distinct gapless numbers.

**(b) Parity cases for Part 04:** P-P1 draft save/edit/send/cancel; P-P2 full receipt; P-P3 short receipt
+ backorder; P-P4 landed costs (own + other supplier, value/qty); P-P5 non-VAT supplier receipt + return;
P-P6 tracked product with batches; P-P7 returns with each refund method + variance guard; P-P8 expiry
report draft → debit note (continues 06b P-I10); P-P9 list search by supplier name (Arabic normalization)
and the detail DTO (payments, journal refs, duplicate-invoice warning).

## 9. Checklist (implementation order)

- [x] U0 Confirm G-P1, G-P2, G-P4d, G-P5, G-P7 and 05's `Supplier` reader exist (manager, before W3).
      Confirmed present: `shared::totals`, `shared::defaults::{branch_prefix,default_cost_center_for,
      purchase_tax_rate}`, `entities::purchases::purchase_orders.received_date_{day,instant}`,
      `parties::service::read::get_supplier`. G-P7's `PaymentStatus` DTO was **not** found in
      `payments::dto` (only `Payment`/`PaymentAllocation`/`PaymentRow` exist there) — see the status
      note atop this file; `purchases::dto::PaymentStatus` is defined locally instead.
- [x] U1 `domains/purchases/mod.rs` (`commands`, `service`, `dto`, `ipc_signatures()`,
      `export_bindings(cfg)`); ask the manager for `pub mod purchases;` + hooks.
- [x] U2 `dto.rs`: every DTO in §2 + the 12 Args structs.
- [x] U3 `service/mod.rs`: the helpers (outstanding, base qty/cost, remaining, returned qty, line account,
      `compute_purchase_totals`).
- [x] U4 `service/read.rs`: U-1, U-2, U-10.
- [x] U5 `service/orders.rs`: U-3, `insert_or_update_po`, U-4, U-5, U-8.
- [x] U6 `service/receive.rs`: `allocate_landed_costs`, U-6 (steps 1–16), U-7.
- [x] U7 `service/returns.rs`: U-9, U-11.
- [x] U8 `commands.rs`: 12 thin commands + `ipc_sig!` lines.
- [x] U9 `types/contract.check.ts` and the 12 switch lines (§6).
- [x] U10 `tests/domain_purchases.rs` (§8a) — written; runs in the deferred time-boxed test pass
      (entry §3.6).

## Gate

`cargo check` clean (manager's W3 build); DB tests written; 12 switch lines present; `contract.check.ts`
type-checks after `bun run bindings`; `bun run memory:check` → 12 commands invoked + registered, 0 contract
gaps. DB tests and parity cases run in the deferred, time-boxed pass (entry §3.6).
