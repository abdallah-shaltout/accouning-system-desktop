# 21 · 01.B — `purchases` contract

> **Status:** done (2026-09-27) · **Inventory:** `docs/backend/contract/purchases.md`
> (regenerate with `bun run contract`) · **Mock spec:** `src/mocks/backend/purchases.ts` (order
> save/send/receive/cancel, landed-cost spreading, purchase returns/debit notes), `src/mocks/backend/inventory.ts`
> (`activeBatchesFor`, `receiveBatch`, FEFO helpers shared with `products.md`), `src/mocks/backend/core.ts`
> (`applyStockChange`, `postJournal`, `purchaseTaxRate`, `logActivity`/`logAudit`), `src/mocks/backend/branches.ts`
> (`branchPrefix`, `defaultCostCenterFor`), `src/mocks/backend/accounts.ts` (`settlementAccountFor`) ·
> **Services:** `src/modules/purchases/services/purchaseService.ts` · **Types:** `src/modules/purchases/types/index.ts`
>
> Purchase orders (draft → ordered → received, or canceled), landed-cost spreading over receipt
> lines, short-delivery backorders, and purchase returns / debit notes with three refund methods
> (docs/v2/09-purchases-payments-expenses.md). This module reuses `products.md`'s `stock`
> shared-manager primitives directly: every receipt and every return calls `applyStockChange`
> (`backend/core.ts`), and every batch-tracked receipt calls `receiveBatch`
> (`backend/inventory.ts`) — no separate posting/stock logic of its own, just this module's own
> landed-cost allocation and AP-side accounting layered on top.

## 1. Endpoints

12 functions, one service file. Disposition: `port` · `frontend` · `dev-only` · `drop`.

