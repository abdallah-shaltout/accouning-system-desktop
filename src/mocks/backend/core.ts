import type { Account, FiscalYear, JournalEntry, JournalEntryType, JournalLine, JournalSourceKind } from '@/modules/accounting/types';
import type { ActivityKind } from '@/modules/core/types';
import type { AuditAction, AuditEntry, AuditFieldDiff } from '@/modules/diagnostics/types';
import type { Product, StockMovementReason } from '@/modules/products/types';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, localDateKey, round2, sum, uid } from '../utils';
import { accountById, accountFor } from './accounts';
import { baseCurrency } from './currency';
import { newCorrelationId } from '@/modules/diagnostics/services/logService';
import { recordPostingTrace } from './posting-trace';

export { accountById, accountFor, settlementAccountFor } from './accounts';

// ---------------------------------------------------------------------------------------------
// Ledger
// ---------------------------------------------------------------------------------------------

export interface PostingLine {
  /** Resolve by system role (preferred — see docs/v2/02-accounting-review.md F3). */
  role?: Parameters<typeof accountFor>[0];
  /** Escape hatch for a specific account id (manual journal, user-picked accounts). Never a hard-coded code. */
  accountId?: string;
  debit?: number;
  credit?: number;
  description?: string;
  partyKind?: 'customer' | 'supplier';
  partyId?: string;
  branchId?: string;
  costCenterId?: string;
  currency?: string;
  amountFc?: number;
  rate?: number;
  /**
   * 18.F1 — optional annotations a caller can attach to explain how it arrived at this line: line
   * discount, invoice-discount allocation, VAT base/rate/rounding, WAC before/after, FX rate. Every
   * field is optional and purely observational — nothing here feeds back into the posted amounts,
   * so a caller that never sets `trace` posts exactly as it always has. Read by
   * `posting-trace.ts` to build the document's `PostingTrace`.
   */
  trace?: {
    lineDiscount?: Record<string, unknown>;
    invoiceDiscount?: Record<string, unknown>;
    vat?: { base: number; rate: number; tax: number; roundingDelta?: number };
    cost?: { qtyBefore: number; avgBefore: number; qtyAfter: number; avgAfter: number };
    fx?: { rate: number; amountFc: number; base: number; roundingDelta?: number };
    note?: string;
  };
}

/** Resolve posting lines to account ids, drop zero lines, and assert debits = credits. */
export function resolvePosting(lines: PostingLine[]) {
  const pairs: { input: PostingLine; resolved: JournalLine }[] = lines.map((l) => {
    const account = l.accountId ? accountById(l.accountId) : accountFor(l.role!, { branchId: l.branchId, currency: l.currency });
    const branchId = l.branchId ?? DEFAULT_BRANCH_ID;
    return {
      input: l,
      resolved: {
        id: uid('jl'),
        accountId: account.id,
        description: l.description,
        debit: round2(l.debit ?? 0),
        credit: round2(l.credit ?? 0),
        partyKind: l.partyKind,
        partyId: l.partyId,
        branchId,
        // v2 phase 9 (docs/v2/10 §3 "Documents fill it from the branch, or from the document or
        // line if the user picked one"): an explicit line-level cost center wins; otherwise fall
        // back to the line's branch's own cost center (undefined pre-phase-9 / no branches yet).
        costCenterId: l.costCenterId ?? db.branches.find((b) => b.id === branchId)?.costCenterId,
        currency: l.currency ?? baseCurrency(),
        amountFc: l.amountFc,
        rate: l.rate,
      },
    };
  });
  const keptPairs = pairs.filter((p) => p.resolved.debit > 0 || p.resolved.credit > 0);
  const resolved = keptPairs.map((p) => p.resolved);
  const totalDebit = sum(resolved, (l) => l.debit);
  const totalCredit = sum(resolved, (l) => l.credit);
  if (Math.abs(totalDebit - totalCredit) > 0.001) {
    throw new ApiError(`القيد غير متوازن: المدين ${totalDebit} ≠ الدائن ${totalCredit}`);
  }
  return { lines: resolved, totalDebit, totalCredit, tracePairs: keptPairs };
}

