/**
 * v2 phase 5 (docs/v2/05-onboarding.md §3, §4): opening-balance posting engine. Additive module —
 * does not touch `core.ts`'s existing fiscal-year/closing logic. Two entry points:
 *
 *   - `postOpeningEntry()` — the wizard's step-8 "opening balances" review screen: one `OPENING`
 *     journal entry built from cash/bank + customers + suppliers + other-balance-sheet lines
 *     (stock is posted separately through `recordStockAdjustment(reason: 'opening')`, which
 *     already credits `openingBalanceEquity` — see `src/mocks/backend/inventory.ts`).
 *   - `closeOpeningBalanceEquity()` — closes 3900 to zero by moving its balance to capital or the
 *     owner's current account (docs/v2/02-accounting-review.md invariant 9).
 *   - `postPartyOpeningBalance()` — the party-form "رصيد سابق من نظام قديم" stub (docs/v2/05 §4):
 *     a small OPENING entry for a single party, added any time (not just during onboarding).
 *
 * "Before first use" editability (docs/v2/05 §3 "Posting"): both the opening entry and the closing
 * entry can be reversed and re-posted until the first sale or purchase posts. `hasFirstUsePosted()`
 * checks whether any SYSTEM invoice/purchase entry already exists.
 */
import type { JournalEntry } from '@/modules/accounting/types';
import type { Product } from '@/modules/products/types';
import { db } from '../db';
import { round2, sum, uid } from '../utils';
import { accountFor } from './accounts';
import { applyStockChange, DEFAULT_BRANCH_ID, logActivity, postJournal, productById, type PostingLine } from './core';
import { receiveBatch } from './inventory';

export interface OpeningCashLine {
  /** 'cash' role (a drawer) or 'bank' role account id — always a specific account, not a role, since a company can have several. */
  accountId: string;
  amount: number;
  currency?: string;
  amountFc?: number;
  rate?: number;
}

export interface OpeningPartyLine {
  partyKind: 'customer' | 'supplier';
  partyId: string;
  amount: number;
  /** 'debit' = the customer owes us / we owe the supplier less; mirrors the party-form stub's `side`. */
  side: 'debit' | 'credit';
}

export interface OpeningOtherLine {
  accountId: string;
  side: 'debit' | 'credit';
  amount: number;
  description?: string;
}

export interface OpeningEntryInput {
  date: string;
  cash: OpeningCashLine[];
  customers: OpeningPartyLine[];
  suppliers: OpeningPartyLine[];
  other: OpeningOtherLine[];
  createdBy: string;
}

/** True once the company has posted its first real sale or purchase (docs/v2/05 §3 "Before first use"). */
export function hasFirstUsePosted(): boolean {
  return db.journalEntries.some((e) => e.sourceRef?.kind === 'invoice' || e.sourceRef?.kind === 'purchaseOrder');
}

/** Current net balance of the 3900 (openingBalanceEquity) account — debit-positive. 0 once onboarding is complete (invariant 9). */
export function openingBalanceEquityNet(): number {
  const account = accountFor('openingBalanceEquity');
  let net = 0;
  for (const e of db.journalEntries) {
    for (const l of e.lines) {
      if (l.accountId === account.id) net += l.debit - l.credit;
    }
  }
  return round2(net);
}

/**
 * Builds the posting lines for the step-8 review screen without posting — used to render the
 * "opening journal entry" preview and the mini balance sheet before the user confirms.
 */
export function buildOpeningLines(input: Omit<OpeningEntryInput, 'date' | 'createdBy'>): PostingLine[] {
  const lines: PostingLine[] = [];
  for (const c of input.cash) {
    if (!c.amount) continue;
    lines.push({ accountId: c.accountId, debit: c.amount, currency: c.currency, amountFc: c.amountFc, rate: c.rate });
  }
  for (const c of input.customers) {
    if (!c.amount) continue;
    lines.push({
      role: 'receivable',
      partyKind: 'customer',
      partyId: c.partyId,
      debit: c.side === 'debit' ? c.amount : 0,
      credit: c.side === 'credit' ? c.amount : 0,
    });
  }
  for (const s of input.suppliers) {
    if (!s.amount) continue;
    lines.push({
      role: 'payable',
      partyKind: 'supplier',
      partyId: s.partyId,
      debit: s.side === 'debit' ? s.amount : 0,
      credit: s.side === 'credit' ? s.amount : 0,
    });
  }
  for (const o of input.other) {
    if (!o.amount) continue;
    lines.push({ accountId: o.accountId, debit: o.side === 'debit' ? o.amount : 0, credit: o.side === 'credit' ? o.amount : 0, description: o.description });
  }
  // The balancing figure goes to 3900 — whichever side is short.
  const debit = sum(lines, (l) => l.debit ?? 0);
  const credit = sum(lines, (l) => l.credit ?? 0);
  const diff = round2(debit - credit);
  if (Math.abs(diff) > 0.001) {
    const equity = accountFor('openingBalanceEquity');
    lines.push(diff > 0 ? { accountId: equity.id, credit: diff } : { accountId: equity.id, debit: -diff });
  }
  return lines;
}

