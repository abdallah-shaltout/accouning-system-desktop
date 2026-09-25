import { clone, db, delay, inDateRange, includesText, session } from '@/mocks';
import { allocatePayment, allocatedTotal, getOpenDocumentsFor, recordPayment, unallocatedAmount, unallocatePayment } from '@/mocks/backend/payments';
import type { PagedQuery, PagedResult } from '@/modules/core/types/paging';
import { allocationStatusFor, type AllocationStatus, type OpenDocument, type Payment, type PaymentAllocationInput, type PaymentFilter, type PaymentInput } from '../types';

import { wrap } from '@/modules/diagnostics/services/defineService';

export type PaymentRow = Payment & { partyName: string; allocated: number; unallocated: number; allocationStatus: AllocationStatus };

function partyName(p: Payment): string {
  return (p.targetType === 'customer' ? db.customers : db.suppliers).find((x) => x.id === p.targetId)?.name ?? '—';
}

function toRow(p: Payment): PaymentRow {
  const allocated = allocatedTotal(p);
  return { ...clone(p), partyName: partyName(p), allocated, unallocated: unallocatedAmount(p), allocationStatus: allocationStatusFor(p.amount, allocated) };
}

export const getPayments = wrap('payments.getPayments', async function getPayments(filter: PaymentFilter = {}): Promise<PaymentRow[]> {
  await delay();
  return db.payments
    .filter(
      (p) =>
        (!filter.type || p.type === filter.type) &&
        (!filter.method || p.method === filter.method) &&
        (!filter.targetId || p.targetId === filter.targetId) &&
        inDateRange(p.date, filter.from, filter.to),
    )
    .map(toRow)
    .filter((p) => (!filter.unallocatedOnly || p.unallocated > 0.005) && includesText([p.number, p.partyName, p.targetRefNumber, p.note], filter.search))
    .sort((a, b) => b.date.localeCompare(a.date));
});

/** Server-mode variant of `getPayments` for `DataTable`: paged, sorted and totalled server-side. */
export const getPaymentsPaged = wrap('payments.getPaymentsPaged', async function getPaymentsPaged(query: PagedQuery<PaymentFilter>): Promise<PagedResult<PaymentRow>> {
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
    .map(toRow)
    .filter((p) => (!filter.unallocatedOnly || p.unallocated > 0.005) && includesText([p.number, p.partyName, p.targetRefNumber, p.note], filter.search));

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
});

export const getPayment = wrap('payments.getPayment', async function getPayment(id: string): Promise<PaymentRow> {
  await delay(100);
  const p = db.payments.find((x) => x.id === id);
  if (!p) throw new Error('السند غير موجود');
  return toRow(p);
});

export const createPayment = wrap('payments.createPayment', async function createPayment(input: PaymentInput): Promise<Payment> {
  await delay();
  return clone(recordPayment(input, session.userId));
});

/** Allocate more of an already-saved payment's unallocated money — "allocate later" (docs/v2/08 §3, 09 §3). */
export const allocateExistingPayment = wrap('payments.allocateExistingPayment', async function allocateExistingPayment(paymentId: string, allocations: PaymentAllocationInput[]): Promise<Payment> {
  await delay();
  return clone(allocatePayment(paymentId, allocations, session.userId));
});

export const removeAllocation = wrap('payments.removeAllocation', async function removeAllocation(paymentId: string, allocationId: string): Promise<Payment> {
  await delay();
  return clone(unallocatePayment(paymentId, allocationId, session.userId));
});

/** Invoices (customer) or confirmed POs (supplier) that still have something outstanding. */
export const getOpenDocuments = wrap('payments.getOpenDocuments', async function getOpenDocuments(targetType: 'customer' | 'supplier', targetId: string): Promise<OpenDocument[]> {
  await delay(150);
  return getOpenDocumentsFor(targetType, targetId);
});
