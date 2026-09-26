import { ApiError, clone, db, delay, inDateRange, includesText, session, uid } from '@/mocks';
import { accountFor } from '@/mocks/backend/accounts';
import { salesTaxRate } from '@/mocks/backend/core';
import { customerBalance } from '@/mocks/backend/balances';
import { previewSaleJournal, recordRefund, recordSale, returnedQtyByLine } from '@/mocks/backend/sales';
import { closeShift, currentOpenShift, forceCloseShift, openShift, recordShiftMovement, shiftSummary } from '@/mocks/backend/shifts';
import { nextNumber } from '@/mocks/db';
import { assertWithinCreditLimit, computeDueDate } from '@/modules/parties/helpers/creditLimit';
import type { Customer } from '@/modules/parties/types';
import type { Payment } from '@/modules/payments/types';
import type { PagedQuery, PagedResult } from '@/modules/core/types/paging';
import { mutate } from '@/mocks/persist';
import { emit } from '@/mocks/events';
import type { StoreSettings } from '@/modules/settings/types';
import { round2 } from '@/modules/core/helpers/numbers';
import { roleCanOverrideCreditLimit } from '@/modules/users/helpers/permissions';
import { invoiceOutstanding } from '../helpers/totals';
import { wrap } from '@/modules/diagnostics/services/defineService';

import type {
  CloseShiftInput,
  HeldSale,
  Invoice,
  InvoiceFilter,
  JournalPreviewLine,
  OpenShiftInput,
  Quotation,
  QuotationStatus,
  Refund,
  RefundInput,
  SaleInput,
  Shift,
} from '../types';

export type InvoiceRow = Invoice & { customerName?: string; cashierName: string; outstanding: number };

export interface InvoiceDetail extends InvoiceRow {
  customer?: Customer;
  refunds: Refund[];
  payments: Payment[];
  journalEntries: { id: string; number: string; description: string }[];
  /** invoiceLineId → quantity already returned. */
  returnedQty: Record<string, number>;
}

function toRow(inv: Invoice): InvoiceRow {
  return {
    ...clone(inv),
    customerName: db.customers.find((c) => c.id === inv.customerId)?.name,
    cashierName: db.users.find((u) => u.id === inv.cashierId)?.name ?? '—',
    outstanding: inv.status === 'REFUNDED' ? 0 : invoiceOutstanding(inv),
  };
}

/** True when an invoice has an outstanding balance past its due date — drives the "overdue" list-v2 filter. */
export const isOverdue = wrap('invoices.isOverdue', function isOverdue(inv: Pick<Invoice, 'status' | 'dueDate' | 'grandTotal' | 'refundedAmount' | 'paidAmount'>): boolean {
  if (inv.status === 'REFUNDED' || !inv.dueDate) return false;
  return invoiceOutstanding(inv) > 0 && inv.dueDate < new Date().toISOString();
});

/** Shared predicate for `getInvoices`/`getInvoicesPaged` (docs/v2/06 §6 "Invoice list v2" filters). */
function matchesFilter(i: Invoice, filter: InvoiceFilter & { openOnly?: boolean }): boolean {
  return (
    (!filter.status || i.status === filter.status) &&
    (!filter.paymentStatus || i.paymentStatus === filter.paymentStatus) &&
    (!filter.customerId || i.customerId === filter.customerId) &&
    (!filter.openOnly || (i.status === 'COMPLETED' && invoiceOutstanding(i) > 0)) &&
    (!filter.source || (i.source ?? 'POS') === filter.source) &&
    (!filter.invoiceType || i.invoiceType === filter.invoiceType) &&
    (!filter.cashierId || i.cashierId === filter.cashierId) &&
    (!filter.overdueOnly || isOverdue(i)) &&
    (filter.minAmount === undefined || i.grandTotal >= filter.minAmount) &&
    (filter.maxAmount === undefined || i.grandTotal <= filter.maxAmount) &&
    inDateRange(i.date, filter.from, filter.to)
  );
}

