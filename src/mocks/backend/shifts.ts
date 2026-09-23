/**
 * v2 phase 7 (docs/v2/06-sales-and-pos.md §5 "Shifts (الورديات) and the cash drawer"). Posting
 * rule (docs/v2/02-accounting-review.md §3 "Shift close"): variance over → Dr cash / Cr cashOver
 * (4330); variance short → Dr cashShort (6320) / Cr cash — invariant 11: "Every closed shift:
 * expected cash − counted cash = the posted over/short amount."
 *
 * Cash movements (tenders/refunds) aren't posted again here — they're already part of the sale's/
 * refund's own journal entry (sales.ts). This module only tracks them on the shift's movement log
 * (for the X/Z report) and posts the *variance* + the optional cash-drop transfer at close.
 */
import type { CloseShiftInput, OpenShiftInput, Shift, ShiftMovementKind } from '@/modules/invoices/types';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, round2, sum, uid } from '../utils';
import { accountFor } from './accounts';
import { logActivity, postJournal } from './core';
import { recordTransferVoucher } from './vouchers';

export function currentOpenShift(terminalId: string): Shift | undefined {
  return db.shifts.find((s) => s.terminalId === terminalId && s.status === 'OPEN');
}

export function openShift(input: OpenShiftInput, userId: string, date = new Date().toISOString()): Shift {
  if (currentOpenShift(input.terminalId)) throw new ApiError('توجد وردية مفتوحة بالفعل على هذا الجهاز', 'CONFLICT');
  if (input.openingFloat < 0) throw new ApiError('رصيد الافتتاح لا يمكن أن يكون سالباً');
  const shift: Shift = {
    id: uid('shift'),
    number: nextNumber('shift'),
    terminalId: input.terminalId,
    branchId: input.branchId,
    status: 'OPEN',
    openedBy: userId,
    openedAt: date,
    openingFloat: round2(input.openingFloat),
    openingDenominations: input.openingDenominations,
    movements: [],
  };
  mutate(() => db.shifts.push(shift));
  logActivity('shift', `فتح وردية ${shift.number} — رصيد افتتاحي ${shift.openingFloat.toFixed(2)}`, userId, date, '/pos/shifts');
  return shift;
}

/** Records a cash movement on the currently-open shift for `terminalId` (no-op if none is open — desk sales/refunds don't require a shift). */
export function recordShiftMovement(
  terminalId: string,
  kind: ShiftMovementKind,
  amount: number,
  userId: string,
  opts: { note?: string; refId?: string; refNumber?: string; at?: string } = {},
): void {
  const shift = currentOpenShift(terminalId);
  if (!shift || amount === 0) return;
  mutate(() =>
    shift.movements.push({
      id: uid('shmv'),
      kind,
      amount: round2(amount),
      note: opts.note,
      refId: opts.refId,
      refNumber: opts.refNumber,
      at: opts.at ?? new Date().toISOString(),
      by: userId,
    }),
  );
}

/** Sales-by-method + expected cash — shared by the X-report (mid-shift) and the close screen. */
export function shiftSummary(shift: Shift) {
  const cashSales = sum(shift.movements.filter((m) => m.kind === 'SALE_CASH'), (m) => m.amount);
  const cashRefunds = sum(shift.movements.filter((m) => m.kind === 'REFUND_CASH'), (m) => m.amount);
  const payIns = sum(shift.movements.filter((m) => m.kind === 'PAY_IN'), (m) => m.amount);
  const payOuts = sum(shift.movements.filter((m) => m.kind === 'PAY_OUT'), (m) => m.amount);
  const bankDrops = sum(shift.movements.filter((m) => m.kind === 'BANK_DROP'), (m) => m.amount);
  const expectedCash = round2(shift.openingFloat + cashSales - cashRefunds + payIns - payOuts - bankDrops);

  // Sales by tender method, for the invoices posted while this shift was open (by shiftId).
  const shiftInvoices = db.invoices.filter((i) => i.shiftId === shift.id);
  const byMethod = new Map<string, number>();
  for (const inv of shiftInvoices) {
    for (const t of inv.tenders ?? []) {
      const method = db.paymentMethods.find((m) => m.id === t.paymentMethodId);
      const label = method?.name ?? t.paymentMethodId;
      byMethod.set(label, round2((byMethod.get(label) ?? 0) + t.amount));
    }
  }
  const salesByMethod = [...byMethod.entries()].map(([label, amount]) => ({ label, amount }));
  const salesTotal = round2(sum(shiftInvoices, (i) => i.grandTotal));

  return { cashSales, cashRefunds, payIns, payOuts, bankDrops, expectedCash, salesByMethod, salesTotal, invoiceCount: shiftInvoices.length };
}