/**
 * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §1): the id every journal line falls
 * back to when a caller doesn't pass one — matches `MAIN_BRANCH_ID` in `./branches.ts` (the seeded/
 * onboarded default branch's real id), so every pre-phase-9 posting path that never learned about
 * branches still lands on the one real `Branch` row instead of a dangling id.
 */
export const DEFAULT_BRANCH_ID = 'branch-main';
export { baseCurrency } from './currency';

/**
 * Fiscal-period guard (docs/v2/02-accounting-review.md B2): a posting date must fall inside an
 * open fiscal year and after the lock date. `allowClosedPeriod` is the
 * `accounting.postToClosedPeriod` override (Phase 1 keeps permissions coarse — see
 * modules/users/helpers/permissions.ts — so callers pass `role === 'admin'` for now).
 */
export function assertOpenPeriod(date: string, allowClosedPeriod = false): void {
  if (allowClosedPeriod) return;
  const key = localDateKey(date);
  const lockDate = db.settings.accounting?.lockDate;
  if (lockDate && key <= lockDate) {
    throw new ApiError(`لا يمكن الترحيل في تاريخ ${key} — الفترة مقفلة حتى ${lockDate}`, 'FORBIDDEN');
  }
  const fy = db.fiscalYears.find((f) => f.startDate <= key && f.endDate >= key);
  if (fy?.isClosed) {
    throw new ApiError(`لا يمكن الترحيل في تاريخ ${key} — السنة المالية "${fy.name}" مقفلة`, 'FORBIDDEN');
  }
}

/**
 * Posts a journal entry. This is the mock backend's single choke point for every ledger posting
 * (sales, purchases, payments, inventory, manual journal all funnel through it), so it's where
 * `mutate()` (debounced IndexedDB snapshot), the fiscal-period check and the `ledger:changed`
 * event are wired in — callers don't need their own wrapper just for the posting itself.
 */
export function postJournal(opts: {
  date: string;
  description: string;
  type: JournalEntryType;
  sourceRef?: { kind: JournalSourceKind; id: string; number?: string };
  lines: PostingLine[];
  createdBy: string;
  allowClosedPeriod?: boolean;
  attachmentIds?: string[];
  templateId?: string;
  correlationId?: string;
}): JournalEntry {
  assertOpenPeriod(opts.date, opts.allowClosedPeriod);
  const { lines, totalDebit, totalCredit, tracePairs } = resolvePosting(opts.lines);
  if (lines.length < 2) throw new ApiError('يجب أن يحتوي القيد على سطرين على الأقل');
  const now = new Date().toISOString();
  const entry: JournalEntry = {
    id: uid('je'),
    number: nextNumber('journal'),
    date: opts.date,
    description: opts.description,
    type: opts.type,
    status: 'POSTED',
    sourceRef: opts.sourceRef,
    lines,
    totalDebit,
    totalCredit,
    createdBy: opts.createdBy,
    createdAt: now,
    postedBy: opts.createdBy,
    postedAt: now,
    attachmentIds: opts.attachmentIds,
    templateId: opts.templateId,
  };
  mutate(() => db.journalEntries.push(entry));
  emit('ledger:changed');
  recordPostingTrace({
    docType: opts.sourceRef?.kind ?? opts.type,
    docId: entry.id,
    correlationId: opts.correlationId ?? newCorrelationId(),
    pairs: tracePairs,
    totalDebit,
    totalCredit,
  });
  return entry;
}

/**
 * A2 — saves a manual entry without posting it (no period check, no GL effect yet): it just sits
 * in the "مسودات بحاجة لترحيل" (drafts to post) list. Stored in `db.journalDrafts`, NOT
 * `db.journalEntries` — every ledger/balance/report reader in the app scans `journalEntries`
 * expecting only posted GL impact, so keeping drafts in a separate table is what keeps them
 * invisible to those readers without editing any of them.
 */