/**
 * Posts the wizard's step-8 opening entry (docs/v2/05 §3 "Posting: one OPENING journal entry").
 * Returns the posted entry. Stock is NOT part of this entry — it's posted separately via
 * `recordStockAdjustment(reason: 'opening')` per product/branch, per the doc ("stock movements with
 * reason opening and stockValue set").
 */
export function postOpeningEntry(input: OpeningEntryInput): JournalEntry {
  const lines = buildOpeningLines(input);
  const entry = postJournal({
    date: input.date,
    description: 'القيد الافتتاحي',
    type: 'OPENING',
    sourceRef: { kind: 'opening', id: 'onboarding', number: 'OPENING' },
    lines,
    createdBy: input.createdBy,
    allowClosedPeriod: true,
  });
  logActivity('journal', 'ترحيل القيد الافتتاحي', input.createdBy, new Date().toISOString(), '/accounting/journal');
  return entry;
}

/**
 * Closes 3900 to zero (docs/v2/05 §3 "Closing 3900", invariant 9): moves whatever balance sits on
 * `openingBalanceEquity` (from the opening entry above AND/or the opening stock-in postings) to
 * `capital` or `ownerCurrent`. A debit balance on 3900 (assets > liabilities+equity supplied) means
 * 3900 needs a credit to zero it out, which debits capital; a credit balance is the mirror.
 */
export function closeOpeningBalanceEquity(date: string, target: 'capital' | 'ownerCurrent', createdBy: string): JournalEntry | undefined {
  const net = openingBalanceEquityNet();
  if (Math.abs(net) < 0.01) return undefined;
  const equity = accountFor('openingBalanceEquity');
  const targetAccount = accountFor(target);
  const lines: PostingLine[] =
    net > 0
      ? [{ accountId: equity.id, credit: net }, { accountId: targetAccount.id, debit: net }]
      : [{ accountId: equity.id, debit: -net }, { accountId: targetAccount.id, credit: -net }];
  const entry = postJournal({
    date,
    description: 'إقفال حساب الأرصدة الافتتاحية (3900) إلى ' + (target === 'capital' ? 'رأس المال' : 'جاري المالك'),
    type: 'CLOSING',
    sourceRef: { kind: 'opening', id: 'onboarding-close', number: 'OPENING-CLOSE' },
    lines,
    createdBy,
    allowClosedPeriod: true,
  });
  logActivity('journal', 'إقفال حساب الأرصدة الافتتاحية', createdBy, new Date().toISOString(), '/accounting/journal');
  return entry;
}

/**
 * Runs the full step-8 posting: the opening entry, then closes 3900. Convenience wrapper for the
 * wizard's "confirm" button and for `seedEmptyCompany`-adjacent flows. Returns both entries (the
 * closing entry is `undefined` if 3900 net was already zero, e.g. no lines were entered).
 */
export function postOpeningBalancesAndClose(
  input: OpeningEntryInput,
  closeTarget: 'capital' | 'ownerCurrent',
): { openingEntry: JournalEntry; closingEntry?: JournalEntry } {
  const openingEntry = postOpeningEntry(input);
  const closingEntry = closeOpeningBalanceEquity(input.date, closeTarget, input.createdBy);
  return { openingEntry, closingEntry };
}

// ---------------------------------------------------------------------------------------------
// Opening stock (docs/v2/05 §3 tab 4 "المخزون") — per branch, reason 'opening', stockValue set.
// ---------------------------------------------------------------------------------------------

export interface OpeningStockLine {
  productId: string;
  qty: number;
  unitCost: number;
  batchNo?: string;
  expiryDate?: string;
}

/**
 * Posts opening stock for one branch: Dr inventory[branch] / Cr openingBalanceEquity, exactly
 * mirroring `postAdjustment`'s STOCK_IN/opening case in `inventory.ts` (kept separate rather than
 * reusing `recordStockAdjustment` directly, since that helper always posts to
 * `DEFAULT_BRANCH_ID` — docs/v2/05 §3 "per branch" needs a real per-branch inventory account and
 * `product.stockByBranch` entry, which only `applyStockChange(..., branchId)` gives us).
 */
export function postOpeningStockForBranch(branchId: string, date: string, lines: OpeningStockLine[], createdBy: string): JournalEntry | undefined {
  const valid = lines.filter((l) => l.qty > 0);
  if (!valid.length) return undefined;
  let total = 0;
  const adjRef = { id: uid('adj'), number: 'OPENING-STOCK' };
  for (const line of valid) {
    const product: Product = productById(line.productId);
    const value = round2(line.qty * (line.unitCost || product.costPrice || 0));
    total += value;
    // `applyStockChange`'s `reason` is the StockMovement audit label ('stock_in'), distinct from
    // the StockAdjustment-level StockInReason ('opening') that decides the credit account below.
    applyStockChange(product, line.qty, value, 'stock_in', adjRef, date, branchId);
    if (product.trackBatches && line.batchNo) {
      receiveBatch(product.id, line.qty, line.unitCost || product.costPrice, line.batchNo, line.expiryDate, date, adjRef);
    }
  }
  total = round2(total);
  if (total <= 0) return undefined;
  const inventoryAccount = accountFor('inventory', { branchId });
  const equity = accountFor('openingBalanceEquity');
  const entry = postJournal({
    date,
    description: `رصيد افتتاحي للمخزون — ${db.branches.find((b) => b.id === branchId)?.name ?? branchId}`,
    type: 'OPENING',
    sourceRef: { kind: 'opening', id: adjRef.id, number: adjRef.number },
    lines: [
      { accountId: inventoryAccount.id, debit: total },
      { accountId: equity.id, credit: total },
    ],
    createdBy,
    allowClosedPeriod: true,
  });
  logActivity('stock', `رصيد افتتاحي للمخزون — ${valid.length} صنف`, createdBy, date, '/inventory');
  return entry;
}

