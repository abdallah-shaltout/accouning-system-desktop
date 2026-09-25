import { ApiError, clone, db, delay, inDateRange, includesText, localDateKey, round2, session, uid } from '@/mocks';
import { closeFiscalYear, closeYearPreChecks, deleteDraftJournal, logActivity, postDraftJournal, reopenFiscalYear, type CloseYearPreCheck } from '@/mocks/backend/core';
import {
  advanceRecurrence,
  deleteJournalTemplate,
  editDraftJournal,
  payVatSettlement,
  postVatSettlement,
  recordManualJournal,
  reverseJournal,
  saveJournalTemplate,
  vatTotalsForPeriod,
  type JournalTemplateInput,
  type VatPeriodTotals,
} from '@/mocks/backend/journal';
import { mutate } from '@/mocks/persist';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import type { PagedQuery, PagedResult } from '@/modules/core/types/paging';
import type { Account, AccountInput, FiscalYear, JournalEntry, JournalEntryInput, JournalFilter, JournalTemplate } from '../types';

import { wrap } from '@/modules/diagnostics/services/defineService';

export type { CloseYearPreCheck, VatPeriodTotals };

export type AccountWithBalance = Account & { balance: number; debitTotal: number; creditTotal: number; hasPostings: boolean };

/** Balance in the account's normal direction (positive = normal). */
export const signedBalance = wrap('accounting.signedBalance', function signedBalance(account: Pick<Account, 'normalSide'>, debit: number, credit: number): number {
  return round2(account.normalSide === 'DEBIT' ? debit - credit : credit - debit);
});

/** Admin-only override for posting into a closed period (accounting.postToClosedPeriod) — this phase
 * keeps permissions coarse (modules/users/helpers/permissions.ts has no fine-grained strings yet). */
function canPostToClosedPeriod(): boolean {
  return useAuthStore().user?.role === 'admin';
}

/** `range` filters which journal entries count toward the balance column (CoA page period filter). Omitted = all time. */
export const getAccounts = wrap('accounting.getAccounts', async function getAccounts(range: { from?: string; to?: string } = {}): Promise<AccountWithBalance[]> {
  await delay();
  const totals = new Map<string, { d: number; c: number }>();
  for (const e of db.journalEntries) {
    if (!inDateRange(e.date, range.from, range.to)) continue;
    for (const l of e.lines) {
      const t = totals.get(l.accountId) ?? { d: 0, c: 0 };
      t.d += l.debit;
      t.c += l.credit;
      totals.set(l.accountId, t);
    }
  }
  return db.accounts
    .map((a) => {
      const t = totals.get(a.id) ?? { d: 0, c: 0 };
      return { ...clone(a), debitTotal: round2(t.d), creditTotal: round2(t.c), balance: signedBalance(a, t.d, t.c), hasPostings: totals.has(a.id) };
    })
    .sort((a, b) => a.code.localeCompare(b.code));
});

/**
 * Header path shown in grey next to an account in pickers, e.g. "المصروفات › العمومية"
 * (docs/v2/03-chart-of-accounts.md §6). Excludes the account itself and the root kind header.
 */
export const accountPath = wrap('accounting.accountPath', function accountPath(account: Pick<Account, 'parentId'>, all: Account[]): string {
  const segments: string[] = [];
  let parent = account.parentId ? all.find((a) => a.id === account.parentId) : undefined;
  while (parent) {
    if (parent.parentId) segments.unshift(parent.name);
    parent = parent.parentId ? all.find((a) => a.id === parent!.parentId) : undefined;
  }
  return segments.join(' › ');
});

/** Rolled-up balance of an account including its sub-accounts, in the account's normal direction. */
export const rolledBalance = wrap('accounting.rolledBalance', function rolledBalance(account: AccountWithBalance, all: AccountWithBalance[]): number {
  return all
    .filter((c) => c.parentId === account.id)
    .reduce((acc, c) => acc + (c.normalSide === account.normalSide ? 1 : -1) * rolledBalance(c, all), account.balance);
});

