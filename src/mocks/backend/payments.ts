import type { OpenDocument, Payment, PaymentAllocation, PaymentAllocationInput, PaymentInput } from '@/modules/payments/types';
import { invoiceOutstanding, paymentStatusFor } from '@/modules/invoices/helpers/totals';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, round2, sum, uid } from '../utils';
import { settlementAccountFor } from './accounts';
import { DEFAULT_BRANCH_ID, logActivity, postJournal, type PostingLine } from './core';
import { purchaseOutstanding } from './purchases';
import { isBaseCurrency } from './currency';

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
      .map((i) => {
        // v2 phase 9 (docs/v2/10 §2): `grandTotal`/`paidAmount`/`invoiceOutstanding()` are all in
        // the INVOICE's own currency (FC for an FC invoice — "lines and totals are in the document
        // currency"). `outstanding`/`total` here are always BASE currency (what the payment form's
        // allocation grid and `validateAllocations`'s AR-amount math compare against, matching
        // every base-currency document); `fcOutstanding` carries the raw FC figure alongside it.
        const fcOutstanding = invoiceOutstanding(i);
        const outstanding = i.currency && i.exchangeRate ? round2(fcOutstanding * i.exchangeRate) : fcOutstanding;
        return {
          id: i.id,
          kind: 'invoice' as const,
          number: i.number,
          date: i.date,
          dueDate: i.dueDate,
          total: i.currency && i.exchangeRate ? round2((i.grandTotal - i.refundedAmount) * i.exchangeRate) : round2(i.grandTotal - i.refundedAmount),
          outstanding,
          currency: i.currency,
          fcOutstanding: i.currency ? fcOutstanding : undefined,
          rate: i.exchangeRate,
        };
      });
  }
  return db.purchaseOrders
    .filter((p) => p.supplierId === targetId && p.status === 'RECEIVED' && purchaseOutstanding(p) > 0)
    .sort((a, b) => a.date.localeCompare(b.date))
    .map((p) => {
      const fcOutstanding = purchaseOutstanding(p);
      const outstanding = p.currency && p.exchangeRate ? round2(fcOutstanding * p.exchangeRate) : fcOutstanding;
      return {
        id: p.id,
        kind: 'purchaseOrder' as const,
        number: p.number,
        date: p.date,
        total: p.currency && p.exchangeRate ? round2((p.grandTotal - p.returnedAmount) * p.exchangeRate) : round2(p.grandTotal - p.returnedAmount),
        outstanding,
        currency: p.currency,
        fcOutstanding: p.currency ? fcOutstanding : undefined,
        rate: p.exchangeRate,
      };
    });
}

export function getOpenDocumentsFor(targetType: 'customer' | 'supplier', targetId: string): OpenDocument[] {
  return openDocumentsFor(targetType, targetId);
}

/** Σ of a payment's allocations, in the CONTROL-ACCOUNT (AR/AP) amount — see `unallocatedAmount` for why this isn't what "unallocated cash" subtracts. */
export function allocatedTotal(payment: Payment): number {
  return sum(payment.allocations, (a) => a.amount);
}

/**
 * Unallocated money still sitting on this payment (docs/v2 C1 "unallocated"), in the base
 * currency actually tendered. v2 phase 9: an FC allocation's `amount` (the AR/AP posting, at the
 * document's own rate) is NOT how much cash it consumed when there's a realized FX gain/loss — the
 * cash consumed is `amountFc × payment.rate` (see `recordPayment`'s `cashConsumed`). Subtracting
 * `allocatedTotal` (the AR-side total) here instead would leave the FX gain/loss amount stranded
 * as phantom "unallocated credit" that was actually already booked to the FX account, not sitting
 * on the party.
 */
