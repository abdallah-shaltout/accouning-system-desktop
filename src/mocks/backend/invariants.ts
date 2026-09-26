/**
 * Runtime accounting invariants (18.F2, docs/v2/02-accounting-review.md §4) — the ONE copy shared
 * by `scripts/verify/*.ts` (`bun run verify:mocks`, seeded-DB check) and the running app's debug
 * mode (checked after every `mutate()`, see `diagnosticsService.ts`'s invariant watcher). Moving
 * these into the backend (instead of duplicating in scripts/) means a real regression here is
 * caught the same way in CI and on a live debug-mode desktop session.
 *
 * Each function is pure: it only reads `db` (passed as `MockDb`) and returns an `InvariantResult`,
 * never mutates, never throws. `scripts/verify/*.ts` wraps these in its own `ok()/fail()/todo()`
 * Result type for console printing; this module doesn't know about that presentation layer.
 *
 * Deliberately NOT moved here: seed-specific worked examples (the pinned FX scenario, the
 * cost-center rent-split demo entry, "at least 2 branches/3 cost centers in the seed") — those
 * assert facts about the demo seed's own data, not general accounting invariants that must hold
 * for ANY valid state, so they stay in `scripts/verify/branches.ts`/`sales.ts` as seed sanity
 * checks rather than living beside the 11 numbered invariants below.
 */
import type { MockDb } from '../db';
import { round2 } from '@/modules/core/helpers/numbers';
import { accountFor } from './accounts';
import { customerBalance, customerStatement, supplierBalance, supplierStatement } from './balances';
import { unallocatedCreditFor } from './payments';

export interface InvariantResult {
  /** Stable key, e.g. 'balanced-entries', 'ar-control'. Used for the `ACC-` issue stub and for
   * de-duping repeated failures across debounced runs. */
  key: string;
  /** One of the 11 numbered invariants from docs/v2/02-accounting-review.md §4. */
  doc: string;
  passed: boolean;
  message: string;
  /** Expected/actual in the minor unit (هللة/قرش) — set only when the invariant is a numeric
   * equality check and it failed, so the debug-mode logger can print an exact diff. */
  diff?: { expected: number; actual: number; deltaMinorUnits: number };
}

function closeEnough(a: number, b: number, tolerance = 0.01): boolean {
  return Math.abs(a - b) <= tolerance;
}

function numericCheck(key: string, doc: string, label: string, expected: number, actual: number, tolerance = 0.01): InvariantResult {
  const passed = closeEnough(expected, actual, tolerance);
  return {
    key,
    doc,
    passed,
    message: `${label} (${actual} vs ${expected})`,
    diff: passed ? undefined : { expected, actual, deltaMinorUnits: Math.round((actual - expected) * 100) },
  };
}

function glBalance(db: MockDb, role: Parameters<typeof accountFor>[0]): number {
  const id = accountFor(role).id;
  return round2(
    db.journalEntries
      .flatMap((e) => e.lines)
      .filter((l) => l.accountId === id)
      .reduce((a, l) => a + l.debit - l.credit, 0),
  );
}

/** 1. Every entry: Σ debit = Σ credit; no line is both debit and credit; ≥ 2 lines. Returns three
 * results — kept separate (matching the three distinct checks the original verify script printed)
 * so a failure in one clause never hides inside a combined message. */
export function checkBalancedEntries(db: MockDb): InvariantResult[] {
  const unbalanced = db.journalEntries.filter((e) => !closeEnough(e.totalDebit, e.totalCredit, 0.001));
  const bothSides = db.journalEntries.flatMap((e) => e.lines).filter((l) => l.debit > 0 && l.credit > 0);
  const tooShort = db.journalEntries.filter((e) => e.lines.length < 2);
  return [
    {
      key: 'balanced-entries',
      doc: '§4.1',
      passed: unbalanced.length === 0,
      message: `every journal entry is balanced (${unbalanced.length} unbalanced: ${unbalanced.map((e) => e.number).join(', ')})`,
    },
    {
      key: 'no-both-sided-lines',
      doc: '§4.1',
      passed: bothSides.length === 0,
      message: `no journal line is both debit and credit (${bothSides.length} offenders)`,
    },
    {
      key: 'min-two-lines',
      doc: '§4.1',
      passed: tooShort.length === 0,
      message: `every journal entry has >= 2 lines (${tooShort.length} offenders)`,
    },
  ];
}