export const getInvoices = wrap('invoices.getInvoices', async function getInvoices(filter: InvoiceFilter & { openOnly?: boolean } = {}): Promise<InvoiceRow[]> {
  await delay();
  return db.invoices
    .filter((i) => matchesFilter(i, filter))
    .map(toRow)
    .filter((r) => includesText([r.number, r.customerName], filter.search))
    .sort((a, b) => b.date.localeCompare(a.date));
});

/** Server-mode variant of `getInvoices` for `DataTable`: paged, sorted and totalled server-side. */
export const getInvoicesPaged = wrap('invoices.getInvoicesPaged', async function getInvoicesPaged(query: PagedQuery<InvoiceFilter & { openOnly?: boolean }>): Promise<PagedResult<InvoiceRow>> {
  await delay();
  const filter = query.filters ?? {};
  let rows = db.invoices
    .filter((i) => matchesFilter(i, filter))
    .map(toRow)
    .filter((r) => includesText([r.number, r.customerName], filter.search));

  const total = rows.length;
  const totals = {
    grandTotal: rows.reduce((a, r) => a + r.grandTotal - r.refundedAmount, 0),
    outstanding: rows.reduce((a, r) => a + r.outstanding, 0),
  };

  const sort = query.sort;
  rows = [...rows].sort((a, b) => {
    if (sort) {
      const va = (a as any)[sort.key];
      const vb = (b as any)[sort.key];
      const dir = sort.dir === 'asc' ? 1 : -1;
      if (typeof va === 'number' && typeof vb === 'number') return (va - vb) * dir;
      return String(va ?? '').localeCompare(String(vb ?? ''), 'ar') * dir;
    }
    return b.date.localeCompare(a.date);
  });

  const start = (query.page - 1) * query.pageSize;
  return { rows: rows.slice(start, start + query.pageSize), total, totals };
});

export const getInvoice = wrap('invoices.getInvoice', async function getInvoice(id: string): Promise<InvoiceDetail> {
  await delay();
  const inv = db.invoices.find((i) => i.id === id);
  if (!inv) throw new ApiError('الفاتورة غير موجودة', 'NOT_FOUND');
  const refunds = db.refunds.filter((r) => r.invoiceId === id);
  const payments = db.payments.filter((p) => p.type === 'RECEIVED' && p.allocations.some((a) => a.targetKind === 'invoice' && a.targetId === id));
  const sourceIds = new Set([id, ...refunds.map((r) => r.id), ...payments.map((p) => p.id)]);
  return {
    ...toRow(inv),
    customer: clone(db.customers.find((c) => c.id === inv.customerId)),
    refunds: clone(refunds),
    payments: clone(payments),
    journalEntries: db.journalEntries
      .filter((e) => e.sourceRef && sourceIds.has(e.sourceRef.id))
      .map((e) => ({ id: e.id, number: e.number, description: e.description })),
    returnedQty: Object.fromEntries(returnedQtyByLine(id)),
  };
});

/** The double-entry the sale *would* post — nothing is saved. */
export const previewSale = wrap('invoices.previewSale', async function previewSale(input: SaleInput): Promise<JournalPreviewLine[]> {
  await delay(120);
  return previewSaleJournal(input, session.userId);
});

/**
 * Credit-limit check (docs/v2/02-accounting-review.md D3, Phase 4's item) runs here, in front of
 * `recordSale` — `src/mocks/backend/sales.ts` is Phase 3's file and isn't touched. The would-be
 * receivable is read off `previewSaleJournal`'s `receivable`-account line (same totals math Phase 3
 * owns; we just read its result). `dueDate` is stamped the same way: computed from the customer's
 * `paymentTermsDays` and written onto the invoice object already pushed into `db.invoices` by
 * `recordSale`.
 */
