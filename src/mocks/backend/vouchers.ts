import type {
  OwnerVoucherInput,
  PaymentVoucherInput,
  ReceiptVoucherInput,
  TransferVoucherInput,
  Voucher,
} from '@/modules/vouchers/types';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, round2, uid } from '../utils';
import { accountById, accountFor } from './accounts';
import { logActivity, postJournal, type PostingLine } from './core';

/**
 * General vouchers (docs/v2/09-purchases-payments-expenses.md §5) — the reference's "fast journal"
 * for money movements that aren't invoices. Each posts a normal system journal entry with a source
 * link and prints as a PDF (generic voucher print route).
 */

function methodAccount(paymentMethodId: string) {
  const method = db.paymentMethods.find((m) => m.id === paymentMethodId && m.active);
  if (!method) throw new ApiError('اختر طريقة الدفع', 'VALIDATION');
  return { role: method.accountRole, name: method.name };
}

function KIND_LABEL(kind: Voucher['kind']): string {
  return { RECEIPT: 'سند قبض عام', PAYMENT: 'سند صرف عام', TRANSFER: 'تحويل بين الحسابات', OWNER: 'سند مالك' }[kind];
}

/** سند قبض عام — Dr method account / Cr chosen account. */
export function recordReceiptVoucher(input: ReceiptVoucherInput, userId: string): Voucher {
  if (!(input.amount > 0)) throw new ApiError('المبلغ يجب أن يكون أكبر من صفر');
  const method = methodAccount(input.paymentMethodId);
  const creditAccount = accountById(input.creditAccountId);
  if (!creditAccount.allowManual) throw new ApiError(`لا يمكن الترحيل يدوياً إلى حساب "${creditAccount.name}"`);

  const voucher: Voucher = { id: uid('vch'), number: nextNumber('voucher'), kind: 'RECEIPT', ...input, createdBy: userId };
  mutate(() => db.vouchers.push(voucher));

  const lines: PostingLine[] = [
    { role: method.role, debit: input.amount },
    { accountId: creditAccount.id, credit: input.amount, costCenterId: input.costCenterId },
  ];
  postJournal({
    date: input.date,
    description: `${KIND_LABEL('RECEIPT')} ${voucher.number} — ${input.description}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'voucher', id: voucher.id, number: voucher.number },
    lines,
    createdBy: userId,
    attachmentIds: input.attachmentIds,
  });
  logActivity('voucher', `${KIND_LABEL('RECEIPT')} ${voucher.number} بقيمة ${input.amount.toFixed(2)}`, userId, input.date, `/vouchers/${voucher.id}`);
  emit('ledger:changed');
  return voucher;
}

/** سند صرف عام — Dr chosen account / Cr method account. */
export function recordPaymentVoucher(input: PaymentVoucherInput, userId: string): Voucher {
  if (!(input.amount > 0)) throw new ApiError('المبلغ يجب أن يكون أكبر من صفر');
  const method = methodAccount(input.paymentMethodId);
  const debitAccount = accountById(input.debitAccountId);
  if (!debitAccount.allowManual) throw new ApiError(`لا يمكن الترحيل يدوياً من حساب "${debitAccount.name}"`);

  const voucher: Voucher = { id: uid('vch'), number: nextNumber('voucher'), kind: 'PAYMENT', ...input, createdBy: userId };
  mutate(() => db.vouchers.push(voucher));

  const lines: PostingLine[] = [
    { accountId: debitAccount.id, debit: input.amount, costCenterId: input.costCenterId },
    { role: method.role, credit: input.amount },
  ];
  postJournal({
    date: input.date,
    description: `${KIND_LABEL('PAYMENT')} ${voucher.number} — ${input.description}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'voucher', id: voucher.id, number: voucher.number },
    lines,
    createdBy: userId,
    attachmentIds: input.attachmentIds,
  });
  logActivity('voucher', `${KIND_LABEL('PAYMENT')} ${voucher.number} بقيمة ${input.amount.toFixed(2)}`, userId, input.date, `/vouchers/${voucher.id}`);
  emit('ledger:changed');
  return voucher;
}