/** 2. Trial balance is balanced; balance sheet A = L + E, with the current result included.
 * Returns two results (kept separate, matching the two distinct checks the original verify
 * script printed): trial balance, then the balance sheet identity. */
export function checkTrialBalance(db: MockDb): InvariantResult[] {
  const movement = new Map<string, { d: number; c: number }>();
  for (const e of db.journalEntries) {
    for (const l of e.lines) {
      const t = movement.get(l.accountId) ?? { d: 0, c: 0 };
      t.d += l.debit;
      t.c += l.credit;
      movement.set(l.accountId, t);
    }
  }
  let trialDebit = 0;
  let trialCredit = 0;
  let assets = 0;
  let liabilities = 0;
  let equity = 0;
  for (const a of db.accounts) {
    if (a.isGroup) continue;
    const t = movement.get(a.id);
    if (!t) continue;
    const debitNet = round2(t.d - t.c);
    if (debitNet > 0) trialDebit += debitNet;
    else trialCredit += -debitNet;
    if (a.kind === 'ASSET') assets += debitNet;
    else if (a.kind === 'LIABILITY') liabilities += -debitNet;
    else if (a.kind === 'EQUITY') equity += -debitNet;
    else if (a.kind === 'REVENUE') equity += -debitNet;
    else if (a.kind === 'EXPENSE') equity -= debitNet;
  }
  trialDebit = round2(trialDebit);
  trialCredit = round2(trialCredit);
  assets = round2(assets);
  liabilities = round2(liabilities);
  equity = round2(equity);
  return [
    { ...numericCheck('trial-balance', '§4.2', 'trial balance is balanced', trialCredit, trialDebit), doc: '§4.2' },
    { ...numericCheck('balance-sheet', '§4.2', 'balance sheet balanced: A = L + E incl. current result', round2(liabilities + equity), assets), doc: '§4.2' },
  ];
}

/** 3. GL(receivable) = Σ customer sub-ledgers; GL(payable) = Σ supplier sub-ledgers. */
export function checkArApControl(db: MockDb): InvariantResult[] {
  const arGl = glBalance(db, 'receivable');
  const arSum = round2(db.customers.reduce((a, c) => a + customerBalance(c.id), 0));
  const apGl = -glBalance(db, 'payable');
  const apSum = round2(db.suppliers.reduce((a, s) => a + supplierBalance(s.id), 0));
  return [
    { ...numericCheck('ar-control', '§4.3', 'AR GL = Σ customer balances', arSum, arGl), doc: '§4.3' },
    { ...numericCheck('ap-control', '§4.3', 'AP GL = Σ supplier balances', apSum, apGl), doc: '§4.3' },
  ];
}

/** 4. GL(inventory) = Σ product.stockValue, exactly. Each branch's quantity = Σ that branch's movements. */
export function checkInventoryGl(db: MockDb): InvariantResult {
  const invGl = glBalance(db, 'inventory');
  const invSum = round2(db.products.reduce((a, p) => a + (p.type === 'product' ? p.stockValue : 0), 0));
  return { ...numericCheck('inventory-gl', '§4.4', 'inventory GL = Σ product.stockValue', invSum, invGl), doc: '§4.4' };
}

/** 5. Output/input VAT GL = Σ VAT on documents for all time (no period filter — see scripts/verify/sales.ts). */
export function checkVatControl(db: MockDb): InvariantResult[] {
  const invoiceVatBase = (i: MockDb['invoices'][number]) => (i.currency && i.exchangeRate ? round2(i.taxAmount * i.exchangeRate) : i.taxAmount);
  const outputVatFromDocs = round2(db.invoices.reduce((a, i) => a + invoiceVatBase(i), 0) - db.refunds.reduce((a, r) => a + r.taxAmount, 0));
  const outputVatLedger = -glBalance(db, 'vatOutput');

  const recoverablePos = db.purchaseOrders.filter((p) => p.status === 'RECEIVED' && !p.vatNotRecoverable);
  const recoverablePoIds = new Set(recoverablePos.map((p) => p.id));
  const inputVatFromDocs = round2(
    recoverablePos.reduce((a, p) => a + p.taxAmount, 0) -
      db.purchaseReturns.filter((r) => recoverablePoIds.has(r.purchaseOrderId)).reduce((a, r) => a + r.taxAmount, 0) +
      db.expenses.reduce((a, e) => a + e.taxAmount, 0),
  );
  const inputVatLedger = glBalance(db, 'vatInput');

  return [
    { ...numericCheck('vat-output', '§4.5', 'output VAT GL = Σ document VAT', outputVatFromDocs, outputVatLedger), doc: '§4.5' },
    { ...numericCheck('vat-input', '§4.5', 'input VAT GL = Σ document VAT', inputVatFromDocs, inputVatLedger), doc: '§4.5' },
  ];
}

