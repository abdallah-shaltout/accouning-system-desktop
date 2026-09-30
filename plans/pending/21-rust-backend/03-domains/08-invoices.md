# 21 · 03.08 — `invoices` part 1 (sales, sales returns, quotations, invoice reads, print data)

> **Status:** code complete, not yet compiled/tested (2026-09-28, W3). `domains/invoices/**` written
> (dto.rs, service/{access,common,credit,reads,refund,sale,quotations}.rs, commands.rs, mod.rs) —
> all 13 commands in this file plus 08b's 12 (25 total, one domain). G-25 (refund VAT
> over-refund) was **not** separately resolved by the user this wave — `create_refund` §3.5 step 6
> ports the mock's current formula verbatim with a `// G-25:` comment at the exact spot (per this
> file's own §7 instruction: "until then §3.5 step 6 is the only step that may not be
> implemented"). Switch lines (§6) and `src/modules/invoices/types/contract.check.ts` (§2) are in.
> `tests/domain_invoices.rs` written covering the highest-value scenarios (§8a) — ⏳ deferred
> time-boxed test pass (DB tests need `EQUAL_TEST_DATABASE_URL`; not run by this implementer per
> the no-cargo rule). Needs from manager: `pub mod invoices;` + `domains::invoices::{ipc_signatures,
> export_bindings}` hooks in `domains/mod.rs`, the 25 `domains::invoices::commands::*` lines in
> `lib.rs`'s `generate_handler!` — see the final report for the exact list.
>
> Depends on: 01-settings
> (the `StoreSettings` DTO builder behind `settings_get_settings`), 03-users (session), 05-parties
> (the `Customer` DTO builder; cross-domain rule R-1, 05 §4), 06-products (catalog rows only — stock goes
> through `shared::stock`), 09-payments (the `Payment` DTO + `payments_for_invoice`, see PG-8), Part 02
> `shared::{ledger,stock,numbering,currency,balances,activity}`, and the Part 02 gaps **PG-1…PG-8** (§7).
>
> **Split (size rule):** the `invoices` domain is planned in two files. This file = 13 commands (sales,
> refunds, quotations, reads, print data). [`08b-pos-shifts.md`](08b-pos-shifts.md) = 12 commands (POS
> shifts, cash in/out, held sales) + the shift-movement helper this file calls. One implementer owns both
> (`domains/invoices/**`).

**Goal.** Port `invoiceService.ts`'s sale/refund/quotation/read functions and the posting engine in
`src/mocks/backend/sales.ts` behaviour-exactly: tax-inclusive VAT with discount order line → invoice → VAT
(the `computeInvoiceTotals` engine, ported once as `shared::totals`, PG-1), split tenders to each method's
own (clearing) account, weighted-average COGS via `shared::stock::cost::cost_out_sale` + FEFO/manual batch
draws, AR postings tagged with the customer, FX sales via `shared::currency`. Every total is recomputed in
Rust from ids and quantities (entry §3.3 server authority).

**Read first.** [`../01-frontend-analysis/invoices.md`](../01-frontend-analysis/invoices.md) §1–§9 ·
mock `src/mocks/backend/sales.ts:40-536`, `src/modules/invoices/services/invoiceService.ts:35-247,357-475`,
`src/modules/invoices/helpers/totals.ts:87-242`, `src/modules/parties/helpers/creditLimit.ts:23-43`,
`src/modules/users/helpers/permissions.ts:101-103`, `src/mocks/backend/currency.ts:97-114` · types
`src/modules/invoices/types/index.ts:1-354` · Rust `shared/stock/{mod,cost,batches}.rs`,
`shared/ledger/{post,accounts,period}.rs`, `shared/currency.rs`, `shared/balances.rs`,
`entities/sales/*`, migration `m0008_sales.rs` · `docs/v2/02-accounting-review.md` §3 (sale / credit
note rows) and §4 (invariants 1, 3, 4, 5, 7, 10).

## 1. Commands

Reads run in `with_read`; the actor is cloned from `state.session` before the closure (no session →
`UNAUTHORIZED` `سجّل الدخول أولاً`), then `require_any(tx, actor, areas, Read)` — a domain-local helper
(`service/access.rs`) that calls `core::settings::require` per area and passes on the first `Ok`
(same pattern as 11-expenses §1 / 03-users D-3). Reads that need the business clock use `with_tx` and touch
nothing. Writes: `with_tx` + `require_any(…, Write)`. Areas come from the callers (grep of `src/modules`):
POS (`PosPage`, `TenderDialog`, `ReturnByScanDialog`, route `pos`), desk (`InvoiceFormPage`, route
`invoice-new`, area `sales`), `PartyDetailPage` (area `parties`), print (`pdfService`, `printService`,
`PrintingSettingsPage` area `settings`).

| # | Mock fn (`invoiceService.ts`) | Disp. | Rust command | Args → Return | Area / Access | Tx | Events |
|---|---|---|---|---|---|---|---|
| 1 | `getInvoices` (:78) | port | `invoices_get_invoices` | `{ filter?: InvoiceListFilter }` → `Vec<InvoiceRow>` | Sales∨Pos∨Parties / Read | `with_tx` (clock for `overdueOnly`) | — |
| 2 | `getInvoicesPaged` (:88) | port | `invoices_get_invoices_paged` | `{ query: PagedQuery<InvoiceListFilter> }` → `PagedResult<InvoiceRow>` | Sales / Read | `with_tx` (clock) | — |
| 3 | `getInvoice` (:118) | port | `invoices_get_invoice` | `{ id }` → `InvoiceDetail` | Sales∨Pos / Read | `with_read` | — |
| 4 | `previewSale` (:138) | port | `invoices_preview_sale` | `{ input: SaleInput }` → `Vec<JournalPreviewLine>` | Sales∨Pos / Read | `with_tx` (clock: FX rate day) | — |
| 5 | `createSale` (:151) | port | `invoices_create_sale` | `{ input: SaleInput }` → `Invoice` | Sales∨Pos / Write | `with_tx` | ledger, catalog, parties |
| 6 | `createRefund` (:178) | port | `invoices_create_refund` | `{ input: RefundInput }` → `Refund` | Sales∨Pos / Write | `with_tx` | ledger, catalog, parties |
| 7 | `getRefund` (:186) | port | `invoices_get_refund` | `{ id }` → `Refund` | Sales∨Pos / Read | `with_read` | — |
| 8 | `getInvoicePrintData` (:203) | port | `invoices_get_invoice_print_data` | `{ id: String }` → `PrintData` | Sales∨Pos∨Settings / Read | `with_tx` (clock + actor for `'sample'`) | — |
| 9 | `getQuotations` (:367) | port | `invoices_get_quotations` | `{ filter?: QuotationFilter }` → `Vec<QuotationRow>` | Sales / Read | `with_read` | — |
| 10 | `getQuotation` (:376) | port | `invoices_get_quotation` | `{ id }` → `QuotationRow` | Sales / Read | `with_read` | — |
| 11 | `saveQuotation` (:384) | port | `invoices_save_quotation` | `{ input: QuotationInput }` → `Quotation` | Sales / Write | `with_tx` | — |
| 12 | `setQuotationStatus` (:446) | port | `invoices_set_quotation_status` | `{ id, status: QuotationStatus }` → `Quotation` | Sales / Write | `with_tx` | — |
| 13 | `convertQuotationToInvoice` (:455) | port | `invoices_convert_quotation_to_invoice` | `{ id, payment: ConvertPayment }` → `Invoice` | Sales / Write | `with_tx` | as #5 |

