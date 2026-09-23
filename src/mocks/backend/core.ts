import type { Account, JournalEntry, JournalSourceKind } from '@/modules/accounting/types';
import type { ActivityKind } from '@/modules/core/types';
import type { Product, StockMovementReason } from '@/modules/products/types';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, round2, sum, uid } from '../utils';

// ---------------------------------------------------------------------------------------------
// Ledger
// ---------------------------------------------------------------------------------------------

export function accountByCode(code: string): Account {
  const account = db.accounts.find((a) => a.code === code);
  if (!account) throw new ApiError(`الحساب ${code} غير موجود في دليل الحسابات`, 'NOT_FOUND');
  return account;
}

export interface PostingLine {
  code?: string;
  accountId?: string;
  debit?: number;
  credit?: number;
  description?: string;
}

/** Resolve posting lines to account ids, drop zero lines, and assert debits = credits. */
export function resolvePosting(lines: PostingLine[]) {
  const resolved = lines
    .map((l) => ({
      id: uid('jl'),
      accountId: l.accountId ?? accountByCode(l.code!).id,
      description: l.description,
      debit: round2(l.debit ?? 0),
      credit: round2(l.credit ?? 0),
    }))
    .filter((l) => l.debit > 0 || l.credit > 0);
  const totalDebit = sum(resolved, (l) => l.debit);
  const totalCredit = sum(resolved, (l) => l.credit);
  if (Math.abs(totalDebit - totalCredit) > 0.001) {
    throw new ApiError(`القيد غير متوازن: المدين ${totalDebit} ≠ الدائن ${totalCredit}`);
  }
  return { lines: resolved, totalDebit, totalCredit };
}

/**
 * Posts a journal entry. This is the mock backend's single choke point for every ledger posting
 * (sales, purchases, payments, inventory, manual journal all funnel through it), so it's where
 * `mutate()` (debounced IndexedDB snapshot) and the `ledger:changed` event are wired in — callers
 * don't need their own `mutate()` wrapper just for the posting itself.
 */
export function postJournal(opts: {
  date: string;
  description: string;
  type: 'SYSTEM' | 'MANUAL';
  sourceRef?: { kind: JournalSourceKind; id: string; number?: string };
  lines: PostingLine[];
  createdBy: string;
}): JournalEntry {
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

/** Cash for cash payments, Bank for card / transfer. */
export function settlementAccount(method: string): string {
  return method === 'cash' || method === 'credit' ? '1110' : '1120';
}

// ---------------------------------------------------------------------------------------------
// Stock
// ---------------------------------------------------------------------------------------------

export function productById(id: string): Product {
  const product = db.products.find((p) => p.id === id);
  if (!product) throw new ApiError('المنتج غير موجود', 'NOT_FOUND');
  return product;
}

/** Change on-hand quantity and append a movement to the stock ledger. Services are not stocked. */
export function applyStockChange(
  product: Product,
  qtyChange: number,
  reason: StockMovementReason,
  ref: { id: string; number: string },
  date: string,
): void {
  if (product.type === 'service' || qtyChange === 0) return;
  mutate(() => {
    product.stockQty = round2(product.stockQty + qtyChange);
    db.stockMovements.push({
      id: uid('mv'),
      date,
      productId: product.id,
      qtyChange,
      reason,
      refId: ref.id,
      refNumber: ref.number,
      balanceAfter: product.stockQty,
    });
  });
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