export function closeShift(shiftId: string, input: CloseShiftInput, userId: string, date = new Date().toISOString()): Shift {
  const shift = db.shifts.find((s) => s.id === shiftId);
  if (!shift) throw new ApiError('الوردية غير موجودة', 'NOT_FOUND');
  if (shift.status !== 'OPEN') throw new ApiError('الوردية مغلقة بالفعل');

  const { expectedCash } = shiftSummary(shift);
  const counted = round2(input.countedCash);
  const variance = round2(counted - expectedCash);

  // Invariant 11 (docs/v2/02-accounting-review.md §4): expected − counted = the posted over/short
  // amount. over (counted > expected): Dr cash / Cr cashOver. short (counted < expected): Dr
  // cashShort / Cr cash.
  if (Math.abs(variance) > 0.005) {
    postJournal({
      date,
      description: `تسوية عجز/زيادة الصندوق — وردية ${shift.number}`,
      type: 'SYSTEM',
      sourceRef: { kind: 'shift', id: shift.id, number: shift.number },
      lines:
        variance > 0
          ? [{ role: 'cash', debit: variance }, { role: 'cashOver', credit: variance }]
          : [{ role: 'cashShort', debit: -variance }, { role: 'cash', credit: -variance }],
      createdBy: userId,
    });
  }

  // Cash drop to the safe/bank (docs/v2/06 §5 "drop cash to the safe/bank, which creates a transfer
  // voucher") — routed through Phase 8's general transfer-voucher helper (src/mocks/backend/
  // vouchers.ts's `recordTransferVoucher`, Dr destination / Cr source), which posts and lists it
  // as a normal تحويل بين الحسابات voucher rather than a bare journal entry.
  if (input.handoverMode === 'DROP' && counted > 0) {
    recordTransferVoucher(
      {
        date,
        amount: counted,
        description: `إيداع نقدية الوردية ${shift.number} إلى الخزينة/البنك`,
        sourceAccountId: accountFor('cash').id,
        destinationAccountId: accountFor('bank').id,
      },
      userId,
    );
  }

  mutate(() => {
    shift.status = 'CLOSED';
    shift.closedBy = userId;
    shift.closedAt = date;
    shift.countedCash = counted;
    shift.closingDenominations = input.closingDenominations;
    shift.expectedCash = expectedCash;
    shift.variance = variance;
    shift.handoverMode = input.handoverMode ?? 'HANDOVER';
    shift.note = input.note;
  });
  logActivity(
    'shift',
    `إغلاق وردية ${shift.number} — المتوقع ${expectedCash.toFixed(2)}، المعدود ${counted.toFixed(2)}، الفرق ${variance.toFixed(2)}`,
    userId,
    date,
    '/pos/shifts',
  );
  emit('ledger:changed');
  return shift;
}

/** Manager force-close (docs/v2/06 §5 "A manager can force-close a shift"): same posting, counted = expected (0 variance) unless a count is supplied. */
export function forceCloseShift(shiftId: string, userId: string, countedCash?: number): Shift {
  const shift = db.shifts.find((s) => s.id === shiftId);
  if (!shift) throw new ApiError('الوردية غير موجودة', 'NOT_FOUND');
  const { expectedCash } = shiftSummary(shift);
  const closed = closeShift(shiftId, { countedCash: countedCash ?? expectedCash, handoverMode: 'HANDOVER', note: 'إغلاق إجباري من المدير' }, userId);
  mutate(() => (closed.forceClosedBy = userId));
  return closed;
}

/** Accounts referenced only so callers building a Z-report card can show a role's account name without importing `accounts.ts` directly. */
export function cashAccountName(): string {
  return accountFor('cash').name;
}