| Function | Disposition (confirmed / changed + why) | Rust command | Request DTO | Response DTO | Writes (confirmed) | Shared | Undo | Notes |
|---|---|---|---|---|---|---|---|---|
| `getPurchaseOrders` | port (confirmed) | `purchases_get_purchase_orders` | `PurchaseFilter & { from?, to? }` | `PurchaseRow[]` | — | — | n/a (read) | `toRow` computes `outstanding` (`purchaseOutstanding`, live, not stored) and `missingSupplierInvoice` per row — both derived, not columns. Sorted by `date` descending. `filter.search` matches `[number, supplierName, note]` via `includesText` (Arabic-normalizing, cross-cutting 01.D). |
| `getPurchaseOrder` | port (confirmed) | `purchases_get_purchase_order` | `{ id: string }` | `PurchaseDetail` | — | — | n/a (read) | Assembles a detail view: the PO's own returns, payments allocated to it, every journal entry whose `sourceRef.id` is the PO/a return/a payment on it, a `productId → display info` map (name/sku/stockQty/type/trackBatches — **not snapshotted on the PO line**, always the product's *current* values), `returnedQty` per product, and a same-supplier duplicate-invoice-number warning. `products` field name reads today's product state, not what it was when the PO was created — Rust must join live, not from a stored snapshot. |
| `savePurchaseOrder` | port (confirmed) | `purchases_save_purchase_order` | `{ input: PurchaseOrderInput, id?: string }` | `PurchaseOrder` | `purchaseOrders`, activity, audit, **conditionally** everything `receivePurchase` writes (`input.confirm`) | activity, **conditionally** ledger/stock/numbering/period | not undoable | Create-or-update by optional `id`; update is refused unless the found order is still `DRAFT`. **`input.confirm: true` is a "save-as-draft-then-receive-in-full-immediately" convenience path** — it calls `receivePurchase` synchronously in the same function body with every line's full base qty, so a Rust command implementing this must run both the create/update and the receipt inside **one transaction**, exactly like `products.md`'s `createProduct` opening-stock case. Document number is **branch-prefixed but drawn from one shared `purchaseOrder` counter across all branches** (`branchPrefix(branchId) + nextNumber('purchaseOrder')`) — same pattern as `products.md`'s `createTransfer`, flagged in §5/§9, not assumed a bug. |
| `sendPurchaseOrderToSupplier` | port (confirmed) | `purchases_send_purchase_order_to_supplier` | `{ id: string }` | `PurchaseOrder` | `purchaseOrders`, activity, audit | activity | not undoable via the registry — a `DRAFT` can simply not be sent again; there's no un-send. The natural way "back" is `cancelPurchaseOrder` (refuses only `RECEIVED`/`CANCELED`, so an `ORDERED` PO can still be canceled) | Refuses unless `status === 'DRAFT'`. Stamps `sentAt = now()`; the PO PDF/print itself is the caller's job (a separate `pdfService`/print route call from the page, not part of this function). |
| `receivePurchaseOrder` | port (confirmed) | `purchases_receive_purchase_order` | `{ id: string, input: ReceivePurchaseInput }` | `PurchaseOrder` | `purchaseOrders`, activity, audit, `productBatches`, `journalEntries`, `counters`, `stockMovements` | activity, ledger, numbering, period, stock | not undoable via the registry — see §4 | **The single most accounting-sensitive endpoint in this module** — see §7's full posting walkthrough. Landed costs are spread over the receiving stock lines by value or qty and folded straight into each line's posted unit cost before `applyStockChange` runs, so `GL(inventory)` already reflects them exactly (no separate landed-cost account). A PO can only be received **once as a whole document** — a short delivery is handled by shrinking this PO's own totals down to what was actually received and (optionally) spinning off a fresh `DRAFT` backorder PO for the shortfall (`input.createBackorder`), never a second receipt against the same PO. |
| `confirmPurchaseOrder` | port (confirmed) | `purchases_confirm_purchase_order` | `{ id: string }` | `PurchaseOrder` | (delegates entirely to `receivePurchase`'s write set) | activity, ledger, numbering, period, stock | not undoable — same as `receivePurchaseOrder` | **Legacy one-step alias**, kept for callers (seed history, the "quick confirm" UI path) that still want "receive everything ordered, in full, today" without building a `ReceivePurchaseInput`. Internally builds one from the PO's own lines (`receivedQty: l.qty * (l.unitFactor ?? 1) - (l.receivedQty ?? 0)` — i.e. "whatever base qty is still outstanding on this line") and calls the exact same `receivePurchase`. Rust must keep this as a genuinely separate command (not just a frontend convenience) since it's a distinct, named IPC surface today — but its body is "build a `ReceivePurchaseInput` from the PO, then call receive," not independent posting logic. |
| `cancelPurchaseOrder` | port (confirmed) | `purchases_cancel_purchase_order` | `{ id: string }` | `PurchaseOrder` | `purchaseOrders`, activity, audit | activity | not undoable (a canceled order stays canceled — there is no "re-open") | **Correctly guarded against a posted order**: refuses unless `status === 'DRAFT' \|\| status === 'ORDERED'`, with the exact message directing the user to use a purchase return instead for `RECEIVED` orders (`'يمكن إلغاء المسودات والأوامر المرسلة فقط — استخدم مرتجع المشتريات للأوامر المستلمة'`). No stock or GL effect either way — a `DRAFT`/`ORDERED` order never posted anything, so "cancel" here is a pure status flip with an audit row, not a reversal. This is exactly the guard named in the task brief's bug-class check (c): confirmed present and correct, not missing. |
| `createPurchaseReturn` | port (confirmed) | `purchases_create_purchase_return` | `PurchaseReturnInput` | `PurchaseReturn` | `purchaseReturns`, `purchaseOrders` (`returnedAmount`/`paymentStatus`), activity, audit, `journalEntries`, `counters`, `productBatches`, `stockMovements` | activity, ledger, numbering, period, stock | not undoable via the registry — see §4 | See §7 for the full posting walkthrough (three-way refund split, inventory-variance guard, non-VAT-supplier symmetry with the original receipt). Refuses on a PO that isn't `RECEIVED`, an empty/absent reason, a line exceeding what's still returnable (`baseQty(poLine) − alreadyReturned`), or a line exceeding **current on-hand stock** (`product.stockQty`, `CONFLICT` — the one function-specific `CONFLICT` in this module, everything else defaults to `VALIDATION`). |
| `getPurchaseReturn` | port (confirmed) | `purchases_get_purchase_return` | `{ id: string }` | `PurchaseReturn` | — | — | n/a (read) | No dedicated detail page/route — used only to feed the debit-note PDF payload. |
| `getActiveBatches` | port (confirmed) | `purchases_get_active_batches` | `{ productId: string }` | `ProductBatch[]` | — | — | n/a (read) | Thin re-export of `products.md`'s `activeBatchesFor` — the debit-note form's batch picker. No new logic to port; Rust should call the same `shared::stock` helper `products` domain uses, not a duplicate. |
| `getDebitNoteDrafts` | port (confirmed) | `purchases_get_debit_note_drafts` | — | `DebitNoteDraft[]` | — | — | n/a (read) | **Reads a table this module doesn't write** — `debitNoteDrafts` rows are created only by `products.returnBatchesToSupplier` (`products.md`, the expiry-report shortcut). This function and `postDebitNoteDraft` are the second half of that two-command flow; `products.md` already documents the split. Unfiltered — every draft, all always `status: 'DRAFT'`. |
| `postDebitNoteDraft` | port (confirmed) | `purchases_post_debit_note_draft` | `{ draftId: string, refundMethod?: RefundMethod }` | `PurchaseReturn` | `debitNoteDrafts` (delete), plus (delegates entirely to `recordPurchaseReturn`'s write set) | activity, ledger, numbering, period, stock | not undoable — same as `createPurchaseReturn`, since this **is** a purchase return once posted | Converts one of `products.md`'s `DebitNoteDraft` rows into a real posted return: traces every draft line's `batchId` back to the **RECEIVED purchase order it was received on** (`ProductBatch.sourceRefId`), refuses if any batch traces to no PO or a not-yet-received one (`لا يمكن استخدام تشغيلة من فتح رصيد لإصدار مرتجع — استخدم الإتلاف بدلاً من ذلك`-style `CONFLICT`, exact message in §3), and refuses if the draft's lines span **more than one** purchase order (`CONFLICT` — a debit note is always against exactly one PO). Deletes the draft row only *after* `recordPurchaseReturn` succeeds — if the return posting throws, the draft survives untouched (confirmed: the `mutate(() => db.debitNoteDrafts = …)` line runs strictly after the `recordPurchaseReturn` call returns, so a thrown `ApiError` mid-posting leaves the draft in place for retry, not silently lost). |

## 2. DTOs → Rust

Only what differs from the generator's hint or needs a scale/enum/relationship decision.

| Type | Field | Rust type | Column | Why |
|---|---|---|---|---|
| `PurchaseOrder` | `subTotal` / `taxAmount` / `grandTotal` / `paidAmount` / `returnedAmount` | `Decimal` (all) | `DECIMAL(19,2)` | Money, `round2` throughout `computePurchaseTotals` (reuses `invoices/helpers/totals.ts`'s engine, same scale as `invoices.md`'s equivalent fields will use). |
| `PurchaseOrder` | `taxRate` | `Decimal` | `DECIMAL(9,4)` | Percentage, not money — same scale as `settings.md`'s `Tax.rate`. Derived (`round2Totals((vat/net)*100)`), not a stored input on the order form. |
| `PurchaseOrder` | `status` | `enum PurchaseStatus { Draft, Ordered, Received, Canceled }` | `ENUM('DRAFT','ORDERED','RECEIVED','CANCELED')` | `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` — TS literals are already upper-snake, same convention as `products.md`'s `StockAdjustment`/`StockTransfer` status enums. |
| `PurchaseOrder` | `paymentStatus` | reuse the shared `PaymentStatus` enum from `invoices`/`payments` (not redefined here) | `ENUM(...)` | Cross-module type (`@/modules/invoices/types`) — define once in `shared::` or `invoices` domain, `purchases` just references it. |
| `PurchaseOrder` | `number` | `String` | `VARCHAR`, `UNIQUE` | **One shared `purchaseOrder` counter across all branches**, prefixed per-branch (`branchPrefix`) — same "shared counter, cosmetic prefix" shape as `products.md`'s `createTransfer`. The uniqueness constraint is on the full prefixed string. |
| `PurchaseOrder` | `landedCosts` | `Vec<LandedCostLine>` | child table `purchase_order_landed_costs(id, purchase_order_id, label, amount, supplier_id, spread_by)` | `amount` `DECIMAL(19,2)` (money). `id` here is a mock-only synthetic id (`uid('lc')`, never referenced elsewhere) — safe to make a genuine UUIDv7 primary key in Rust with no migration concern. |
| `LandedCostLine` / `LandedCostLineInput` | `spreadBy` | `enum LandedCostSpread { Value, Qty }` | `ENUM('value','qty')` | `#[serde(rename_all = "lowercase")]`. |
| `PurchaseOrder` | `invoiceDiscount` | `Option<InvoiceDiscount>` (new struct `{ pct: Option<Decimal>, amount: Option<Decimal> }`, shared with `invoices`/`sales`) | inline columns `invoice_discount_pct DECIMAL(9,4)`, `invoice_discount_amount DECIMAL(19,2)`, both nullable | `pct` is a percentage (scale 4 like `Tax.rate`), `amount` is money (scale 2) — mutually exclusive by convention (the totals engine picks whichever is set), not enforced as an XOR constraint anywhere in the mock, so Rust shouldn't add one either (would be a stricter behavior change, not requested). |
| `PurchaseLine` | `qty` / `receivedQty` | `Decimal` (both) | `DECIMAL(19,4)` | Quantity. **Rounding point note (same as `products.md`'s flagged discrepancy):** `baseQty`/`lineRemaining` round with `round2`, not `round4`, throughout this file — Rust must match the mock's actual 2dp rounding for qty here too, not the master plan's general "qty = round4" rule (already flagged as an open master-plan documentation question in `products.md` §9; this module reconfirms the same pattern, not a new finding). |
| `PurchaseLine` | `costPrice` / `landedCostShare` | `Decimal` (both) | `DECIMAL(19,4)` | Cost precision — `landedCostShare` accumulates via repeated `round2` additions in the mock (`poLine.landedCostShare = round2((poLine.landedCostShare ?? 0) + landedShare)`), so Rust should also round at 2dp at this exact accumulation point to stay byte-identical for the parity harness, even though the column can hold 4dp. |
| `PurchaseLine` | `unitFactor` | `Decimal` | `DECIMAL(19,6)` | Conversion factor — same precision as `products.md`'s `ProductUnit.factor`. Snapshotted at save time (never re-derived from the product's current unit definition once saved), so a later unit-factor edit on the product never retroactively changes a posted PO's math — Rust must persist this as a plain column, not compute it on read. |
| `PurchaseLine` | `discount` | `Decimal` | `DECIMAL(19,2)` when `discountIsPct` is false, but the same physical column holds a percentage (0-100 range) when `discountIsPct` is true | The mock stores both shapes in one untyped `number` field distinguished by the sibling `discountIsPct: boolean` — Rust keeps the same "one column, a bool decides the unit" shape rather than splitting into two nullable columns, to match `computePurchaseTotals`'s exact input contract (it reads `discount`+`discountIsPct` together, same as the sales engine). |
| `PurchaseOrder` | `currency` / `exchangeRate` | `Option<String>` (ISO code) / `Option<Decimal>` | `VARCHAR(3)` / `DECIMAL(19,6)` | FX rate scale matches `settings.md`'s `ExchangeRate.rate` (6dp) — confirmed by this module's own review, not assumed. **Contract note:** despite `PurchaseOrderInput`/`PurchaseOrder` carrying `currency`/`exchangeRate` fields (docs/v2 phase-9 FX support), **no code path in `purchases.ts` actually reads or applies them** — `computePurchaseTotals`, `receivePurchase` and `recordPurchaseReturn` all compute everything in base currency using `costPrice` directly, with no `toBase()`/FX conversion call anywhere in this file. This looks like a **future/unfinished field**, not a bug to fix (nothing is silently wrong — the fields are simply inert today, same as `PaymentMethod.feePct` was found inert in `settings.md`). Flagged in §9 for confirmation, not changed. |
| `PurchaseOrder` | `attachmentIds` | `Vec<Uuid>` | child table or JSON array | Same ownership pattern as `products.md`'s `Product.imageIds` — the blob storage itself lives in `core`'s `AttachmentField`, this module only stores the id list. |
| `PurchaseReturn` | `subTotal` / `taxAmount` / `grandTotal` / `settledToPayable` / `cashBack` | `Decimal` (all) | `DECIMAL(19,2)` | Money, `round2`. `settledToPayable`'s TS type is bare `number` (not annotated `_decimal_` by the generator, since the interface literal doesn't carry the hint) — confirmed it's money by reading `recordPurchaseReturn` (`Math.min(totals.grandTotal, purchaseOutstanding(po))`), not an integer or ratio. |
| `PurchaseReturn` | `number` | `String` | `VARCHAR`, `UNIQUE` | **Not branch-prefixed at all** (`nextNumber('purchaseReturn')` called directly, no `branchPrefix()` wrapper) — inconsistent with `PurchaseOrder.number`'s branch-prefix convention in the same file. Not a bug (returns aren't shown in a per-branch list the same way), but worth flagging in §9 since Part 02's numbering design should decide deliberately whether purchase returns get a branch prefix too, for consistency, or stay as today. |
| `PurchaseReturn` | `refundMethod` | `enum RefundMethod { Cash, BankTransfer, Credit }` | `ENUM('cash','bank_transfer','credit')` | `#[serde(rename_all = "snake_case")]` — TS literal is already snake_case (`bank_transfer`). Defaults to `'credit'` server-side when the input omits it (never trusts an absent value as "cash" — E1's original bug is fixed; the mock now always honors/derives an explicit method). |
| `PurchaseReturn` | `lines` (`DebitNoteLine[]`) | `Vec<DebitNoteLine>` | child table `purchase_return_lines(return_id, product_id, qty, cost_price, batch_id)` | `qty` `DECIMAL(19,4)` (2dp actual rounding, see the qty note above), `costPrice` `DECIMAL(19,4)` — this is the **return-time cost snapshot** (`baseUnitCost(poLine) + landedCostShare/baseQty`), never re-derived from the product's current average cost. |
| `PurchaseReturn` | `fromDraftId` | `Option<Uuid>` | `UUID` nullable, FK-like reference to a since-deleted `debit_note_drafts` row | **This FK will dangle by design** — `postDebitNoteFromDraft` deletes the draft row after posting, so `fromDraftId` on the resulting `PurchaseReturn` points at a row that no longer exists. Model as a plain `UUID` column with **no** foreign-key constraint (or an `ON DELETE SET NULL`-style soft reference), never a hard FK that would block the draft's deletion or force keeping deleted drafts around just to satisfy referential integrity. |
| Route field (activity/audit `link` only, not a DTO field) | `/purchases/${id}` (every write in this file: save, send, receive, cancel, return) | stays a path string in `db.audit`/`db.activity`, **by the same explicit `route-ok` design as `products.md`'s equivalent links** — parsed server-side by `entityFromLink` (`ENTITY_TO_ACTIVITY_KIND['purchaseOrder'] → 'purchase'`, `['purchaseReturn'] → 'purchase_return'`), never navigated by the UI directly as a raw string | — | **Not part of F7's 61-link remediation count** — confirmed this module contributes zero functions to that list, same conclusion as `products.md`. No DTO returned to the UI carries a raw path-string route field; this module has no `_route_`-hinted field in its types at all. |

## 3. Validation and errors

| Function | Rule (source: mock check line) | Code | Exact Arabic message |
|---|---|---|---|
| `savePurchaseOrder` (`validateLines`) | supplier must exist | `NOT_FOUND` (default `ApiError` code is `VALIDATION`, but this call passes no code override — **confirmed default `VALIDATION`, not `NOT_FOUND`**, see note below) | `اختر المورد` |
| `savePurchaseOrder` | supplier must be active | `VALIDATION` (default code) | `المورد غير نشط` |
| `savePurchaseOrder` | at least one line | `VALIDATION` (default code) | `أضف صنفاً واحداً على الأقل` |
| `savePurchaseOrder` (per line) | product must exist | `NOT_FOUND` (via `productById`, which does pass `'NOT_FOUND'` explicitly) | `المنتج غير موجود` |
| `savePurchaseOrder` (per line) | `qty > 0` | `VALIDATION` (default code) | `الكمية يجب أن تكون أكبر من صفر` |
| `savePurchaseOrder` (per line) | `costPrice >= 0` | `VALIDATION` (default code) | `سعر التكلفة لا يمكن أن يكون سالباً` |
| `savePurchaseOrder` (update path) | order must exist | `NOT_FOUND` | `أمر الشراء غير موجود` |
| `savePurchaseOrder` (update path) | order must still be `DRAFT` | `VALIDATION` (default code) | `لا يمكن تعديل أمر شراء تم إرساله أو استلامه أو إلغاؤه` |
| `sendPurchaseOrderToSupplier` | order must exist | `NOT_FOUND` | `أمر الشراء غير موجود` |
| `sendPurchaseOrderToSupplier` | must be `DRAFT` | `VALIDATION` (default code) | `لا يمكن إرسال إلا مسودة` |
| `receivePurchaseOrder`/`confirmPurchaseOrder` | order must exist | `NOT_FOUND` | `أمر الشراء غير موجود` |
| `receivePurchaseOrder`/`confirmPurchaseOrder` | must be `DRAFT` or `ORDERED` | `VALIDATION` (default code) | `تم استلام أمر الشراء هذا بالفعل أو تم إلغاؤه` |
| `receivePurchaseOrder` | at least one input line | `VALIDATION` (default code) | `لا توجد كميات للاستلام` |
| `receivePurchaseOrder` (per line) | line's product must exist on the PO | `VALIDATION` (default code) | `صنف غير موجود في أمر الشراء` |
| `receivePurchaseOrder` (per line) | `receivedQty >= 0` | `VALIDATION` (default code) | `الكمية المستلمة لا يمكن أن تكون سالبة` |
| `receivePurchaseOrder` (per line) | `receivedQty` can't exceed the line's remaining qty (+0.0001 tolerance) | `VALIDATION` (default code) | `` الكمية المستلمة لـ "${productName}" أكبر من المتبقي في الأمر `` |
| `receivePurchaseOrder` | at least one line with `receivedQty > 0` | `VALIDATION` (default code) | `أدخل كمية استلام واحدة على الأقل` |
| `cancelPurchaseOrder` | order must exist | `NOT_FOUND` | `أمر الشراء غير موجود` |
| `cancelPurchaseOrder` | must be `DRAFT` or `ORDERED` (not `RECEIVED`/`CANCELED`) | `VALIDATION` (default code) | `يمكن إلغاء المسودات والأوامر المرسلة فقط — استخدم مرتجع المشتريات للأوامر المستلمة` |
| `createPurchaseReturn` | order must exist | `NOT_FOUND` | `أمر الشراء غير موجود` |
| `createPurchaseReturn` | order must be `RECEIVED` | `VALIDATION` (default code) | `يمكن الإرجاع من أوامر الشراء المستلمة فقط` |
| `createPurchaseReturn` | `reason` non-empty after trim | `VALIDATION` (default code) | `سبب الإرجاع مطلوب` |
| `createPurchaseReturn` | at least one line with `qty > 0` | `VALIDATION` (default code) | `اختر صنفاً واحداً على الأقل للإرجاع` |
| `createPurchaseReturn` (per line) | line's product must exist on the PO | `VALIDATION` (default code) | `الصنف غير موجود في أمر الشراء` |
| `createPurchaseReturn` (per line) | `qty` can't exceed what's still returnable (`baseQty(poLine) − already returned`) | `VALIDATION` (default code) | `` لا يمكن إرجاع أكثر من ${remaining} من "${productName}" `` |
| `createPurchaseReturn` (per line, stock-tracked product only) | `qty` can't exceed **current on-hand stock** | `CONFLICT` (explicit — the one function-specific non-default code in this module) | `` المخزون الحالي من "${productName}" (${stockQty}) أقل من كمية الإرجاع `` |
| `postDebitNoteDraft` | draft must exist | `NOT_FOUND` | `مسودة الإرجاع غير موجودة` |
| `postDebitNoteDraft` | draft must have at least one line | `VALIDATION` (default code) | `لا توجد أصناف في المسودة` |
| `postDebitNoteDraft` (per line) | the line's batch must trace to a `RECEIVED` purchase order | `CONFLICT` | `إحدى التشغيلات ليست من أمر شراء مستلم — استخدم الإتلاف بدلاً من الإرجاع لهذه التشغيلة` |
| `postDebitNoteDraft` | all draft lines must trace to the **same** purchase order | `CONFLICT` | `تشغيلات المسودة من أوامر شراء مختلفة — أنشئ مرتجعاً منفصلاً لكل أمر شراء` |
| `getPurchaseOrder` | order must exist | `NOT_FOUND` | `أمر الشراء غير موجود` |
| `getPurchaseReturn` | return must exist | `NOT_FOUND` | `إشعار المدين غير موجود` |

**On the `VALIDATION` vs `NOT_FOUND` inconsistency in `validateLines`:** `savePurchaseOrder`'s
supplier-existence check (`if (!supplier) throw new ApiError('اختر المورد')`) uses the **default**
`ApiError` code (`VALIDATION`), not `NOT_FOUND`, even though semantically the supplier doesn't
exist. This reads as intentional, not a bug — the Arabic message ("Choose a supplier") is phrased
as a form-validation nudge ("you didn't pick one"), not a "this id doesn't exist" error, which is
the right UX for a required dropdown field the user simply left empty or picked a stale id from a
cache. Contrast with `productById`'s `line.productId` checks in the same function, which **do**
raise `NOT_FOUND` — because a missing product mid-line is a genuine data-integrity surprise, not a
"pick something" nudge. Rust should replicate this exact code choice per call site, not
normalize both to `NOT_FOUND`.

**Not re-checked server-side today (frontend Zod only):** none — like `settings.md`, this module
has no dedicated Zod validator file (`src/modules/purchases/validators/` does not exist); every
rule above is an inline `ApiError` check in `purchaseService.ts`/`backend/purchases.ts`, which is
what Rust re-implements 1:1. `PurchaseFormPage.vue` likely layers its own client-side form
validation (required fields, etc.), but that's UI convenience, not a separate rule set this module
needs to port — the mock's own checks above are the full server-side contract.

## 4. Undo matrix (every function that writes)

| Function | Undoable? | Compensation (existing fn) | Refused when | Period rule (D7) |
|---|---|---|---|---|
| `savePurchaseOrder` (draft save/edit) | No, but n/a — a `DRAFT` has no GL/stock effect; editing or leaving it as a permanent unsent draft is not a posting to undo | — | n/a | n/a |
| `savePurchaseOrder` (`confirm: true`) | No via the registry — same reasoning as `receivePurchaseOrder` below, since it delegates straight into `receivePurchase` | none automatic (a purchase return is the natural correction) | n/a | Subject to `assertOpenPeriod(input.date)` inside the delegated `receivePurchase`/`postJournal` call |
| `sendPurchaseOrderToSupplier` | No, but n/a — no GL/stock effect, a pure status flip + timestamp. The natural "undo" is `cancelPurchaseOrder`, which remains available on an `ORDERED` order | `cancelPurchaseOrder` (manual, not the undo registry — a distinct user action) | n/a | n/a |
| `receivePurchaseOrder` / `confirmPurchaseOrder` | **No via the registry — the intended correction is a new `createPurchaseReturn`/`postDebitNoteDraft`**, exactly matching master plan §3 rule 7's model ("undo = calling the existing compensating operation for that document type," which for a posted purchase is a return, never an edit or a reversal-in-place of the receipt itself) | `createPurchaseReturn` (manual, a distinct document — not an automated undo of the receipt) | n/a | Subject to `assertOpenPeriod(input.date)` inside `postJournal` — a receipt dated into a closed period is refused at post time, same as any other document |
| `cancelPurchaseOrder` | No, but n/a — cancels a `DRAFT`/`ORDERED` order that never posted anything, so there's nothing to reverse. There is no "un-cancel"; a canceled order is a dead end (the user creates a fresh PO) | — | n/a | n/a — no posting happens in this function |
| `createPurchaseReturn` | **No via the registry — this function itself IS the compensation** for `receivePurchaseOrder`'s posting. There is no further "undo a return"; correcting an over-return means posting a fresh purchase (a new receipt), not reversing the debit note | n/a (it is itself the undo target for a receipt) | n/a | Subject to `assertOpenPeriod(date)` (defaults to `now()` when the caller doesn't pass one) inside `postJournal` |
| `postDebitNoteDraft` | No — same reasoning as `createPurchaseReturn` (it delegates entirely into it); the `debitNoteDrafts` row it deletes has no GL effect of its own to undo, and the resulting `PurchaseReturn`'s only "undo" would be a fresh purchase, same as above | n/a | n/a | Same as `createPurchaseReturn` |