export const createSale = wrap('invoices.createSale', async function createSale(input: SaleInput): Promise<Invoice> {
  await delay(350);
  if (input.customerId) {
    const customer = db.customers.find((c) => c.id === input.customerId);
    if (customer && (customer.creditLimit ?? 0) > 0) {
      const receivableCode = accountFor('receivable').code;
      const preview = previewSaleJournal(input, session.userId);
      const newReceivable = preview.find((l) => l.accountCode === receivableCode)?.debit ?? 0;
      assertWithinCreditLimit({
        customer,
        currentBalance: customerBalance(customer.id),
        newReceivable,
        canOverride: roleCanOverrideCreditLimit(db.users.find((u) => u.id === session.userId)?.role),
      });
    }
  }
  const invoice = recordSale(input, session.userId);
  if (invoice.customerId && invoiceOutstanding(invoice) > 0) {
    const customer = db.customers.find((c) => c.id === invoice.customerId);
    // v2 phase 7 (§2 desk form "due date (date + customer terms)... editable"): an explicit override
    // from the form wins over the computed customer-terms date.
    const dueDate = input.dueDateOverride ?? computeDueDate(invoice.date, customer?.paymentTermsDays);
    if (dueDate) mutate(() => (invoice.dueDate = dueDate));
  }
  return clone(invoice);
});

export const createRefund = wrap('invoices.createRefund', async function createRefund(input: RefundInput): Promise<Refund> {
  await delay();
  return clone(recordRefund(input, session.userId));
});

/** v2 phase 11b (docs/v2/12-documents-pdf-excel.md §3 "credit note"): looks up a single refund by
 * id for `pdfService`'s credit-note payload — refunds don't have their own detail route/page
 * (shown inline on the invoice they belong to), so this is the first standalone getter. */
export const getRefund = wrap('invoices.getRefund', async function getRefund(id: string): Promise<Refund> {
  await delay();
  const refund = db.refunds.find((r) => r.id === id);
  if (!refund) throw new ApiError('إشعار الدائن غير موجود', 'NOT_FOUND');
  return clone(refund);
});

export interface PrintData {
  invoice: Invoice;
  customer?: Customer;
  cashierName: string;
  settings: StoreSettings;
  /** true when rendering the settings "test print" sample. */
  sample?: boolean;
}

/** Everything a printed invoice needs. `id = 'sample'` returns a demo invoice (not saved) for test prints. */
export const getInvoicePrintData = wrap('invoices.getInvoicePrintData', async function getInvoicePrintData(id: string): Promise<PrintData> {
  await delay(150);
  if (id === 'sample') {
    const now = new Date().toISOString();
    const products = db.products.filter((p) => p.type === 'product').slice(0, 3);
    const lines = products.map((p, i) => ({ id: `s-${i}`, productId: p.id, name: p.name, qty: i + 1, price: p.price, costPrice: p.costPrice, discount: 0 }));
    const subTotal = lines.reduce((a, l) => a + l.qty * l.price, 0);
    // v2 doc 18.D: the test-print sample used to hard-code 15% — now reads the store's actual
    // active sales-tax rate, so an Egyptian company's test print shows 14%, not a stray 15%.
    const taxRate = salesTaxRate();
    const taxAmount = round2((subTotal * taxRate) / 100);
    return {
      invoice: {
        id: 'sample',
        number: `${db.settings.invoiceNumberPrefix}000000`,
        date: now,
        cashierId: session.userId,
        status: 'COMPLETED',
        paymentStatus: 'PAID',
        lines,
        subTotal,
        discountRate: 0,
        discountAmount: 0,
        taxRate,
        taxAmount,
        grandTotal: subTotal + taxAmount,
        paymentMethod: 'cash',
        paidAmount: subTotal + taxAmount,
        refundedAmount: 0,
        tenderedAmount: Math.ceil((subTotal + taxAmount) / 100) * 100,
      },
      cashierName: db.users.find((u) => u.id === session.userId)?.name ?? '—',
      settings: clone(db.settings),
      sample: true,
    };
  }
  const inv = db.invoices.find((i) => i.id === id);
  if (!inv) throw new ApiError('الفاتورة غير موجودة', 'NOT_FOUND');
  return {
    invoice: clone(inv),
    customer: clone(db.customers.find((c) => c.id === inv.customerId)),
    cashierName: db.users.find((u) => u.id === inv.cashierId)?.name ?? '—',
    settings: clone(db.settings),
  };
});