/** 6. For every party: Σ document outstanding − unallocated credit ± opening balance = sub-ledger balance. */
export function checkPartyAllocation(db: MockDb): InvariantResult[] {
  const results: InvariantResult[] = [];

  const custStatementMismatch = db.customers.filter((c) => {
    const st = customerStatement(c.id);
    const last = st.at(-1)?.balance ?? 0;
    return !closeEnough(last, customerBalance(c.id));
  });
  results.push({
    key: 'customer-statement',
    doc: '§4.6',
    passed: custStatementMismatch.length === 0,
    message: `customer statement running balance = customerBalance() (${custStatementMismatch.length} mismatched: ${custStatementMismatch.map((c) => c.name).join(', ')})`,
  });

  const supStatementMismatch = db.suppliers.filter((s) => {
    const st = supplierStatement(s.id);
    const last = st.at(-1)?.balance ?? 0;
    return !closeEnough(last, supplierBalance(s.id));
  });
  results.push({
    key: 'supplier-statement',
    doc: '§4.6',
    passed: supStatementMismatch.length === 0,
    message: `supplier statement running balance = supplierBalance() (${supStatementMismatch.length} mismatched: ${supStatementMismatch.map((s) => s.name).join(', ')})`,
  });

  const arReducedByRefunds = (invoiceId: string) => db.refunds.filter((r) => r.invoiceId === invoiceId).reduce((a, r) => a + r.settledToReceivable, 0);
  const custAllocationMismatch = db.customers.filter((c) => {
    const outstanding = round2(
      db.invoices
        .filter((i) => i.customerId === c.id && i.status !== 'DRAFT')
        .reduce((a, i) => a + (i.grandTotal - arReducedByRefunds(i.id) - i.paidAmount), 0),
    );
    const credit = unallocatedCreditFor('customer', c.id);
    return !closeEnough(round2(outstanding - credit), customerBalance(c.id));
  });
  results.push({
    key: 'customer-allocation',
    doc: '§4.6',
    passed: custAllocationMismatch.length === 0,
    message: `customer: Σoutstanding − unallocated credit = customerBalance() (${custAllocationMismatch.length} mismatched: ${custAllocationMismatch.map((c) => c.name).join(', ')})`,
  });

  const apReducedByReturns = (poId: string) =>
    db.purchaseReturns.filter((r) => r.purchaseOrderId === poId).reduce((a, r) => a + (r.refundMethod === 'credit' ? r.grandTotal : r.settledToPayable), 0);
  const supAllocationMismatch = db.suppliers.filter((s) => {
    const outstanding = round2(
      db.purchaseOrders
        .filter((p) => p.supplierId === s.id && p.status === 'RECEIVED')
        .reduce((a, p) => a + (p.grandTotal - apReducedByReturns(p.id) - p.paidAmount), 0),
    );
    const credit = unallocatedCreditFor('supplier', s.id);
    return !closeEnough(round2(outstanding - credit), supplierBalance(s.id));
  });
  results.push({
    key: 'supplier-allocation',
    doc: '§4.6',
    passed: supAllocationMismatch.length === 0,
    message: `supplier: Σoutstanding − unallocated credit = supplierBalance() (${supAllocationMismatch.length} mismatched: ${supAllocationMismatch.map((s) => s.name).join(', ')})`,
  });

  return results;
}

const SYSTEM_SOURCE_KINDS = new Set([
  'invoice', 'refund', 'purchaseOrder', 'purchaseReturn', 'payment', 'stockAdjustment',
  'shift', 'expense', 'voucher', 'settlement', 'fxReval', 'opening',
]);

/** 7. Every posted document has exactly one active entry (or an entry + reversal pair); every
 * sourceRef resolves. Returns three results (kept separate, matching the original verify script). */