No function in this module posts through `reverseJournal` (the manual-entry reversal path) —
every ledger-touching write here posts directly via `postJournal` and is corrected only by a new,
opposite document (a purchase return corrects a receipt; nothing corrects a return except a new
purchase), matching `products.md`'s identical conclusion for stock adjustments/transfers and the
master plan §3 rule 7 model exactly.

## 5. Concurrency under D8 (several terminals on one DB)

| Race | Rows | Settled by |
|---|---|---|
| Two terminals `savePurchaseOrder`/`confirmPurchaseOrder`/`postDebitNoteDraft` at the same instant | `counters.purchaseOrder` / `counters.purchaseReturn` (document numbering) | Standard `shared::numbering` mechanism — `SELECT … FOR UPDATE` on the counter row inside the same transaction as the insert, identical to every other document-number sequence in the app. **`savePurchaseOrder`'s number is one shared counter across all branches**, prefixed per-branch (§1/§2) — the same locking row is contended by every branch's purchase orders, not partitioned per branch; matches `products.md`'s `createTransfer` finding exactly, so if branch-partitioned numbering is ever wanted, it should be decided once for both modules together, not module-by-module. |
| Two terminals `receivePurchaseOrder`/`confirmPurchaseOrder`/`createPurchaseReturn`/`postDebitNoteDraft` against the **same product's** stock at (near) the same instant | `products.stock_qty`, `products.stock_value`, `products.cost_price`, `product_branch_stock` | **The same sharpest race `products.md` already identified for `applyStockChange`** — this module is one of that primitive's callers, not a separate risk. A receipt/return must run inside the same transaction as a `SELECT … FOR UPDATE` on the product row, serializing concurrent movements so one doesn't silently overwrite another's average-cost recalculation. No separate lock design needed here — `shared::stock::apply_change` owning this lock (as `products.md` §5 already specifies) covers this module's callers too. |
| Two terminals `receivePurchaseOrder` against the **same PO** (e.g. one terminal receiving the last remaining qty while another submits a partial receipt on the same line) | `purchase_orders`, `purchase_order_lines.received_qty` | **A gap not present in `products.md`, specific to this module:** `receivePurchase`'s own-line "does this exceed what's remaining" check (`rl.receivedQty > lineRemaining(poLine) + 0.0001`) reads `poLine.receivedQty` and computes remaining **before** writing the new cumulative value — under real concurrency, two terminals could both read "5 remaining," both accept a receipt of 5, and the PO ends up over-received (received > ordered) with no single check ever seeing the true post-write state. This needs a `SELECT … FOR UPDATE` on the purchase-order row (not just the product row) held for the whole receiving transaction, so the second terminal's remaining-qty check runs against the first terminal's already-committed `received_qty`, not a stale read. **Flagged as a required Part 02/03 lock, not fixed in the mock** (the mock is single-process, so this race cannot manifest there — nothing to change in `src/mocks/backend/purchases.ts` itself). |
| Two terminals `createPurchaseReturn`/`postDebitNoteDraft` against the **same PO's** "how much is still returnable" figure | `purchase_orders.returned_amount`, `purchase_return_lines` (via `returnedQtyByProduct`) | Same shape as the receiving race above — `returnedQtyByProduct` re-derives the already-returned quantity by re-scanning `purchaseReturns` at read time rather than a maintained counter, so two concurrent returns on the same PO/product need the **same purchase-order row lock** held for the duration of the return's validation-and-post, not a separate mechanism. |
| Two terminals `cancelPurchaseOrder` on the same order, or one cancels while another concurrently `sendPurchaseOrderToSupplier`s/`receivePurchaseOrder`s it | `purchase_orders.status` | A `SELECT … FOR UPDATE` on the purchase-order row for the status check-then-write in every one of `sendPurchaseOrderToSupplier`/`receivePurchaseOrder`/`cancelPurchaseOrder`/`createPurchaseReturn` — the same single lock point serves all of these status-guarded transitions, so this is one lock discipline to implement once in the domain's command layer, not five separate mechanisms. |

