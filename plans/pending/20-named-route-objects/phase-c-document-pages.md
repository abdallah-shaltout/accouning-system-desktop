# 20.C — Pages: sales/POS, purchases, payments, vouchers, expenses, parties

**Status: pending.** Depends on 20.B.

Goal: the document modules navigate only with named route objects. These modules hold the
create → detail → print → back flows, so they go first, together with the print-page `?back`
change that ties them together.

Convert every site one-to-one (same destination, query and back target). The count in brackets
is what `check-routes.js` reports per file today. The file must reach 0.

## Tasks

### C1 — Print back target: `?back=<path>` → `?from=pos` (Decision 9)
- [ ] `invoices/pages/InvoicePrintPage.vue:61`: `backTo` becomes
  `route.query.from === 'pos' ? { name: 'pos' } : id === 'sample' ? { name: 'settings-printing' } : { name: 'invoice', params: { id } }`.
- [ ] `invoices/pages/PosPage.vue:505`: `{ name: 'invoice-print', params: { id }, query: { auto: '1', from: 'pos' } }`.
- [ ] `invoices/pages/InvoiceFormPage.vue:281`: `andPrint ? { name: 'invoice-print', params: { id }, query: { auto: '1' } } : { name: 'invoice', params: { id } }`
  (its old `back` was the default target, so it is dropped).
- [ ] `settings/pages/PrintingSettingsPage.vue:118`: `{ name: 'invoice-print', params: { id: 'sample' }, query: { mode, width } }`
  (its old `back` was also the default).

### C2 — invoices (sales, quotations, POS, shifts)
- [ ] `InvoiceDetailPage` [7]: `back` → `invoices`; print → `invoice-print`; refund → `invoice-refund`;
  pay → `{ name: 'payment-new', query: { type: 'RECEIVED', party, ref } }`; product/customer/journal links.
- [ ] `InvoiceFormPage` [5]: quotation saved → `quotation`; `back` → `invoices` (C1 covers line 281).
- [ ] `InvoiceListPage` [2], `QuotationListPage` [2], `QuotationDetailPage` [3], `RefundPage` [2]
  (`:back` → `{ name: 'invoice', params: { id } }`), `ShiftReportPage` [1], `ShiftsManagerPage` [1] (`pos-shift-report`).
- [ ] `PosPage` [4]: `'/pos/shifts'` → `pos-shifts`; back button `to="/"` → `home`.

### C3 — purchases
- [ ] `PurchaseDetailPage` [10]: `back`, `purchase-edit` / `purchase-receive` / `purchase-return` / `purchase-print`,
  supplier, product and journal links, pay → `{ name: 'payment-new', query: { type: 'PAID', party, ref } }`.
- [ ] `PurchaseFormPage` [3]: line 235's ternary has the same target in both branches → one
  `{ name: 'purchase', params: { id: po.id } }`. Lines 85 and 230 are the same.
- [ ] `PurchaseListPage` [2], `PurchasePrintPage` [1], `PurchaseReceivePage` [4] (labels:
  `{ name: 'labels', query }` at line 111), `PurchaseReturnPage` [3].

### C4 — payments and vouchers
- [ ] `PaymentListPage` [4] (`payment-new` + `query.type`, party links), `PaymentFormPage` [2]
  (`{ name: 'payments', query: { highlight } }`: the e2e flow `refund_payment.py:70` reads `highlight=`
  from the URL, and the URL does not change), `PaymentDetailPage` [4] (`docLink()` returns `AppRoute`).
- [ ] `VoucherListPage` [2], `VoucherFormPage` [2], `VoucherDetailPage` [3], `VoucherPrintPage` [1],
  `CardSettlementPage` [1] (`payments`).

### C5 — expenses
- [ ] `ExpenseListPage` [2], `ExpenseFormPage` [2], `ExpenseDetailPage` [2], `RecurringExpensesPage` [1].

### C6 — parties (one helper instead of path building in 4 files)
Customer and supplier screens build paths from `isCustomer` in many places, and `duplicateLink()` is
copied in `PartyFormModal.vue:65` and `PartyFormPage.vue:196` (CLAUDE.md rule 2: never duplicate).
- [ ] New `parties/helpers/partyRoutes.ts` (pure, typed):
  `partyRoute(kind, 'list' | 'detail' | 'new' | 'edit', id?) → AppRoute`, and
  `partyRouteById(id)` for the duplicate warning (keeps today's `id.startsWith('sup')` rule).
- [ ] `PartyDetailPage` [12]: `base` computed → `partyRoute(kind, 'list')`; edit; linked party; `docLink()`
  and `payLink()` return `AppRoute`; the invoice/purchase row clicks at lines 263 and 299.
- [ ] `PartyFormPage` [2 + ternaries]: `back`, cancel, saved → detail, linked party, `duplicateLink` → helper.
- [ ] `PartyListPage` [2], `PartyFormModal` [1] (`customer-new` / `supplier-new`, `duplicateLink` → helper).
- [ ] `bun run memory` (new helper file = structural).

## Gate
- [ ] `node scripts/check-routes.js`: 0 findings in `invoices`, `purchases`, `payments`, `vouchers`,
  `expenses`, `parties`, and in `PrintingSettingsPage`.
- [ ] By hand: POS sale → print → back lands on POS. Invoice form "save & print" → print → back lands on the
  invoice. Printing settings sample → back lands on printing settings. Purchase create → receive → return → print → back.
  Customer and supplier: list → detail → edit → cancel, the duplicate-name warning link, the linked-party link.
- [ ] `bun run build`, `bun run check`, `bun run verify:mocks`, `bun run memory`, full e2e suite, no console errors.