export function draftJournal(opts: {
  date: string;
  description: string;
  lines: PostingLine[];
  createdBy: string;
  attachmentIds?: string[];
  templateId?: string;
}): JournalEntry {
  const { lines, totalDebit, totalCredit } = resolvePosting(opts.lines);
  const now = new Date().toISOString();
  const entry: JournalEntry = {
    id: uid('je'),
    number: nextNumber('journal'),
    date: opts.date,
    description: opts.description,
    type: 'MANUAL',
    status: 'DRAFT',
    lines,
    totalDebit,
    totalCredit,
    createdBy: opts.createdBy,
    createdAt: now,
    attachmentIds: opts.attachmentIds,
    templateId: opts.templateId,
  };
  mutate(() => db.journalDrafts.push(entry));
  return entry;
}

/** Updates a saved draft's lines/description/date in place (edit before posting). */
export function updateDraftJournal(
  id: string,
  opts: { date: string; description: string; lines: PostingLine[]; attachmentIds?: string[] },
): JournalEntry {
  const entry = db.journalDrafts.find((e) => e.id === id);
  if (!entry) throw new ApiError('المسودة غير موجودة', 'NOT_FOUND');
  const { lines, totalDebit, totalCredit } = resolvePosting(opts.lines);
  mutate(() => {
    entry.date = opts.date;
    entry.description = opts.description;
    entry.lines = lines;
    entry.totalDebit = totalDebit;
    entry.totalCredit = totalCredit;
    if (opts.attachmentIds) entry.attachmentIds = opts.attachmentIds;
  });
  return entry;
}

export function deleteDraftJournal(id: string): void {
  const entry = db.journalDrafts.find((e) => e.id === id);
  if (!entry) throw new ApiError('المسودة غير موجودة', 'NOT_FOUND');
  mutate(() => (db.journalDrafts = db.journalDrafts.filter((e) => e.id !== id)));
}

/** Posts a previously saved draft: moves it from `journalDrafts` into the real ledger. */
export function postDraftJournal(id: string, userId: string, allowClosedPeriod = false): JournalEntry {
  const draft = db.journalDrafts.find((e) => e.id === id);
  if (!draft) throw new ApiError('المسودة غير موجودة', 'NOT_FOUND');
  assertOpenPeriod(draft.date, allowClosedPeriod);
  const now = new Date().toISOString();
  const posted: JournalEntry = { ...draft, status: 'POSTED', postedBy: userId, postedAt: now };
  mutate(() => {
    db.journalEntries.push(posted);
    db.journalDrafts = db.journalDrafts.filter((e) => e.id !== id);
  });
  emit('ledger:changed');
  return posted;
}

// ---------------------------------------------------------------------------------------------
// Stock
// ---------------------------------------------------------------------------------------------

export function productById(id: string): Product {
  const product = db.products.find((p) => p.id === id);
  if (!product) throw new ApiError('المنتج غير موجود', 'NOT_FOUND');
  return product;
}

/**
 * Change on-hand quantity AND stock value together (docs/v2/02-accounting-review.md A1/A2):
 * `valueChange` must be exactly the amount posted to the inventory account for this movement, so
 * `GL(inventory) = Σ product.stockValue` holds exactly. `costPrice` is re-derived as the 4-decimal
 * average (`stockValue / stockQty`) for display; it is never the posting input any more. Services
 * are not stocked.
 *
 * v2 phase 9 (docs/v2/07-products-and-inventory.md §4 "Branch stock"): `branchId` (defaulting to
 * `DEFAULT_BRANCH_ID` — every pre-phase-9 caller keeps posting to the one implicit branch) also
 * updates `product.stockByBranch[branchId]`, which always sums back to `stockQty`/`stockValue`.
 */