## 6. Events and side effects

- **Activity/audit rows** written for **every** write function in this module: `savePurchaseOrder`
  (`logActivity('purchase', …)` on the draft-save path; the `confirm: true` path additionally logs
  through the delegated `receivePurchase` call), `sendPurchaseOrderToSupplier`, `receivePurchaseOrder`/
  `confirmPurchaseOrder`, `cancelPurchaseOrder` (all `activityKind: 'purchase'`), `createPurchaseReturn`/
  `postDebitNoteDraft` (`activityKind: 'purchase_return'`, via `recordPurchaseReturn`). **Confirmed: no
  silent write anywhere in this module** — every state-changing function calls `logActivity`, which
  itself calls `logAudit` internally (`backend/core.ts:331-343`), so there is no case here of an
  activity-feed entry without a matching `db.audit` row, unlike the catalog-master-data gap `products.md`
  flagged for its own module. This module has zero audit-coverage gaps to fix or flag.
- **Events emitted:** `parties:changed` — `receivePurchaseOrder`/`confirmPurchaseOrder` (a supplier's
  payable balance changed) and `createPurchaseReturn`/`postDebitNoteDraft` (same reason, the balance
  moved the other way). **No `ledger:changed` and no `catalog:changed` from this module at all**,
  despite every receive/return posting a journal entry and moving stock — this is a **narrower gap than
  `products.md`'s equivalent finding** (which was "only `ledger:changed`, missing `catalog:changed`");
  here **neither** GL-refresh event fires from a function that demonstrably posts to the GL and moves
  `stockQty`/`stockValue`. A screen listening only to `ledger:changed` (e.g. the journal/GL viewer) will
  not live-refresh after a purchase receipt or return completes on another terminal, and neither will a
  plain stock-aware product list. Flagged as a contract gap in §8/§9 — not fixed here per the task's
  conservative instruction, since emitting a new event from a hot financial-posting path is exactly the
  kind of "safe-looking" change that deserves a deliberate decision rather than an unreviewed patch,
  even though the change itself doesn't touch posting math (only which events fire after it succeeds).
