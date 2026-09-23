import type { JournalEntry, JournalEntryInput } from '@/modules/accounting/types';
import { db } from '../db';
import { mutate } from '../persist';
import { ApiError } from '../utils';
import { logActivity, postJournal } from './core';

/**
 * Manual-entry control-account rules (docs/v2/02-accounting-review.md B1):
 *  - AR/AP (`requiresParty`): a manual line needs a party. Allowed with one, e.g. write-offs or
 *    reclassification.
 *  - Inventory, VAT output and VAT input (`allowManual: false`): blocked entirely. Use a stock
 *    adjustment or a VAT settlement instead.
 * Opening balances go through the onboarding wizard (Phase 5), not the manual journal.
 */
export function recordManualJournal(input: JournalEntryInput, userId: string, isAdmin = false): JournalEntry {
  if (!input.description.trim()) throw new ApiError('أدخل بيان القيد');
  for (const line of input.lines) {
    const account = db.accounts.find((a) => a.id === line.accountId);
    if (!account) throw new ApiError('اختر الحساب لكل سطر');
    if (!account.active) throw new ApiError(`الحساب "${account.name}" غير نشط`);
    if (account.isGroup) throw new ApiError(`"${account.name}" حساب رئيسي (تجميعي) ولا يقبل الترحيل المباشر`);
    if (line.debit < 0 || line.credit < 0) throw new ApiError('المبالغ لا يمكن أن تكون سالبة');
    if (line.debit > 0 && line.credit > 0) throw new ApiError('السطر الواحد إما مدين أو دائن');
    if ((line.debit > 0 || line.credit > 0) && !account.allowManual) {
      throw new ApiError(`لا يمكن الترحيل يدوياً على حساب "${account.name}" — استخدم تسوية المخزون أو تسوية ضريبة القيمة المضافة`, 'FORBIDDEN');
    }
    if (account.requiresParty && (line.debit > 0 || line.credit > 0) && !line.partyId) {
      throw new ApiError(`السطر على حساب "${account.name}" يتطلب اختيار عميل أو مورد`, 'VALIDATION');
    }
  }
  const entry = postJournal({
    date: input.date,
    description: input.description.trim(),
    type: 'MANUAL',
    lines: input.lines,
    createdBy: userId,
    allowClosedPeriod: isAdmin,
  });
  logActivity('journal', `قيد يدوي ${entry.number} — ${entry.description}`, userId, entry.date, `/accounting/journal/${entry.id}`);
  return entry;
}

/** Reversal = a new mirrored entry; the original stays for the audit trail. */
export function reverseJournal(id: string, userId: string, isAdmin = false): JournalEntry {
  const original = db.journalEntries.find((e) => e.id === id);
  if (!original) throw new ApiError('القيد غير موجود', 'NOT_FOUND');
  if (original.type !== 'MANUAL') throw new ApiError('القيود الآلية تُعكس من المستند المصدر (مرتجع/إلغاء)');
  if (original.reversed || original.reversalOfId) throw new ApiError('هذا القيد معكوس بالفعل');

  const reversal = postJournal({
    date: new Date().toISOString(),
    description: `عكس القيد ${original.number} — ${original.description}`,
    type: 'MANUAL',
    lines: original.lines.map((l) => ({
      accountId: l.accountId,
      description: l.description,
      debit: l.credit,
      credit: l.debit,
      partyKind: l.partyKind,
      partyId: l.partyId,
    })),
    createdBy: userId,
    allowClosedPeriod: isAdmin,
  });
  mutate(() => {
    reversal.reversalOfId = original.id;
    original.reversed = true;
  });
  logActivity('journal', `عكس القيد ${original.number}`, userId, reversal.date, `/accounting/journal/${reversal.id}`);
  return reversal;
}
