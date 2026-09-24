import type { JournalEntry, JournalEntryInput, JournalTemplate } from '@/modules/accounting/types';
import { db } from '../db';
import { mutate } from '../persist';
import { ApiError, localDateKey, round2, uid } from '../utils';
import { accountFor } from './accounts';
import { draftJournal, logActivity, postJournal, updateDraftJournal, type PostingLine } from './core';
import { recordPaymentVoucher } from './vouchers';

/**
 * Manual-entry control-account rules (docs/v2/02-accounting-review.md B1):
 *  - AR/AP (`requiresParty`): a manual line needs a party. Allowed with one, e.g. write-offs or
 *    reclassification.
 *  - Inventory, VAT output and VAT input (`allowManual: false`): blocked entirely. Use a stock
 *    adjustment or a VAT settlement instead.
 * Opening balances go through the onboarding wizard (Phase 5), not the manual journal.
 */
function validateManualLines(input: JournalEntryInput): void {
  if (!input.description.trim()) throw new ApiError('أدخل بيان القيد');
  if (input.lines.length < 2) throw new ApiError('يجب أن يحتوي القيد على سطرين على الأقل');
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
    // v2 phase 9 (docs/v2/10 §3 "Expense accounts can be marked requiresCostCenter"): only enforced
    // once cost centers are actually on — a company that never turned the feature on never has to
    // pick one, matching the "invisible until needed" rule for every dimension in this phase.
    if (account.requiresCostCenter && db.settings.features?.costCenters && (line.debit > 0 || line.credit > 0) && !line.costCenterId) {
      throw new ApiError(`السطر على حساب "${account.name}" يتطلب اختيار مركز تكلفة`, 'VALIDATION');
    }
  }
}

/**
 * v2 phase 9 (docs/v2/10 §3 "توزيع" split action): splits one manual-entry line by percentages
 * across several cost centers, e.g. rent 60% branch A / 40% branch B. Pure helper — the caller
 * (journal entry form) replaces the one line with these in its own `lines[]` before submitting;
 * this never touches `db` itself. Σ split amounts = the original amount exactly (largest-remainder
 * rounding, the same halala-safe technique `totals.ts` uses).
 */
export function splitLineByCostCenters(
  line: { accountId: string; description?: string; debit: number; credit: number; partyKind?: 'customer' | 'supplier'; partyId?: string; branchId?: string },
  splits: { costCenterId: string; pct: number }[],
): JournalEntryInput['lines'] {
  const amount = line.debit > 0 ? line.debit : line.credit;
  const isDebit = line.debit > 0;
  const pctSum = splits.reduce((a, s) => a + s.pct, 0) || 1;
  const raw = splits.map((s) => (amount * s.pct) / pctSum);
  const rounded = raw.map((r) => round2(r));
  let diff = round2(amount - rounded.reduce((a, r) => a + r, 0));
  if (diff !== 0 && rounded.length) {
    const order = raw.map((r, i) => ({ i, rem: r - rounded[i] })).sort((a, b) => (diff > 0 ? b.rem - a.rem : a.rem - b.rem));
    rounded[order[0].i] = round2(rounded[order[0].i] + diff);
    diff = 0;
  }
  return splits.map((s, i) => ({
    accountId: line.accountId,
    description: line.description,
    debit: isDebit ? rounded[i] : 0,
    credit: isDebit ? 0 : rounded[i],
    partyKind: line.partyKind,
    partyId: line.partyId,
    branchId: line.branchId,
    costCenterId: s.costCenterId,
  }));
}

function toPostingLines(input: JournalEntryInput): PostingLine[] {
  return input.lines.map((l) => ({
    accountId: l.accountId,
    description: l.description,
    debit: l.debit,
    credit: l.credit,
    partyKind: l.partyKind,
    partyId: l.partyId,
    branchId: l.branchId,
    costCenterId: l.costCenterId,
  }));
}

export function recordManualJournal(input: JournalEntryInput, userId: string, isAdmin = false): JournalEntry {
  validateManualLines(input);
  if (input.asDraft) {
    return draftJournal({
      date: input.date,
      description: input.description.trim(),
      lines: toPostingLines(input),
      createdBy: userId,
      attachmentIds: input.attachmentIds,
      templateId: input.templateId,
    });
  }
  const entry = postJournal({
    date: input.date,
    description: input.description.trim(),
    type: 'MANUAL',
    lines: toPostingLines(input),
    createdBy: userId,
    allowClosedPeriod: isAdmin,
    attachmentIds: input.attachmentIds,
    templateId: input.templateId,
  });
  logActivity('journal', `قيد يدوي ${entry.number} — ${entry.description}`, userId, entry.date, `/accounting/journal/${entry.id}`);
  return entry;
}

