import type { Payment, PaymentInput } from '@/modules/payments/types';
import { invoiceOutstanding, paymentStatusFor } from '@/modules/invoices/helpers/totals';
import { db, nextNumber } from '../db';
import { ApiError, round2, uid } from '../utils';
import { logActivity, postJournal, settlementAccount } from './core';
import { purchaseOutstanding } from './purchases';

/** One payment settles exactly one invoice (received) or one purchase order (paid). */
export function recordPayment(input: PaymentInput, userId: string): Payment {
  const amount = round2(input.amount);
  if (!(amount > 0)) throw new ApiError('المبلغ يجب أن يكون أكبر من صفر');

  let refNumber: string;
  let partyName: string;
  let posting;

  if (input.type === 'RECEIVED') {
    const customer = db.customers.find((c) => c.id === input.targetId);
    if (!customer) throw new ApiError('اختر العميل');
    const invoice = db.invoices.find((i) => i.id === input.targetRef && i.customerId === customer.id);
    if (!invoice) throw new ApiError('اختر الفاتورة المراد سدادها');
    const outstanding = invoiceOutstanding(invoice);
    if (amount > outstanding + 0.001) throw new ApiError(`المبلغ أكبر من المتبقي على الفاتورة (${outstanding.toFixed(2)})`);
    invoice.paidAmount = round2(invoice.paidAmount + amount);
    invoice.paymentStatus = paymentStatusFor(invoice.grandTotal - invoice.refundedAmount, invoice.paidAmount);
    refNumber = invoice.number;
    partyName = customer.name;
    posting = [
      { code: settlementAccount(input.method), debit: amount },
      { code: '1130', credit: amount },
    ];
  } else {
    const supplier = db.suppliers.find((s) => s.id === input.targetId);
    if (!supplier) throw new ApiError('اختر المورد');
    const po = db.purchaseOrders.find((p) => p.id === input.targetRef && p.supplierId === supplier.id);
    if (!po || po.status !== 'CONFIRMED') throw new ApiError('اختر أمر الشراء المراد سداده');
    const outstanding = purchaseOutstanding(po);
    if (amount > outstanding + 0.001) throw new ApiError(`المبلغ أكبر من المتبقي على أمر الشراء (${outstanding.toFixed(2)})`);
    po.paidAmount = round2(po.paidAmount + amount);
    po.paymentStatus = paymentStatusFor(po.grandTotal - po.returnedAmount, po.paidAmount);
    refNumber = po.number;
    partyName = supplier.name;
    posting = [
      { code: '2100', debit: amount },
      { code: settlementAccount(input.method), credit: amount },
    ];
  }

  const payment: Payment = {
    id: uid('pay'),
    number: nextNumber('payment'),
    date: input.date,
    type: input.type,
    targetType: input.type === 'RECEIVED' ? 'customer' : 'supplier',
    targetId: input.targetId,
    targetRef: input.targetRef,
    targetRefNumber: refNumber,
    amount,
    method: input.method,
    note: input.note,
  };
  db.payments.push(payment);

  postJournal({
    date: input.date,
    description: input.type === 'RECEIVED' ? `سند قبض ${payment.number} من ${partyName} — ${refNumber}` : `سند صرف ${payment.number} إلى ${partyName} — ${refNumber}`,
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
  return payment;
}