export function applyStockChange(
  product: Product,
  qtyChange: number,
  valueChange: number,
  reason: StockMovementReason,
  ref: { id: string; number: string },
  date: string,
  branchId: string = DEFAULT_BRANCH_ID,
): void {
  if (product.type === 'service' || product.stockMode === 'none' || (qtyChange === 0 && valueChange === 0)) return;
  mutate(() => {
    product.stockQty = round2(product.stockQty + qtyChange);
    product.stockValue = round2(product.stockValue + valueChange);
    product.costPrice = product.stockQty > 0.0001 ? round4(product.stockValue / product.stockQty) : 0;
    if (!product.stockByBranch) product.stockByBranch = {};
    const cur = product.stockByBranch[branchId] ?? { qty: 0, value: 0 };
    product.stockByBranch[branchId] = { qty: round2(cur.qty + qtyChange), value: round2(cur.value + valueChange) };
    db.stockMovements.push({
      id: uid('mv'),
      date,
      productId: product.id,
      qtyChange,
      valueChange,
      reason,
      refId: ref.id,
      refNumber: ref.number,
      balanceAfter: product.stockQty,
    });
  });
}

export function round4(n: number): number {
  return Math.round((n + Number.EPSILON) * 10000) / 10000;
}

// ---------------------------------------------------------------------------------------------
// Activity feed + business audit (18.B4)
// ---------------------------------------------------------------------------------------------

/** `ActivityKind` -> the coarser `AuditAction` used when a call site doesn't say one explicitly. */
const DEFAULT_ACTION_BY_KIND: Partial<Record<ActivityKind, AuditAction>> = {
  auth: 'login',
  settings: 'settings',
};

/** Entity kind guessed from an activity `link` like `/invoices/inv-12` or `/inventory/adjustments/adj-3`. */
function entityFromLink(kind: ActivityKind, link?: string): { entity: string; entityId: string } {
  if (link) {
    const segments = link.split('?')[0].split('/').filter(Boolean);
    const last = segments.at(-1);
    const parent = segments.at(-2);
    if (last && !/^\d+$/.test(last) && last !== parent) {
      return { entity: parent ?? kind, entityId: last };
    }
  }
  return { entity: kind, entityId: uid('unk') };
}

/**
 * `logActivity` becomes a thin adapter (18.B4): every call still appends the same one-line
 * `ActivityEntry` the feed has always read, and now also appends a structured `AuditEntry` to
 * `db.audit` — append-only, included in `backupArchive` (it's business data, not a diagnostic
 * log; see plan doc 18 decision 3). Call sites that need explicit `entity`/`action`/before-after
 * diffs should call `logAudit` directly instead (it also updates the activity feed).
 */
export function logActivity(kind: ActivityKind, message: string, userId: string, date: string, link?: string): void {
  const { entity, entityId } = entityFromLink(kind, link);
  logAudit({
    entity,
    entityId,
    action: DEFAULT_ACTION_BY_KIND[kind] ?? 'create',
    userId,
    at: date,
    message,
    link,
    activityKind: kind,
  });
}

export interface LogAuditInput {
  entity: string;
  entityId: string;
  entityLabel?: string;
  action: AuditAction;
  before?: AuditFieldDiff[];
  after?: AuditFieldDiff[];
  userId: string;
  branchId?: string;
  at?: string;
  reason?: string;
  message: string;
  link?: string;
  /** Also appended to the legacy `activity` feed under this kind. Defaults to a reasonable guess from `entity`. */
  activityKind?: ActivityKind;
}

const ENTITY_TO_ACTIVITY_KIND: Record<string, ActivityKind> = {
  invoice: 'sale',
  refund: 'refund',
  purchaseOrder: 'purchase',
  purchaseReturn: 'purchase_return',
  payment: 'payment',
  voucher: 'voucher',
  expense: 'expense',
  journal: 'journal',
  product: 'product',
  customer: 'party',
  supplier: 'party',
  user: 'user',
  branch: 'settings',
  costCenter: 'settings',
  shift: 'shift',
  approval: 'approval',
  adjustment: 'stock',
  count: 'stock',
  transfer: 'stock',
};