function validateAccount(input: AccountInput, exceptId?: string) {
  if (!/^\d{1,8}$/.test(input.code)) throw new ApiError('رمز الحساب أرقام فقط (حتى 8 أرقام)');
  if (!input.name.trim()) throw new ApiError('اسم الحساب مطلوب');
  if (db.accounts.some((a) => a.id !== exceptId && a.code === input.code)) throw new ApiError('رمز الحساب مستخدم من قبل', 'CONFLICT');
  if (input.parentId) {
    if (input.parentId === exceptId) throw new ApiError('لا يمكن أن يكون الحساب أباً لنفسه');
    const parent = db.accounts.find((a) => a.id === input.parentId);
    if (!parent) throw new ApiError('الحساب الأب غير موجود');
    if (!parent.isGroup) throw new ApiError('الحساب الأب يجب أن يكون حساباً رئيسياً (تجميعياً)');
    if (parent.kind !== input.kind) throw new ApiError(`الحساب الأب من نوع مختلف (${parent.kind})`);
    if (!input.code.startsWith(parent.code)) throw new ApiError(`رمز الحساب يجب أن يبدأ برمز الحساب الأب (${parent.code})`);
  }
  if (input.isGroup && input.allowManual) throw new ApiError('الحسابات الرئيسية (التجميعية) لا تقبل الترحيل المباشر');
}

export const saveAccount = wrap('accounting.saveAccount', async function saveAccount(input: AccountInput, id?: string): Promise<Account> {
  await delay();
  validateAccount(input, id);
  let account: Account;
  if (id) {
    const found = db.accounts.find((a) => a.id === id);
    if (!found) throw new ApiError('الحساب غير موجود', 'NOT_FOUND');
    // System accounts keep their code, kind and header/leaf shape — the posting rules depend on them.
    if (!found.canDelete && (input.code !== found.code || input.kind !== found.kind || input.isGroup !== found.isGroup)) {
      throw new ApiError('لا يمكن تغيير رمز أو نوع أو شكل (رئيسي/فرعي) حساب أساسي في النظام');
    }
    if (found.isGroup !== input.isGroup) {
      if (db.accounts.some((a) => a.parentId === found.id)) throw new ApiError('للحساب حسابات فرعية — لا يمكن جعله فرعياً (postable)');
      if (db.journalEntries.some((e) => e.lines.some((l) => l.accountId === found.id))) throw new ApiError('للحساب قيود مسجلة — لا يمكن جعله رئيسياً (تجميعياً)');
    }
    mutate(() => Object.assign(found, { ...input, name: input.name.trim(), parentId: input.parentId || null }));
    account = found;
  } else {
    account = { id: uid('acc'), ...input, name: input.name.trim(), parentId: input.parentId || null, canDelete: true };
    mutate(() => db.accounts.push(account));
  }
  logActivity('journal', `${id ? 'تعديل' : 'إضافة'} الحساب ${account.code} — ${account.name}`, session.userId, new Date().toISOString(), '/accounting/accounts');
  return clone(account);
});

export const deleteAccount = wrap('accounting.deleteAccount', async function deleteAccount(id: string): Promise<void> {
  await delay();
  const account = db.accounts.find((a) => a.id === id);
  if (!account) throw new ApiError('الحساب غير موجود', 'NOT_FOUND');
  if (!account.canDelete) throw new ApiError('حساب أساسي في النظام ولا يمكن حذفه');
  if (db.accounts.some((a) => a.parentId === id)) throw new ApiError('للحساب حسابات فرعية — احذفها أولاً', 'CONFLICT');
  if (db.journalEntries.some((e) => e.lines.some((l) => l.accountId === id))) throw new ApiError('للحساب قيود مسجلة — يمكنك إيقافه بدلاً من حذفه', 'CONFLICT');
  mutate(() => (db.accounts = db.accounts.filter((a) => a.id !== id)));
});

/**
 * Move an account under a new parent (drag-to-reparent on the CoA tree). Same `kind`-change
 * validation as `saveAccount` (docs/v2/03-chart-of-accounts.md §6: "an account can't move under a
 * different kind").
 */