/** Convenience: posts opening stock to the single default branch (single-branch companies). */
export function postOpeningStockDefault(date: string, lines: OpeningStockLine[], createdBy: string): JournalEntry | undefined {
  return postOpeningStockForBranch(DEFAULT_BRANCH_ID, date, lines, createdBy);
}

// ---------------------------------------------------------------------------------------------
// Party-form "رصيد سابق من نظام قديم" stub (docs/v2/05 §4)
// ---------------------------------------------------------------------------------------------

export interface PartyOpeningBalanceInput {
  partyKind: 'customer' | 'supplier';
  partyId: string;
  amount: number;
  side: 'debit' | 'credit';
  asOfDate: string;
  createdBy: string;
}

/**
 * Posts a small OPENING entry for one party added after (or during) onboarding — docs/v2/05 §4:
 * "Dr receivable[p] / Cr openingBalanceEquity (mirrored for suppliers) — UNLESS the as-of date is
 * after go-live, in which case the credit side goes straight to capital instead of 3900 (with a
 * note), keeping 3900 at zero."
 */
export function postPartyOpeningBalance(input: PartyOpeningBalanceInput): JournalEntry | undefined {
  if (!input.amount) return undefined;
  const goLiveDate = db.settings.onboarding?.goLiveDate;
  const afterGoLive = !!goLiveDate && input.asOfDate > goLiveDate;
  const role = input.partyKind === 'customer' ? ('receivable' as const) : ('payable' as const);
  const partyDebit = input.partyKind === 'customer' ? input.side === 'debit' : input.side === 'credit';

  const counterRole = afterGoLive ? 'capital' : 'openingBalanceEquity';
  const counterAccount = accountFor(counterRole);
  const partyLine: PostingLine = {
    role,
    partyKind: input.partyKind,
    partyId: input.partyId,
    debit: partyDebit ? input.amount : 0,
    credit: partyDebit ? 0 : input.amount,
  };
  const counterLine: PostingLine = { accountId: counterAccount.id, debit: partyDebit ? 0 : input.amount, credit: partyDebit ? input.amount : 0 };

  const entry = postJournal({
    date: input.asOfDate,
    description: `رصيد افتتاحي — ${input.partyKind === 'customer' ? 'عميل' : 'مورد'}${afterGoLive ? ' (بعد تاريخ البدء — أُقفل مباشرة إلى رأس المال)' : ''}`,
    type: 'OPENING',
    sourceRef: { kind: 'opening', id: uid('popen'), number: 'OPENING' },
    lines: [partyLine, counterLine],
    createdBy: input.createdBy,
    allowClosedPeriod: true,
  });
  logActivity(
    'party',
    `رصيد افتتاحي (${input.amount}) — ${input.partyKind === 'customer' ? 'عميل' : 'مورد'}`,
    input.createdBy,
    new Date().toISOString(),
    input.partyKind === 'customer' ? `/customers/${input.partyId}` : `/suppliers/${input.partyId}`,
  );
  return entry;
}

/**
 * Reverses a previously posted party opening-balance entry (docs/v2/05 §4 "Editing: allowed until
 * a payment is allocated to it"). Callers check allocation state before calling this.
 */
export function reversePartyOpeningBalance(entryId: string, userId: string): JournalEntry {
  const original = db.journalEntries.find((e) => e.id === entryId);
  if (!original) throw new Error('القيد غير موجود');
  const reversedLines: PostingLine[] = original.lines.map((l) => ({
    accountId: l.accountId,
    debit: l.credit,
    credit: l.debit,
    partyKind: l.partyKind,
    partyId: l.partyId,
    branchId: l.branchId,
    costCenterId: l.costCenterId,
    currency: l.currency,
    amountFc: l.amountFc,
    rate: l.rate,
    description: l.description,
  }));
  const reversal = postJournal({
    date: new Date().toISOString().slice(0, 10),
    description: `عكس: ${original.description}`,
    type: 'OPENING',
    lines: reversedLines,
    createdBy: userId,
    allowClosedPeriod: true,
  });
  const originalRef = db.journalEntries.find((e) => e.id === entryId)!;
  originalRef.reversed = true;
  reversal.reversalOfId = entryId;
  return reversal;
}