/** Structured audit write (18.B4) — the entry point new call sites should prefer over `logActivity`
 * when they can state entity/action/diffs explicitly. Always also appends to `db.activity` so
 * every existing feed/notification reader keeps working unchanged. Append-only: there is no
 * update/delete for `db.audit` anywhere in the backend. */
export function logAudit(input: LogAuditInput): void {
  const at = input.at ?? new Date().toISOString();
  const entry: AuditEntry = {
    id: uid('aud'),
    entity: input.entity,
    entityId: input.entityId,
    entityLabel: input.entityLabel,
    action: input.action,
    before: input.before,
    after: input.after,
    userId: input.userId,
    branchId: input.branchId,
    at,
    reason: input.reason,
    message: input.message,
    link: input.link,
  };
  mutate(() => {
    if (!db.audit) db.audit = [];
    db.audit.push(entry);
    db.activity.push({
      id: uid('act'),
      kind: input.activityKind ?? ENTITY_TO_ACTIVITY_KIND[input.entity] ?? 'settings',
      message: input.message,
      userId: input.userId,
      date: at,
      link: input.link,
    });
  });
}

/** Field-by-field diff of two plain objects for `AuditEntry.before`/`after` — only changed keys,
 * never the whole document (18.B4 "before/after field diff only"). Shallow by design: pass the
 * flattened fields that matter, not nested structures. */
export function diffFields(before: Record<string, unknown>, after: Record<string, unknown>): { before: AuditFieldDiff[]; after: AuditFieldDiff[] } {
  const beforeDiff: AuditFieldDiff[] = [];
  const afterDiff: AuditFieldDiff[] = [];
  const keys = new Set([...Object.keys(before), ...Object.keys(after)]);
  for (const field of keys) {
    const b = before[field];
    const a = after[field];
    if (JSON.stringify(b) === JSON.stringify(a)) continue;
    beforeDiff.push({ field, before: b });
    afterDiff.push({ field, after: a });
  }
  return { before: beforeDiff, after: afterDiff };
}

// ---------------------------------------------------------------------------------------------
// Tax
// ---------------------------------------------------------------------------------------------

export function salesTaxRate(): number {
  const tax =
    db.taxes.find((t) => t.id === db.settings.defaultTaxId && t.active) ??
    db.taxes.find((t) => t.type === 'OUTPUT' && t.isDefault && t.active);
  return tax?.rate ?? 0;
}

export function purchaseTaxRate(): number {
  const tax = db.taxes.find((t) => t.type === 'INPUT' && t.isDefault && t.active);
  return tax?.rate ?? 0;
}

export function userById(id: string) {
  return db.users.find((u) => u.id === id);
}

/** Re-exported for callers that still need the raw Account (reports, pickers). */
export type { Account };

// ---------------------------------------------------------------------------------------------
// Fiscal-year closing wizard (docs/v2/02-accounting-review.md B2)
// ---------------------------------------------------------------------------------------------

export interface CloseYearPreCheck {
  key: 'drafts' | 'trialBalance' | 'openingEquity';
  label: string;
  passed: boolean;
  detail: string;
}

/** Trial balance per the account's normal side, over all POSTED entries dated inside the year. */
function trialBalanceFor(fy: { startDate: string; endDate: string }): { debit: number; credit: number } {
  const totals = new Map<string, { d: number; c: number }>();
  for (const e of db.journalEntries) {
    if (e.date.slice(0, 10) < fy.startDate || e.date.slice(0, 10) > fy.endDate) continue;
    for (const l of e.lines) {
      const t = totals.get(l.accountId) ?? { d: 0, c: 0 };
      t.d += l.debit;
      t.c += l.credit;
      totals.set(l.accountId, t);
    }
  }
  let debit = 0;
  let credit = 0;
  for (const a of db.accounts) {
    if (a.isGroup) continue;
    const t = totals.get(a.id);
    if (!t) continue;
    const net = round2(t.d - t.c);
    if (net > 0) debit += net;
    else credit -= net;
  }
  return { debit: round2(debit), credit: round2(credit) };
}