export const reparentAccount = wrap('accounting.reparentAccount', async function reparentAccount(id: string, newParentId: string | null): Promise<Account> {
  await delay();
  const account = db.accounts.find((a) => a.id === id);
  if (!account) throw new ApiError('الحساب غير موجود', 'NOT_FOUND');
  if (newParentId === id) throw new ApiError('لا يمكن أن يكون الحساب أباً لنفسه');
  let cursor = newParentId ? db.accounts.find((a) => a.id === newParentId) : undefined;
  if (newParentId && !cursor) throw new ApiError('الحساب الأب غير موجود');
  if (cursor && !cursor.isGroup) throw new ApiError('الحساب الأب يجب أن يكون حساباً رئيسياً (تجميعياً)');
  if (cursor && cursor.kind !== account.kind) throw new ApiError(`لا يمكن نقل الحساب إلى مجموعة من نوع مختلف (${cursor.kind})`);
  // No cycles: the new parent can't be a descendant of the account being moved.
  while (cursor) {
    if (cursor.id === id) throw new ApiError('لا يمكن نقل الحساب إلى أحد فروعه');
    cursor = cursor.parentId ? db.accounts.find((a) => a.id === cursor!.parentId) : undefined;
  }
  mutate(() => (account.parentId = newParentId));
  logActivity('journal', `نقل الحساب ${account.code} — ${account.name}`, session.userId, new Date().toISOString(), '/accounting/accounts');
  return clone(account);
});

// --- Journal ---------------------------------------------------------------------------------

/** `sourceLink` = app route of the document behind a SYSTEM entry. */
export type JournalRow = JournalEntry & { createdByName: string; sourceLink?: string; sourceLabel?: string; attachmentCount: number };

const SOURCE_LABEL: Record<string, string> = {
  invoice: 'فاتورة مبيعات',
  refund: 'مرتجع مبيعات',
  purchaseOrder: 'أمر شراء',
  purchaseReturn: 'مرتجع مشتريات',
  payment: 'سند',
  stockAdjustment: 'تسوية مخزون',
};

function matchesJournalFilter(e: JournalEntry, filter: JournalFilter): boolean {
  return (
    (!filter.type || e.type === filter.type) &&
    (!filter.status || e.status === filter.status) &&
    (!filter.accountId || e.lines.some((l) => l.accountId === filter.accountId || accountIsDescendantOf(l.accountId, filter.accountId!))) &&
    (!filter.partyId || e.lines.some((l) => l.partyId === filter.partyId)) &&
    (!filter.userId || e.createdBy === filter.userId) &&
    (!filter.sourceKind || e.sourceRef?.kind === filter.sourceKind) &&
    (filter.reversed === undefined || !!e.reversed === filter.reversed) &&
    (filter.hasAttachments === undefined || (e.attachmentIds ?? []).length > 0 === filter.hasAttachments) &&
    (filter.minAmount === undefined || e.totalDebit >= filter.minAmount) &&
    (filter.maxAmount === undefined || e.totalDebit <= filter.maxAmount) &&
    inDateRange(e.date, filter.from, filter.to) &&
    includesText([e.number, e.description, e.sourceRef?.number], filter.search)
  );
}

/** `accountId` filter includes an account's children (A1: "account (includes children)"). */
function accountIsDescendantOf(accountId: string, ancestorId: string): boolean {
  let cursor = db.accounts.find((a) => a.id === accountId);
  while (cursor?.parentId) {
    if (cursor.parentId === ancestorId) return true;
    cursor = db.accounts.find((a) => a.id === cursor!.parentId);
  }
  return false;
}

function toRow(e: JournalEntry): JournalRow {
  return {
    ...clone(e),
    createdByName: db.users.find((u) => u.id === e.createdBy)?.name ?? '—',
    sourceLink: sourceLink(e),
    sourceLabel: e.sourceRef ? SOURCE_LABEL[e.sourceRef.kind] : undefined,
    attachmentCount: e.attachmentIds?.length ?? 0,
  };
}

/** Posted entries + drafts (drafts live in a separate table — see `db.journalDrafts`'s doc comment). */
function allEntriesAndDrafts(): JournalEntry[] {
  return [...db.journalEntries, ...db.journalDrafts];
}