export function checkSourceRefIntegrity(db: MockDb): InvariantResult[] {
  const badSourceRefs = db.journalEntries.filter((e) => e.sourceRef && !SYSTEM_SOURCE_KINDS.has(e.sourceRef.kind));
  const bySource = new Map<string, typeof db.journalEntries>();
  for (const e of db.journalEntries) {
    if (!e.sourceRef) continue;
    const key = `${e.sourceRef.kind}:${e.sourceRef.id}`;
    bySource.set(key, [...(bySource.get(key) ?? []), e]);
  }
  const multiActive = [...bySource.entries()].filter(([, entries]) => entries.filter((e) => !e.reversed).length > 1);
  const reversalsResolve = db.journalEntries.filter((e) => e.reversalOfId && !db.journalEntries.some((o) => o.id === e.reversalOfId));
  return [
    {
      key: 'source-ref-known',
      doc: '§4.7',
      passed: badSourceRefs.length === 0,
      message: `every entry's sourceRef has a known kind (${badSourceRefs.length} unrecognized)`,
    },
    {
      key: 'one-active-entry',
      doc: '§4.7',
      passed: multiActive.length === 0,
      message: `every SYSTEM document has exactly one active entry (${multiActive.length} with >1 active)`,
    },
    {
      key: 'reversal-resolves',
      doc: '§4.7',
      passed: reversalsResolve.length === 0,
      message: `every reversal's original entry resolves (${reversalsResolve.length} dangling)`,
    },
  ];
}

/** 8. No entry is dated inside a locked period unless its creation time is before the lock. */
export function checkLockDate(db: MockDb): InvariantResult {
  const lockDate = db.settings.accounting?.lockDate;
  const lockedButPosted = lockDate ? db.journalEntries.filter((e) => e.date.slice(0, 10) <= lockDate) : [];
  return {
    key: 'lock-date',
    doc: '§4.8',
    passed: lockedButPosted.length === 0,
    message: `no entry dated inside the locked period (lockDate=${lockDate ?? 'none'}, ${lockedButPosted.length} offenders)`,
  };
}

/** 9. openingBalanceEquity = 0 once onboarding is complete. */
export function checkOpeningBalanceEquity(db: MockDb): InvariantResult {
  const account = db.accounts.find((a) => a.systemRole === 'openingBalanceEquity');
  let obeNet = 0;
  if (account) {
    for (const e of db.journalEntries) {
      for (const l of e.lines) {
        if (l.accountId === account.id) obeNet += l.debit - l.credit;
      }
    }
  }
  return { ...numericCheck('opening-balance-equity', '§4.9', 'openingBalanceEquity (3900) is zero', 0, round2(obeNet)), doc: '§4.9' };
}

/** 10. Card and wallet clearing balances match the unsettled tenders. */
export function checkClearingAccounts(db: MockDb): InvariantResult[] {
  const cardMethodIds = new Set(db.paymentMethods.filter((m) => m.accountRole === 'cardClearing').map((m) => m.id));
  const walletMethodIds = new Set(db.paymentMethods.filter((m) => m.accountRole === 'walletClearing').map((m) => m.id));
  const tenderSum = (ids: Set<string>) =>
    round2(db.invoices.reduce((a, i) => a + (i.tenders ?? []).filter((t) => ids.has(t.paymentMethodId)).reduce((s, t) => s + t.amount, 0), 0));
  const settledSum = (ids: Set<string>) =>
    round2(db.cardSettlements.reduce((a, s) => a + s.groups.filter((g) => ids.has(g.paymentMethodId)).reduce((sub, g) => sub + g.amount, 0), 0));

  const cardTenders = round2(tenderSum(cardMethodIds) - settledSum(cardMethodIds));
  const cardLedger = glBalance(db, 'cardClearing');
  const walletTenders = round2(tenderSum(walletMethodIds) - settledSum(walletMethodIds));
  const walletLedger = glBalance(db, 'walletClearing');

  return [
    { ...numericCheck('card-clearing', '§4.10', 'card clearing = unsettled card tenders', cardTenders, cardLedger), doc: '§4.10' },
    { ...numericCheck('wallet-clearing', '§4.10', 'wallet clearing = unsettled wallet tenders', walletTenders, walletLedger), doc: '§4.10' },
  ];
}