export function unallocatedAmount(payment: Payment): number {
  const cashConsumed = sum(payment.allocations, (a) => (a.amountFc !== undefined && payment.rate ? round2(a.amountFc * payment.rate) : a.amount));
  return Math.max(0, round2(payment.amount - cashConsumed));
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
  // `invoice.grandTotal`/`po.grandTotal` are in the DOCUMENT's own currency (docs/v2/10 §2:
  // "Lines and totals are in the document currency") — so `paidAmount` must track in that same
  // currency to stay comparable. `alloc.amount` is the BASE-currency amount posted to the AR/AP
  // control account (used for the payment's own `allocatedTotal`/base-currency bookkeeping);
  // `alloc.amountFc`, when set, is what actually settles the document's own (FC) total.
  if (alloc.targetKind === 'invoice') {
    const invoice = db.invoices.find((i) => i.id === alloc.targetId);
    if (!invoice) throw new ApiError('الفاتورة غير موجودة', 'NOT_FOUND');
    const settled = alloc.amountFc !== undefined ? alloc.amountFc : alloc.amount;
    invoice.paidAmount = round2(invoice.paidAmount + sign * settled);
    invoice.paymentStatus = paymentStatusFor(invoice.grandTotal - invoice.refundedAmount, invoice.paidAmount);
  } else if (alloc.targetKind === 'purchaseOrder') {
    const po = db.purchaseOrders.find((p) => p.id === alloc.targetId);
    if (!po) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
    const settled = alloc.amountFc !== undefined ? alloc.amountFc : alloc.amount;
    po.paidAmount = round2(po.paidAmount + sign * settled);
    po.paymentStatus = paymentStatusFor(po.grandTotal - po.returnedAmount, po.paidAmount);
  }
  void date;
}

/**
 * v2 phase 9 realized FX (docs/v2/10 §2 worked example): when an allocation's target document
 * carries a foreign currency, `input.amount` is read as the BASE-currency amount actually being
 * applied (matching every other field on `PaymentAllocationInput` — never asks the caller for an
 * FC amount directly). The allocation posted against the RECEIVABLE/PAYABLE control account must
 * use the document's OWN rate ("partial allocations use the invoice's rate for AR" — this is what
 * keeps the party's FC sub-ledger net to exactly 0 on a full settlement); the gap between that and
 * the base cash actually moved is the realized FX gain/loss for this allocation.
 */
function validateAllocations(payment: Payment, inputs: PaymentAllocationInput[]): PaymentAllocation[] {
  const openDocs = openDocumentsFor(payment.targetType, payment.targetId);
  const already = allocatedTotal(payment);
  let remaining = round2(payment.amount - already);
  const rows: PaymentAllocation[] = [];
  for (const input of inputs) {
    if (!(input.amount > 0)) continue;
    const doc = openDocs.find((d) => d.id === input.targetId && d.kind === input.targetKind);
    if (!doc) throw new ApiError('المستند غير موجود ضمن المستندات المفتوحة لهذا الطرف');
    const cashAmount = round2(input.amount); // base currency, at the PAYMENT's own rate
    if (cashAmount > remaining + 0.005) throw new ApiError('إجمالي التخصيص أكبر من مبلغ السند');

    if (doc.currency && doc.rate && doc.fcOutstanding !== undefined) {
      // FC document: the caller's `cashAmount` is how much base-currency cash is being applied;
      // convert it to FC at the PAYMENT's rate (payment.rate, when the payment itself is FC) to
      // find how much of the FC balance this settles, then re-value that FC portion at the
      // DOCUMENT's own rate for the AR/AP posting — the difference is the realized FX.
      let fcSettled: number;
      if (payment.rate) {
        fcSettled = round2(cashAmount / payment.rate);
      } else {
        // Base-currency payment settling an FC document: only a FULL settlement has an unambiguous
        // FX rate (the document's own) — a partial base-currency amount against an FC balance would
        // need its own rate picker, which is out of this phase's scope (build precisely what the
        // worked example needs, not guess at an unspecified partial-base-vs-FC UX).
        if (round2(cashAmount - doc.outstanding) !== 0) {
          throw new ApiError(`التخصيص الجزئي بالعملة الأساسية على مستند بعملة ${doc.currency} غير مدعوم — خصص المبلغ كاملاً أو استخدم دفعة بنفس العملة`);
        }
        fcSettled = doc.fcOutstanding;
      }
      if (fcSettled > doc.fcOutstanding + 0.005) throw new ApiError(`الكمية المخصصة لـ ${doc.number} أكبر من المتبقي عليه (${doc.fcOutstanding.toFixed(2)} ${doc.currency})`);
      const arAmount = round2(fcSettled * doc.rate); // posts against AR/AP at the DOCUMENT's rate
      if (arAmount > doc.outstanding + 0.01) throw new ApiError(`المبلغ المخصص لـ ${doc.number} أكبر من المتبقي عليه (${doc.outstanding.toFixed(2)})`);
      const fxGainLoss = round2(cashAmount - arAmount); // + = gain (cash side stronger than the invoice's rate)
      remaining = round2(remaining - cashAmount);
      rows.push({
        id: uid('alloc'),
        targetKind: input.targetKind,
        targetId: input.targetId,
        targetNumber: doc.number,
        amount: arAmount,
        date: payment.date,
        amountFc: fcSettled,
        fxGainLoss: fxGainLoss || undefined,
      });
      continue;
    }

    if (cashAmount > doc.outstanding + 0.005) throw new ApiError(`المبلغ المخصص لـ ${doc.number} أكبر من المتبقي عليه (${doc.outstanding.toFixed(2)})`);
    remaining = round2(remaining - cashAmount);
    rows.push({ id: uid('alloc'), targetKind: input.targetKind, targetId: input.targetId, targetNumber: doc.number, amount: cashAmount, date: payment.date });
  }
  return rows;
}

