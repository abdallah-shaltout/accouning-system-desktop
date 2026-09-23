import type { OpenDocument, Payment, PaymentAllocation, PaymentAllocationInput, PaymentInput } from '@/modules/payments/types';
import { invoiceOutstanding, paymentStatusFor } from '@/modules/invoices/helpers/totals';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, round2, sum, uid } from '../utils';
import { settlementAccountFor } from './accounts';
import { logActivity, postJournal, type PostingLine } from './core';
import { purchaseOutstanding } from './purchases';

/**
 * Payments with allocation (docs/v2/02-accounting-review.md C1, docs/v2/09-purchases-payments-
 * expenses.md §3): a payment posts ONCE — Dr method account / Cr receivable-or-payable — for the
 * full `amount`. `allocations[]` link it to one or more open documents as a SUB-LEDGER record only;
 * they never touch the GL. Unallocated money (amount − Σallocations) simply sits as a credit on the
 * party (read off the ledger balance itself — no separate "advances" bucket needs to be maintained,
 * since the GL already carries the full amount against receivable/payable).
 *
 * `invoice.paidAmount` / `po.paidAmount` keep meaning exactly what they meant before this phase —
 * "Σ money applied to this document" — except now that sum can come from several payments' worth of
 * allocations instead of one payment settling one document 1:1. `invoiceOutstanding`/
 * `purchaseOutstanding` (docs/v2/invoices & purchases, not owned by this phase) already compute
 * `total − paidAmount − returned/refunded`, i.e. exactly "total − Σallocations − Σcredit notes" per
 * the C1 fix, as long as this file keeps `paidAmount` in sync — which it does on every
 * allocate/reallocate/un-allocate.
 */

function openDocumentsFor(targetType: 'customer' | 'supplier', targetId: string): OpenDocument[] {
  if (targetType === 'customer') {
    return db.invoices
      .filter((i) => i.customerId === targetId && i.status === 'COMPLETED' && invoiceOutstanding(i) > 0)
      .sort((a, b) => a.date.localeCompare(b.date))
      .map((i) => ({
        id: i.id,
        kind: 'invoice' as const,
        number: i.number,
        date: i.date,
        dueDate: i.dueDate,
        total: round2(i.grandTotal - i.refundedAmount),
        outstanding: invoiceOutstanding(i),
      }));
  }
  return db.purchaseOrders
    .filter((p) => p.supplierId === targetId && p.status === 'CONFIRMED' && purchaseOutstanding(p) > 0)
    .sort((a, b) => a.date.localeCompare(b.date))
    .map((p) => ({
      id: p.id,
      kind: 'purchaseOrder' as const,
      number: p.number,
      date: p.date,
      total: round2(p.grandTotal - p.returnedAmount),
      outstanding: purchaseOutstanding(p),
    }));
}

export function getOpenDocumentsFor(targetType: 'customer' | 'supplier', targetId: string): OpenDocument[] {
  return openDocumentsFor(targetType, targetId);
}

/** Σ of a payment's allocations. */
export function allocatedTotal(payment: Payment): number {
  return sum(payment.allocations, (a) => a.amount);
}

/** Unallocated money still sitting on this payment (docs/v2 C1 "unallocated"). */
export function unallocatedAmount(payment: Payment): number {
  return Math.max(0, round2(payment.amount - allocatedTotal(payment)));
}

/** Σ unallocated credit across every RECEIVED/PAID payment for a party — shown on the party page. */
export function unallocatedCreditFor(targetType: 'customer' | 'supplier', targetId: string): number {
  const type = targetType === 'customer' ? 'RECEIVED' : 'PAID';
  return sum(
    db.payments.filter((p) => p.type === type && p.targetType === targetType && p.targetId === targetId),
    unallocatedAmount,
  );
}

function applyAllocationToDocument(alloc: PaymentAllocation, sign: 1 | -1, date: string): void {
  if (alloc.targetKind === 'invoice') {
    const invoice = db.invoices.find((i) => i.id === alloc.targetId);
    if (!invoice) throw new ApiError('الفاتورة غير موجودة', 'NOT_FOUND');
    invoice.paidAmount = round2(invoice.paidAmount + sign * alloc.amount);
    invoice.paymentStatus = paymentStatusFor(invoice.grandTotal - invoice.refundedAmount, invoice.paidAmount);
  } else if (alloc.targetKind === 'purchaseOrder') {
    const po = db.purchaseOrders.find((p) => p.id === alloc.targetId);
    if (!po) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
    po.paidAmount = round2(po.paidAmount + sign * alloc.amount);
    po.paymentStatus = paymentStatusFor(po.grandTotal - po.returnedAmount, po.paidAmount);
  }
  void date;
}

function validateAllocations(payment: Payment, inputs: PaymentAllocationInput[]): PaymentAllocation[] {
  const openDocs = openDocumentsFor(payment.targetType, payment.targetId);
  const already = allocatedTotal(payment);
  let remaining = round2(payment.amount - already);
  const rows: PaymentAllocation[] = [];
  for (const input of inputs) {
    if (!(input.amount > 0)) continue;
    const doc = openDocs.find((d) => d.id === input.targetId && d.kind === input.targetKind);
    if (!doc) throw new ApiError('المستند غير موجود ضمن المستندات المفتوحة لهذا الطرف');
    const amount = round2(input.amount);
    if (amount > doc.outstanding + 0.005) throw new ApiError(`المبلغ المخصص لـ ${doc.number} أكبر من المتبقي عليه (${doc.outstanding.toFixed(2)})`);
    if (amount > remaining + 0.005) throw new ApiError('إجمالي التخصيص أكبر من مبلغ السند');
    remaining = round2(remaining - amount);
    rows.push({ id: uid('alloc'), targetKind: input.targetKind, targetId: input.targetId, targetNumber: doc.number, amount, date: payment.date });
  }
  return rows;
}