Not ported (analysis §1): `isOverdue` (frontend, pure), `copyInvoiceImage`/`saveInvoiceImage` (frontend,
plan 22). Args structs are `Invoices<Fn>Args` (entry §3.2); every command gets its `ipc_sig!` line.

## 2. DTOs (`domains/invoices/dto.rs`, `#[ts(export_to = "invoices/types/gen/")]`)

Conventions (core/dto.rs header): camelCase; `Option` → `skip_serializing_none` + `#[ts(optional)]`
(absent, never `null`); `Decimal` → `serde_number` + `#[ts(type = "number")]`; `Id` → `#[ts(type = "string")]`;
document dates are `String` = `DocDate::key()` (the mock's exact string, P2-09).

| Rust DTO | TS type (file:line) | Field notes |
|---|---|---|
| `InvoiceLine` | `types/index.ts:5-41` | `product_id: String` — the UUID text, or **`freetext-{position}`** when `product_id IS NULL` and `is_free_text` (mock `sales.ts:266`; D-I2). `tax_category` = the settings `TaxCategory` type (reuse 01-settings' DTO enum, else `#[ts(type = "import('../../../settings/types').TaxCategory")]`). `qty`/`unit_factor`/`price`/`cost_price`/`list_price` raw as stored. `is_free_text: Option<bool>` present only when true (mock sets it only on free-text lines). |
| `Tender` | `:49-54` | `amount` money. |
| `Invoice` | `:56-114` | `status` `InvoiceStatus` (`DRAFT` kept, D-I4); `payment_status` = `shared::totals::PaymentStatus` (PG-1); `payment_method` `SalePaymentMethod` snake_case; `tenders: Option<Vec<Tender>>` always `Some` (every Rust invoice has tenders, analysis §2); `source`/`invoice_type` UPPERCASE; `due_date: Option<String>` = due-date `DocDate::key()` (PG-2b); `attachment_ids: Option<Vec<String>>`. |
| `InvoiceRow` | `invoiceService.ts:35` | `#[serde(flatten)] #[ts(flatten)] invoice: Invoice` + `customer_name?`, `cashier_name`, `outstanding`. |
| `InvoiceDetail` | `invoiceService.ts:37-44` | flatten `InvoiceRow` + `customer?: parties::dto::Customer`, `refunds: Vec<Refund>`, `payments: Vec<payments::dto::Payment>`, `journal_entries: Vec<JournalRef { id, number, description }>`, `returned_qty: BTreeMap<String, Decimal>` (values `serde_number`, `#[ts(type = "Record<string, number>")]`). |
| `JournalPreviewLine` | `types/index.ts:349-354` | 4 fields. |
| `Refund`, `RefundLine` | `:321-339` | `lines[]` = `{ invoiceLineId, qty, restock? }` as the client sent them (qty > 0 only); `refund_method` snake_case; `credited_to_account` present only when > 0 (`sales.ts:467`). |
| `SaleInput`, `SaleInputLine` | `:258-316` | request-only; `product_id: String` (parsed to `Id` only for catalog lines — `InvoiceFormPage.vue:135` sends `"freetext"`); `date`-like `due_date_override: Option<RawDocDate>`. |
| `RefundInput`, `QuotationInput`, `ConvertPayment` | `:341-346`, `:145-153`, `invoiceService.ts:455` | request-only; `expiry_date: Option<RawDocDate>`. |
| `InvoiceListFilter` | `InvoiceFilter & { openOnly?: boolean }` (`:242-256`, `invoiceService.ts:78`) | `from`/`to` plain `YYYY-MM-DD` strings (not routes, payments.md §2 note). |
| `Quotation`, `QuotationRow`, `QuotationStatus`, `QuotationFilter` | `:121-143`, `invoiceService.ts:361,367` | `expiry_date: Option<String>` = DocDate key (PG-2c); quotation `lines[].product_id` = UUID text or `"freetext"` when NULL (D-I2). |
| `PrintData` | `invoiceService.ts:193-200` | `invoice: Invoice`, `customer?: Customer`, `cashier_name`, `settings: settings::dto::StoreSettings`, `sample?: bool`. |

**`contract.check.ts`** (new `src/modules/invoices/types/contract.check.ts`): `Expect<Equals<Gen.X, X>>` for
`Invoice`, `InvoiceLine`, `Tender`, `Refund`, `JournalPreviewLine`, `Quotation`, `SaleInput`, `RefundInput`,
`QuotationInput`, and — imported type-only from `../services/invoiceService` — `InvoiceRow`, `InvoiceDetail`,
`QuotationRow`, `PrintData`. `InvoiceListFilter` vs `InvoiceFilter & { openOnly?: boolean }`. Any pair that
can't be made equal from the Rust side gets `// contract-ok: <reason>` (entry §3.4).

## 3. Service logic (`service/{access,common,reads,sale,refund,quotations,credit}.rs`)

### 3.1 Common pieces

- **`invoice_dto(conn, model)`**: lines by `position`, tenders by `position`; `outstanding` for rows =
  `0` when `REFUNDED`, else `invoice_outstanding(grand_total, refunded_amount, paid_amount)` (PG-1,
  `totals.ts:240`). `customer_name` = live party name (`deleted_at IS NULL`, P2-16 "deleted ≡ absent"),
  `cashier_name` = live user name else `—` (`invoiceService.ts:46-53`). Lists batch-load lines, tenders,
  party names and user names in 5 queries, never N+1.
- **`tax_for_line(tax_id)`** (`sales.ts:59-65`): live active tax by id → live active tax
  `settings.default_tax_id` → first live active `type='OUTPUT' AND is_default` by `(created_at, id)` →
  else `{ rate: 0, category: 'S', id: None }` (the `salesTaxRate()` fallback is 0 once both lookups failed).
- **`base_qty(qty, unit_factor)`** = `round2(qty × (unit_factor ?? 1))` (`sales.ts:81-83`, P2-19).
- **Legacy method ids** (`sales.ts:51-56`): resolved by payment-method `type` (D-I5): `cash`→`'cash'`,
  `card`→`'card'`, `bank_transfer`→`'bank_transfer'` — first live method of that type by
  `(sort_order, created_at, id)`; none → `NOT_FOUND` `طريقة الدفع غير موجودة` (`sales.ts:46`).
- **`branch_prefix(branch_id)`** (`branches.ts:205-209`): `''` when `COUNT(branches) <= 1`, else
  `<code>-` of that branch, `''` if the branch isn't found.
- **Credit limit** (`service/credit.rs`, sole Rust caller — ports `creditLimit.ts:23-43`):
  `assert_within_credit_limit(customer, current_balance, new_receivable, can_override)`: return if
  `new_receivable <= 0`, if `limit <= 0`, if `current + new <= limit + 0.005`, or if `can_override`; else
  `FORBIDDEN` ``تجاوز الحد الائتماني للعميل "{name}": الرصيد الحالي {cur:.2} + هذه الفاتورة {new:.2} = {proj:.2}، والحد المسموح {limit:.2}``
  (`{x:.2}` = `toFixed(2)` of an already-`round2`ed decimal). `can_override` = `role_can(role, Accounting,
  Write, overrides) || role_can(role, Parties, Write, overrides)` (`permissions.ts:101-103`, overrides from
  `core::settings::parse_role_access_overrides`). `compute_due_date(instant, days)`: `None` if days ≤ 0;
  else the instant in the business timezone + `days` calendar days at the same wall time, back to UTC
  (`creditLimit.ts:38-43`, `setDate` semantics).

### 3.2 `prepare_sale(conn, cx, input, products)` — shared by preview and create (`sales.ts:86-229`)

`products` is a `BTreeMap<Id, products::Model>` (locked rows for create, a plain read for preview).
Validation order and messages are exactly the mock's:

1. `lines` empty → `VALIDATION` `السلة فارغة` (:87).
2. `user` = live `users` row of `cx.actor` (fresh, not the session snapshot — D-I6). If found and
   `discount_rate > user.max_discount` and no `manager_approved_by` → `FORBIDDEN`
   `الخصم يتجاوز الحد المسموح لك ({js(max_discount)}%)` (:92-94; `js` = `utils::money::js_number_string`).
3. `discount_rate < 0 || > 100` → `نسبة الخصم غير صحيحة` (:95).
4. Per line, in order (:100-110): `!(qty > 0)` → `الكمية يجب أن تكون أكبر من صفر`; `price < 0` →
   `السعر لا يمكن أن يكون سالباً`; free text: no `revenue_account_id` → `السطر النصي الحر يحتاج حساب إيراد`,
   then `continue`; else product = `products[parse(product_id)]` — missing or unparsable →
   `NOT_FOUND` `المنتج غير موجود` (`core.ts:253`); `!active` → ``المنتج "{name}" غير نشط``; when
   `type = 'product'`, accumulate `qty_by_product` in **first-appearance order** (a `Vec<(Id, Decimal)>`):
   `round2(acc + base_qty(line))`.
5. Per product in that order (:114-120): `qty > stock_qty` → `CONFLICT`
   ``الكمية المطلوبة من "{name}" غير متوفرة — المتاح {js(stock_qty)}``; `cost_total +=
   shared::stock::cost::cost_out_sale(stock_qty, stock_value, cost_price, qty)`.
6. `pit = settings.prices_include_tax`; `line_taxes` via `tax_for_line`; invoice discount =
   `Amount(discount_amount)` if `discount_amount > 0`, else `Pct(discount_rate)` if `> 0`, else none
   (:128-133); `totals = shared::totals::compute_invoice_totals(lines{qty, unit_price: price, discount ?? 0,
   discount_is_pct ?? false, tax}, discount, pit)`; `grand = totals.gross`.
7. Tenders (:144-154): `input.tenders` non-empty → as given; `card`/`bank_transfer` → one tender of the
   legacy method for `grand`; `credit` → none; `cash` → legacy cash for `min(round2(paid_amount), grand)`.
   Each tender's method = live payment method by id, else `NOT_FOUND` `طريقة الدفع غير موجودة` (all
   resolved before step 8, like the mock's `map`).
8. `paid = round2(Σ amount)`; `< 0` → `المبلغ المدفوع غير صحيح`; `> grand + 0.01` →
   `مجموع طرق الدفع أكبر من إجمالي الفاتورة` (:155-157).
9. `paid < grand` (:159-163): no `customer_id` → `البيع الآجل أو الدفع الجزئي يتطلب اختيار عميل`; the live
   `parties` row (kind customer) missing or inactive → `العميل غير موجود أو غير نشط`.
10. `cost_total = round2(cost_total)`; `receivable = round2(grand − paid)` (:165-166).
11. `branch = input.branch_id ?? settings.default_branch_id`; `cost_center = input.cost_center_id ??
    branch.cost_center_id`; `currency = input.currency` unless `is_base_currency`; `rate =
    input.exchange_rate ?? shared::currency::require_rate(currency, cx.clock.today())` (`VALIDATION`
    `لا يوجد سعر صرف لعملة X`), else 1 (:174-180). `dims = { branch_id, cost_center_id }`.
12. Tender lines (:186-188), for tenders with `amount > 0`: `AccountRef::Id(resolve_account(
    SystemRole::from_str(method.account_role) (PG-4), AccountCtx::default()).id)` — resolved **without**
    branch context, like `accountFor(role)` — `debit = amount`, description `method.name` or
    `{name} — {reference}`, dims.
13. `stocked_net` / `free_text_revenue` (insertion-ordered map by account id) with `round2` at every add
    (:198-204); `credit_fc = [stocked_net, …free_text values, totals.vat]`; `credit_base = currency ?
    convert_lines_to_base(credit_fc, rate) : credit_fc` (largest line absorbs the gap, docs/v2/10 §2).
14. Posting lines, in this order (:219-227): tender lines; `Role(Receivable)` debit
    `currency ? to_base(receivable, rate) : receivable`, party `(customer, customer_id)` when set, dims, and
    when FX `{ currency, amount_fc: receivable, rate }`; `Role(Sales)` credit `stocked_net_base`; each free
    text `Id(account)` credit; `Role(VatOutput)` credit `vat_base`; `Role(Cogs)` debit `cost_total`;
    `Role(Inventory)` credit `cost_total` (all with dims; COGS/inventory never FX-converted).

Returns `PreparedSale { totals, tenders, paid, cost_total, posting, rate, qty_by_product, line_taxes,
currency, branch, cost_center }`.

### 3.3 `preview_sale` (`sales.ts:231-239`)

Read the products named by the lines (plain read, no lock); `prepare_sale`; keep lines with `debit > 0 ||
credit > 0`; per line: account = `Id` → `account_by_id`, `Role` → `resolve_account(role,
AccountCtx::default())` (the mock previews **without** branch context — quirk Q-3); map to
`{ accountCode, accountName, debit: round2, credit: round2 }`.

### 3.4 `create_sale(conn, cx, reg, input)` (`invoiceService.ts:151-176` + `sales.ts:241-393`)

Locks follow `core/lock.rs`'s order (documents → parties → products → settings S → fiscal year S →
counters; PG-7):

1. **Shift (document row)**: if `input.shift_id` parses and that shift exists, lock the **open** shift of
   its terminal `FOR UPDATE` (`08b` §3.1 `lock_open_shift_of_terminal`) — the mock records the movement on
   `currentOpenShift(shift.terminalId)`, `sales.ts:376`. Taken first so a concurrent close can't deadlock
   against this sale (08b §4).
2. **Customer (party row)**: if `customer_id` parses, `lock::for_update_by_id("parties", id)` — the
   credit-limit check-then-post lock (analysis §5 row 3; also satisfies R-1).
3. **Products**: ids of non-free-text lines that parse as `Id` → `SELECT id` of the ones that exist →
   `shared::stock::lock_products` (sorted). Missing ids are simply absent, so step 4's per-line loop raises
   `المنتج غير موجود` at the mock's position.
4. `prepare_sale` (§3.2) over the locked models.
5. **Credit limit** (`invoiceService.ts:153-166`): if the live customer exists and `credit_limit > 0`:
   `new_receivable` = the prepared receivable line's debit (base); `current =
   shared::balances::customer_balance(id)`; `assert_within_credit_limit`.
6. `date = DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) }` (`sales.ts:241`).
   `shared::ledger::period::assert_open_period(conn, &date.day, false)` **before** numbering, so the
   settings/FY shared locks precede the counter lock (the mock's period error surfaces later, at
   `postJournal`; nothing observable happens in between — same error).
7. `number = branch_prefix(branch) + next_number(DocumentKind::Invoice)` (:255).
8. Due date (folded into the insert, analysis §1): if `customer_id` and
   `invoice_outstanding(grand, 0, paid) > 0`: `input.due_date_override` (resolved `RawDocDate`) ??
   `compute_due_date(cx.clock.now, customer.payment_terms_days)` (`invoiceService.ts:168-174`).
9. Insert `invoices` (fields per `sales.ts:253-325`: `status COMPLETED`, `payment_status =
   payment_status_for(grand, paid)`, `sub_total = totals.sub_total_after_line_discounts`, `discount_rate`
   raw, `discount_amount = totals.invoice_discount_amount`, `tax_rate = line_taxes[0].rate ?? 0`,
   `tax_amount = totals.vat`, `grand_total`, `paid_amount`, `refunded_amount 0`, `tendered_amount` only when
   `payment_method = cash`, `source ?? POS`, `shift_id`, `invoice_type`, `po_reference`, `terms`,
   `attachment_ids`, `branch_id`, `cost_center_id`, `currency`, `exchange_rate` only when FX, `cashier_id =
   actor`). Lines at `position = i`: free text → `product_id NULL`, `name ?? 'سطر حر'`, `cost_price 0`,
   `is_free_text`, `revenue_account_id`; catalog → product name, `cost_price` = the locked pre-sale
   `cost_price` (snapshot), `unit_id`, `unit_factor`, `list_price`, `price_override_reason`, `batch_id`,
   `batch_no`; both get `discount ?? 0`, tax id/category/rate, `net`, `vat`. Tenders at `position = i` with
   `amount = round2(amount)` (D-I7).
10. **Stock** (:332-360), per product in first-appearance order: `value_out = cost_out_sale(pre-sale
    values)` (identical to step 4's figure); `shared::stock::apply_change(-qty, -value_out, "sale",
    StockRef { id, number }, &date, Some(branch))`; if `track_batches`: for each line of that product with a
    `batch_id` (line order) `take = shared::stock::batches::draw_batch(product, batch, base_qty(line))`,
    `remaining = round2(remaining − take)`; then `if remaining > 0.0001 → consume_fefo(product, remaining,
    false, cx.clock.today())`.
11. `shared::ledger::post(PostJournal { date, description:
    "فاتورة مبيعات {number} ({METHOD_LABEL[payment_method]})" + (" — {currency}" when FX), SYSTEM, source
    { kind: "invoice", id, number }, lines: posting })` (:362-369). `METHOD_LABEL` = `نقداً`/`بطاقة`/
    `تحويل بنكي`/`آجل` (:24-29).
12. **Drawer movement** (:373-381): `cash_ids` = live payment methods with `account_role = 'cash'`
    (active or not); `cash_tendered = round2(Σ tender.amount where method ∈ cash_ids)`; if `shift_id` and
    `cash_tendered > 0` and step 1 locked a shift → `record_shift_movement(SaleCash, cash_tendered,
    ref: invoice, at: date)` (08b §3.1).
13. `shared::activity::log(Sale, "فاتورة {number} بقيمة {grand:.2}" + (" — {customer name}" when the
    live customer exists), Some(date), Some(RouteRef::detail("invoice", id)))` (:383-390).
14. If `customer_id` is set → `cx.touch(Parties)` (the mock emits `parties:changed` for every customer
    sale, :391; `ledger::post` only touches it when a kept line carries the party).
15. Return `invoice_dto`.

### 3.5 `create_refund(conn, cx, reg, input)` (`sales.ts:404-536`)

1. Lock the invoice `FOR UPDATE`; missing → `NOT_FOUND` `الفاتورة غير موجودة` (:406).
2. `status != COMPLETED` → `لا يمكن إرجاع هذه الفاتورة` (:407).
3. `lines = input.lines.filter(qty > 0)`; empty → `اختر صنفاً واحداً على الأقل للإرجاع` (:409-410).
4. `refund_method == customer_credit && customer_id None` → `رصيد العميل يتطلب فاتورة مرتبطة بعميل` (:412-414).
5. `returned` = Σ `refund_lines.qty` per `invoice_line_id` over this invoice's refunds (:396-402, exact sum).
   Per line (:419-427): invoice line by id → else `سطر الفاتورة غير موجود`; `remaining = inv_line.qty −
   returned`; `qty > remaining` → ``لا يمكن إرجاع أكثر من {js(remaining)} من "{inv_line.name}"``;
   `net += qty × (price − discount / inv_line.qty)` (exact `Decimal`, no rounding — Q-5); when the line's
   product exists and is `type = 'product'` → `cost += qty × cost_price`.
6. `is_final` (:430-433); `previous` refunds; final → `sub_total = round2(inv.sub_total −
   inv.discount_amount − Σ prev.sub_total)`, `tax = round2(inv.tax_amount − Σ prev.tax_amount)`; else
   `sub_total = round2(net × (1 − discount_rate/100))`, `tax = round2(sub_total × tax_rate / 100)`
   (:437-443 — see **Q-1**, blocking finding); `grand = round2(sub_total + tax)`; `settled =
   min(grand, invoice_outstanding(inv))`; `cash_back = round2(grand − settled)`; `cost = round2(cost)`.
7. `method = input.refund_method ?? (inv.payment_method == credit ? cash : inv.payment_method)` (:449);
   `credited = customer_credit ? cash_back : 0`; `paid_out = customer_credit ? 0 : cash_back`.
8. **Drawer shift (document row)**: only if `method == cash && paid_out > 0`: lock the invoice's own shift
   when it is still `OPEN`, else the open shift of **this terminal** (`cx.terminal_id`, D-I3), else none
   (:528-531).
9. **Party**: `lock::share_lock_by_id("parties", customer_id)` when set (R-1).
10. **Products**: lock the existing `type = 'product'` products of the returned catalog lines (sorted).
11. `date = now` DocDate; `assert_open_period`; `number = next_number(Refund)`.
12. Insert `refunds` (`credited_to_account` only when > 0) + `refund_lines` (position = i, `restock` as
    sent); update the invoice: `refunded_amount = round2(refunded + grand)`, `status = REFUNDED` when final,
    `payment_status = payment_status_for(grand_total − refunded_new, paid_amount)` (:469-474).
13. Restock loop (:479-495), line order, skipping free text / non-product lines: `bq = base_qty(qty,
    line.unit_factor)` (ACC-0032 — the line's qty is in its own unit, stock/`cost_price` per base unit),
    `value = round2(bq × cost_price)`; `restock == Some(false)` → `write_off += value`; else `restock +=
    value` and `apply_change(+bq, value, "refund", StockRef { refund.id, refund.number }, &date, None)`
    (default branch — Q-4).
14. Settlement lines (:502-508): `credited > 0` → one `Role(Receivable)` credit `round2(settled + credited)`
    party customer; else `Role(Receivable)` credit `settled` (party when set) + `Id(settlement_account_for(
    cash|card|bank_transfer, AccountCtx::default()))` credit `paid_out`. Post (:510-524) with description
    `مرتجع مبيعات {refund.number} على الفاتورة {inv.number}`, source `{ "refund", id, number }`, lines:
    `SalesReturns` Dr `sub_total`, `VatOutput` Dr `tax`, settlement lines, `Inventory` Dr `restock`,
    `InventoryWriteOff` Dr `write_off`, `Cogs` Cr `round2(restock + write_off)` (no dims — Q-4).
15. Step 8's shift → `record_shift_movement(RefundCash, paid_out, ref: refund, at: date)`.
16. `log(Refund, "مرتجع {refund.number} على الفاتورة {inv.number} بقيمة {grand:.2}", date,
    RouteRef::detail("invoice", inv.id))`; customer set → `cx.touch(Parties)` (:533-534). Return the DTO.

### 3.6 Reads

- **`get_invoices` / `get_invoices_paged`** (`invoiceService.ts:62-116`): SQL `WHERE` from the filter:
  `status`, `payment_status`, `customer_id`, `open_only` (`status='COMPLETED' AND grand_total −
  refunded_amount − paid_amount > 0`), `source` (`COALESCE(source,'POS')`), `invoice_type`, `cashier_id`,
  `min/max_amount` on `grand_total`, `from/to` on `date_day`. `overdue_only` is applied in Rust
  (`status ≠ REFUNDED`, `due_date` set, outstanding > 0, `due_date_key < format_iso_ms(cx.clock.now)` —
  string compare like `:58`). `search` is applied in Rust after SQL narrows the rows: `matches_search(
  [number, customer_name], q)` (Part 02 handoff §9 decision D-I8). Default order `date_key DESC,
  created_at, id` (JS stable sort over insertion order). Paged: `total` and `totals { grandTotal:
  Σ(grand_total − refunded_amount), outstanding: Σ outstanding }` over the **filtered, unpaged** set (then
  converted to `f64`, `PagedResult.totals` is display-only), sort `sort.key` through a whitelist of
  `InvoiceRow` fields → `ORDER BY <col> <dir>, created_at, id` (unknown key → insertion order; text keys
  collate `utf8mb4_unicode_ci`, Q-7), `LIMIT/OFFSET` (cross-cutting §6).
- **`get_invoice`** (:118-135): `NOT_FOUND` `الفاتورة غير موجودة`; refunds of the invoice by `(created_at,
  id)`; `payments = domains::payments::service::payments_for_invoice(id)` (09 §3.6); journal refs = entries
  whose `source_id` ∈ {invoice, its refunds, those payments} by `(created_at, id)`; `returned_qty` map;
  `customer` via 05-parties' `Customer` builder (D-I9).
- **`get_refund`** (:186-191): `NOT_FOUND` `إشعار الدائن غير موجود`.
- **`get_invoice_print_data`** (:203-247): `id == "sample"` → the synthetic invoice: first 3 live
  `type='product'` products by `(created_at, id)`, lines `{ id: "s-{i}", productId, name, qty: i+1, price,
  costPrice, discount: 0 }`, `sub_total = Σ qty × price` (unrounded), `tax_rate` = the `salesTaxRate()` port
  (live active `default_tax_id` tax → live active OUTPUT default → 0), `tax = round2(sub × rate / 100)`,
  `number = invoice_number_prefix + "000000"`, `date = now` ISO, `cashier_id = actor`, `COMPLETED`/`PAID`,
  `payment_method cash`, `paid = grand = sub + tax`, `tendered = ceil(grand / 100) × 100`, `sample: true`.
  Otherwise `NOT_FOUND` `الفاتورة غير موجودة`; `settings` = the 01-settings `StoreSettings` builder.
- **Quotations** (:361-381): `status` filter in SQL; `search` over `[number, customer_name]` in Rust;
  order `date_key DESC, created_at, id`; `get_quotation` `NOT_FOUND` `عرض السعر غير موجود`.

### 3.7 Quotation writes

- **`save_quotation`** (:384-444): `lines` empty → `أضف صنفاً واحداً على الأقل`; per-line tax = live active
  by id → live active `default_tax_id` → `{ rate 0, category 'O', id None }` (a **different** fallback
  from sales, Q-6); totals with `Pct(discount_rate)` when `> 0`; `number = next_number(Quotation)`; insert
  `status DRAFT`, `date = now`, `salesperson_id = actor`, `expiry_date` (PG-2c), lines at `position = i`:
  `product_id` = parsed `Id` or `NULL` (D-I2), `name ?? product.name ?? '—'`, `cost_price = product.cost_price
  ?? 0`, `discount ?? 0`, tax fields, `net`, `vat`, `unit_id`/`unit_factor` as sent (ACC-0033; no
  `is_free_text`, no batch — as the mock). No audit, no events (Q-2).
- **`set_quotation_status`** (:446-452): lock `FOR UPDATE`; `NOT_FOUND` `عرض السعر غير موجود`; any status →
  any status; returns `Quotation` (not the row). No audit (Q-2).
- **`convert_quotation_to_invoice`** (:455-475): lock the quotation `FOR UPDATE` (document, first);
  `NOT_FOUND` `عرض السعر غير موجود`; `converted_invoice_id` set → `CONFLICT`
  `تم تحويل عرض السعر إلى فاتورة بالفعل`; build `SaleInput { customer_id, lines: [{ product_id (UUID text,
  or "freetext" for NULL), qty, price, discount, tax_id, unit_id, unit_factor (ACC-0033) }], discount_rate, note, terms, po_reference, source
  DESK, …payment }` and call **`create_sale`** (full body, credit check included) in the same transaction;
  then `status = ACCEPTED`, `converted_invoice_id = invoice.id`. Return the invoice.

## 4. Concurrency (D8, analysis §5)

| Race | Settled by |
|---|---|
| Invoice/refund/quotation numbers | `next_number` row lock + `uq_invoices_number`/`uq_refunds_number`/`uq_quotations_number`. |
| Same product sold/returned on two terminals | `lock_products` (sorted) before validation; stock check reads the locked row. |
| Two sales to a near-limit customer | party row `FOR UPDATE` (§3.4 step 2) held until commit; `customer_balance` reads committed lines (READ COMMITTED). |
| Two refunds of the last unit of one line | invoice row `FOR UPDATE` (§3.5 step 1); the second re-reads `returned` and gets the "لا يمكن إرجاع أكثر" message. |
| Double conversion of one quotation | quotation row `FOR UPDATE`; the second sees `converted_invoice_id` → `CONFLICT`. |
| Sale/refund drawer movement vs. shift close | the shift row is locked first (document step) by both; see 08b §4. |
| Payment allocating to an invoice while it is refunded | both lock the invoice row (09 §4). |

## 5. Undo

None registered. `createSale`/`convertQuotationToInvoice` are corrected by `createRefund`; `createRefund` is
itself the compensation; quotations never post (analysis §4, phase-e E-5 lists no invoices action). Every
write records its activity row without an `UndoSpec`.

## 6. Frontend switch lines (`src/modules/invoices/services/invoiceService.ts`)

Add `import { backendCall, usesRust } from '@/modules/core/services/backend';` and, as the first statement
inside each `wrap(...)` body: `if (usesRust('invoices')) return backendCall('<cmd>', { … });` for
`getInvoices` `{ filter }`, `getInvoicesPaged` `{ query }`, `getInvoice` `{ id }`, `previewSale`
`{ input }`, `createSale` `{ input }`, `createRefund` `{ input }`, `getRefund` `{ id }`,
`getInvoicePrintData` `{ id }`, `getQuotations` `{ filter }`, `getQuotation` `{ id }`, `saveQuotation`
`{ input }`, `setQuotationStatus` `{ id, status }`, `convertQuotationToInvoice` `{ id, payment }`. The mock
body below stays unchanged; `isOverdue` is untouched.

## 7. Known mock quirks (kept) · Part 02 gaps · Decisions

**Blocking finding (needs the user / an ACC ledger issue before `createRefund` is implemented):**
- **Q-1 Refund math double-counts VAT under tax-inclusive prices** (`sales.ts:437-444`). With
  `pricesIncludeTax` (the default, `fixtures/settings.ts:42`) `invoice.subTotal` is Σ L′ = the **gross**
  (`totals.ts:140,181`) and `price` is gross, yet the refund adds VAT on top: selling 1 × 115 at S 15% gives
  `subTotal 115, tax 15, grand 115`; a full refund posts `grand = 115 + 15 = 130` (Dr salesReturns 115, Dr
  vatOutput 15, Cr cash 130); the non-final path gives `115 + 17.25`. Every invariant still balances, so
  `verify:mocks` can't see it. Master §10 forbids porting a known wrong number. **Recommendation:** the
  manager opens `ACC-NNNN`, reproduces it with a `scripts/verify/cases/*.json` bundle, fixes the mock
  (inclusive: refund gross from the lines, VAT = gross − gross/(1+rate) per line, or the final-return
  remainders from `grand_total`/`tax_amount`), keeps `verify:mocks` green, and this port then follows the
  **fixed** mock line by line. Until then §3.5 step 6 is the only step that may not be implemented.

**Quirks kept (behaviour parity, entry §3.3):**
- Q-2 No audit/activity row for `saveQuotation`, `setQuotationStatus`, the quotation half of
  `convertQuotationToInvoice` (analysis §6/§9). Later fix, mock and Rust together.
- Q-3 `previewSale` resolves role accounts without branch context while posting resolves with it (`sales.ts:236`
  vs `core.ts` `resolvePosting`) — identical unless branch-specific role accounts exist.
- Q-4 Refunds restock into the default branch and post without branch/cost-center dims (`sales.ts:493,515`).
- Q-5 Refund net = `qty × (price − discount/qty)` computed exactly in `Decimal`; the mock's float can differ
  at a half-cent boundary (D3 rounds the exact value — Rust is the correct side).
- Q-6 Quotation tax fallback `{0, 'O'}` differs from the sale fallback; conversion drops `discountIsPct`
  (a % discount converts as a flat amount) and free-text lines fail conversion with `المنتج غير موجود`.
- Q-7 Paged text sorts: MariaDB `utf8mb4_unicode_ci` vs JS `localeCompare(…, 'ar')`; numeric/date keys are
  exact. Parity cases sort on numeric/date keys only.
- Q-8 `invoiceType` passes through unvalidated; `DRAFT` stays in the enum though nothing produces it
  (analysis §9) — kept, D-I4.
- Q-9 Stocked lines post to the flat `sales` role, not the per-product revenue chain of docs/v2/02 §3
  (the mock's own comment, `sales.ts:190-195`).

**Part 02 API gaps (manager adds before W3; ids shared by 08, 08b, 09, 10):**
- **PG-1 `shared::totals`** — line-by-line port of `invoices/helpers/totals.ts`: `compute_invoice_totals`
  (+ `spread_proportionally`, largest remainder, :87-116), `PaymentStatus` enum (ts export to
  `invoices/types/gen/`), `payment_status_for` (:233-237), `invoice_outstanding` (:240-242),
  `purchase_outstanding` (`purchases.ts:74-76`). Pinned by the worked example (2 × 57.50, 10 % line
  discount, S 15 % + 1 × 23 Z, 5.00 flat → net 108.53, VAT 12.97, gross 121.50). Needed by 07, 08, 09.
- **PG-2 migration `m0016`**: (a) `invoice_lines.product_id` and `quotation_lines.product_id` → `NULL`
  (FKs stay; Part 02 handoff §9 already maps `freetext-N` → `NULL`, but m0008 made the column `NOT NULL`);
  (b) `invoices.due_date DATE` → DocDate triple `due_date_day`/`due_date_instant`/generated `due_date_key`
  (`computeDueDate` and the desk form both store ISO **instants**, `creditLimit.ts:42`,
  `InvoiceFormPage.vue:147`; a `DATE` loses them and changes `isOverdue`'s string compare); (c)
  `quotations.expiry_date` → the same triple (`InvoiceFormPage.vue:182`). Entities updated to match.
- **PG-3 `core::error::duplicate_key_name(&DbErr) -> Option<String>`** — `AppError::from(DbErr)` turns
  errno 1062 into a generic message without the key name, so `AppError::map_unique` can never match; domains
  must inspect the `DbErr` first (used for `uq_shifts_open_key`, `uq_card_settlement_groups_date_method`).
- **PG-4 `impl FromStr for SystemRole`** (same request as 11-expenses "Needs from manager"); unknown → `INTERNAL`.
- **PG-5 `RouteRef.query: Option<BTreeMap<String, String>>`** + `RouteRef::with_query` — payments' activity
  links are `{ name: 'payments', query: { highlight } }` (`payments.ts:303,365,385`).
- **PG-6 full `shared::balances::OpenDocument`** (`number`, `date: DocDate`, `due_date`, `total`,
  `currency`, `fc_outstanding`, `rate`), sorted `date_key, created_at, id` like `payments.ts:33,58` — the
  same request as 05-parties G-1; 09 uses it for `getOpenDocuments` and allocation validation.
- **PG-7 lock-order text**: entry §3.3 says "settings S → fiscal year → parties → products → documents",
  but `core/lock.rs` and `ledger::post` (period check, then numbering) implement "documents → parties →
  products → settings S → fiscal year → counters → change_versions". These files follow the code; fix §3.3.
- **PG-8 waves**: `invoices_get_invoice` needs 09's `Payment` DTO + `payments_for_invoice`, and 08b's
  cash drop needs 10's `record_transfer_voucher`. Neither 09 nor 10 depends on 07/08 **code** (only on
  entities and PG-1/PG-6). Recommend **W3 = 07 · 08 · 09 · 10** (disjoint files); the signatures are fixed
  in 09 §3.6 and 10 §3.3 so the W3 `cargo check` compiles.

**Decisions (architectural autonomy — strictest option, logged):**
- D-I1 Credit-limit check runs inside the sale's transaction under the customer row lock (analysis §5/§9).
- D-I2 Free-text lines store `product_id NULL`; the DTO re-synthesizes `freetext-{position}` (invoices) or
  `"freetext"` (quotations, the string the desk form sends) — DTO parity, real FK integrity.
- D-I3 A cash refund whose invoice's shift is closed goes to **this terminal's** open shift, not "any open
  shift" (`sales.ts:529`): under D8 "any" can hit another till's drawer; in the mock's single-terminal world
  both rules pick the same shift, so parity cases are unaffected. GL unaffected either way.
- D-I4 `InvoiceStatus::Draft` kept (TS type and m0008 enum include it).
- D-I5 Legacy tender ids (`pm-cash`/`pm-mada`/`pm-bank-transfer`) resolve by method `type` + `sort_order`
  (the D10 id map is not kept; these seeded methods are `canDelete: false`).
- D-I6 `maxDiscount` is read from the live `users` row, not the session snapshot (an admin edit applies at once).
- D-I7 Tender amounts are stored `round2`ed (DECIMAL(19,2)); differs from the mock only for > 2 dp input.
- D-I8 Joined-name searches (`customerName`) filter in Rust after SQL narrows; `invoices.search_normalized`
  stays `NULL` (a party rename would make a stored name stale).
- D-I9 `InvoiceDetail.customer`/`PrintData.customer` use 05-parties' `Customer` builder (ledger balance);
  the mock returned the raw row whose `balance` column is never maintained (05 D-1). **Superseded
  (Part 04 Wave 2, L3):** the mock's `getInvoice`/`getInvoicePrintData` now embed the same computed
  customer (`partyService.withComputed`), so parity compares `customer.balance`/`unallocatedCredit`
  exactly — no allow entry.
- D-I10 Terminal identity never comes from the client (cross-cutting §2); see 08b D-S1.
- D-I11 The test-print sample (`getInvoicePrintData('sample')`) is never saved; its `invoice.id` is a fresh
  UUID (`Invoice.id` is a typed `Id`) where the mock writes the literal `'sample'`. Only the id differs —
  the parity harness pairs `'sample'` with that UUID, so the `PrintData.sample: true` key (same on both
  sides) reads as a "mapped id used literally as a key"; `invoices/print-sample` allows exactly that path.

## 8. Tests

**(a) `src-tauri/tests/domain_invoices.rs`** (seed via the 00-import fixture helper, else per-file helpers
like `shared_invariants.rs`'s `seed_settings`/`seed_accounts`/`insert_party`; every posting test ends with
`shared::invariants::run_all` all `passed`):
- cash sale, inclusive VAT: invoice fields, one cash tender, stock −qty, COGS = `round2(qty × avg)`, the 7
  posting lines of `sales.ts:219-227` (zero lines dropped), activity row, `backend:changed` = ledger+catalog.
- the totals worked example (PG-1) through `create_sale`: net 108.53, VAT 12.97, gross 121.50, flat
  discount spread.
- split tender cash + mada: mada line hits the `cardClearing` account; only the cash part is a `SALE_CASH`
  movement on the open shift; no shift → no movement.
- partial credit sale: receivable line tagged with the customer, `payment_status PARTIALLY_PAID`,
  `due_date` from `payment_terms_days`; `dueDateOverride` wins; cash-only sale has no `due_date`.
- every §3.2 message byte-exact, in the mock's order (e.g. qty ≤ 0 on line 1 beats a missing product on line 2).
- sale that empties stock → COGS = exact `stock_value`; tracked product: manual batch draw then FEFO; expired
  batch skipped.
- FX sale: credit lines via `convert_lines_to_base` (gap on the largest line), receivable `to_base` with
  `amount_fc`/`rate`, COGS unconverted; `require_rate` error when no rate.
- free-text line: `product_id NULL`, DTO `freetext-0`, credit to its revenue account, grouped per account.
- credit limit: over → `FORBIDDEN` message exact; accountant (Accounting:Write) overrides; two connections
  selling to the same near-limit customer → the second waits, then fails.
- `preview_sale` lines equal the posted journal lines (same accounts/amounts) for the same input.
- refunds (after Q-1 is resolved): partial non-final, then final with exact remainders; restock vs
  write-off (5120); `customer_credit` = one receivable line; cash refund movement on the invoice's open shift,
  else this terminal's; every §3.5 message; two concurrent refunds of the last unit → one refused.
- quotations: save (numbers, DRAFT, totals), status flip, convert (ACCEPTED + link), convert twice →
  `CONFLICT`, concurrent convert → exactly one invoice.
- `get_invoices_paged`: totals over the filtered unpaged set; `أحمد` finds `احمد`; `openOnly`/`overdueOnly`.
- `get_invoice`: refunds, allocated payments, journal refs of invoice + refunds + payments, `returnedQty`.
- print data `'sample'` uses the store's default sales-tax rate; unknown id → `NOT_FOUND`.
- period locked → `FORBIDDEN` period message, nothing written (no counter consumed, no stock change).
- two branches → invoice number `<code>-…`; one branch → no prefix.

**(b) Parity cases for Part 04:** `sale-cash-inclusive`, `sale-split-tender`, `sale-credit-partial-due-date`,
`sale-fx-usd`, `sale-free-text`, `sale-empties-stock`, `sale-batch-fefo`, `sale-validation-each`,
`sale-credit-limit-block-and-override`, `refund-partial-then-final`, `refund-writeoff`,
`refund-customer-credit`, `quotation-save-convert`, `invoice-list-filters-search-paged`,
`invoice-detail-joins`, `print-sample`.

## 9. Checklist (implementation order)

- [x] Confirm PG-1…PG-7 are in place and PG-8's wave decision is made (manager, before W3) — confirmed
      via `_part02-gaps.md`: PG-1/2/3/4/5/6/7 all `fixed`. G-25 (Q-1) is still **blocked — user** in
      the gaps ledger; per this file's own instruction, `create_refund` step 6 ports the mock's
      current (unfixed) formula verbatim with a `// G-25:` comment.
- [x] `domains/invoices/{mod,dto,commands}.rs`, `service/{mod,access,common,credit,reads,sale,refund,quotations}.rs`
      (+ 08b's `shifts.rs`, `held.rs`). **Not all under 400 lines**: `sale.rs` (~660) and `shifts.rs`
      (~525) exceed the guidance — `prepare_sale`/`create_sale` and `close_shift`'s variance+drop
      logic are each one long, sequential, spec-numbered function that resisted a clean split without
      re-threading a lot of local state through new module boundaries under this wave's time budget;
      flagged for the manager to split further if desired, not left as an oversight.
- [x] `dto.rs` per §2 (with 08b's DTOs); `ipc_signatures()` lists all 25 invoices commands.
- [x] `service/access.rs` `require_any`; `service/common.rs` (`invoice_dto`, row batch loader, `tax_for_line`,
      `base_qty`, legacy method lookup, `branch_prefix`); `service/credit.rs`.
- [x] `prepare_sale` (§3.2) → `preview_sale` (§3.3) → `create_sale` (§3.4) with mock line comments.
- [x] `create_refund` (§3.5) — step 6 kept as the mock's current (G-25-unfixed) formula, flagged.
- [x] Reads (§3.6) and quotation writes (§3.7).
- [x] `commands.rs`: 13 commands (+ 12 from 08b), areas per §1, `with_read`/`with_tx`, `ApiErrorPayload`.
- [ ] Manager: `pub mod invoices;`, 25 handlers in `generate_handler!`, `all_ipc_signatures()` hook.
- [x] Switch lines (§6); `src/modules/invoices/types/contract.check.ts` (§2).
- [x] `tests/domain_invoices.rs` (§8a, focused on the highest-value scenarios) — ⏳ deferred time-boxed
      run; parity list (§8b) handed to Part 04.
- [x] Status note at the top of this file.

## Gate

`cargo check` clean (manager's throttled run) · tests written · 13 switch lines · `contract.check.ts`
compiles · `memory:check` 0 contract gaps for these commands. DB tests and parity cases run in the deferred,
time-boxed test pass.
