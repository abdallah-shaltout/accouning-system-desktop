# 21 · 01.B — `invoices` contract

> **Status:** done (2026-09-27) · **Inventory:** `docs/backend/contract/invoices.md`
> (regenerate with `bun run contract`) · **Mock spec:** `src/mocks/backend/sales.ts` (createSale/
> createRefund posting engine — the accounting-sensitive core of this module), `src/mocks/backend/shifts.ts`
> (open/close/force-close shift, cash-in/out, X/Z report), `src/mocks/backend/currency.ts` (FX —
> already reviewed in `settings.md`, reused here as-is), `src/mocks/backend/accounts.ts`
> (`settlementAccountFor`), `src/mocks/backend/inventory.ts` (`consumeFefo`, reused from
> `products.md`), `src/mocks/backend/core.ts` (`applyStockChange`, `postJournal`,
> `assertOpenPeriod`, `logActivity`/`logAudit`) · **Services:** `src/modules/invoices/services/invoiceService.ts`
> · **Types:** `src/modules/invoices/types/index.ts`
>
> Sales (`createSale`), refunds (`createRefund`), quotations, held sales (POS "F6"), and POS shifts
> (open/close/force-close, cash-in/out, X/Z report) — the most accounting-sensitive module reviewed
> so far in this session: every sale line consumes weighted-average cost (and FEFO batches for
> batch-tracked products), split-tender payments post one line per method to its own
> clearing/settlement account, sales can be multi-currency (FX, converted to base at posting time,
> reusing `settings.md`'s `toBase`/`convertLinesToBase`/`requireRate` exactly), and shift close posts
> a cash over/short variance plus an optional cash-drop transfer voucher. **Out of scope, pre-existing
> from plan 22 (in-progress, uncommitted invoice-templates work):** `copyInvoiceImage` and
> `saveInvoiceImage` in `src/modules/invoices/services/invoiceImageService.ts` — both already show up
> correctly as `frontend` in the regenerated inventory (webview/plugin-only, no backend data access).
> Neither was reviewed, modified, or judged for correctness here, nor were
> `src/modules/invoices/components/templates/`, `src/modules/invoices/controllers/useInvoiceDoc.ts`,
> or `src/modules/invoices/helpers/invoiceTemplates.ts` — someone else's in-progress work.

## 1. Endpoints

28 functions total in the regenerated inventory (26 core + 2 out-of-scope plan-22 additions), one
service file (`invoiceService.ts`), plus `invoiceImageService.ts` for the two plan-22 functions.
Disposition: `port` · `frontend`.

| Function | Disposition (confirmed / changed + why) | Rust command | Request DTO | Response DTO | Writes (confirmed) | Shared | Undo | Notes |
|---|---|---|---|---|---|---|---|---|
| `getInvoices` | port (confirmed) | `invoices_get_invoices` | `InvoiceFilter & { openOnly?: boolean }` | `InvoiceRow[]` | — | — | n/a (read) | `toRow` computes `outstanding` live (`invoiceOutstanding`, never stored). `filter.search` matches `[number, customerName]` via `includesText` (Arabic-normalizing, cross-cutting 01.D). Sorted by `date` descending. |
| `getInvoicesPaged` | port (confirmed) | `invoices_get_invoices_paged` | `PagedQuery<InvoiceFilter & { openOnly?: boolean }>` | `PagedResult<InvoiceRow>` | — (generator's "writes: invoices" is a **false positive** — confirmed by reading the body: it only `.filter`/`.map`/`.sort`/`.slice`s a local copy, no `mutate()` anywhere; the array methods the generator flags, e.g. `.sort`, run on the locally-derived `rows` array, not `db.invoices` itself) | — | n/a (read) | Same filtering as `getInvoices` plus server-side sort/paging and two aggregate `totals` fields (`grandTotal − refundedAmount` summed, `outstanding` summed) computed over the **filtered, unpaged** set — Rust must compute these as a second aggregate query (or a single query with window aggregates), not just sum the returned page. |
| `getInvoice` | port (confirmed) | `invoices_get_invoice` | `{ id: string }` | `InvoiceDetail` | — | — | n/a (read) | Assembles a detail view: refunds on this invoice, payments allocated to it (`payments[].allocations` where `targetKind === 'invoice'`), every journal entry whose `sourceRef.id` is the invoice **or one of its refunds or payments** (`sourceIds` set), and `returnedQty` per line (`returnedQtyByLine`). Rust must join across all three source-ref kinds, not just the invoice's own id. |
| `previewSale` | port (confirmed) | `invoices_preview_sale` | `SaleInput` | `JournalPreviewLine[]` | — | currency | n/a (read) | **Read-only preview of the exact posting `createSale` would produce** — calls the same `prepareSale`/`previewSaleJournal` internals. Must stay byte-identical to what `createSale` actually posts (same rounding, same FX conversion, same tender-account resolution), since the POS/desk-form UI shows this before the user commits. Zero-value lines (`debit`/`credit` both 0) are filtered out before returning. |
| `createSale` | port (confirmed) | `invoices_create_sale` | `SaleInput` | `Invoice` | `invoices`, `journalEntries`, `counters`, `productBatches`, `stockMovements`, activity, audit | activity, currency, ledger, numbering, period, stock | **not undoable via the registry — the compensation is `createRefund`**, a distinct document, never an edit/reversal-in-place of the sale itself | See §7 for the full posting walkthrough. Credit-limit check (`assertWithinCreditLimit`, `parties/helpers/creditLimit.ts`) runs in the **service layer**, in front of `recordSale`, using the same `previewSaleJournal` computation to find the would-be new receivable — Rust must run this check inside the same transaction as the post, not as a separate pre-flight call (a raced concurrent sale could otherwise slip through between check and post; see §5). `dueDate` is stamped by the service after `recordSale` returns (a second small write on the same invoice object, from `input.dueDateOverride` or `computeDueDate(date, customer.paymentTermsDays)`) — Rust should fold this into the same insert, not a follow-up update. |
| `createRefund` | port (confirmed) | `invoices_create_refund` | `RefundInput` | `Refund` | `refunds`, `invoices` (`refundedAmount`, `status`, `paymentStatus`), `journalEntries`, `counters`, activity, audit | activity, ledger, numbering, period, stock | **not undoable — this function itself IS the compensation** for `createSale`'s posting (master plan §3 rule 7's model exactly: a sales return corrects a sale, nothing corrects a refund except a fresh sale) | See §7 for the full posting walkthrough (three-way refund settlement: pay down outstanding first, then cash/card/bank/customer-credit for the rest; restock-vs-write-off per line). |
| `getRefund` | port (confirmed) | `invoices_get_refund` | `{ id: string }` | `Refund` | — | — | n/a (read) | No dedicated detail page/route — feeds the credit-note PDF payload only (docs/v2/12 §3), same pattern as `purchases.md`'s `getPurchaseReturn`. |
| `getInvoicePrintData` | port (confirmed) | `invoices_get_invoice_print_data` | `{ id: string }` | `PrintData` | — | — | n/a (read) | **`id === 'sample'` is a special case that returns a fabricated demo invoice, never saved** — used only by the settings "test print" screen. Rust must keep this exact synthetic branch (reading the store's real active sales-tax rate via `salesTaxRate()`, not a hard-coded rate — v2 doc 18.D fix) rather than treating `'sample'` as a real lookup that 404s. |
| `openPosShift` | port (confirmed) | `invoices_open_pos_shift` | `OpenShiftInput` | `Shift` | `shifts`, activity, audit | activity, numbering | not undoable — no compensation exists or is needed; a mistakenly-opened shift is closed (`closePosShift`/`forceClosePosShift`), never "un-opened" | Refuses if a shift is already `OPEN` on that `terminalId` (`CONFLICT`) or `openingFloat < 0`. |
| `getXReport` | port (confirmed) | `invoices_get_x_report` | `{ shiftId: string }` | `ShiftRow` | — | — | n/a (read) | Thin re-export of `getShift` — "mid-shift snapshot, same shape as the close screen, without closing anything." Rust should call the same `shared`/domain read helper `getShift` uses, not duplicate the summary computation. |
| `closePosShift` | port (confirmed) | `invoices_close_pos_shift` | `{ shiftId: string, input: CloseShiftInput }` | `Shift` | `shifts`, `journalEntries` (variance, conditionally), `vouchers`+`journalEntries` (cash-drop, conditionally), `counters`, activity, audit | activity, ledger, numbering, period | **not undoable — see §4.** A closed shift cannot be reopened; the only "fix" for a wrong count is a fresh manual journal adjustment, a genuinely different accounting action, not a compensation of the close itself | **The most accounting-sensitive endpoint in this module besides `createSale`/`createRefund`** — see §5 point 3 and §7 for the full walkthrough (variance posting + optional cash-drop transfer voucher, both inside one logical close). |
| `forceClosePosShift` | port (confirmed) | `invoices_force_close_pos_shift` | `{ shiftId: string, countedCash?: number }` | `Shift` | (delegates entirely to `closeShift`'s write set) | activity, ledger, numbering, period | not undoable — same as `closePosShift` | Manager screen (`/pos/shifts`) for a shift a cashier left open. Defaults `countedCash` to `expectedCash` when omitted (0 variance), and always posts with `handoverMode: 'HANDOVER'` — a force-close can never trigger the cash-drop branch, by construction, regardless of what the shift's own settings might suggest. Stamps `forceClosedBy` as a **second small mutation after `closeShift` returns** (same "two-step write" shape as `createSale`'s `dueDate` stamp) — Rust should fold this into the same transaction/insert, not a follow-up update. |
| `recordCashInOut` | port (confirmed) | `invoices_record_cash_in_out` | `{ terminalId: string, kind: 'PAY_IN' \| 'PAY_OUT' \| 'BANK_DROP', amount: number, note?: string }` | `void` | `shifts` (movement log only — **no GL posting**) | — | not undoable (a mis-entered pay-in/out is corrected by a new opposite movement, not a reversal) | **Never touches the ledger** — only appends to `shift.movements` for the X/Z report and close-screen expected-cash math (confirmed: no `postJournal` call anywhere in this function or `recordShiftMovement`). Refuses when no shift is open (`CONFLICT`) or `amount <= 0`. Emits `ledger:changed` even though nothing posted to the GL — see §6/§8, a naming mismatch, not a functional gap. **Known incomplete feature, confirmed not a bug:** the code comment (`invoiceService.ts:308-317`) explicitly documents that a `PAY_OUT` should eventually also create a categorized expense voucher (`expenseService.ts::recordExpense`) but intentionally doesn't yet — the drawer accounting itself (this movement log) is correct either way; only a *separate* expense-side journal entry is the still-missing piece. Not a fix candidate for this review. |
| `getHeldSales` | port (confirmed) | `invoices_get_held_sales` | `{ terminalId: string }` | `HeldSale[]` | — | — | n/a (read) | Sorted by `heldAt` descending. |
| `holdSale` | port (confirmed) | `invoices_hold_sale` | `Omit<HeldSale, 'id' \| 'heldAt' \| 'heldBy'>` | `HeldSale` | `heldSales` | — | n/a — no GL/stock effect, a parked cart | **No audit/activity row** — see §6/§8. Not treated as a bug fix here (see reasoning in §8): a held sale is a personal, transient POS cart snapshot, not a posted document or master data, so the session's "writes with no audit trail" bug class is judged not to apply with the same force it did for `products.md`'s `deleteDraftAdjustment` (an inventory-adjacent master-data delete). Flagged as a genuine open question in §9 instead of guessed at. |
| `resumeHeldSale` | port (confirmed) | `invoices_resume_held_sale` | `{ id: string }` | `HeldSale` | `heldSales` (delete) | — | n/a | Refuses `NOT_FOUND` if the id doesn't exist. Same no-audit-row situation as `holdSale`. |
| `discardHeldSale` | port (confirmed) | `invoices_discard_held_sale` | `{ id: string }` | `void` | `heldSales` (delete) | — | n/a | **Idempotent delete, not a bug:** if the id doesn't exist, the `.filter()` is a silent no-op (no error) — this reads as intentional "delete if present" REST semantics (discarding a cart that's already gone is not an error state a cashier needs to see), not the session's "silent skip instead of throwing" bug class (which targets validation-shaped conditions a user would expect to be told about). Not changed. |
| `getQuotations` | port (confirmed) | `invoices_get_quotations` | `{ status?: QuotationStatus, search?: string }` | `QuotationRow[]` | — | — | n/a (read) | Sorted by `date` descending. |
| `getQuotation` | port (confirmed) | `invoices_get_quotation` | `{ id: string }` | `QuotationRow` | — | — | n/a (read) | |
| `saveQuotation` | port (confirmed) | `invoices_save_quotation` | inline input type (see contract inventory; effectively `QuotationInput`) | `Quotation` | `quotations`, `counters` | numbering | not undoable — a quotation never posts to the GL or touches stock (by design, per the type's own doc comment), so there is nothing to compensate; editing/deleting a draft quotation is a plain CRUD action outside this endpoint's own scope (there is no `updateQuotation`/`deleteQuotation` — only `setQuotationStatus`) | Uses the **same** `computeInvoiceTotals` engine as `createSale` (imported dynamically: `await import('../helpers/totals')`), so its totals math must byte-match a sale's, but it **never calls `postJournal` or `applyStockChange`** — confirmed by reading the function body end to end. |
| `setQuotationStatus` | port (confirmed) | `invoices_set_quotation_status` | `{ id: string, status: QuotationStatus }` | `Quotation` | `quotations` | — | not undoable | Plain status flip, no guard on which transitions are legal (any status → any status is accepted) — confirmed as read, not assumed a gap; the UI presumably only offers the sensible transitions (SENT→ACCEPTED/REJECTED etc.), same trust-the-caller shape settings.md found in `updateCostCenter`. **No audit/activity row** — see §6/§8; same reasoning as held sales (a quotation status change is business-visible but non-financial, and the function is a thin CRUD toggle) — flagged, not fixed. |
| `convertQuotationToInvoice` | port (confirmed) | `invoices_convert_quotation_to_invoice` | `{ id: string, payment: { paymentMethod, paidAmount, tenderedAmount? } }` | `Invoice` | `quotations` (`status`, `convertedInvoiceId`) + everything `createSale` writes | (delegates to `createSale`'s shared managers) | not undoable via the registry — same as `createSale`, since it **is** a sale once converted; a `createRefund` against the resulting invoice is the correction path, same as any other sale | Refuses `CONFLICT` if the quotation was already converted (`convertedInvoiceId` set) — correctly guards against double-conversion. Calls the **exported `createSale` wrapped service function**, not the raw `recordSale` mock internal — Rust's command must run the full `createSale` command body (credit-limit check included) inside the same transaction as the quotation-status update, not call a lower-level function that skips it. The `quotations.status = 'ACCEPTED'`/`convertedInvoiceId` write happens **after** `createSale` returns — both must commit together or not at all. |
| `isOverdue` | **frontend** (confirmed) | — | — | — | — | — | n/a | Pure computation (`invoiceOutstanding(inv) > 0 && inv.dueDate < now`) — no data access, callable with just the fields already on hand. Used both as a service export and inline by `matchesFilter`. |
| `isTauriMode` (n/a — not in this module) | — | — | — | — | — | — | — | not applicable, listed for completeness only in settings.md |
| `copyInvoiceImage` | **frontend** (confirmed, out of scope — plan 22) | — | — | — | — | — | n/a | Plan 22's in-progress work (`invoiceImageService.ts`). Correctly classified by the generator (webview/`document`/`navigator` only). Not reviewed further. |
| `saveInvoiceImage` | **frontend** (confirmed, out of scope — plan 22) | — | — | — | — | — | n/a | Same file, same plan-22 scope note. Uses `@tauri-apps/api`/`plugin-dialog` — a native save, correctly `frontend`. Not reviewed further. |

## 2. DTOs → Rust

Only what differs from the generator's hint or needs a scale/enum/relationship decision.

| Type | Field | Rust type | Column | Why |
|---|---|---|---|---|
| `Invoice` | `subTotal` / `discountAmount` / `taxAmount` / `grandTotal` / `paidAmount` / `refundedAmount` / `tenderedAmount` | `Decimal` (all) | `DECIMAL(19,2)` | Money, `round2` throughout `computeInvoiceTotals`/`prepareSale`/`recordSale`. |
| `Invoice` | `discountRate` / `taxRate` | `Decimal` (both) | `DECIMAL(9,4)` | Percentages, not money — same scale as `settings.md`'s `Tax.rate`. `taxRate` is a **snapshot of `lineTaxes[0]?.rate`** (the first line's rate stands in for "the" invoice rate, for legacy print/refund/report code that predates per-line tax) — not derived from `taxAmount`/`subTotal`, so Rust must persist it as its own column, not compute it on read. |
| `Invoice` | `status` | `enum InvoiceStatus { Draft, Completed, Refunded }` | `ENUM('DRAFT','COMPLETED','REFUNDED')` | `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]`. **Note:** `DRAFT` is in the type union but **no code path in `sales.ts` ever creates an invoice with this status** — `recordSale` always sets `status: 'COMPLETED'` directly; `DRAFT` appears to be vestigial from an earlier design (quotations now own the "not yet posted" concept as their own entity). Not a bug — flagged in §9 for confirmation before Rust decides whether to keep the variant at all. |
| `Invoice` | `paymentStatus` | reuse the shared `PaymentStatus` enum (`purchases.md` already specified this as a cross-module type, defined once) | `ENUM('UNPAID','PARTIALLY_PAID','PAID')` | `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]`. Computed by `paymentStatusFor(total, paid)` (`invoices/helpers/totals.ts:233`) — a pure function of two already-rounded decimals, safe to port verbatim as a Rust helper in the same `shared`/domain-adjacent module both `invoices` and `purchases` domains call. |
| `Invoice` | `paymentMethod` | `enum SalePaymentMethod { Cash, Card, BankTransfer, Credit }` | `ENUM('cash','card','bank_transfer','credit')` | `#[serde(rename_all = "snake_case")]` — legacy single-tender snapshot, kept for old print/report code paths even when `tenders[]` (below) is the real posting source of truth. |
| `Invoice` | `source` | `enum InvoiceSource { Pos, Desk }` | `ENUM('POS','DESK')` | `#[serde(rename_all = "UPPERCASE")]`. Defaults to `'POS'` server-side when omitted (`recordSale`'s `input.source ?? 'POS'`) — Rust must apply the same default, not leave the column nullable. |
| `Invoice` | `invoiceType` | `Option<enum InvoiceType { Standard, Simplified }>` | `ENUM('STANDARD','SIMPLIFIED')` nullable | `#[serde(rename_all = "UPPERCASE")]`. Passed straight through from `input.invoiceType` with **no server-side derivation or validation** in `sales.ts` — confirmed by reading the whole `recordSale` body; the "automatic" logic the type's doc comment (`docs/v2/06 §2 "Invoice type is automatic"`) refers to must live in the desk-form UI, not this function. Flagged for confirmation in §9 (a UI-only rule Rust can't enforce unless it's ported too). |
| `Invoice` | `tenders` | `Vec<Tender>` | child table `invoice_tenders(invoice_id, payment_method_id, amount, reference)` | Always present in a Rust-posted invoice (even the legacy single-method path gets synthesized into a one-element `tenders` array by `prepareSale` before `recordSale` ever writes the row — confirmed at `sales.ts:310`, `tenders: tenders.map(...)`) — so the Rust schema can make this a real child table with **no nullable legacy fallback needed**, unlike the mock's optional field (kept optional in TS only for pre-v2 callers that never existed in Rust). |
| `Tender` | `amount` | `Decimal` | `DECIMAL(19,2)` | Money. |
| `Invoice` | `currency` / `exchangeRate` | `Option<String>` (ISO code) / `Option<Decimal>` | `VARCHAR(3)` / `DECIMAL(19,6)` | FX rate scale matches `settings.md`'s `ExchangeRate.rate` (6dp) — **this module is the one where FX fields are actually wired up and posting-tested** (unlike `purchases.md`'s inert `currency`/`exchangeRate`, confirmed there as unused): `prepareSale` reads `input.currency`/`input.exchangeRate` (falling back to `requireRate(currency)`), converts every credit-side posting line to base via `convertLinesToBase` with the documented "largest line absorbs the rounding gap" rule (docs/v2/10 §2), and COGS/inventory post in base currency unconverted either way (cost basis is never FX). Rust's `shared::currency` module (already speced from `settings.md`) is the **direct, tested** dependency here, not a future one. |
| `Invoice` | `branchId` / `costCenterId` | `Uuid` / `Uuid` | `UUID` | `branchId` defaults to `DEFAULT_BRANCH_ID` when omitted; `costCenterId` resolves via `defaultCostCenterFor(branchId, input.costCenterId)` (same helper `purchases.md`/`products.md` already documented) — always resolved to a real id before the row is written, never left null. |
| `Invoice` | `attachmentIds` | `Vec<Uuid>` | child table or JSON array | Same ownership pattern as `purchases.md`'s `PurchaseOrder.attachmentIds` / `products.md`'s `Product.imageIds` — id list only, blob storage lives in `core`'s `AttachmentField`. |
| `InvoiceLine` | `qty` / `unitFactor` | `Decimal` (both) | `DECIMAL(19,4)` / `DECIMAL(19,6)` | **Rounding point note (same pattern `purchases.md`/`products.md` already flagged):** `baseQty()` (`sales.ts:81-83`) rounds with `round2`, not `round4`, despite qty conceptually deserving more precision — Rust must match this exact 2dp rounding point for base-unit qty aggregation, reconfirming the master-plan documentation discrepancy (rule 5 says qty = round4; the mock actually uses round2 everywhere qty is touched) rather than treating it as a new finding. `unitFactor` itself (the *conversion factor*, not the qty) keeps 6dp precision, same as `purchases.md`'s `PurchaseLine.unitFactor`. |
| `InvoiceLine` | `price` / `costPrice` / `discount` / `net` / `vat` / `listPrice` | `Decimal` (all) | `DECIMAL(19,2)` | Money, `round2` via `computeInvoiceTotals`. `costPrice` is a **snapshot of the product's cost at sale time** (`product.costPrice`), never re-derived later — a subsequent purchase/weighted-average change never retroactively changes a posted invoice's line cost. |
| `InvoiceLine` | `taxId` / `taxCategory` / `taxRate` | `Option<Uuid>` / `enum TaxCategory` (shared from `settings`) / `Decimal` | `UUID` nullable / `ENUM` / `DECIMAL(9,4)` | Snapshotted per line at sale time via `taxForLine()` — a tax later edited/deleted in Settings never changes a historical invoice's line-level VAT category or rate. `taxId` can be `None` when no tax was assigned (falls back to `{ rate: 0, category: 'O' }` — out-of-scope, not "0% standard"). |
| `InvoiceLine` | `batchId` / `batchNo` | `Option<Uuid>` / `Option<String>` | `UUID` nullable / `VARCHAR` nullable | The manually-picked-or-FEFO-resolved batch this line drew from (batch-tracked products only) — reused from `products.md`'s `ProductBatch` entity, no new modeling needed here. |
| `InvoiceLine` | `isFreeText` / `revenueAccountId` | `bool` / `Option<Uuid>` | `BOOLEAN` / `UUID` nullable | Desk-form free-text service line not tied to a catalog product — `productId` on such a line is a **synthetic string** (`freetext-${i}`, confirmed at `sales.ts:266`), never a real product FK. Rust's `product_id` column on `invoice_lines` must therefore be **nullable with no FK constraint enforced**, or the synthetic-id convention must be dropped entirely in favor of a genuinely nullable FK — recommend the latter for Rust (a real `NULL` is cleaner than a fake string id), a schema-only decision, not a behavior change, flagged in §9. |
| `Quotation` | `subTotal`/`discountAmount`/`taxAmount`/`grandTotal` | `Decimal` (all) | `DECIMAL(19,2)` | Money, same `computeInvoiceTotals` engine as `Invoice`. |
| `Quotation` | `status` | `enum QuotationStatus { Draft, Sent, Accepted, Rejected, Expired }` | `ENUM(...)` | `#[serde(rename_all = "UPPERCASE")]`. |
| `Quotation` | `convertedInvoiceId` | `Option<Uuid>` | `UUID` nullable, FK to `invoices` | Set exactly once, by `convertQuotationToInvoice`; never cleared. |
| `HeldSale` | `discountRate` | `Decimal` | `DECIMAL(9,4)` | Percentage. |
| `HeldSale` | `lines[].qty`/`price`/`discount`/`listPrice` | `Decimal` (all) | `DECIMAL(19,4)`/`DECIMAL(19,2)`×3 | A held sale is an **unvalidated draft cart** — none of its numbers have been through `computeInvoiceTotals` yet (that only happens on `resumeHeldSale` → re-entering the POS cart → eventual `createSale`), so Rust should store these as plain decimals with no invariant checks at write time, matching the mock's total absence of validation in `holdSale`. |
| `Shift` | `openingFloat` / `countedCash` / `expectedCash` / `variance` | `Decimal` (all) | `DECIMAL(19,2)` | Money, `round2`. `variance = countedCash − expectedCash`, signed (can be negative). |
| `Shift` | `status` | `enum ShiftStatus { Open, Closed }` | `ENUM('OPEN','CLOSED')` | `#[serde(rename_all = "UPPERCASE")]`. |
| `Shift` | `handoverMode` | `Option<enum HandoverMode { Handover, Drop }>` | `ENUM('HANDOVER','DROP')` nullable | `#[serde(rename_all = "UPPERCASE")]`. Defaults to `'HANDOVER'` server-side when the close input omits it (`closeShift`'s `input.handoverMode ?? 'HANDOVER'`). |
| `Shift` | `movements` | `Vec<ShiftMovement>` | child table `shift_movements(id, shift_id, kind, amount, note, ref_id, ref_number, at, by)` | Append-only log — no update/delete anywhere in the mock (confirmed: `recordShiftMovement` only ever `.push()`es). |
| `ShiftMovement` | `kind` | `enum ShiftMovementKind { SaleCash, RefundCash, PayIn, PayOut, BankDrop }` | `ENUM('SALE_CASH','REFUND_CASH','PAY_IN','PAY_OUT','BANK_DROP')` | `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]`. |
| `ShiftMovement` | `amount` | `Decimal` | `DECIMAL(19,2)` | Money, always positive (direction is implied by `kind`, never a signed amount — confirmed: every call site passes a positive value, e.g. `recordShiftMovement(..., cashTendered, ...)` where `cashTendered` is a sum of positive tenders). |
| `Refund` | `subTotal`/`taxAmount`/`grandTotal`/`settledToReceivable`/`cashBack`/`creditedToAccount` | `Decimal` (all) | `DECIMAL(19,2)` | Money, `round2`. `settledToReceivable`'s TS type is bare `number` (not generator-hinted as `_decimal_`, same situation `purchases.md` found for `PurchaseReturn.settledToPayable`) — confirmed money by reading `recordRefund` (`Math.min(grandTotal, invoiceOutstanding(invoice))`). |
| `Refund` | `refundMethod` | `enum RefundMethod { Cash, Card, BankTransfer, CustomerCredit }` | `ENUM('cash','card','bank_transfer','customer_credit')` | `#[serde(rename_all = "snake_case")]`. Defaults server-side to `invoice.paymentMethod === 'credit' ? 'cash' : invoice.paymentMethod` when the input omits it — Rust must replicate this exact fallback (a credit-sale refund can never default to `'credit'`/`'customer_credit'` implicitly; it explicitly falls back to cash unless the caller opts into `customer_credit`). |
| `Refund` | `lines` | `Vec<{ invoice_line_id: Uuid, qty: Decimal, restock: bool }>` | child table `refund_lines(refund_id, invoice_line_id, qty, restock)` | `restock` defaults to `true` when omitted in the input (every line not explicitly `restock: false` goes back to stock) — confirmed by `line.restock === false` being the only branch that write-offs (`sales.ts:487`), so `undefined`/`true` both restock. |
| Route field (activity/audit `link` only) | `/invoices/${invoice.id}` — used by **both** `createSale`'s and `createRefund`'s `logActivity` calls, and `'/pos/shifts'` for `openShift`/`closeShift` | stays a path string in `db.audit`/`db.activity`, parsed server-side by `entityFromLink` (`kind` param disambiguates `'sale'` vs `'refund'` even though the link shape is identical) — same explicit `route-ok` pattern `purchases.md`/`products.md` already confirmed | — | **Not part of F7's 61-link remediation count** — no DTO returned to the UI carries a raw path-string route field in this module (confirmed: no `_route_`-hinted field appears in `docs/backend/contract/invoices.md`'s type list). Only internal audit-log links, same conclusion as the last two modules. |

## 3. Validation and errors

| Function | Rule (source: mock check line) | Code | Exact Arabic message |
|---|---|---|---|
| `createSale`/`previewSale` (`prepareSale`) | cart must not be empty | `VALIDATION` (default code) | `السلة فارغة` |
| `createSale`/`previewSale` | discount above the cashier's `maxDiscount` requires `managerApprovedBy` | `FORBIDDEN` | `` الخصم يتجاوز الحد المسموح لك (${user.maxDiscount}%) `` |
| `createSale`/`previewSale` | `discountRate` in `[0, 100]` | `VALIDATION` (default code) | `نسبة الخصم غير صحيحة` |
| `createSale`/`previewSale` (per line) | `qty > 0` | `VALIDATION` (default code) | `الكمية يجب أن تكون أكبر من صفر` |
| `createSale`/`previewSale` (per line) | `price >= 0` | `VALIDATION` (default code) | `السعر لا يمكن أن يكون سالباً` |
| `createSale`/`previewSale` (per line, free-text) | `revenueAccountId` required | `VALIDATION` (default code) | `السطر النصي الحر يحتاج حساب إيراد` |
| `createSale`/`previewSale` (per line, catalog product) | product must be active | `VALIDATION` (default code) | `` المنتج "${product.name}" غير نشط `` |
| `createSale`/`previewSale` (aggregated per product) | requested base qty can't exceed `product.stockQty` | `CONFLICT` | `` الكمية المطلوبة من "${product.name}" غير متوفرة — المتاح ${product.stockQty} `` |
| `createSale`/`previewSale` | `paidAmount` (sum of tenders) can't be negative | `VALIDATION` (default code) | `المبلغ المدفوع غير صحيح` |
| `createSale`/`previewSale` | sum of tenders can't exceed `grandTotal` (+0.01 tolerance) | `VALIDATION` (default code) | `مجموع طرق الدفع أكبر من إجمالي الفاتورة` |
| `createSale`/`previewSale` | a partial/credit sale (`paidAmount < grandTotal`) requires a customer | `VALIDATION` (default code) | `البيع الآجل أو الدفع الجزئي يتطلب اختيار عميل` |
| `createSale`/`previewSale` | that customer must exist and be active | `VALIDATION` (default code) | `العميل غير موجود أو غير نشط` |
| `createSale` (service layer, `assertWithinCreditLimit`) | new receivable can't exceed the customer's credit limit, unless the role can override | `FORBIDDEN` (per `parties/helpers/creditLimit.ts` — not re-derived here; already the contract of that shared helper) | (from `creditLimit.ts`, not this module's own message — see `parties.md`) |
| `getInvoice`/`getInvoicePrintData` (non-sample) | invoice must exist | `NOT_FOUND` | `الفاتورة غير موجودة` |
| `getRefund` | refund must exist | `NOT_FOUND` | `إشعار الدائن غير موجود` |
| `createRefund` (`recordRefund`) | invoice must exist | `NOT_FOUND` | `الفاتورة غير موجودة` |
| `createRefund` | invoice must be `COMPLETED` | `VALIDATION` (default code) | `لا يمكن إرجاع هذه الفاتورة` |
| `createRefund` | `customer_credit` refund method requires the invoice to have a customer | `VALIDATION` (default code) | `رصيد العميل يتطلب فاتورة مرتبطة بعميل` |
| `createRefund` | at least one line with `qty > 0` | `VALIDATION` (default code) | `اختر صنفاً واحداً على الأقل للإرجاع` |
| `createRefund` (per line) | the line must exist on the invoice | `VALIDATION` (default code) | `سطر الفاتورة غير موجود` |
| `createRefund` (per line) | `qty` can't exceed what's still returnable (`invLine.qty − already returned`) | `VALIDATION` (default code) | `` لا يمكن إرجاع أكثر من ${remaining} من "${invLine.name}" `` |
| `openPosShift` | terminal must not already have an `OPEN` shift | `CONFLICT` | `توجد وردية مفتوحة بالفعل على هذا الجهاز` |
| `openPosShift` | `openingFloat >= 0` | `VALIDATION` (default code) | `رصيد الافتتاح لا يمكن أن يكون سالباً` |
| `closePosShift`/`forceClosePosShift` | shift must exist | `NOT_FOUND` | `الوردية غير موجودة` |
| `closePosShift`/`forceClosePosShift` | shift must be `OPEN` | `VALIDATION` (default code) | `الوردية مغلقة بالفعل` |
| `recordCashInOut` | a shift must be open on that terminal | `CONFLICT` | `لا توجد وردية مفتوحة` |
| `recordCashInOut` | `amount > 0` | `VALIDATION` (default code) | `المبلغ يجب أن يكون أكبر من صفر` |
| `resumeHeldSale` | held sale must exist | `NOT_FOUND` | `لا يوجد بيع معلّق بهذا المعرف` |
| `getQuotation` | quotation must exist | `NOT_FOUND` | `عرض السعر غير موجود` |
| `saveQuotation` | at least one line | `VALIDATION` (default code) | `أضف صنفاً واحداً على الأقل` |
| `setQuotationStatus` | quotation must exist | `NOT_FOUND` | `عرض السعر غير موجود` |
| `convertQuotationToInvoice` | quotation must exist | `NOT_FOUND` | `عرض السعر غير موجود` |
| `convertQuotationToInvoice` | quotation must not already be converted | `CONFLICT` | `تم تحويل عرض السعر إلى فاتورة بالفعل` |
| `convertQuotationToInvoice` (delegated) | everything `createSale` validates (see above) | (as above) | (as above) |

**On the discard/resume/hold family:** no VAT-number-shaped validation exists anywhere in this
module (checked per the task brief's bug-class (a) — this module never reads or writes a party's
tax-id string; VAT here means "value-added tax amount/rate," an unrelated concept). Confirmed absent
in `discardHeldSale`, `resumeHeldSale`, `holdSale` specifically, since the brief called them out by
name.

**Not re-checked server-side today (frontend Zod only):** none — like `purchases.md`, this module
has no dedicated Zod validator file (`src/modules/invoices/validators/` does not exist); every rule
above is an inline `ApiError` check in `invoiceService.ts`/`backend/sales.ts`/`backend/shifts.ts`,
which is what Rust re-implements 1:1. The POS cart / desk invoice form / shift dialogs layer their
own client-side UX validation, but that's convenience, not a separate server-side rule set.

## 4. Undo matrix (every function that writes)

| Function | Undoable? | Compensation (existing fn) | Refused when | Period rule (D7) |
|---|---|---|---|---|
| `createSale` | No via the registry — **`createRefund` is the compensation**, exactly the master-plan §3 rule 7 model (a new, opposite document, never an edit/reversal-in-place) | `createRefund` | n/a | Subject to `assertOpenPeriod(date)` inside `postJournal` — a sale dated into a closed period is refused at post time |
| `createRefund` | No via the registry — **it IS the compensation** for `createSale`; a refund is corrected only by a fresh sale (re-selling the item), never by "undoing a refund" | n/a (it is itself the undo target) | n/a | Subject to `assertOpenPeriod(date)` inside `postJournal` |
| `convertQuotationToInvoice` | No via the registry — same as `createSale`, since it delegates straight into it; `createRefund` against the resulting invoice is the correction path | `createRefund` (against the resulting invoice, not the quotation) | n/a | Same as `createSale` |
| `saveQuotation` | No, but n/a — never posts to the GL or touches stock; a "wrong" quotation is corrected by creating a new one or `setQuotationStatus`-ing it to `REJECTED`/`EXPIRED`, not by reversing a posting that never happened | — | n/a | n/a |
| `setQuotationStatus` | No, but n/a — a pure status flip with no accounting effect | — | n/a | n/a |
| `openPosShift` | No, but n/a — no GL effect; a mistakenly-opened shift is closed normally (`closePosShift`), not "un-opened" | `closePosShift` (manual, not the undo registry — a distinct user action, and closing isn't really "undoing" an open) | n/a | n/a |
| `closePosShift`/`forceClosePosShift` | **No via the registry, and genuinely not reversible as a single compensating call — see the detailed reasoning below.** | none automatic | — | The variance posting and the optional cash-drop transfer both run through `postJournal`/`recordTransferVoucher`, both subject to `assertOpenPeriod(date)` |
| `recordCashInOut` | No, but n/a — never posts to the GL (movement-log only); a mis-entered pay-in/out is corrected by a new opposite movement (e.g. a `PAY_IN` to offset a wrong `PAY_OUT`), not a reversal of the log entry itself | — | n/a | n/a — no posting happens in this function |
| `holdSale` / `resumeHeldSale` / `discardHeldSale` | No, but n/a — a held sale never posts to the GL or touches stock; it's a parked cart, and "undo" for these three is simply doing the opposite action (hold again, discard, etc.), not a compensating accounting operation | — | n/a | n/a |

**Why `closePosShift` cannot simply be "reopened" (per the task brief's explicit ask):** a shift
close is not one atomic, self-contained fact the way a sale or a purchase receipt is — it can
**conditionally post up to two separate things** in the same call: (1) a cash over/short variance
journal entry (`Dr cash / Cr cashOver` or `Dr cashShort / Cr cash`, only when `|variance| > 0.005`),
and (2) a cash-drop transfer voucher (`Dr bank / Cr cash`, only when `handoverMode === 'DROP' &&
counted > 0`) — which itself calls `recordTransferVoucher` (`backend/vouchers.ts`), a **different**
posting engine with its own document/journal-entry pair. "Reopening" a shift would need to:
reverse the variance entry (if any), reverse the transfer voucher (if any, which then needs
*its own* documented reversal path — vouchers weren't reviewed yet in this session, that's
`vouchers.md`'s job), flip `shift.status` back to `OPEN`, and un-stamp `closedBy`/`closedAt`/
`countedCash`/`expectedCash`/`variance`/`handoverMode` — but by the time a manager notices a bad
close, **new sales/refunds may already have posted against a *different*, newly-opened shift on
that terminal** (nothing in `openShift` prevents opening a new shift immediately after a bad
close), so "reopening" the old one would create two concurrently-open shifts on one terminal, which
`shiftSummary`'s per-shift movement aggregation (`db.invoices.filter(i => i.shiftId === shift.id)`)
never anticipates. **Conclusion: there is no safe generic "undo close" operation** — the correct
fix for a wrong shift close (per master plan §3 rule 7's "existing compensating operation" model)
is a **manual journal entry** correcting the variance amount directly (the same tool an accountant
already has for any other posting mistake), not a new undo code path invented for this review. This
matches `settings.md`'s conclusion for `restoreFromArchive` (an operational safety net — here,
"post a correcting manual entry" — rather than a registry-driven undo).

No function in this module posts through `reverseJournal` (the manual-entry reversal path) — every
ledger-touching write here (`createSale`, `createRefund`, `closePosShift`'s variance,
`closePosShift`'s cash-drop) posts directly via `postJournal`/`recordTransferVoucher` and is
corrected only by a new, opposite document or (for shift variance) a manual journal entry — the
same conclusion `purchases.md` and `products.md` already reached for their own posting functions.

## 5. Concurrency under D8 (several terminals on one DB)

| Race | Rows | Settled by |
|---|---|---|
| Two terminals `createSale`/`convertQuotationToInvoice`/`createRefund` at the same instant (document numbering) | `counters.invoice` / `counters.refund` | Standard `shared::numbering` mechanism — `SELECT … FOR UPDATE` on the counter row inside the same transaction as the insert, identical to every other document-number sequence in the app. |
| Two terminals `createSale`/`createRefund` against the **same product's** stock at (near) the same instant | `products.stock_qty`, `products.stock_value`, `products.cost_price`, `product_branch_stock`, `product_batches` (FEFO consumption) | **The same sharpest race `products.md` already identified for `applyStockChange`/`consumeFefo`** — this module is one of that primitive's callers, not a separate risk; `shared::stock::apply_change` owning the product-row lock (as `products.md` §5 specifies) covers this module's sale/refund callers too, including the FEFO batch-consumption path. |
| Two terminals `createSale` against the **same customer** near their credit limit at the same instant | `customers.balance` (derived via `customerBalance()`, not a stored column) | **A genuine new race this module introduces, not present in `products.md`/`purchases.md`:** `assertWithinCreditLimit` reads `customerBalance(customer.id)` (a live re-scan of open invoices/payments, not a locked counter) **before** `recordSale` posts — two terminals selling to the same near-limit customer at once could both pass the check against the same stale balance and jointly exceed the credit limit. This needs a row lock on the **customer** row (`SELECT … FOR UPDATE` on `customers` for the duration of the credit-check-then-post transaction), analogous to the product-row lock for stock, but on the party side — a **new lock requirement this module contributes**, not inherited from an earlier one. Flagged as a Part 02/03 design requirement (§9), not fixable in the mock (single-process, so the race cannot manifest there). |
| Two terminals `openPosShift` on the **same `terminalId`** at (near) the same instant | `shifts` (the "is one already open" check) | `currentOpenShift(terminalId)`'s existence check must run against a locking read — a unique **partial** constraint (`UNIQUE(terminal_id) WHERE status = 'OPEN'`, MariaDB-compatible via a generated column or an application-level `SELECT … FOR UPDATE` scoped to that terminal's shift rows) so two concurrent opens on the same terminal can't both succeed; the loser gets the same `CONFLICT` the mock throws today. |
| Two terminals `recordCashInOut`/`createSale` (cash tender) on the **same open shift** at (near) the same instant | `shifts.movements` (append-only) | A pure append is naturally race-safe for the log itself (no read-modify-write on existing rows), but the **X/Z report and close-screen `expectedCash`** are computed by re-summing `movements` at read time (`shiftSummary`) — Rust should compute this with a single aggregate query at close time inside the same transaction as the status flip, so a movement appended a moment after the summary was read (but before the close committed) is either fully included or fully excluded, never split. This is the shift-close analog of `purchases.md`'s "PO row lock for the remaining-qty check" finding — same shape (read-then-decide needs the summary read locked against concurrent appends), different document type. |
| Two terminals `closePosShift`/`forceClosePosShift` on the **same shift** at (near) the same instant | `shifts.status` | `SELECT … FOR UPDATE` on the shift row for the status check-then-write — same single-lock-point discipline `purchases.md` specified for its own status-guarded transitions (`sendPurchaseOrderToSupplier`/`receivePurchaseOrder`/`cancelPurchaseOrder`), applied here to shift close instead of PO status. |
| Two terminals `resumeHeldSale`/`discardHeldSale` on the **same held-sale id** at (near) the same instant | `heldSales` | Low-stakes (a held sale is a personal draft cart, not shared state two cashiers would normally race on — the id is only known to the terminal that created it), but for correctness: the delete should be `DELETE ... WHERE id = ?` with the affected-row-count checked, so the second concurrent caller gets a clean `NOT_FOUND` instead of a silently-successful double-resume. No new lock primitive needed — ordinary row-level delete semantics suffice. |

## 6. Events and side effects

- **Activity/audit rows** written for: `createSale` (`logActivity('sale', …, '/invoices/${id}')`),
  `createRefund` (`logActivity('refund', …, '/invoices/${invoiceId}')` — note: the link points at the
  **invoice**, not a refund detail route, since refunds have no detail page, confirmed intentional),
  `openPosShift` (`logActivity('shift', …, '/pos/shifts')`), `closePosShift` (`logActivity('shift', …,
  '/pos/shifts')`, message includes expected/counted/variance). `forceClosePosShift` delegates into
  `closeShift`, so it gets the same audit row (with `forceClosedBy` stamped as a follow-up field, not
  part of the audited message). `convertQuotationToInvoice` gets `createSale`'s audit row (the
  quotation-side status/link mutation itself is **not separately audited** — see below).
- **No `logAudit`/`logActivity` call (silent writes), confirmed by reading every write function end
  to end:** `holdSale`, `resumeHeldSale`, `discardHeldSale` (all three — held-sale CRUD has zero audit
  trail), `saveQuotation`, `setQuotationStatus`, and `convertQuotationToInvoice`'s own
  quotation-side mutation (`status = 'ACCEPTED'`, `convertedInvoiceId = invoice.id` — this specific
  write, not the `createSale` call it makes, which is separately audited). **Not treated as a fix
  candidate in this review** — see §8 for the reasoning (these are all either transient POS state or
  non-financial document-status changes, judged not to carry the same audit weight as
  `products.md`'s master-data/`purchases.md`'s posting-document findings) — recorded as an open
  question in §9 instead of a guessed-at fix, per the task's conservative instruction for this module.
- **`recordCashInOut` has no audit/activity row at all** (not even the generic `logActivity`) despite
  moving real cash in/out of the drawer — confirmed by reading the whole function; only
  `recordShiftMovement` appends to `shift.movements` (which is itself unaudited, just a log array).
  This is a **sharper gap than the held-sale one**: pay-ins/pay-outs are real till-cash events an
  owner would want in the audit trail, not a personal UI draft. Flagged as the strongest candidate
  for a "should get one" verdict among this module's gaps, but **still not applied here** (see §8) —
  adding an audit call to a function that also touches the shift-movement log is a small, low-risk
  change, but the task's brief asks for conservatism on every write path in this module, and this one
  is adjacent to the drawer-reconciliation numbers `closePosShift` later depends on, so it's recorded
  as a decision for §9 rather than guessed at.
- **Events emitted:** `parties:changed` from `createSale`/`createRefund` (only `if
  (invoice.customerId)` — a cash sale/refund with no customer emits nothing, correctly, since no
  party balance changed). `ledger:changed` from `closePosShift`/`forceClosePosShift` (unconditional,
  even when no variance posted — arguably a bit generous, but harmless: a UI refresh with nothing
  changed costs nothing) and from `recordCashInOut` (**a naming mismatch, not a functional bug**:
  this function never posts to the GL, only appends a shift movement, yet emits the GL-refresh event;
  the practical effect is a false-positive UI refresh of ledger-watching screens, not a wrong number
  anywhere — flagged in §8 as a low-risk fix candidate, not applied per the conservative instruction
  for this module). **No `ledger:changed` from `createSale`/`createRefund` themselves** despite both
  posting journal entries — confirmed by reading both functions end to end; this is the **same class
  of gap** `purchases.md` found for its own posting functions (which also don't emit `ledger:changed`),
  now confirmed present here too, in this module's two highest-volume posting functions. **No
  `catalog:changed`** from either despite both moving `stockQty`/`stockValue`/batches — same shape as
  `purchases.md`'s finding. Not fixed here (see §8) — this is the exact kind of event-coverage
  decision the task brief said to flag rather than patch in a hot posting path.
- **Attachments:** `Invoice.attachmentIds` is a plain id-list array, same pattern as
  `purchases.md`'s `PurchaseOrder.attachmentIds` — blob CRUD lives in `core`'s `AttachmentField`,
  already seam-clean.
- **Printing:** `getInvoicePrintData` feeds the A4/thermal print templates and the settings
  test-print sample; the actual print/PDF rendering is a separate `pdfService`/print-route call the
  page makes, not part of this module's service functions (same split `purchases.md` documented for
  its own PO/debit-note PDFs).

## 7. Aggregations (reports / analytics / dashboard / insights only)

Not a reporting module — no `report`/`analytics`/`dashboard` output here. But `createSale`,
`createRefund` and `closePosShift` are the three most posting-heavy functions Part 03 will port, so
their exact accounting math is documented precisely (same reason `purchases.md`/`products.md`
documented their own posting functions under this heading):

| Output (DTO field) | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `createSale`'s journal lines (`prepareSale`/`recordSale`) | — | Every non-free-text line's base qty aggregated per product for the stock/COGS decision; COGS = `round2(qty × avgCost)`, or the product's **exact remaining `stockValue`** when the sale empties the stock (A1/A2 "nothing left over" rule, confirmed against `docs/v2/02-accounting-review.md` line 73-74) | by tender method (one posting line per non-zero tender, each to its own account via `tenderAccountId` — card/wallet go to their **clearing** account, not straight to bank/cash, per the C3 fix `sales.ts:32-42` documents), by free-text line's chosen revenue account (grouped, not one line per free-text line) | `round2` at **every** intermediate accumulation point: per-product aggregated base qty (`baseQty`), `costTotal` (both the per-product accumulation and the final total), `paidAmount` (sum of tenders), `receivable` (`grandTotal − paidAmount`), `stockedNet`/`freeTextRevenue` accumulation, and — for FX sales — every credit-side amount a second time through `convertLinesToBase` (which itself rounds per-line then re-distributes the rounding gap onto the largest line, docs/v2/10 §2). Rust must round at each named point in this order, not just once from an exact sum — the same "round early and often" shape `purchases.md`/`products.md` already documented for their own postings. | `sales.ts:86-229` (`prepareSale`), `sales.ts:241-393` (`recordSale`) |
| `createSale`'s posting shape | — | `Dr <tender account per method> (Σ tender.amount, filtering out zero-amount tenders)` / `Dr receivable[customer] (grandTotal − paidAmount, only when > 0, tagged with FC amount+rate when currency ≠ base)` / `Cr sales (Σ stocked-line net, base-currency-converted when FX)` / `Cr <free-text revenue account per account> (Σ net for lines routed there)` / `Cr vatOutput (Σ vat, base-currency-converted when FX)` / `Dr cogs (costTotal, always base currency, never converted)` / `Cr inventory (costTotal, always base currency, never converted)` | by `role`/`accountId`, by `partyId` for the receivable line, dimensioned by `branchId`/`costCenterId` on every line | (see rounding point row above) | `sales.ts:219-227` |
| `createSale`'s FX handling (v2 phase 9, confirmed against `docs/v2/10-branches-currencies-cost-centers.md` §2) | `exchangeRates` (via `requireRate`, reused from `settings.md`) | Only engaged when `input.currency` is set and differs from the base currency (`isBaseCurrency` check) — `rate` is either the caller-supplied `input.exchangeRate` or looked up via `requireRate(currency)` (throws `VALIDATION` "no exchange rate for currency X" if none exists for that date). **COGS/inventory are NEVER converted** — cost basis is always base currency regardless of the sale's currency, confirmed by reading `costTotal`'s posting line, which uses the raw (never-FX'd) value both in FC and base sales alike. | — | `round2` on `toBase(amount, rate) = round2(fc × rate)` per converted line, then `convertLinesToBase`'s largest-line rounding-gap redistribution across the credit-side lines (`stockedNet`, each free-text account, `vat`) — **not** applied to the receivable/tender (debit) side, which round independently via their own `toBaseAmt`/`fc()` calls; only credit lines share the redistribution pool. Rust must replicate this asymmetry exactly (debit lines round independently; only the named credit lines pool their rounding gap). | `sales.ts:174-227` |
| `createRefund`'s journal lines (`recordRefund`) | — | **Refund split** (mirrors `purchases.md`'s `recordPurchaseReturn` shape exactly, confirmed): `settledToReceivable = min(grandTotal, invoiceOutstanding(invoice))` first pays down whatever's still outstanding on the original invoice; any remainder either pays out through the chosen `refundMethod`'s settlement account, **or**, for `customer_credit`, is added as an *extra* credit to the customer's receivable (`creditedToAccount`) instead of touching any settlement account at all — confirmed: when `creditedToAccount > 0`, the posting collapses to a single combined receivable-credit line (`settledToReceivable + creditedToAccount`), with **no settlement-account line at all** in that branch. | by `role`/`partyId` | `round2` at every step: **is-this-the-final-return** detection (`isFinal`, comparing cumulative returned qty per line against the original), the exact-remainder subTotal/taxAmount when final (`invoice.subTotal − invoice.discountAmount − Σ previous refunds' subTotal`, avoiding leftover cents on the last return of an invoice — the same "final return uses exact remainders" pattern as `purchases.md`'s equivalent, confirmed independently derived, not copy-pasted logic, but the identical accounting shape), the proportional (non-final) subTotal/taxAmount otherwise, `grandTotal`, `settledToReceivable`, `cashBack`, and `cost`. | `sales.ts:404-536` (`recordRefund`) |
| `createRefund`'s restock-vs-write-off split | `products` | Per line: `restock !== false` re-averages the value back into stock via `applyStockChange` (A1/A2 re-averaging, the value moves the same way it left); `restock === false` books the line's cost to `inventoryWriteOff` (5120) instead, **never re-entering stock** — confirmed no `applyStockChange` call on that branch, matching `docs/v2/02-accounting-review.md`'s A4+A5 "write-offs use their own account so the owner sees them." | — | `round2` on `value = qty × invLine.costPrice` per line, and the running `restockValue`/`writeOffValue` accumulators | `sales.ts:479-495` |
| `createRefund`'s posting shape | — | `Dr salesReturns (subTotal)` / `Dr vatOutput (taxAmount)` / `[Dr/Cr receivable[customer] + settlement-account line, per the split above]` / `Dr inventory (restockValue)` / `Dr inventoryWriteOff (writeOffValue)` / `Cr cogs (restockValue + writeOffValue)` | — | (see rounding rows above) | `sales.ts:510-524` |
| `createRefund`'s shift-drawer interaction (cash refunds only) | `shifts` | Only when `refundMethod === 'cash' && paidOut > 0`: finds the shift the **original invoice** was sold under, if it's still `OPEN`; **falls back to any currently-open shift on any terminal** if that one is closed or doesn't exist (`sales.ts:529` — `?? db.shifts.find(s => s.status === 'OPEN')`). **Flagged, not changed** — see §9; this fallback could attribute a cash refund's drawer movement to the wrong terminal's shift when the original sale's shift has since closed, which would skew that *other* terminal's expected-cash math at its own close, even though it never touches the GL journal entry itself (the posting above is correct regardless of which shift's movement log records it). | — | — | `sales.ts:528-531` |
| `closePosShift`'s variance posting (invariant 11, confirmed against `docs/v2/02-accounting-review.md` line 226 "Shift close: cash over → Dr cash / Cr cashOver; short → Dr cashShort / Cr cash") | `shifts` (via `shiftSummary`) | Only when `\|variance\| > 0.005` — `expectedCash = round2(openingFloat + cashSales − cashRefunds + payIns − payOuts − bankDrops)`, `variance = round2(counted − expectedCash)` | — | `round2` on every term of `expectedCash` and on `variance` itself | `shifts.ts:93-117` |
| `closePosShift`'s cash-drop transfer (docs/v2/06 §5 "drop cash to the safe/bank... creates a transfer voucher") | — | Only when `handoverMode === 'DROP' && counted > 0` — delegates entirely to `recordTransferVoucher` (`backend/vouchers.ts`, `Dr bank / Cr cash` for the full `counted` amount), a **separate posting engine not reviewed in this module** (that's `vouchers.md`'s scope) — this module's contract is simply "calls it with these exact arguments," not the voucher engine's own internals. | — | (delegated — see `vouchers.md` when written) | `shifts.ts:123-134` |

## 8. Contract fixes needed in the mock (→ 01.C)

Per the task's conservative instruction for this accounting-sensitive module: **no mock code
changes were applied in this review.** Every bug class named in the session brief was checked
against this module specifically, including the two named call sites (`discardHeldSale`/
`resumeHeldSale`) and every other write function, and found either already fixed, genuinely absent,
or judged not to warrant a guessed-at fix given the conservative instruction:

- [x] **(a) Hard-coded Saudi-only VAT-number regex — confirmed not applicable, including at the two
      named call sites.** `discardHeldSale`, `resumeHeldSale`, and every other function in this
      module never reads or validates a party's tax-id string — "VAT" in this module always means
      the tax amount/rate on a sale line, an unrelated concept from the `settings.md`/`parties.md`
      bug class. Confirmed absent by reading every function end to end, not just grepping for
      "vat"/"tax."
- [x] **(b) Hard-delete/cancel with no reference/state check — confirmed not applicable at the level
      this bug class targets.** This module's only "deletes" are `resumeHeldSale`/`discardHeldSale`
      against `db.heldSales`, which is **transient POS-draft state with no downstream references
      anywhere in the schema** (nothing points at a `HeldSale.id` — it's a leaf table by
      construction), so there is no reference check to be missing, unlike `settings.md`'s
      `deleteTax`/`deletePaymentMethod` (master data referenced by posted documents) or
      `purchases.md`'s `cancelPurchaseOrder` (a document with real accounting state). The module's
      genuinely state-guarded transitions (`createRefund` requires `COMPLETED`,
      `convertQuotationToInvoice` requires not-already-converted, `closePosShift` requires `OPEN`)
      were all independently confirmed present and correctly worded in §3 — nothing missing.
- [x] **(c) Writes with no audit trail — found, but judged not to warrant an applied fix; see the
      detailed reasoning in §6 and the open question in §9.** `holdSale`/`resumeHeldSale`/
      `discardHeldSale` (transient POS draft state), `saveQuotation`/`setQuotationStatus`/
      `convertQuotationToInvoice`'s own quotation-side write (non-financial document status), and
      `recordCashInOut` (real cash movement, but movement-log-only, no GL effect) all write with no
      `logActivity`/`logAudit` call. Unlike `products.md`'s `deleteDraftAdjustment` (an inventory
      master-data delete with no audit trail, fixed there) or `settings.md`'s destructive
      master-data deletes (fixed there), none of this module's gaps are a **destructive** action on
      **persistent business data** — they're either ephemeral UI state or non-posting status flips.
      The one genuinely closer call, `recordCashInOut` (real drawer cash, no audit row at all), is
      explicitly **not** treated as an automatic "same bug, just fix it" case here, because the task
      brief asks for conservatism on every write in this specific module and this write feeds
      numbers `closePosShift` later reconciles against — recorded as a decision for the user in §9
      instead.
- [x] **(d) Plain `Error` instead of `ApiError` — none found.** Every throw site in
      `src/mocks/backend/sales.ts` and `src/mocks/backend/shifts.ts` uses `ApiError` (confirmed by
      reading both files end to end and by a targeted search finding zero plain `throw new
      Error(...)` occurrences in either file) — unlike `settings.md`'s `restoreFromArchive` gap
      (still open there), this module has no plain-`Error` throw anywhere.
- [x] **(e) Silent skip instead of throwing on a validation-shaped condition — checked, one
      candidate found and judged intentional, not a bug.** `discardHeldSale`'s silent no-op when the
      id doesn't exist reads as idempotent-delete semantics (discarding an already-gone cart is not
      a state a cashier needs to be told about), the same class of harmless default `purchases.md`'s
      `allocateLandedCosts` zero-amount skip and `products.md`'s unmatched-`productId` price-list
      skip were judged to be. `recordShiftMovement`'s own silent no-op when no shift is open (or
      `amount === 0`) is **by design**, documented in its own doc comment ("no-op if none is
      open — desk sales/refunds don't require a shift") — not a gap, a deliberate optional-shift
      design. No case in this module rises to "a user-visible validation failure was silently
      swallowed."

Two items are genuine contract observations worth recording precisely, but neither is a mock
**bug** in the sense of producing a wrong number — both are listed in §9 since they need a decision
or at least a conscious "leave as-is" confirmation, not a guess:

- [ ] **`ledger:changed`/`catalog:changed` are not emitted from `createSale`/`createRefund`**
      despite both posting to the GL and moving stock (§6) — the same class of gap `purchases.md`
      already found and left unfixed for the identical conservative reason (a hot posting-path
      change, even a UI-refresh-only one, deserves a deliberate decision). **Not changed here.**
- [ ] **`recordCashInOut` emits `ledger:changed` despite never posting to the GL** (§6) — the
      inverse naming mismatch from the point above (an event fires when nothing changed on the
      ledger side, rather than not firing when something did). Low-risk (a spurious UI refresh, not
      a wrong number), but still a hot POS-drawer code path — **not changed here**, flagged in §9.

## 9. Open questions (→ decisions in `00-MASTER-PLAN.md`)

- **Should `holdSale`/`resumeHeldSale`/`discardHeldSale`, `saveQuotation`/`setQuotationStatus`'s own
  write, and `recordCashInOut` get an audit/activity row?** (§6/§8) Recommend: yes for
  `recordCashInOut` (real cash movement an owner would want in the trail — the strongest candidate),
  optional/low-priority for the quotation-status flip (business-visible but non-financial), and
  probably no for held-sale CRUD (transient per-terminal draft state, same tier as an unsent form
  draft — auditing it would add noise without a clear reader). **Needs a decision** before Part 03's
  `invoices` domain decides whether `shared::activity` should special-case "these three call sites
  intentionally don't audit," matching how `settings.md`'s §9 flagged its own quieter writes for the
  same kind of judgment call.
- **`recordCashInOut` emits `ledger:changed` though it never posts to the GL, while `createSale`/
  `createRefund` (which do post) emit neither `ledger:changed` nor `catalog:changed`** (§6/§8): this
  reads as event *names* not matching actual GL/stock impact anywhere in this module. Recommend
  fixing both directions together, once, when `cross-cutting.md`'s event-payload/refresh design
  (01.D, F6) is written — not module-by-module, since `purchases.md` found the identical
  missing-`ledger:changed`-on-posting gap for its own functions. **Needs a decision**, not applied
  here.
- **`createRefund`'s cash-drawer shift fallback can attribute a refund's movement to the wrong
  terminal's shift** (§7, `sales.ts:529`): when the original sale's shift has since closed, the
  refund's cash-out is logged against *any* other currently-open shift system-wide, not necessarily
  the terminal actually processing the refund. This never affects the GL journal entry (which is
  always correct regardless of shift attribution), only which shift's X/Z report and expected-cash
  math sees the movement. Recommend: resolve the shift from the **terminal actually performing the
  refund** (a parameter the current `RefundInput` doesn't carry — a contract-shape decision, not a
  guessable one-line fix) rather than "any open shift," but this needs a decision on whether
  `RefundInput` should gain a `terminalId` field before Part 03 implements this command. **Needs a
  decision — not changed in the mock**, since guessing at the right fallback (refuse instead? attach
  to no shift instead?) risks silently changing which shift's numbers a real cashier already relies
  on.
- **New Part 02/03 concurrency requirement this module contributes** (§5): a customer-row lock for
  the credit-limit check-then-post in `createSale`, alongside the already-specified product-row lock
  for stock — recorded as a design note for whoever builds `domains/invoices/service.rs`, not a
  question needing a user decision.
- **`Invoice.status` includes `'DRAFT'` but no code path in `sales.ts` ever produces it** (§2) —
  confirm whether Rust should keep the variant (some future desk-invoice draft-save feature) or drop
  it as vestigial now that `Quotation` owns the "not yet posted" concept as its own entity. **Needs
  confirmation**, not a narrow bug.
- **`InvoiceLine.invoiceType`/`SaleInput.invoiceType` has no server-side derivation** (§2) — the
  type's own doc comment says "automatic" (docs/v2/06 §2), but `sales.ts` passes the client-supplied
  value straight through with no validation. Confirm whether the "automatic" rule needs to move into
  the Rust command (a real validation/derivation Rust would then own) or stays a client-only UX
  convenience Rust simply trusts. **Needs confirmation.**
- **Free-text line `productId` synthetic-string convention (`freetext-${i}`)** (§2): recommend Rust
  uses a genuinely nullable `product_id` column instead of carrying the mock's synthetic-string
  workaround forward — a schema-only decision for Part 02-B, not a behavior change.
- **Cross-referenced, not re-opened here:** the missing `ledger:changed`/`catalog:changed` emission
  on posting functions (§6/§8) is the same pattern `purchases.md` already flagged for its own
  functions — the actual decision (which events every posting function should emit) should be made
  once, in `cross-cutting.md` (01.D, F6), not module-by-module.

## Gate

- [x] Every inventory function is in §1 with a confirmed disposition (28/28 in the regenerated
      inventory: 26 core `port` functions + `isOverdue` `frontend` + the 2 out-of-scope plan-22
      `frontend` functions already correctly classified; no disposition changed from the generator's
      own heuristic, so no `scripts/contract/config.ts` override was needed for this module).
- [x] Every write function is in §4 (9 write functions/families: `createSale`, `createRefund`,
      `convertQuotationToInvoice`, `saveQuotation`, `setQuotationStatus`, `openPosShift`,
      `closePosShift`/`forceClosePosShift`, `recordCashInOut`, `holdSale`/`resumeHeldSale`/
      `discardHeldSale`).
- [x] Every DTO field needing a non-default mapping is in §2.
- [x] `bun run contract:check` is green (no override added — `bun run contract` re-run to confirm no
      drift; no mock code was changed in this review, so no `verify:mocks` regression was possible,
      but the baseline was re-confirmed anyway). Verified: `bun run verify:mocks` **128 ok, 0
      failed** both before and after this review (no mock file was edited). Also green: `bun run
      build`, `bun run check`, `bun run memory` → `bun run memory:check` (0 new seam violations),
      `bun run diag:check`.