- **Attachments:** `PurchaseOrder.attachmentIds` is a plain array of `AttachmentField` ids
  (ownerRef presumably `purchaseOrder:<id>`, not confirmed by name in this file) — same
  store-the-id-list-only pattern as `products.md`'s `Product.imageIds`; the blob CRUD lives in
  `core`'s `AttachmentField`/`AttachmentViewer`, already seam-clean.
- **Printing:** none directly in these two files — the PO PDF/print (for `sendPurchaseOrderToSupplier`'s
  "send to supplier" flow) and the debit-note PDF (`getPurchaseReturn`'s stated purpose in its own
  doc comment) are separate `pdfService`/print-route calls the page makes, not part of this module's
  service functions.

## 7. Aggregations (reports / analytics / dashboard / insights only)

Not a reporting module — no `report`/`analytics`/`dashboard` output here. But `receivePurchase` and
`recordPurchaseReturn` are the two most posting-heavy functions Part 03 will port, so their exact
accounting math is documented precisely (not a report, a write, described here for the same reason
`products.md` documented `postAdjustment`/transfer postings under this heading):

| Output (DTO field) | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `receivePurchase`'s journal lines | — | **Pass 1** (`receivedValued`): each receiving line's value at the PO's own order price (`baseUnitCost(poLine) × receivedQty`), split service vs. stock. **Landed-cost spread** (`allocateLandedCosts`): each landed-cost line's `amount` is allocated across the stock-only receiving lines by `value` or `qty` weight, and simultaneously bucketed into `ownSupplierTotal` (added to this PO's own AP) or `otherSupplierLines` (a separate AP credit per other supplier, e.g. a shipping company). **Posting**: `Dr inventory (Σ postedValue, i.e. order-price value + landed-cost share, per stock line, then re-averaged into the product via `applyStockChange`) / Dr <service account> (Σ per-account service-line value) / Dr vatInput **or** freightIn (VAT — see the non-VAT-supplier branch below) / Cr payable[supplier] (Σ received value + VAT + ownSupplierTotal) / Cr payable[otherSupplier] (per other-supplier landed-cost line)`. | by `role` (inventory/service-account/vatInput-or-freightIn/payable), and by `partyId` for every payable line (the PO's own supplier, plus one line per distinct other-supplier landed-cost line) | `round2` at **every** intermediate accumulation point, not just once at the end: per-line `value` (pass 1), per-product `shareByProduct` accumulation (inside `allocateLandedCosts`'s loop), `postedValue` per stock line, the running `inventoryValue` total, `vatOnReceipt`, and the final `apAmount`. Rust must round at each of these named points in this exact order — summing already-rounded intermediate values and rounding again at the end, rather than rounding once from an exact sum, is the whole point of the parity harness's cent-level diffing, and `products.md` flagged the identical "round early and often" shape for its own adjustment/transfer postings. | `purchases.ts:228-369` (`receivePurchase`), landed-cost spread at `:187-213` (`allocateLandedCosts`) |
| `receivePurchase`'s non-VAT-supplier branch (review E3, confirmed still in effect) | `suppliers` (`vatNumber` presence) | `vatNotRecoverable = input.vatNotRecoverable ?? !supplier?.vatNumber` — **client can override the auto-detected value**, the mock doesn't force it | — | `round2` on `vatOnReceipt = po.taxAmount × min(1, receivedRatio)`, where `receivedRatio` is itself `round2Totals(Σreceived value / (po.subTotal \|\| 1))` — a short/partial receipt only claims (or cost-adds) a proportional slice of the PO's total VAT, never the full amount on a partial receipt. When `vatNotRecoverable` is true, the VAT amount posts to `freightIn` (labeled "ضريبة مدخلات غير مستردة … أُضيفت إلى التكلفة" — the same "cost, not a dedicated account" fallback role `purchaseLineAccountId` itself defaults to) **instead of** `vatInput`, and is **never routed through `applyStockChange`** — it does not become part of `stockValue`, staying a separate expense-like line even though conceptually it's "added to cost." This is a deliberate simplification (confirmed against docs/v2/02-accounting-review.md E3: "input VAT... is added to the cost, with a warning") that keeps `GL(inventory) = Σ product.stockValue` (A1/A2) exactly, rather than trying to fold VAT into the per-unit stock cost, which would require re-touching every stock line's value a second time. Rust must reproduce this exact non-folding behavior, not "improve" it by actually adding VAT into stock cost — that would be an accounting-math change requiring its own review and regression case, well outside a contract-review fix. | `purchases.ts:299-318` |
| `receivePurchase`'s short-delivery backorder (`input.createBackorder`) | `purchase_orders` (a fresh row) | Only lines still short (`lineRemaining(l) > 0.0001`) after this receipt, re-expressed in the line's own unit (`round2(lineRemaining(l) / (l.unitFactor ?? 1))`) | — | `round2` on the re-expressed backorder-line qty | `purchases.ts:357-366` — calls `savePurchase` recursively with `confirm: false`, so the backorder is always created as its own fresh `DRAFT`, one posting per document preserved (the original PO's own totals are shrunk to what it actually received; the backorder gets entirely separate totals when it's later received) |
| `recordPurchaseReturn`'s journal lines | — | **Refund split** (E1, confirmed fixed): `settledToPayable = min(totals.grandTotal, purchaseOutstanding(po))` first pays down whatever's still outstanding on the original PO's AP; any remainder (`cashBack = round2(totals.grandTotal − settledToPayable)`) goes out through the **chosen** `refundMethod`'s settlement account (`settlementAccountFor` — cash or bank; `'credit'` posts `cashBack` back onto the supplier's payable instead of any cash/bank account, i.e. a `credit`-method "refund" never touches a settlement account at all, both `debit` amounts in that posting-line pair collapse to `0`/`cashBack` depending on which of the two payable-vs-settlement lines is live). **Posting**: `Dr payable[supplier] (settledToPayable) / Dr <settlement account> (cashBack, only if refundMethod ≠ credit) / Dr payable[supplier] (cashBack, only if refundMethod = credit) / Cr inventory (Σ valueOut per stock line) / Cr <service account> (Σ per-account service-line value) / Cr vatInput **or** freightIn (mirrors whichever the original receipt used — see the symmetry note below) / [variance line, either direction]`. | by `role`/`partyId`, same shape as the receipt | `round2` at every accumulation point again: per-line `atPurchasePrice`, `newQty`/`newValue` (the negative-value/zero-qty guard), `valueOut`, the running `variance`, `inventoryValue`, and the final `totals` from `computePurchaseTotals`. | `purchases.ts:396-521` (`recordPurchaseReturn`) |
| `recordPurchaseReturn`'s inventory-variance guard (A1/A2, confirmed against `docs/v2/02-accounting-review.md`'s "Purchase return" section) | `products` | Per stock line: if crediting the line's full `atPurchasePrice` would leave `product.stockValue` negative, **or** leave a non-zero residual value at zero remaining qty (`newQty <= 0.0001 && |newValue| > 0.001`), the credit is capped at exactly what's there (`valueOut = product.stockValue`) and the shortfall (`atPurchasePrice − valueOut`) is booked to `inventoryVariance` (role `inventoryVariance`, net-signed: `Dr` if the accumulated `variance` ends up positive, `Cr` if negative) instead of silently driving `GL(inventory)` away from `Σ product.stockValue`. This is the **exact same guard shape** `products.md` documented for `completeStockCount`'s stocktake postings (both directions can appear, net-signed) — same invariant, different trigger (a return exceeding the cost basis actually on hand, vs. a physical count mismatch). | `round2` on `atPurchasePrice`, `newValue`, `valueOut`, and the running `variance` accumulator | `purchases.ts:459-470` |
| `recordPurchaseReturn`'s non-VAT-supplier symmetry (E3) | `purchase_orders.vat_not_recoverable` (read, not re-derived) | Reads the **flag already stored on the PO from the original receipt** (`po.vatNotRecoverable`), never re-checks the supplier's current VAT-number status — so a debit note against a PO received under the non-VAT branch always credits `freightIn` (mirroring what was debited at receipt), even if the supplier's VAT number was added to their profile afterward. This is correct symmetry (undoing exactly what was posted, not what the current supplier state would suggest), confirmed intentional by reading the receipt/return pair together — a debit note that credited `vatInput` for a receipt that never debited it would silently create a `vatInput` account that never nets to the true recoverable amount. | — | `purchases.ts:501-505` |
| `postDebitNoteFromDraft` | `product_batches` (`sourceRefId` join to `purchase_orders`) | Not its own aggregation — a batch-id → single-PO resolution step, then a straight `recordPurchaseReturn` call; documented in §1, not duplicated here. | — | — | `purchases.ts:547-575` |

## 8. Contract fixes needed in the mock (01.C)

Per the task's conservative instruction for this accounting-sensitive module: **no mock code
changes were applied in this review.** Every bug class named in the session brief was checked and
found either already fixed in the current mock or genuinely absent from this module:

- [x] **(a) Hard-coded Saudi-only VAT-number regex — not applicable.** This module never reads or
      validates a VAT/tax-id string (`suppliers.vatNumber` is only ever *read* — for the
      non-recoverable-VAT branch and the duplicate-invoice-number check — never written or
      pattern-validated here). The bug class flagged in `settings.md`/`parties.md` lives entirely
      in those two modules; nothing to check here beyond confirming absence.
- [x] **(b) `cancelPurchaseOrder` reference/state check before a destructive action — confirmed
      correct, not a gap.** It refuses on `RECEIVED`/`CANCELED` orders (`status !== 'DRAFT' &&
      status !== 'ORDERED'`), directing the user to a purchase return instead once stock/payments
      have moved. This is exactly the guard the task brief asked to verify; it was already present
      and correctly worded, so nothing to fix.
- [x] **(c) Writes with no audit trail — none found.** Every one of this module's 6 write functions
      (`savePurchaseOrder`, `sendPurchaseOrderToSupplier`, `receivePurchaseOrder`/
      `confirmPurchaseOrder`, `cancelPurchaseOrder`, `createPurchaseReturn`/`postDebitNoteDraft`)
      calls `logActivity`, which itself writes `db.audit` (confirmed by reading
      `backend/core.ts:331-343` — `logActivity` is not a separate, lesser trail than `logAudit`, it
      *is* `logAudit` under the hood). No batch gap like `products.md`'s catalog-master-data finding
      exists in this module.
- [x] **(d) Plain `Error` instead of `ApiError` — none found.** All 30 throw sites in
      `src/mocks/backend/purchases.ts` use `ApiError` (`grep` count above: 30/30). Unlike
      `settings.md`'s `restoreFromArchive` gap, this module has no plain-`Error` throw anywhere.
- [x] **(e) Silent skip instead of throwing on a validation-shaped condition — none found that rise
      to a bug.** The one silent-`continue` pattern in this file (`allocateLandedCosts`'s
      `if (!(lc.amount > 0)) continue`) skips a landed-cost line with a non-positive amount rather
      than throwing — but a `0`/negative landed-cost line is a no-op by construction (there's
      nothing to spread), not a validation failure a user would expect to be told about; the form
      itself wouldn't normally produce a negative landed-cost amount, and silently ignoring "zero
      freight" is the same class of harmless default `setPriceListValues` uses for an unmatched
      `productId` in `products.md`. Not flagged as a fix candidate.

Two items are genuine contract observations, but neither is a mock **bug** — both are recorded here
because they're the kind of finding this phase exists to surface, and both are also listed in §9
since they need a decision, not a guess:

- [ ] **`currency`/`exchangeRate` fields are inert.** `PurchaseOrder`/`PurchaseOrderInput` carry
      FX fields that no code path in `purchases.ts` reads (§2) — confirm whether Part 03's purchases
      domain is expected to wire these up (multi-currency purchase orders) or whether they're
      vestigial from a phase-9 feature that shipped only on the sales side. **Not changed here** —
      wiring up FX conversion in a purchase-posting function is exactly the kind of accounting-math
      change this review is instructed not to guess at.
- [ ] **Neither `ledger:changed` nor `catalog:changed` fires from any posting function in this
      module** (§6) — `receivePurchaseOrder`/`confirmPurchaseOrder`/`createPurchaseReturn`/
      `postDebitNoteDraft` all emit only `parties:changed`, despite posting journal entries and
      moving stock. **Not changed here** — even though adding an `emit('ledger:changed')` call
      looks low-risk (it's a UI-refresh signal, not a posting-math change), the task's conservative
      instruction was to flag rather than guess at anything in this hot financial-posting path;
      recorded as a §9 decision instead of an applied fix.

## 9. Open questions (→ decisions in `00-MASTER-PLAN.md`)

Nothing in this module needed an immediately-applicable fix (§8), so everything below is either a
cross-module pattern already flagged once in `products.md` (recorded here only for completeness,
not re-litigated) or a genuinely new question specific to purchases:

- **Missing `ledger:changed`/`catalog:changed` events on every posting function** (§6/§8): should
  `receivePurchaseOrder`, `confirmPurchaseOrder`, `createPurchaseReturn` and `postDebitNoteDraft`
  also emit `ledger:changed` (they post journal entries and change stock, but today only signal
  `parties:changed`)? This is a **stronger version** of the gap `products.md` flagged for
  `sendTransfer`/`receiveTransfer`/`rejectTransfer` (which at least emit `ledger:changed`) — here
  neither GL nor stock-refresh event fires at all from a purchase receipt or return. Recommend
  adding `emit('ledger:changed')` alongside the existing `emit('parties:changed')` on all four
  functions for consistency with every other GL-posting function in the app, but **not applied
  here** since it touches the same hot posting functions this review was told to be conservative
  about, even though the change itself is UI-refresh-only. Needs a decision before Part 03 wires up
  cross-terminal refresh under D8.
- **`currency`/`exchangeRate` on `PurchaseOrder` are inert** (§2/§8): confirm whether multi-currency
  purchase orders are in scope for Part 03 (in which case Rust needs real `toBase()`/FX-conversion
  logic this mock never implements — a genuinely new accounting feature, not a port) or whether
  these fields should be dropped/ignored as unfinished. **Needs a decision** — this is a
  scope-defining question for the domain, not a narrow bug.
- **`PurchaseReturn.number` has no branch prefix, unlike `PurchaseOrder.number`** (§2): both are
  branch-scoped documents in a multi-branch store, but only purchase orders get `branchPrefix()`.
  Confirmed as read from the code, not assumed a bug — recommend deciding once, together with
  `products.md`'s identical "one shared counter across branches" question for `StockTransfer`, so
  Part 02's numbering design handles all of these consistently rather than case-by-case.
- **Cross-referenced, not re-opened here:** the "one shared `purchaseOrder` counter across all
  branches, just prefixed" pattern (§1/§5) is the same shape `products.md` already flagged for
  `createTransfer`'s `stockTransfer` counter — listed here for this module's own record, but the
  actual decision (global vs. per-branch counters) should be made once for every affected document
  type, not module-by-module.
- **The receiving/returning concurrency gap under D8** (§5: a purchase-order row lock is needed for
  the receiving and returning "how much is remaining/returnable" checks, not just the product-row
  lock `products.md` already specifies) is a **Part 02/03 implementation requirement**, not an open
  question needing a decision — recorded in §5 as a design note for whoever builds
  `domains/purchases/service.rs`, not repeated as a question here.

## Gate

- [x] Every inventory function is in §1 with a confirmed disposition (12/12, all `port`; no
      disposition changed from the generator's own heuristic, so no `scripts/contract/config.ts`
      override was needed for this module).
- [x] Every write function is in §4 (6 write functions: `savePurchaseOrder`,
      `sendPurchaseOrderToSupplier`, `receivePurchaseOrder`/`confirmPurchaseOrder`,
      `cancelPurchaseOrder`, `createPurchaseReturn`/`postDebitNoteDraft`).
- [x] Every DTO field needing a non-default mapping is in §2.
- [x] `bun run contract:check` is green (no override added — `bun run contract` re-run to confirm
      no drift; no mock code was changed in this review, so no verify:mocks re-check was strictly
      needed, but it was re-run anyway to confirm the baseline). Verified: `bun run verify:mocks`
      **128 ok, 0 failed** both before and after this review (no mock file was edited). Also green:
      `bun run build`, `bun run check`, `bun run memory:check` (0 new seam violations),
      `bun run diag:check`.