/** تحويل بين الحسابات — Dr destination / Cr source, with an optional fee line. This is what a shift's cash-drop (Phase 7) needs — usable/importable once Phase 7 wires it up. */
export function recordTransferVoucher(input: TransferVoucherInput, userId: string): Voucher {
  if (!(input.amount > 0)) throw new ApiError('المبلغ يجب أن يكون أكبر من صفر');
  if (input.sourceAccountId === input.destinationAccountId) throw new ApiError('اختر حسابين مختلفين للتحويل');
  const source = accountById(input.sourceAccountId);
  const destination = accountById(input.destinationAccountId);
  const fee = round2(input.feeAmount ?? 0);
  if (fee > 0 && !input.feeAccountId) throw new ApiError('اختر حساب العمولة');

  const voucher: Voucher = { id: uid('vch'), number: nextNumber('voucher'), kind: 'TRANSFER', ...input, feeAmount: fee || undefined, createdBy: userId };
  mutate(() => db.vouchers.push(voucher));

  const lines: PostingLine[] = [
    { accountId: destination.id, debit: input.amount },
    { accountId: source.id, credit: round2(input.amount + fee) },
  ];
  if (fee > 0) lines.push({ accountId: input.feeAccountId!, debit: fee });

  postJournal({
    date: input.date,
    description: `${KIND_LABEL('TRANSFER')} ${voucher.number} — ${input.description}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'voucher', id: voucher.id, number: voucher.number },
    lines,
    createdBy: userId,
    attachmentIds: input.attachmentIds,
  });
  logActivity('voucher', `${KIND_LABEL('TRANSFER')} ${voucher.number} بقيمة ${input.amount.toFixed(2)}`, userId, input.date, `/vouchers/${voucher.id}`);
  emit('ledger:changed');
  return voucher;
}

/** مسحوبات/إضافة رأس مال — Dr drawings/Cr cash (drawings), or the reverse (contribution). */
export function recordOwnerVoucher(input: OwnerVoucherInput, userId: string): Voucher {
  if (!(input.amount > 0)) throw new ApiError('المبلغ يجب أن يكون أكبر من صفر');
  const cashAccount = accountById(input.cashAccountId);
  const drawingsAccount = accountFor('drawings');

  const voucher: Voucher = { id: uid('vch'), number: nextNumber('voucher'), kind: 'OWNER', ...input, createdBy: userId };
  mutate(() => db.vouchers.push(voucher));

  const lines: PostingLine[] =
    input.direction === 'drawings'
      ? [{ accountId: drawingsAccount.id, debit: input.amount }, { accountId: cashAccount.id, credit: input.amount }]
      : [{ accountId: cashAccount.id, debit: input.amount }, { accountId: drawingsAccount.id, credit: input.amount }];

  postJournal({
    date: input.date,
    description: `${KIND_LABEL('OWNER')} ${voucher.number} — ${input.direction === 'drawings' ? 'مسحوبات شخصية' : 'إضافة رأس مال'}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'voucher', id: voucher.id, number: voucher.number },
    lines,
    createdBy: userId,
    attachmentIds: input.attachmentIds,
  });
  logActivity('voucher', `${KIND_LABEL('OWNER')} ${voucher.number} بقيمة ${input.amount.toFixed(2)}`, userId, input.date, `/vouchers/${voucher.id}`);
  emit('ledger:changed');
  return voucher;
}

export function getVoucherById(id: string): Voucher {
  const voucher = db.vouchers.find((v) => v.id === id);
  if (!voucher) throw new ApiError('السند غير موجود', 'NOT_FOUND');
  return voucher;
}

export { KIND_LABEL as voucherKindLabel };