/** Pre-checks run before a year can be closed (B2 step 1): no drafts, trial balance balanced, 3900 = 0. */
export function closeYearPreChecks(fiscalYearId: string): CloseYearPreCheck[] {
  const fy = db.fiscalYears.find((f) => f.id === fiscalYearId);
  if (!fy) throw new ApiError('السنة المالية غير موجودة', 'NOT_FOUND');

  const draftsInYear = db.journalDrafts.filter((d) => d.date.slice(0, 10) >= fy.startDate && d.date.slice(0, 10) <= fy.endDate);
  const tb = trialBalanceFor(fy);
  const openingEquity = accountFor('openingBalanceEquity');
  let obeNet = 0;
  for (const e of db.journalEntries) {
    for (const l of e.lines) {
      if (l.accountId === openingEquity.id) obeNet += l.debit - l.credit;
    }
  }
  obeNet = round2(obeNet);

  return [
    {
      key: 'drafts',
      label: 'لا توجد مسودات قيود بحاجة للترحيل',
      passed: draftsInYear.length === 0,
      detail: draftsInYear.length ? `${draftsInYear.length} مسودة بحاجة للترحيل أو الحذف` : 'لا توجد مسودات',
    },
    {
      key: 'trialBalance',
      label: 'ميزان المراجعة متوازن',
      passed: Math.abs(tb.debit - tb.credit) < 0.01,
      detail: `مدين ${tb.debit} — دائن ${tb.credit}`,
    },
    {
      key: 'openingEquity',
      label: 'حساب الأرصدة الافتتاحية (3900) صفر',
      passed: Math.abs(obeNet) < 0.01,
      detail: `الرصيد الحالي: ${obeNet}`,
    },
  ];
}

/**
 * Closes a fiscal year (B2 steps 2-4): posts the closing entry (every revenue/expense account →
 * retained earnings 3250), locks the year, and opens the next one if it doesn't already exist.
 * Every pre-check must pass first — callers should call `closeYearPreChecks` and confirm with the
 * user before calling this.
 */