/** Updates a saved draft (edit before posting) — same validation as a fresh manual entry. */
export function editDraftJournal(id: string, input: JournalEntryInput): JournalEntry {
  validateManualLines(input);
  return updateDraftJournal(id, {
    date: input.date,
    description: input.description.trim(),
    lines: toPostingLines(input),
    attachmentIds: input.attachmentIds,
  });
}

/**
 * Reversal = a new mirrored entry; the original stays for the audit trail (B3 fix: caller-chosen
 * date + a required reason, stored on both entries).
 */
export function reverseJournal(id: string, userId: string, date: string, reason: string, isAdmin = false): JournalEntry {
  const original = db.journalEntries.find((e) => e.id === id);
  if (!original) throw new ApiError('القيد غير موجود', 'NOT_FOUND');
  if (original.type !== 'MANUAL') throw new ApiError('القيود الآلية تُعكس من المستند المصدر (مرتجع/إلغاء)');
  if (original.reversed || original.reversalOfId) throw new ApiError('هذا القيد معكوس بالفعل');
  if (!reason.trim()) throw new ApiError('سبب العكس مطلوب');

  const reversal = postJournal({
    date,
    description: `عكس القيد ${original.number} — ${original.description}`,
    type: 'MANUAL',
    lines: original.lines.map((l) => ({
      accountId: l.accountId,
      description: l.description,
      debit: l.credit,
      credit: l.debit,
      partyKind: l.partyKind,
      partyId: l.partyId,
      branchId: l.branchId,
      costCenterId: l.costCenterId,
    })),
    createdBy: userId,
    allowClosedPeriod: isAdmin,
  });
  mutate(() => {
    reversal.reversalOfId = original.id;
    reversal.reversalReason = reason.trim();
    original.reversed = true;
    original.reversalReason = reason.trim();
  });
  logActivity('journal', `عكس القيد ${original.number}`, userId, reversal.date, `/accounting/journal/${reversal.id}`);
  return reversal;
}

// ---------------------------------------------------------------------------------------------
// Templates + recurring entries (A2/A3)
// ---------------------------------------------------------------------------------------------

export interface JournalTemplateInput {
  name: string;
  description: string;
  lines: JournalTemplate['lines'];
  recurrence?: JournalTemplate['recurrence'];
}

function validateTemplateLines(lines: JournalTemplate['lines']): void {
  if (!lines.length || lines.length < 2) throw new ApiError('يجب أن يحتوي القالب على سطرين على الأقل');
  for (const line of lines) {
    const account = db.accounts.find((a) => a.id === line.accountId);
    if (!account) throw new ApiError('اختر الحساب لكل سطر');
    if (account.requiresParty && (line.debit > 0 || line.credit > 0) && !line.partyId) {
      throw new ApiError(`السطر على حساب "${account.name}" يتطلب اختيار عميل أو مورد`, 'VALIDATION');
    }
  }
}

export function saveJournalTemplate(input: JournalTemplateInput, userId: string, id?: string): JournalTemplate {
  if (!input.name.trim()) throw new ApiError('اسم القالب مطلوب');
  validateTemplateLines(input.lines);
  let template: JournalTemplate;
  if (id) {
    const found = db.journalTemplates.find((t) => t.id === id);
    if (!found) throw new ApiError('القالب غير موجود', 'NOT_FOUND');
    mutate(() => Object.assign(found, { name: input.name.trim(), description: input.description, lines: input.lines, recurrence: input.recurrence }));
    template = found;
  } else {
    template = { id: uid('jtpl'), name: input.name.trim(), description: input.description, lines: input.lines, recurrence: input.recurrence, createdAt: new Date().toISOString(), createdBy: userId };
    mutate(() => db.journalTemplates.push(template));
  }
  return template;
}

export function deleteJournalTemplate(id: string): void {
  const template = db.journalTemplates.find((t) => t.id === id);
  if (!template) throw new ApiError('القالب غير موجود', 'NOT_FOUND');
  mutate(() => (db.journalTemplates = db.journalTemplates.filter((t) => t.id !== id)));
}

/** Advances a recurring template's `nextDate` by one period after it's been posted (A3). */
export function advanceRecurrence(templateId: string): void {
  const template = db.journalTemplates.find((t) => t.id === templateId);
  if (!template?.recurrence) return;
  const next = new Date(template.recurrence.nextDate);
  if (template.recurrence.every === 'month') next.setMonth(next.getMonth() + 1);
  else if (template.recurrence.every === 'quarter') next.setMonth(next.getMonth() + 3);
  else next.setFullYear(next.getFullYear() + 1);
  mutate(() => {
    template.recurrence!.nextDate = localDateKey(next);
  });
}