/** Records a payment (one GL posting) with an optional initial set of allocations (sub-ledger only). */
export function recordPayment(input: PaymentInput, userId: string): Payment {
  const amount = round2(input.amount);
  if (!(amount > 0)) throw new ApiError('المبلغ يجب أن يكون أكبر من صفر');

  let partyName: string;
  const branchId = input.branchId ?? DEFAULT_BRANCH_ID;
  const currency = input.currency && !isBaseCurrency(input.currency) ? input.currency : undefined;

  if (input.type === 'RECEIVED') {
    const customer = db.customers.find((c) => c.id === input.targetId);
    if (!customer) throw new ApiError('اختر العميل');
    partyName = customer.name;
  } else {
    const supplier = db.suppliers.find((s) => s.id === input.targetId);
    if (!supplier) throw new ApiError('اختر المورد');
    partyName = supplier.name;
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
    branchId,
    currency,
    amountFc: currency ? input.amountFc : undefined,
    rate: currency ? input.rate : undefined,
  };

  const allocations = input.allocations?.length ? validateAllocations(payment, input.allocations) : [];
  payment.allocations = allocations;
  if (allocations.length === 1) {
    payment.targetRef = allocations[0].targetId;
    payment.targetRefNumber = allocations[0].targetNumber;
  }
  // v2 phase 9 realized FX (docs/v2/10 §2 worked example): Σ what actually posts against the
  // receivable/payable control account is Σ allocation.amount (each at ITS OWN document's rate),
  // not the raw cash `amount` — the gap is the realized FX gain/loss for this payment. Unallocated
  // money (no FC document behind it yet) always posts at the raw cash amount — there's nothing to
  // revalue until it's allocated to something.
  const allocatedControlAmount = round2(sum(allocations, (a) => a.amount));
  // Cash actually consumed by each allocation: FC allocations consumed `amountFc × payment.rate`
  // of the tendered cash; base allocations consumed exactly `amount` (their control-account amount).
  const cashConsumed = round2(sum(allocations, (a) => (a.amountFc !== undefined && payment.rate ? round2(a.amountFc * payment.rate) : a.amount)));
  const unallocated = round2(amount - cashConsumed);
  const controlAmount = round2(allocatedControlAmount + unallocated);
  const fxGainLoss = round2(sum(allocations, (a) => a.fxGainLoss ?? 0));
  if (fxGainLoss) payment.fxGainLoss = fxGainLoss;
  // Party's FC ledger (docs/v2/10 §2 "party ledger in USD nets to exactly 0"): the control-account
  // line needs its own `amountFc` so `customerBalanceFc()`/`supplierBalanceFc()` net out against
  // the invoice's `amountFc` exactly — Σ allocation.amountFc (unallocated FC money has no FC amount
  // of its own to report, since it isn't tied to a specific FC document's rate yet).
  const controlAmountFc = round2(sum(allocations, (a) => a.amountFc ?? 0)) || undefined;

  const dim = { branchId };
  const fxLine: PostingLine | undefined =
    fxGainLoss > 0
      ? { role: 'fxGain', credit: fxGainLoss, description: 'فرق عملة محقق', ...dim }
      : fxGainLoss < 0
        ? { role: 'fxLoss', debit: -fxGainLoss, description: 'فرق عملة محقق', ...dim }
        : undefined;

  // Settlement account: an FC cash/bank account when one is seeded for this currency (docs/v2/10 §2
  // "Cash and bank: accounts can have a currency") — `settlementAccountFor` falls back to the plain
  // base-currency cash/bank account when none matches, so this is a no-op for base-currency payments.
  const settlementAccount = settlementAccountFor(input.method, { branchId, currency });
  const controlFc = controlAmountFc !== undefined && currency ? { currency, amountFc: controlAmountFc, rate: round2(allocatedControlAmount / controlAmountFc) } : {};
  const posting: PostingLine[] =
    input.type === 'RECEIVED'
      ? [
          { accountId: settlementAccount.id, debit: amount, ...dim, ...(currency ? { currency, amountFc: input.amountFc, rate: input.rate } : {}) },
          { role: 'receivable' as const, credit: controlAmount, partyKind: 'customer' as const, partyId: input.targetId, ...dim, ...controlFc },
          ...(fxLine ? [fxLine] : []),
        ]
      : [
          { role: 'payable' as const, debit: controlAmount, partyKind: 'supplier' as const, partyId: input.targetId, ...dim, ...controlFc },
          { accountId: settlementAccount.id, credit: amount, ...dim, ...(currency ? { currency, amountFc: input.amountFc, rate: input.rate } : {}) },
          ...(fxLine ? [fxLine] : []),
        ];

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

  // v2 phase 9 realized FX ("allocate later"): the original payment's GL entry posted the FULL
  // cash amount against receivable/payable as unallocated (no FX assumed yet). Allocating it now
  // to an FC document re-values that slice at the document's own rate — the difference needs its
  // own small FX entry (the original entry is never edited after posting).
  const newFx = round2(sum(newRows, (r) => r.fxGainLoss ?? 0));
  if (newFx) {
    // At record time, this money posted as unallocated — a full base-currency credit (RECEIVED) or
    // debit (PAID) to the control account. Linking it now to an FC document re-values that slice to
    // the document's own rate (`arAmount` in validateAllocations); `newFx` is exactly the gap, so it
    // partially REVERSES the control account by `newFx` (a gain means the control account was
    // over-credited/over-debited by that much) and books the FX account for the same amount.
    const dim = { branchId: payment.branchId };
    const newFc = round2(sum(newRows, (r) => r.amountFc ?? 0)) || undefined;
    const fc = newFc !== undefined && payment.currency ? { currency: payment.currency, amountFc: newFc, rate: payment.rate } : {};
    const controlLine: PostingLine =
      payment.type === 'RECEIVED'
        ? { role: 'receivable' as const, debit: newFx > 0 ? newFx : 0, credit: newFx < 0 ? -newFx : 0, partyKind: 'customer' as const, partyId: payment.targetId, ...dim, ...fc }
        : { role: 'payable' as const, credit: newFx > 0 ? newFx : 0, debit: newFx < 0 ? -newFx : 0, partyKind: 'supplier' as const, partyId: payment.targetId, ...dim, ...fc };
    const fxLine: PostingLine =
      newFx > 0 ? { role: 'fxGain', credit: newFx, description: 'فرق عملة محقق (تخصيص لاحق)', ...dim } : { role: 'fxLoss', debit: -newFx, description: 'فرق عملة محقق (تخصيص لاحق)', ...dim };
    mutate(() => (payment.fxGainLoss = round2((payment.fxGainLoss ?? 0) + newFx)));
    postJournal({
      date: new Date().toISOString(),
      description: `فرق عملة محقق — تخصيص لاحق على سند ${payment.number}`,
      type: 'SYSTEM',
      sourceRef: { kind: 'payment', id: payment.id, number: payment.number },
      lines: [controlLine, fxLine],
      createdBy: userId,
    });
  }

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