export const getJournalEntries = wrap('accounting.getJournalEntries', async function getJournalEntries(filter: JournalFilter = {}): Promise<JournalRow[]> {
  await delay();
  return allEntriesAndDrafts()
    .filter((e) => matchesJournalFilter(e, filter))
    .sort((a, b) => b.date.localeCompare(a.date) || b.number.localeCompare(a.number))
    .map(toRow);
});

/** Server-mode variant of `getJournalEntries` for `DataTable`: paged, sorted and totalled server-side. */
export const getJournalEntriesPaged = wrap('accounting.getJournalEntriesPaged', async function getJournalEntriesPaged(query: PagedQuery<JournalFilter>): Promise<PagedResult<JournalRow>> {
  await delay();
  const filter = query.filters ?? {};
  let rows: JournalRow[] = allEntriesAndDrafts()
    .filter((e) => matchesJournalFilter(e, filter))
    .map(toRow);

  const total = rows.length;
  const totals = { totalDebit: rows.reduce((a, r) => a + r.totalDebit, 0), totalCredit: rows.reduce((a, r) => a + r.totalCredit, 0) };

  const sort = query.sort;
  rows = [...rows].sort((a, b) => {
    if (sort) {
      const va = (a as any)[sort.key];
      const vb = (b as any)[sort.key];
      const dir = sort.dir === 'asc' ? 1 : -1;
      if (typeof va === 'number' && typeof vb === 'number') return (va - vb) * dir;
      return String(va ?? '').localeCompare(String(vb ?? ''), 'ar') * dir;
    }
    return b.date.localeCompare(a.date) || b.number.localeCompare(a.number);
  });

  const start = (query.page - 1) * query.pageSize;
  return { rows: rows.slice(start, start + query.pageSize), total, totals };
});

export const getJournalEntry = wrap('accounting.getJournalEntry', async function getJournalEntry(id: string): Promise<JournalRow & { reversedById?: string; reversedByNumber?: string; related: JournalRow[] }> {
  await delay();
  const entry = allEntriesAndDrafts().find((e) => e.id === id);
  if (!entry) throw new ApiError('القيد غير موجود', 'NOT_FOUND');
  const reversal = db.journalEntries.find((e) => e.reversalOfId === id);
  // A3 "القيود المرتبطة": the reversal pair, and any other entry sharing the same source document.
  const related = entry.sourceRef
    ? db.journalEntries.filter((e) => e.id !== id && e.sourceRef?.kind === entry.sourceRef!.kind && e.sourceRef?.id === entry.sourceRef!.id)
    : [];
  return {
    ...toRow(entry),
    reversedById: reversal?.id,
    reversedByNumber: reversal?.number,
    related: related.map(toRow),
  };
});

export const createJournalEntry = wrap('accounting.createJournalEntry', async function createJournalEntry(input: JournalEntryInput): Promise<JournalEntry> {
  await delay();
  return clone(recordManualJournal(input, session.userId, canPostToClosedPeriod()));
});

/** Edit a saved draft in place. */
export const updateJournalDraft = wrap('accounting.updateJournalDraft', async function updateJournalDraft(id: string, input: JournalEntryInput): Promise<JournalEntry> {
  await delay();
  return clone(editDraftJournal(id, input));
});

/** Posts a previously saved draft. */
export const postJournalDraft = wrap('accounting.postJournalDraft', async function postJournalDraft(id: string): Promise<JournalEntry> {
  await delay();
  return clone(postDraftJournal(id, session.userId, canPostToClosedPeriod()));
});

export const deleteJournalDraft = wrap('accounting.deleteJournalDraft', async function deleteJournalDraft(id: string): Promise<void> {
  await delay();
  deleteDraftJournal(id);
});

export const reverseJournalEntry = wrap('accounting.reverseJournalEntry', async function reverseJournalEntry(id: string, date: string, reason: string): Promise<JournalEntry> {
  await delay();
  return clone(reverseJournal(id, session.userId, date, reason, canPostToClosedPeriod()));
});