// =================================================================================================
// v2 phase 7 §5 — Shifts (docs/v2/06-sales-and-pos.md §5)
// =================================================================================================

export type ShiftRow = Shift & ReturnType<typeof shiftSummary> & { openedByName: string; closedByName?: string };

function toShiftRow(s: Shift): ShiftRow {
  return {
    ...clone(s),
    ...shiftSummary(s),
    openedByName: db.users.find((u) => u.id === s.openedBy)?.name ?? '—',
    closedByName: s.closedBy ? db.users.find((u) => u.id === s.closedBy)?.name : undefined,
  };
}

/** The currently-open shift for a terminal, or undefined. Used by the POS shift bar to decide whether to show "open shift" or the running totals. */
export const getCurrentShift = wrap('invoices.getCurrentShift', async function getCurrentShift(terminalId: string): Promise<ShiftRow | undefined> {
  await delay(80);
  const shift = currentOpenShift(terminalId);
  return shift ? toShiftRow(shift) : undefined;
});

export const getShifts = wrap('invoices.getShifts', async function getShifts(filter: { status?: 'OPEN' | 'CLOSED' } = {}): Promise<ShiftRow[]> {
  await delay();
  return db.shifts
    .filter((s) => !filter.status || s.status === filter.status)
    .map(toShiftRow)
    .sort((a, b) => b.openedAt.localeCompare(a.openedAt));
});

export const getShift = wrap('invoices.getShift', async function getShift(id: string): Promise<ShiftRow> {
  await delay();
  const shift = db.shifts.find((s) => s.id === id);
  if (!shift) throw new ApiError('الوردية غير موجودة', 'NOT_FOUND');
  return toShiftRow(shift);
});

export const openPosShift = wrap('invoices.openPosShift', async function openPosShift(input: OpenShiftInput): Promise<Shift> {
  await delay(200);
  return clone(openShift(input, session.userId));
});

/** Mid-shift snapshot (§5 "X-report"): same shape as the close screen, just without closing anything. */
export const getXReport = wrap('invoices.getXReport', async function getXReport(shiftId: string): Promise<ShiftRow> {
  await delay(120);
  return getShift(shiftId);
});

export const closePosShift = wrap('invoices.closePosShift', async function closePosShift(shiftId: string, input: CloseShiftInput): Promise<Shift> {
  await delay(250);
  return clone(closeShift(shiftId, input, session.userId));
});

/** Manager screen (§5 "/pos/shifts"): force-close an open shift left behind by a cashier. */
export const forceClosePosShift = wrap('invoices.forceClosePosShift', async function forceClosePosShift(shiftId: string, countedCash?: number): Promise<Shift> {
  await delay(250);
  return clone(forceCloseShift(shiftId, session.userId, countedCash));
});

/**
 * Pay-in/pay-out from the shift bar (F10). docs/v2/06-sales-and-pos.md §1 says a pay-out for a
 * small expense should also create an expense voucher paid from the drawer — Phase 8's
 * `modules/expenses/services/expenseService.ts::recordExpense` now exists and could post that
 * voucher, but it requires picking an expense category (a full form field this quick F10 dialog
 * intentionally doesn't have — it's a one-amount-and-a-note drawer log, not the expense form).
 * Left as a follow-up UX decision rather than wired in here: this call still records the shift's
 * own cash movement, so the X/Z report and close-screen expected-cash math are correct either way
 * — only the *separate* expense-side journal entry (for reporting a pay-out as a categorized
 * expense) is the still-missing piece, not the drawer accounting itself.
 */
