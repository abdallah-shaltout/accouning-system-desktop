import { clone, db, delay, inDateRange, includesText, session } from '@/mocks';
import { recordPayment } from '@/mocks/backend/payments';
import { purchaseOutstanding } from '@/mocks/backend/purchases';
import { invoiceOutstanding } from '@/modules/invoices/helpers/totals';
import type { PagedQuery, PagedResult } from '@/modules/core/types/paging';
import type { OpenDocument, Payment, PaymentFilter, PaymentInput } from '../types';

export type PaymentRow = Payment & { partyName: string };

function partyName(p: Payment): string {
  return (p.targetType === 'customer' ? db.customers : db.suppliers).find((x) => x.id === p.targetId)?.name ?? '—';
}

export async function getPayments(filter: PaymentFilter = {}): Promise<PaymentRow[]> {
  await delay();
  return db.payments
    .filter(
      (p) =>
        (!filter.type || p.type === filter.type) &&
        (!filter.method || p.method === filter.method) &&
        (!filter.targetId || p.targetId === filter.targetId) &&
        inDateRange(p.date, filter.from, filter.to),
    )
    .map((p) => ({ ...clone(p), partyName: partyName(p) }))
    .filter((p) => includesText([p.number, p.partyName, p.targetRefNumber, p.note], filter.search))
    .sort((a, b) => b.date.localeCompare(a.date));
}

/** Server-mode variant of `getPayments` for `DataTable`: paged, sorted and totalled server-side. */
export async function getPaymentsPaged(query: PagedQuery<PaymentFilter>): Promise<PagedResult<PaymentRow>> {
  await delay();
  const filter = query.filters ?? {};
  let rows: PaymentRow[] = db.payments
    .filter(
      (p) =>
        (!filter.type || p.type === filter.type) &&
        (!filter.method || p.method === filter.method) &&
        (!filter.targetId || p.targetId === filter.targetId) &&
        inDateRange(p.date, filter.from, filter.to),
    )
    .map((p) => ({ ...clone(p), partyName: partyName(p) }))
    .filter((p) => includesText([p.number, p.partyName, p.targetRefNumber, p.note], filter.search));

  const total = rows.length;
  const totals = { amount: rows.reduce((a, r) => a + r.amount, 0) };

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

export async function createPayment(input: PaymentInput): Promise<Payment> {
  await delay();
  return clone(recordPayment(input, session.userId));
}

/** Invoices (customer) or confirmed POs (supplier) that still have something outstanding. */
export async function getOpenDocuments(targetType: 'customer' | 'supplier', targetId: string): Promise<OpenDocument[]> {
  await delay(150);
  if (targetType === 'customer') {
    return db.invoices
      .filter((i) => i.customerId === targetId && i.status === 'COMPLETED' && invoiceOutstanding(i) > 0)
      .sort((a, b) => a.date.localeCompare(b.date))
      .map((i) => ({ id: i.id, number: i.number, date: i.date, total: i.grandTotal - i.refundedAmount, outstanding: invoiceOutstanding(i) }));
  }
  return db.purchaseOrders
    .filter((p) => p.supplierId === targetId && p.status === 'CONFIRMED' && purchaseOutstanding(p) > 0)
    .sort((a, b) => a.date.localeCompare(b.date))
    .map((p) => ({ id: p.id, number: p.number, date: p.date, total: p.grandTotal - p.returnedAmount, outstanding: purchaseOutstanding(p) }));
}
