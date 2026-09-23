import type { Account, JournalEntry, JournalLine, JournalSourceKind } from '@/modules/accounting/types';
import type { ActivityKind } from '@/modules/core/types';
import type { Product, StockMovementReason } from '@/modules/products/types';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, localDateKey, round2, sum, uid } from '../utils';
import { accountById, accountFor } from './accounts';

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
}

/** Resolve posting lines to account ids, drop zero lines, and assert debits = credits. */
export function resolvePosting(lines: PostingLine[]) {
  const resolved: JournalLine[] = lines
    .map((l) => {
      const account = l.accountId ? accountById(l.accountId) : accountFor(l.role!, { branchId: l.branchId, currency: l.currency });
      return {
        id: uid('jl'),
        accountId: account.id,
        description: l.description,
        debit: round2(l.debit ?? 0),
        credit: round2(l.credit ?? 0),
        partyKind: l.partyKind,
        partyId: l.partyId,
        branchId: l.branchId ?? DEFAULT_BRANCH_ID,
        costCenterId: l.costCenterId,
        currency: l.currency ?? BASE_CURRENCY,
        amountFc: l.amountFc,
        rate: l.rate,
      };
    })
    .filter((l) => l.debit > 0 || l.credit > 0);
  const totalDebit = sum(resolved, (l) => l.debit);
  const totalCredit = sum(resolved, (l) => l.credit);
  if (Math.abs(totalDebit - totalCredit) > 0.001) {
    throw new ApiError(`القيد غير متوازن: المدين ${totalDebit} ≠ الدائن ${totalCredit}`);
  }
  return { lines: resolved, totalDebit, totalCredit };
}

/**
 * Single default branch id used on every journal line until Phase 9 builds real branches. Kept as
 * a named constant (not a magic string sprinkled around) so the Phase 9 migration is a one-line
 * change once `Branch` entities exist.
 */
export const DEFAULT_BRANCH_ID = 'branch-main';
export const BASE_CURRENCY = 'SAR';

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
  type: 'SYSTEM' | 'MANUAL';
  sourceRef?: { kind: JournalSourceKind; id: string; number?: string };
  lines: PostingLine[];
  createdBy: string;
  allowClosedPeriod?: boolean;
}): JournalEntry {
  assertOpenPeriod(opts.date, opts.allowClosedPeriod);
  const { lines, totalDebit, totalCredit } = resolvePosting(opts.lines);
  if (lines.length < 2) throw new ApiError('يجب أن يحتوي القيد على سطرين على الأقل');
  const entry: JournalEntry = {
    id: uid('je'),
    number: nextNumber('journal'),
    date: opts.date,
    description: opts.description,
    type: opts.type,
    sourceRef: opts.sourceRef,
    lines,
    totalDebit,
    totalCredit,
    createdBy: opts.createdBy,
  };
  mutate(() => db.journalEntries.push(entry));
  emit('ledger:changed');
  return entry;
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
 */
export function applyStockChange(
  product: Product,
  qtyChange: number,
  valueChange: number,
  reason: StockMovementReason,
  ref: { id: string; number: string },
  date: string,
): void {
  if (product.type === 'service' || (qtyChange === 0 && valueChange === 0)) return;
  mutate(() => {
    product.stockQty = round2(product.stockQty + qtyChange);
    product.stockValue = round2(product.stockValue + valueChange);
    product.costPrice = product.stockQty > 0.0001 ? round4(product.stockValue / product.stockQty) : 0;
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
// Activity feed
// ---------------------------------------------------------------------------------------------

export function logActivity(kind: ActivityKind, message: string, userId: string, date: string, link?: string): void {
  mutate(() => db.activity.push({ id: uid('act'), kind, message, userId, date, link }));
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