export const recordCashInOut = wrap('invoices.recordCashInOut', async function recordCashInOut(terminalId: string, kind: 'PAY_IN' | 'PAY_OUT' | 'BANK_DROP', amount: number, note?: string): Promise<void> {
  await delay(150);
  const shift = currentOpenShift(terminalId);
  if (!shift) throw new ApiError('لا توجد وردية مفتوحة', 'CONFLICT');
  if (!(amount > 0)) throw new ApiError('المبلغ يجب أن يكون أكبر من صفر');
  recordShiftMovement(terminalId, kind, amount, session.userId, { note });
  emit('ledger:changed');
});

// =================================================================================================
// v2 phase 7 §1 — Held sales (POS "F6")
// =================================================================================================

export const getHeldSales = wrap('invoices.getHeldSales', async function getHeldSales(terminalId: string): Promise<HeldSale[]> {
  await delay(80);
  return clone(db.heldSales.filter((h) => h.terminalId === terminalId)).sort((a, b) => b.heldAt.localeCompare(a.heldAt));
});

export const holdSale = wrap('invoices.holdSale', async function holdSale(input: Omit<HeldSale, 'id' | 'heldAt' | 'heldBy'>): Promise<HeldSale> {
  await delay(120);
  const held: HeldSale = { ...input, id: uid('hold'), heldAt: new Date().toISOString(), heldBy: session.userId };
  mutate(() => db.heldSales.push(held));
  return clone(held);
});

export const resumeHeldSale = wrap('invoices.resumeHeldSale', async function resumeHeldSale(id: string): Promise<HeldSale> {
  await delay(80);
  const held = db.heldSales.find((h) => h.id === id);
  if (!held) throw new ApiError('لا يوجد بيع معلّق بهذا المعرف', 'NOT_FOUND');
  mutate(() => (db.heldSales = db.heldSales.filter((h) => h.id !== id)));
  return clone(held);
});

export const discardHeldSale = wrap('invoices.discardHeldSale', async function discardHeldSale(id: string): Promise<void> {
  await delay(80);
  mutate(() => (db.heldSales = db.heldSales.filter((h) => h.id !== id)));
});

// =================================================================================================
// v2 phase 7 §2 — Quotations (docs/v2/06-sales-and-pos.md §2 "Quotations")
// =================================================================================================

export type QuotationRow = Quotation & { customerName?: string };

function toQuotationRow(q: Quotation): QuotationRow {
  return { ...clone(q), customerName: db.customers.find((c) => c.id === q.customerId)?.name };
}

export const getQuotations = wrap('invoices.getQuotations', async function getQuotations(filter: { status?: QuotationStatus; search?: string } = {}): Promise<QuotationRow[]> {
  await delay();
  return db.quotations
    .filter((q) => !filter.status || q.status === filter.status)
    .map(toQuotationRow)
    .filter((r) => includesText([r.number, r.customerName], filter.search))
    .sort((a, b) => b.date.localeCompare(a.date));
});

export const getQuotation = wrap('invoices.getQuotation', async function getQuotation(id: string): Promise<QuotationRow> {
  await delay();
  const q = db.quotations.find((x) => x.id === id);
  if (!q) throw new ApiError('عرض السعر غير موجود', 'NOT_FOUND');
  return toQuotationRow(q);
});