// ---------------------------------------------------------------------------------------------
// VAT settlement (A4 / docs/v2/02-accounting-review.md §3 "VAT settlement" / "VAT payment")
// ---------------------------------------------------------------------------------------------

export interface VatPeriodTotals {
  outputVat: number;
  inputVat: number;
  net: number; // + payable to the authority, − refundable
}

/** Output/input VAT posted for the period, straight from the GL (matches the VAT accounts exactly). */
export function vatTotalsForPeriod(from: string, to: string): VatPeriodTotals {
  const outputAccount = accountFor('vatOutput');
  const inputAccount = accountFor('vatInput');
  let outputVat = 0;
  let inputVat = 0;
  for (const e of db.journalEntries) {
    const key = localDateKey(e.date);
    if (key < from || key > to) continue;
    for (const l of e.lines) {
      if (l.accountId === outputAccount.id) outputVat += l.credit - l.debit;
      if (l.accountId === inputAccount.id) inputVat += l.debit - l.credit;
    }
  }
  outputVat = round2(outputVat);
  inputVat = round2(inputVat);
  return { outputVat, inputVat, net: round2(outputVat - inputVat) };
}

/**
 * Posts the settlement entry: Dr output VAT (closes it), Cr input VAT (closes it), the
 * difference to VAT payable (2155) — a debit there when refundable.
 */
export function postVatSettlement(from: string, to: string, userId: string, isAdmin = false): JournalEntry {
  const totals = vatTotalsForPeriod(from, to);
  if (totals.outputVat === 0 && totals.inputVat === 0) throw new ApiError('لا توجد حركة ضريبية في هذه الفترة');
  const outputAccount = accountFor('vatOutput');
  const inputAccount = accountFor('vatInput');
  const payableAccount = accountFor('vatPayable');

  const lines: PostingLine[] = [];
  if (totals.outputVat > 0) lines.push({ accountId: outputAccount.id, debit: totals.outputVat, description: 'إقفال ضريبة المخرجات' });
  if (totals.inputVat > 0) lines.push({ accountId: inputAccount.id, credit: totals.inputVat, description: 'إقفال ضريبة المدخلات' });
  if (totals.net > 0) lines.push({ accountId: payableAccount.id, credit: totals.net, description: 'صافي الضريبة المستحقة' });
  else if (totals.net < 0) lines.push({ accountId: payableAccount.id, debit: -totals.net, description: 'صافي الضريبة القابلة للاسترداد' });

  const entry = postJournal({
    date: to,
    description: `تسوية ضريبة القيمة المضافة — من ${from} إلى ${to}`,
    type: 'VAT_SETTLEMENT',
    lines,
    createdBy: userId,
    allowClosedPeriod: isAdmin,
  });
  logActivity('journal', `تسوية ضريبة القيمة المضافة ${entry.number}`, userId, entry.date, `/accounting/journal/${entry.id}`);
  return entry;
}

/**
 * "سداد" (pay) — payment-to-the-authority posting: Dr VAT payable, Cr the chosen payment method's
 * settlement account. Routes through Phase 8's shared voucher helper (`recordPaymentVoucher`) like
 * every other cash-out flow in the app, so it picks up real payment methods (fees, clearing
 * accounts, per-branch overrides) instead of hard-coding `cash`/`bank` — this closes the phase-13b
 * follow-up left when payment methods didn't have a shared posting path yet (Phase 3). Returns the
 * voucher's underlying journal entry, same return shape callers already expected.
 */
export function payVatSettlement(amount: number, paymentMethodId: string, userId: string): JournalEntry {
  if (amount <= 0) throw new ApiError('لا يوجد مبلغ مستحق للسداد');
  const payableAccount = accountFor('vatPayable');
  const voucher = recordPaymentVoucher(
    {
      date: new Date().toISOString(),
      description: 'سداد ضريبة القيمة المضافة لمصلحة الزكاة والضريبة والجمارك',
      amount,
      paymentMethodId,
      debitAccountId: payableAccount.id,
    },
    userId,
  );
  const entry = db.journalEntries.find((e) => e.sourceRef?.kind === 'voucher' && e.sourceRef.id === voucher.id);
  if (!entry) throw new ApiError('تعذر إنشاء قيد السداد', 'CONFLICT');
  return entry;
}