/** Records a payment (one GL posting) with an optional initial set of allocations (sub-ledger only). */
export function recordPayment(input: PaymentInput, userId: string): Payment {
  const amount = round2(input.amount);
  if (!(amount > 0)) throw new ApiError('المبلغ يجب أن يكون أكبر من صفر');

  let partyName: string;
  let posting: PostingLine[];

  if (input.type === 'RECEIVED') {
    const customer = db.customers.find((c) => c.id === input.targetId);
    if (!customer) throw new ApiError('اختر العميل');
    partyName = customer.name;
    posting = [
      { accountId: settlementAccountFor(input.method).id, debit: amount },
      { role: 'receivable' as const, credit: amount, partyKind: 'customer' as const, partyId: customer.id },
    ];
  } else {
    const supplier = db.suppliers.find((s) => s.id === input.targetId);
    if (!supplier) throw new ApiError('اختر المورد');
    partyName = supplier.name;
    posting = [
      { role: 'payable' as const, debit: amount, partyKind: 'supplier' as const, partyId: supplier.id },
      { accountId: settlementAccountFor(input.method).id, credit: amount },
    ];
  }

  const payment: Payment = {
    id: uid('pay'),
    number: nextNumber('payment'),
    date: input.date,
    type: input.type,
    targetType: input.type === 'RECEIVED' ? 'customer' : 'supplier',
    targetId: input.targetId,
    amount,
    method: input.method,
    note: input.note,
    allocations: [],
  };

  const allocations = input.allocations?.length ? validateAllocations(payment, input.allocations) : [];
  payment.allocations = allocations;
  if (allocations.length === 1) {
    payment.targetRef = allocations[0].targetId;
    payment.targetRefNumber = allocations[0].targetNumber;
  }

  mutate(() => {
    db.payments.push(payment);
    for (const alloc of allocations) applyAllocationToDocument(alloc, 1, payment.date);
  });

  postJournal({
    date: input.date,
    description:
      input.type === 'RECEIVED'
        ? `سند قبض ${payment.number} من ${partyName}${payment.targetRefNumber ? ` — ${payment.targetRefNumber}` : ''}`
        : `سند صرف ${payment.number} إلى ${partyName}${payment.targetRefNumber ? ` — ${payment.targetRefNumber}` : ''}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'payment', id: payment.id, number: payment.number },
    lines: posting,
    createdBy: userId,
  });

  logActivity(
    'payment',
    `${input.type === 'RECEIVED' ? 'تحصيل' : 'سداد'} ${amount.toFixed(2)} ${input.type === 'RECEIVED' ? 'من' : 'إلى'} ${partyName}`,
    userId,
    input.date,
    `/payments?highlight=${payment.id}`,
  );
  emit('parties:changed');
  return payment;
}

/**
 * Allocate more of an already-recorded payment's unallocated money to open documents — "allocate
 * later" (docs/v2/08 §3 "المدفوعات", docs/v2/09 §3 "unallocated"). No new GL entry: only the
 * sub-ledger `allocations[]` and the target documents' `paidAmount` change.
 */
export function allocatePayment(paymentId: string, inputs: PaymentAllocationInput[], userId: string): Payment {
  const payment = db.payments.find((p) => p.id === paymentId);
  if (!payment) throw new ApiError('السند غير موجود', 'NOT_FOUND');
  const newRows = validateAllocations(payment, inputs);
  if (!newRows.length) throw new ApiError('لم يتم إدخال أي تخصيص');
  mutate(() => {
    payment.allocations.push(...newRows);
    for (const alloc of newRows) applyAllocationToDocument(alloc, 1, payment.date);
    if (payment.allocations.length === 1) {
      payment.targetRef = payment.allocations[0].targetId;
      payment.targetRefNumber = payment.allocations[0].targetNumber;
    }
  });
  const partyName = (payment.targetType === 'customer' ? db.customers : db.suppliers).find((x) => x.id === payment.targetId)?.name ?? '—';
  logActivity(
    'payment',
    `تخصيص ${sum(newRows, (r) => r.amount).toFixed(2)} من سند ${payment.number} (${partyName}) على ${newRows.map((r) => r.targetNumber).join('، ')}`,
    userId,
    new Date().toISOString(),
    `/payments?highlight=${payment.id}`,
  );
  emit('parties:changed');
  return payment;
}

/** Remove one allocation row, freeing its money back onto the payment as unallocated credit. No GL entry. */
export function unallocatePayment(paymentId: string, allocationId: string, userId: string): Payment {
  const payment = db.payments.find((p) => p.id === paymentId);
  if (!payment) throw new ApiError('السند غير موجود', 'NOT_FOUND');
  const alloc = payment.allocations.find((a) => a.id === allocationId);
  if (!alloc) throw new ApiError('التخصيص غير موجود', 'NOT_FOUND');
  mutate(() => {
    applyAllocationToDocument(alloc, -1, payment.date);
    payment.allocations = payment.allocations.filter((a) => a.id !== allocationId);
    if (payment.targetRef === alloc.targetId) {
      payment.targetRef = payment.allocations[0]?.targetId;
      payment.targetRefNumber = payment.allocations[0]?.targetNumber;
    }
  });
  logActivity('payment', `إلغاء تخصيص ${alloc.amount.toFixed(2)} من سند ${payment.number} عن ${alloc.targetNumber}`, userId, new Date().toISOString(), `/payments?highlight=${payment.id}`);
  emit('parties:changed');
  return payment;
}