/** Never posts to the ledger or touches stock — see the `Quotation` type's doc comment. */
export const saveQuotation = wrap('invoices.saveQuotation', async function saveQuotation(input: {
  customerId?: string;
  expiryDate?: string;
  lines: SaleInput['lines'];
  discountRate: number;
  note?: string;
  terms?: string;
  poReference?: string;
}): Promise<Quotation> {
  await delay(250);
  if (!input.lines.length) throw new ApiError('أضف صنفاً واحداً على الأقل');
  const pricesIncludeTax = db.settings.pricesIncludeTax !== false;
  const { computeInvoiceTotals } = await import('../helpers/totals');
  const lineTaxes = input.lines.map((l) => {
    const tax = (l.taxId && db.taxes.find((t) => t.id === l.taxId && t.active)) ?? db.taxes.find((t) => t.id === db.settings.defaultTaxId && t.active);
    return tax ? { rate: tax.rate, category: tax.category, id: tax.id } : { rate: 0, category: 'O' as const };
  });
  const totals = computeInvoiceTotals(
    input.lines.map((l, i) => ({ qty: l.qty, unitPrice: l.price, discount: l.discount ?? 0, discountIsPct: l.discountIsPct ?? false, tax: lineTaxes[i] })),
    input.discountRate > 0 ? { pct: input.discountRate } : undefined,
    pricesIncludeTax,
  );
  const id = uid('quo');
  const quotation: Quotation = {
    id,
    number: nextNumber('quotation'),
    date: new Date().toISOString(),
    expiryDate: input.expiryDate,
    customerId: input.customerId,
    salespersonId: session.userId,
    status: 'DRAFT',
    lines: input.lines.map((l, i) => {
      const lr = totals.lines[i];
      const product = db.products.find((p) => p.id === l.productId);
      return {
        id: `${id}-l${i + 1}`,
        productId: l.productId,
        name: l.name ?? product?.name ?? '—',
        qty: l.qty,
        price: l.price,
        costPrice: product?.costPrice ?? 0,
        discount: l.discount ?? 0,
        taxId: lineTaxes[i].id,
        taxCategory: lineTaxes[i].category,
        taxRate: lineTaxes[i].rate,
        net: lr.net,
        vat: lr.vat,
      };
    }),
    discountRate: input.discountRate,
    discountAmount: totals.invoiceDiscountAmount,
    taxAmount: totals.vat,
    subTotal: totals.subTotalAfterLineDiscounts,
    grandTotal: totals.gross,
    note: input.note,
    terms: input.terms,
    poReference: input.poReference,
  };
  mutate(() => db.quotations.push(quotation));
  return clone(quotation);
});

export const setQuotationStatus = wrap('invoices.setQuotationStatus', async function setQuotationStatus(id: string, status: QuotationStatus): Promise<Quotation> {
  await delay(150);
  const q = db.quotations.find((x) => x.id === id);
  if (!q) throw new ApiError('عرض السعر غير موجود', 'NOT_FOUND');
  mutate(() => (q.status = status));
  return clone(q);
});

/** "Convert → invoice" (§2): copies every line as-is into a real sale; the quotation is marked ACCEPTED and linked. */
export const convertQuotationToInvoice = wrap('invoices.convertQuotationToInvoice', async function convertQuotationToInvoice(id: string, payment: { paymentMethod: SaleInput['paymentMethod']; paidAmount: number; tenderedAmount?: number }): Promise<Invoice> {
  await delay(300);
  const q = db.quotations.find((x) => x.id === id);
  if (!q) throw new ApiError('عرض السعر غير موجود', 'NOT_FOUND');
  if (q.convertedInvoiceId) throw new ApiError('تم تحويل عرض السعر إلى فاتورة بالفعل', 'CONFLICT');
  const invoice = await createSale({
    customerId: q.customerId,
    lines: q.lines.map((l) => ({ productId: l.productId, qty: l.qty, price: l.price, discount: l.discount, taxId: l.taxId })),
    discountRate: q.discountRate,
    note: q.note,
    terms: q.terms,
    poReference: q.poReference,
    source: 'DESK',
    ...payment,
  });
  mutate(() => {
    q.status = 'ACCEPTED';
    q.convertedInvoiceId = invoice.id;
  });
  return invoice;
});