export function closeFiscalYear(fiscalYearId: string, userId: string): { fiscalYear: FiscalYear; closingEntry: JournalEntry; nextYear?: FiscalYear } {
  const fy = db.fiscalYears.find((f) => f.id === fiscalYearId);
  if (!fy) throw new ApiError('السنة المالية غير موجودة', 'NOT_FOUND');
  if (fy.isClosed) throw new ApiError('السنة المالية مقفلة بالفعل');
  const checks = closeYearPreChecks(fiscalYearId);
  const failed = checks.find((c) => !c.passed);
  if (failed) throw new ApiError(`تعذر إقفال السنة: ${failed.label} — ${failed.detail}`, 'FORBIDDEN');

  const totals = new Map<string, { d: number; c: number }>();
  for (const e of db.journalEntries) {
    if (e.date.slice(0, 10) < fy.startDate || e.date.slice(0, 10) > fy.endDate) continue;
    for (const l of e.lines) {
      const t = totals.get(l.accountId) ?? { d: 0, c: 0 };
      t.d += l.debit;
      t.c += l.credit;
      totals.set(l.accountId, t);
    }
  }
  const lines: PostingLine[] = [];
  let net = 0; // + = net profit (credit to retained earnings)
  for (const a of db.accounts) {
    if (a.isGroup) continue;
    const t = totals.get(a.id);
    if (!t) continue;
    if (a.kind === 'REVENUE') {
      const balance = round2(t.c - t.d); // credit-normal
      if (balance === 0) continue;
      lines.push(balance > 0 ? { accountId: a.id, debit: balance } : { accountId: a.id, credit: -balance });
      net += balance;
    } else if (a.kind === 'EXPENSE') {
      const balance = round2(t.d - t.c); // debit-normal
      if (balance === 0) continue;
      lines.push(balance > 0 ? { accountId: a.id, credit: balance } : { accountId: a.id, debit: -balance });
      net -= balance;
    }
  }
  net = round2(net);
  const retained = accountFor('retainedEarnings');
  if (net !== 0) lines.push(net > 0 ? { accountId: retained.id, credit: net } : { accountId: retained.id, debit: -net });

  const closingEntry = postJournal({
    date: fy.endDate,
    description: `قيد إقفال السنة المالية ${fy.name}`,
    type: 'CLOSING',
    lines,
    createdBy: userId,
    allowClosedPeriod: true,
  });

  mutate(() => {
    fy.isClosed = true;
    fy.closingEntryId = closingEntry.id;
    fy.closedAt = new Date().toISOString();
    fy.closedBy = userId;
  });
  logActivity('journal', `إقفال السنة المالية ${fy.name}`, userId, new Date().toISOString(), '/accounting/fiscal-years');

  // Open the next year automatically if it doesn't exist yet (B2 step 4).
  let nextYear = db.fiscalYears.find((f) => f.startDate > fy.endDate);
  if (!nextYear) {
    const nextStart = new Date(fy.endDate);
    nextStart.setDate(nextStart.getDate() + 1);
    const nextEnd = new Date(nextStart);
    nextEnd.setFullYear(nextEnd.getFullYear() + 1);
    nextEnd.setDate(nextEnd.getDate() - 1);
    const nextName = String(Number(fy.name.match(/\d+/)?.[0] ?? new Date(fy.startDate).getFullYear()) + 1 || new Date(nextStart).getFullYear());
    nextYear = {
      id: uid('fy'),
      name: nextName,
      startDate: localDateKey(nextStart),
      endDate: localDateKey(nextEnd),
      isClosed: false,
    };
    mutate(() => db.fiscalYears.push(nextYear!));
  }

  return { fiscalYear: fy, closingEntry, nextYear };
}

/** Reopens a closed year: reverses the closing entry and unlocks the year. Admin-only (checked by the caller/service). */
export function reopenFiscalYear(fiscalYearId: string, userId: string): FiscalYear {
  const fy = db.fiscalYears.find((f) => f.id === fiscalYearId);
  if (!fy) throw new ApiError('السنة المالية غير موجودة', 'NOT_FOUND');
  if (!fy.isClosed) throw new ApiError('السنة المالية غير مقفلة');
  if (fy.closingEntryId) {
    const closing = db.journalEntries.find((e) => e.id === fy.closingEntryId);
    if (closing && !closing.reversed) {
      const reversal = postJournal({
        date: new Date().toISOString(),
        description: `عكس قيد إقفال السنة المالية ${fy.name}`,
        type: 'CLOSING',
        lines: closing.lines.map((l) => ({
          accountId: l.accountId,
          description: l.description,
          debit: l.credit,
          credit: l.debit,
          partyKind: l.partyKind,
          partyId: l.partyId,
        })),
        createdBy: userId,
        allowClosedPeriod: true,
      });
      mutate(() => {
        reversal.reversalOfId = closing.id;
        reversal.reversalReason = 'إعادة فتح السنة المالية';
        closing.reversed = true;
      });
    }
  }
  mutate(() => {
    fy.isClosed = false;
    fy.closingEntryId = undefined;
    fy.closedAt = undefined;
    fy.closedBy = undefined;
  });
  logActivity('journal', `إعادة فتح السنة المالية ${fy.name}`, userId, new Date().toISOString(), '/accounting/fiscal-years');
  return fy;
}
