import { ApiError, clone, db, delay, inDateRange, includesText, localDateKey, round2, session, uid } from '@/mocks';
import { logActivity } from '@/mocks/backend/core';
import { recordManualJournal, reverseJournal } from '@/mocks/backend/journal';
import { mutate } from '@/mocks/persist';
import type { PagedQuery, PagedResult } from '@/modules/core/types/paging';
import type { Account, AccountGroup, AccountInput, FiscalYear, JournalEntry, JournalEntryInput, JournalFilter } from '../types';

export type AccountWithBalance = Account & { balance: number; debitTotal: number; creditTotal: number; hasPostings: boolean };

/** Balance in the account's normal direction (positive = normal). */
export function signedBalance(account: Pick<Account, 'normalSide'>, debit: number, credit: number): number {
  return round2(account.normalSide === 'DEBIT' ? debit - credit : credit - debit);
}

export async function getAccountGroups(): Promise<AccountGroup[]> {
  await delay(100);
  return clone(db.accountGroups);
}

export async function getAccounts(): Promise<AccountWithBalance[]> {
  await delay();
  const totals = new Map<string, { d: number; c: number }>();
  for (const e of db.journalEntries) {
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
}

function validateAccount(input: AccountInput, exceptId?: string) {
  if (!/^\d{3,8}$/.test(input.code)) throw new ApiError('رمز الحساب أرقام فقط (3 إلى 8 أرقام)');
  if (!input.name.trim()) throw new ApiError('اسم الحساب مطلوب');
  if (db.accounts.some((a) => a.id !== exceptId && a.code === input.code)) throw new ApiError('رمز الحساب مستخدم من قبل', 'CONFLICT');
  const group = db.accountGroups.find((g) => g.id === input.groupId);
  if (!group) throw new ApiError('اختر المجموعة');
  if (!input.code.startsWith(group.code)) throw new ApiError(`رمز الحساب يجب أن يبدأ برقم المجموعة (${group.code})`);
  if (input.parentId) {
    if (input.parentId === exceptId) throw new ApiError('لا يمكن أن يكون الحساب أباً لنفسه');
    const parent = db.accounts.find((a) => a.id === input.parentId);
    if (!parent || parent.groupId !== input.groupId) throw new ApiError('الحساب الأب يجب أن يكون في نفس المجموعة');
  }
}

export async function saveAccount(input: AccountInput, id?: string): Promise<Account> {
  await delay();
  validateAccount(input, id);
  let account: Account;
  if (id) {
    const found = db.accounts.find((a) => a.id === id);
    if (!found) throw new ApiError('الحساب غير موجود', 'NOT_FOUND');
    // System accounts keep their code and group — the posting rules depend on them.
    if (!found.canDelete && (input.code !== found.code || input.groupId !== found.groupId)) {
      throw new ApiError('لا يمكن تغيير رمز أو مجموعة حساب أساسي في النظام');
    }
    mutate(() => Object.assign(found, { ...input, name: input.name.trim(), parentId: input.parentId || undefined }));
    account = found;
  } else {
    account = { id: uid('acc'), ...input, name: input.name.trim(), parentId: input.parentId || undefined, canDelete: true };
    mutate(() => db.accounts.push(account));
  }
  logActivity('journal', `${id ? 'تعديل' : 'إضافة'} الحساب ${account.code} — ${account.name}`, session.userId, new Date().toISOString(), '/accounting/accounts');
  return clone(account);
}

export async function deleteAccount(id: string): Promise<void> {
  await delay();
  const account = db.accounts.find((a) => a.id === id);
  if (!account) throw new ApiError('الحساب غير موجود', 'NOT_FOUND');
  if (!account.canDelete) throw new ApiError('حساب أساسي في النظام ولا يمكن حذفه');
  if (db.accounts.some((a) => a.parentId === id)) throw new ApiError('للحساب حسابات فرعية — احذفها أولاً', 'CONFLICT');
  if (db.journalEntries.some((e) => e.lines.some((l) => l.accountId === id))) throw new ApiError('للحساب قيود مسجلة — يمكنك إيقافه بدلاً من حذفه', 'CONFLICT');
  mutate(() => (db.accounts = db.accounts.filter((a) => a.id !== id)));
}

export async function renameAccountGroup(id: string, name: string): Promise<AccountGroup> {
  await delay();
  const group = db.accountGroups.find((g) => g.id === id);
  if (!group) throw new ApiError('المجموعة غير موجودة', 'NOT_FOUND');
  if (!name.trim()) throw new ApiError('اسم المجموعة مطلوب');
  mutate(() => (group.name = name.trim()));
  return clone(group);
}

// --- Journal ---------------------------------------------------------------------------------

/** `sourceLink` = app route of the document behind a SYSTEM entry. */
export type JournalRow = JournalEntry & { createdByName: string; sourceLink?: string };

export async function getJournalEntries(filter: JournalFilter = {}): Promise<JournalRow[]> {
  await delay();
  return db.journalEntries
    .filter(
      (e) =>
        (!filter.type || e.type === filter.type) &&
        (!filter.accountId || e.lines.some((l) => l.accountId === filter.accountId)) &&
        inDateRange(e.date, filter.from, filter.to) &&
        includesText([e.number, e.description, e.sourceRef?.number], filter.search),
    )
    .sort((a, b) => b.date.localeCompare(a.date) || b.number.localeCompare(a.number))
    .map((e) => ({ ...clone(e), createdByName: db.users.find((u) => u.id === e.createdBy)?.name ?? '—', sourceLink: sourceLink(e) }));
}

/** Server-mode variant of `getJournalEntries` for `DataTable`: paged, sorted and totalled server-side. */
export async function getJournalEntriesPaged(query: PagedQuery<JournalFilter>): Promise<PagedResult<JournalRow>> {
  await delay();
  const filter = query.filters ?? {};
  let rows: JournalRow[] = db.journalEntries
    .filter(
      (e) =>
        (!filter.type || e.type === filter.type) &&
        (!filter.accountId || e.lines.some((l) => l.accountId === filter.accountId)) &&
        inDateRange(e.date, filter.from, filter.to) &&
        includesText([e.number, e.description, e.sourceRef?.number], filter.search),
    )
    .map((e) => ({ ...clone(e), createdByName: db.users.find((u) => u.id === e.createdBy)?.name ?? '—', sourceLink: sourceLink(e) }));

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
}

export async function getJournalEntry(id: string): Promise<JournalRow & { reversedById?: string; reversedByNumber?: string }> {
  await delay();
  const entry = db.journalEntries.find((e) => e.id === id);
  if (!entry) throw new ApiError('القيد غير موجود', 'NOT_FOUND');
  const reversal = db.journalEntries.find((e) => e.reversalOfId === id);
  return {
    ...clone(entry),
    createdByName: db.users.find((u) => u.id === entry.createdBy)?.name ?? '—',
    sourceLink: sourceLink(entry),
    reversedById: reversal?.id,
    reversedByNumber: reversal?.number,
  };
}

export async function createJournalEntry(input: JournalEntryInput): Promise<JournalEntry> {
  await delay();
  return clone(recordManualJournal(input, session.userId));
}

export async function reverseJournalEntry(id: string): Promise<JournalEntry> {
  await delay();
  return clone(reverseJournal(id, session.userId));
}

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

export async function getFiscalYears(): Promise<FiscalYear[]> {
  await delay(120);
  return clone([...db.fiscalYears].sort((a, b) => b.startDate.localeCompare(a.startDate)));
}

/** The open fiscal year containing today (fallback: the latest one). Reports default to it. */
export async function getCurrentFiscalYear(): Promise<FiscalYear | undefined> {
  await delay(60);
  const today = localDateKey(new Date());
  const current = db.fiscalYears.find((f) => f.startDate <= today && f.endDate >= today) ?? [...db.fiscalYears].sort((a, b) => b.startDate.localeCompare(a.startDate))[0];
  return current ? clone(current) : undefined;
}

export async function saveFiscalYear(input: Omit<FiscalYear, 'id'>, id?: string): Promise<FiscalYear> {
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
}