/** Where the SYSTEM entry came from, as an app route. */
function sourceLink(entry: Pick<JournalEntry, 'sourceRef'>): string | undefined {
  const ref = entry.sourceRef;
  if (!ref) return undefined;
  switch (ref.kind) {
    case 'invoice':
      return `/invoices/${ref.id}`;
    case 'refund': {
      const r = db.refunds.find((x) => x.id === ref.id);
      return r ? `/invoices/${r.invoiceId}` : undefined;
    }
    case 'purchaseOrder':
      return `/purchases/${ref.id}`;
    case 'purchaseReturn': {
      const r = db.purchaseReturns.find((x) => x.id === ref.id);
      return r ? `/purchases/${r.purchaseOrderId}` : undefined;
    }
    case 'payment':
      return `/payments?highlight=${ref.id}`;
    case 'stockAdjustment':
      return `/inventory/adjustments/${ref.id}`;
  }
}

// --- Fiscal years ----------------------------------------------------------------------------

export const getFiscalYears = wrap('accounting.getFiscalYears', async function getFiscalYears(): Promise<FiscalYear[]> {
  await delay(120);
  return clone([...db.fiscalYears].sort((a, b) => b.startDate.localeCompare(a.startDate)));
});

/** The open fiscal year containing today (fallback: the latest one). Reports default to it. */
export const getCurrentFiscalYear = wrap('accounting.getCurrentFiscalYear', async function getCurrentFiscalYear(): Promise<FiscalYear | undefined> {
  await delay(60);
  const today = localDateKey(new Date());
  const current = db.fiscalYears.find((f) => f.startDate <= today && f.endDate >= today) ?? [...db.fiscalYears].sort((a, b) => b.startDate.localeCompare(a.startDate))[0];
  return current ? clone(current) : undefined;
});

export const saveFiscalYear = wrap('accounting.saveFiscalYear', async function saveFiscalYear(input: Omit<FiscalYear, 'id'>, id?: string): Promise<FiscalYear> {
  await delay();
  if (!input.name.trim()) throw new ApiError('اسم السنة المالية مطلوب');
  if (!input.startDate || !input.endDate || input.endDate <= input.startDate) throw new ApiError('تاريخ النهاية يجب أن يكون بعد تاريخ البداية');
  const overlap = db.fiscalYears.find((f) => f.id !== id && input.startDate <= f.endDate && input.endDate >= f.startDate);
  if (overlap) throw new ApiError(`الفترة تتداخل مع السنة المالية ${overlap.name}`, 'CONFLICT');
  let fy: FiscalYear;
  if (id) {
    const found = db.fiscalYears.find((f) => f.id === id);
    if (!found) throw new ApiError('السنة المالية غير موجودة', 'NOT_FOUND');
    mutate(() => Object.assign(found, input));
    fy = found;
  } else {
    fy = { id: uid('fy'), ...input };
    mutate(() => db.fiscalYears.push(fy));
  }
  logActivity('settings', `${id ? 'تعديل' : 'إضافة'} السنة المالية ${fy.name}`, session.userId, new Date().toISOString(), '/accounting/fiscal-years');
  return clone(fy);
});

// --- Posting settings (lock date) -------------------------------------------------------------

export const getLockDate = wrap('accounting.getLockDate', async function getLockDate(): Promise<string | undefined> {
  await delay(60);
  return db.settings.accounting?.lockDate;
});

export const saveLockDate = wrap('accounting.saveLockDate', async function saveLockDate(lockDate: string | undefined): Promise<void> {
  await delay();
  mutate(() => {
    db.settings.accounting = { ...db.settings.accounting, lockDate: lockDate || undefined };
  });
  logActivity('settings', lockDate ? `تحديد تاريخ القفل ${lockDate}` : 'إزالة تاريخ القفل', session.userId, new Date().toISOString(), '/accounting/fiscal-years');
});

// --- Fiscal-year closing wizard ----------------------------------------------------------------

export const getCloseYearPreChecks = wrap('accounting.getCloseYearPreChecks', async function getCloseYearPreChecks(fiscalYearId: string): Promise<CloseYearPreCheck[]> {
  await delay();
  return closeYearPreChecks(fiscalYearId);
});

export const closeYear = wrap('accounting.closeYear', async function closeYear(fiscalYearId: string): Promise<{ fiscalYear: FiscalYear; closingEntry: JournalEntry; nextYear?: FiscalYear }> {
  await delay(300);
  const result = closeFiscalYear(fiscalYearId, session.userId);
  return clone(result);
});