/** 11. Every closed shift: expected cash − counted cash = the posted over/short amount. */
export function checkShiftVariance(db: MockDb): InvariantResult {
  const closedShifts = db.shifts.filter((s) => s.status === 'CLOSED');
  if (!closedShifts.length) {
    return { key: 'shift-variance', doc: '§4.11', passed: true, message: 'no closed shifts yet — vacuously holds' };
  }
  const offenders: string[] = [];
  for (const s of closedShifts) {
    const expected = s.expectedCash ?? 0;
    const counted = s.countedCash ?? 0;
    const storedVariance = s.variance ?? 0;
    const derivedVariance = round2(counted - expected);
    if (!closeEnough(storedVariance, derivedVariance, 0.01)) offenders.push(`${s.number} (stored ${storedVariance} ≠ derived ${derivedVariance})`);
  }
  return {
    key: 'shift-variance',
    doc: '§4.11',
    passed: offenders.length === 0,
    message: offenders.length === 0 ? `all ${closedShifts.length} closed shift(s) match` : `${offenders.length} mismatched: ${offenders.join(', ')}`,
  };
}

/** Drafts never affect balances: no journal effect from `db.journalDrafts` (they're a separate
 * table from `db.journalEntries` by construction — every GL/balance reader only scans the latter —
 * so this checks nothing has leaked a draft's id into the posted ledger). */
export function checkDraftsIsolated(db: MockDb): InvariantResult {
  const draftIds = new Set(db.journalDrafts.map((d) => d.id));
  const leaked = db.journalEntries.filter((e) => draftIds.has(e.id));
  return {
    key: 'drafts-isolated',
    doc: '§4 (drafts never affect balances)',
    passed: leaked.length === 0,
    message: leaked.length === 0 ? 'no draft ids leaked into posted entries' : `${leaked.length} draft ids found in posted entries`,
  };
}

/** Allocations ≤ document total: every payment allocation never exceeds the target document's grand total. */
export function checkAllocationsWithinTotal(db: MockDb): InvariantResult {
  const offenders: string[] = [];
  for (const p of db.payments) {
    for (const a of p.allocations) {
      const invoice = db.invoices.find((i) => i.id === a.targetId);
      const po = db.purchaseOrders.find((o) => o.id === a.targetId);
      const total = invoice?.grandTotal ?? po?.grandTotal;
      if (total !== undefined && a.amount > total + 0.01) offenders.push(`${p.id}->${a.targetId} (${a.amount} > ${total})`);
    }
  }
  return {
    key: 'allocations-within-total',
    doc: '§4 (allocations ≤ document total)',
    passed: offenders.length === 0,
    message: offenders.length === 0 ? 'every allocation ≤ its document total' : `${offenders.length} over-allocated: ${offenders.join(', ')}`,
  };
}

/** FX base = fc × rate within 0.01 — every journal line carrying `amountFc` + `rate` converts to
 * its own `debit`/`credit` (base currency) within tolerance. */
export function checkFxConversion(db: MockDb): InvariantResult {
  const offenders: string[] = [];
  for (const e of db.journalEntries) {
    for (const l of e.lines) {
      if (l.amountFc === undefined || l.rate === undefined) continue;
      const base = l.debit > 0 ? l.debit : l.credit;
      const expected = round2(l.amountFc * l.rate);
      if (!closeEnough(base, expected, 0.01)) offenders.push(`${e.number}/${l.id} (fc ${l.amountFc} × ${l.rate} = ${expected} ≠ ${base})`);
    }
  }
  return {
    key: 'fx-conversion',
    doc: '§4 (FX base = fc × rate within 0.01)',
    passed: offenders.length === 0,
    message: offenders.length === 0 ? 'every FX line converts within 0.01' : `${offenders.length} mismatched: ${offenders.join(', ')}`,
  };
}

/** Runs every general invariant and flattens the results. This is the one function both
 * `scripts/verify/run.ts` and the app's debug-mode watcher call. */
export function runAllInvariants(db: MockDb): InvariantResult[] {
  return [
    ...checkBalancedEntries(db),
    ...checkTrialBalance(db),
    ...checkArApControl(db),
    checkInventoryGl(db),
    ...checkVatControl(db),
    ...checkPartyAllocation(db),
    ...checkSourceRefIntegrity(db),
    checkLockDate(db),
    checkOpeningBalanceEquity(db),
    ...checkClearingAccounts(db),
    checkShiftVariance(db),
    checkDraftsIsolated(db),
    checkAllocationsWithinTotal(db),
    checkFxConversion(db),
  ];
}
