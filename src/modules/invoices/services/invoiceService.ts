import { ApiError, clone, db, delay, inDateRange, includesText, session } from '@/mocks';
import { accountFor } from '@/mocks/backend/accounts';
import { customerBalance } from '@/mocks/backend/balances';
import { previewSaleJournal, recordRefund, recordSale, returnedQtyByLine } from '@/mocks/backend/sales';
import { assertWithinCreditLimit, computeDueDate } from '@/modules/parties/helpers/creditLimit';
import type { Customer } from '@/modules/parties/types';
import type { Payment } from '@/modules/payments/types';
import type { PagedQuery, PagedResult } from '@/modules/core/types/paging';
import { mutate } from '@/mocks/persist';
import type { StoreSettings } from '@/modules/settings/types';
import { roleCanOverrideCreditLimit } from '@/modules/users/helpers/permissions';
import { invoiceOutstanding } from '../helpers/totals';
import type { Invoice, InvoiceFilter, JournalPreviewLine, Refund, RefundInput, SaleInput } from '../types';

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

export async function getInvoices(filter: InvoiceFilter & { openOnly?: boolean } = {}): Promise<InvoiceRow[]> {
  await delay();
  return db.invoices
    .filter(
      (i) =>
        (!filter.status || i.status === filter.status) &&
        (!filter.paymentStatus || i.paymentStatus === filter.paymentStatus) &&
        (!filter.customerId || i.customerId === filter.customerId) &&
        (!filter.openOnly || (i.status === 'COMPLETED' && invoiceOutstanding(i) > 0)) &&
        inDateRange(i.date, filter.from, filter.to),
    )
    .map(toRow)
    .filter((r) => includesText([r.number, r.customerName], filter.search))
    .sort((a, b) => b.date.localeCompare(a.date));
}

/** Server-mode variant of `getInvoices` for `DataTable`: paged, sorted and totalled server-side. */
export async function getInvoicesPaged(query: PagedQuery<InvoiceFilter & { openOnly?: boolean }>): Promise<PagedResult<InvoiceRow>> {
  await delay();
  const filter = query.filters ?? {};
  let rows = db.invoices
    .filter(
      (i) =>
        (!filter.status || i.status === filter.status) &&
        (!filter.paymentStatus || i.paymentStatus === filter.paymentStatus) &&
        (!filter.customerId || i.customerId === filter.customerId) &&
        (!filter.openOnly || (i.status === 'COMPLETED' && invoiceOutstanding(i) > 0)) &&
        inDateRange(i.date, filter.from, filter.to),
    )
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
}

export async function getInvoice(id: string): Promise<InvoiceDetail> {
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
}

/** The double-entry the sale *would* post — nothing is saved. */
export async function previewSale(input: SaleInput): Promise<JournalPreviewLine[]> {
  await delay(120);
  return previewSaleJournal(input, session.userId);
}

/**
 * Credit-limit check (docs/v2/02-accounting-review.md D3, Phase 4's item) runs here, in front of
 * `recordSale` — `src/mocks/backend/sales.ts` is Phase 3's file and isn't touched. The would-be
 * receivable is read off `previewSaleJournal`'s `receivable`-account line (same totals math Phase 3
 * owns; we just read its result). `dueDate` is stamped the same way: computed from the customer's
 * `paymentTermsDays` and written onto the invoice object already pushed into `db.invoices` by
 * `recordSale`.
 */
export async function createSale(input: SaleInput): Promise<Invoice> {
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
    const dueDate = computeDueDate(invoice.date, customer?.paymentTermsDays);
    if (dueDate) mutate(() => (invoice.dueDate = dueDate));
  }
  return clone(invoice);
}

export async function createRefund(input: RefundInput): Promise<Refund> {
  await delay();
  return clone(recordRefund(input, session.userId));
}

export interface PrintData {
  invoice: Invoice;
  customer?: Customer;
  cashierName: string;
  settings: StoreSettings;
  /** true when rendering the settings "test print" sample. */
  sample?: boolean;
}

/** Everything a printed invoice needs. `id = 'sample'` returns a demo invoice (not saved) for test prints. */
export async function getInvoicePrintData(id: string): Promise<PrintData> {
  await delay(150);
  if (id === 'sample') {
    const now = new Date().toISOString();
    const products = db.products.filter((p) => p.type === 'product').slice(0, 3);
    const lines = products.map((p, i) => ({ id: `s-${i}`, productId: p.id, name: p.name, qty: i + 1, price: p.price, costPrice: p.costPrice, discount: 0 }));
    const subTotal = lines.reduce((a, l) => a + l.qty * l.price, 0);
    const taxAmount = Math.round(subTotal * 15) / 100;
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
        taxRate: 15,
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
}