/** Admin-only (checked here, same coarse `role === 'admin'` pattern as `canPostToClosedPeriod`). */
export const reopenYear = wrap('accounting.reopenYear', async function reopenYear(fiscalYearId: string): Promise<FiscalYear> {
  await delay();
  if (useAuthStore().user?.role !== 'admin') throw new ApiError('إعادة فتح السنة المالية للمدير فقط', 'FORBIDDEN');
  return clone(reopenFiscalYear(fiscalYearId, session.userId));
});

// --- Journal templates & recurring entries (A2/A3) ----------------------------------------------

export const getJournalTemplates = wrap('accounting.getJournalTemplates', async function getJournalTemplates(): Promise<JournalTemplate[]> {
  await delay(120);
  return clone([...db.journalTemplates].sort((a, b) => a.name.localeCompare(b.name, 'ar')));
});

export const getJournalTemplate = wrap('accounting.getJournalTemplate', async function getJournalTemplate(id: string): Promise<JournalTemplate> {
  await delay();
  const template = db.journalTemplates.find((t) => t.id === id);
  if (!template) throw new ApiError('القالب غير موجود', 'NOT_FOUND');
  return clone(template);
});

export const createOrUpdateJournalTemplate = wrap('accounting.createOrUpdateJournalTemplate', async function createOrUpdateJournalTemplate(input: JournalTemplateInput, id?: string): Promise<JournalTemplate> {
  await delay();
  return clone(saveJournalTemplate(input, session.userId, id));
});

export const removeJournalTemplate = wrap('accounting.removeJournalTemplate', async function removeJournalTemplate(id: string): Promise<void> {
  await delay();
  deleteJournalTemplate(id);
});

/** Used by the entry form's "load a template" action: pre-fills the grid from a saved template. */
export const loadTemplateIntoEntry = wrap('accounting.loadTemplateIntoEntry', async function loadTemplateIntoEntry(id: string): Promise<JournalTemplate> {
  await delay(80);
  return getJournalTemplate(id);
});

/**
 * Posts a recurring template's entry for its current `nextDate` and advances the schedule.
 * There's no scheduler here — this is invoked from the "posted due entries" list action, and,
 * since Phase 10, also from the home/insight-engine card's "ترحيل الآن" action ("recurring-journal-due"
 * rule in `modules/core/services/insightRules.ts`, docs/v2/11-journal-dashboard-insights.md D2).
 */
export const postRecurringTemplate = wrap('accounting.postRecurringTemplate', async function postRecurringTemplate(id: string): Promise<JournalEntry> {
  await delay();
  const template = db.journalTemplates.find((t) => t.id === id);
  if (!template) throw new ApiError('القالب غير موجود', 'NOT_FOUND');
  if (!template.recurrence) throw new ApiError('القالب ليس متكرراً');
  const entry = recordManualJournal(
    {
      date: template.recurrence.nextDate,
      description: template.description || template.name,
      lines: template.lines,
      templateId: template.id,
    },
    session.userId,
    canPostToClosedPeriod(),
  );
  advanceRecurrence(id);
  return clone(entry);
});

// --- VAT settlement --------------------------------------------------------------------------

export const getVatPeriodTotals = wrap('accounting.getVatPeriodTotals', async function getVatPeriodTotals(from: string, to: string): Promise<VatPeriodTotals> {
  await delay();
  return vatTotalsForPeriod(from, to);
});

export const submitVatSettlement = wrap('accounting.submitVatSettlement', async function submitVatSettlement(from: string, to: string): Promise<JournalEntry> {
  await delay(300);
  return clone(postVatSettlement(from, to, session.userId, canPostToClosedPeriod()));
});

/** Payment-to-the-authority posting, routed through a real payment method (see `payVatSettlement`'s doc comment). */
export const payVatSettlementNow = wrap('accounting.payVatSettlementNow', async function payVatSettlementNow(amount: number, paymentMethodId: string): Promise<JournalEntry> {
  await delay(300);
  return clone(payVatSettlement(amount, paymentMethodId, session.userId));
});
