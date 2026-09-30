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
import { convertLinesToBase } from './currency';
import { ApiError } from '../utils';

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
  // `+ 1e-9`: a difference of exactly one tolerance step (4849.52 − 4849.51) is 0.01000000000022 in
  // binary floating point; the Rust port compares exact decimals (`shared::invariants::close_enough`),
  // so without the epsilon the two copies disagree on the boundary.
  return Math.abs(a - b) <= tolerance + 1e-9;
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

type Role = Parameters<typeof accountFor>[0];

/**
 * ACC-0022: the role's account, or `undefined` when the chart has none (the `basic` template has no
 * card/wallet clearing account; an empty company has no chart yet). `runAllInvariants` must never
 * throw: a role with no account has had nothing posted to it, so its GL balance is 0 and each check
 * compares that 0 with its own documents — a pass when there are none, a real failure when documents
 * exist that the ledger can't carry.
 */
function roleAccount(role: Role): ReturnType<typeof accountFor> | undefined {
  try {
    return accountFor(role);
  } catch (e) {
    if (e instanceof ApiError && e.code === 'NOT_FOUND') return undefined;
    throw e;
  }
}

function glBalance(db: MockDb, role: Role, skipEntry?: (e: MockDb['journalEntries'][number]) => boolean): number {
  const id = roleAccount(role)?.id;
  if (!id) return 0;
  return round2(
    db.journalEntries
      .filter((e) => !skipEntry?.(e))
      .flatMap((e) => e.lines)
      .filter((l) => l.accountId === id)
      .reduce((a, l) => a + l.debit - l.credit, 0),
  );
}

/** A party's balance, or 0 when the chart has no receivable/payable account (nothing can be posted to it). */
function partyBalance(kind: 'customer' | 'supplier', id: string): number {
  if (!roleAccount(kind === 'customer' ? 'receivable' : 'payable')) return 0;
  return kind === 'customer' ? customerBalance(id) : supplierBalance(id);
}

function partyStatementEnd(kind: 'customer' | 'supplier', id: string): number {
  if (!roleAccount(kind === 'customer' ? 'receivable' : 'payable')) return 0;
  const st = kind === 'customer' ? customerStatement(id) : supplierStatement(id);
  return st.at(-1)?.balance ?? 0;
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
  const arSum = round2(db.customers.reduce((a, c) => a + partyBalance('customer', c.id), 0));
  const apGl = -glBalance(db, 'payable');
  const apSum = round2(db.suppliers.reduce((a, s) => a + partyBalance('supplier', s.id), 0));
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

/**
 * ACC-0020: a VAT settlement (`postVatSettlement`, journal.ts) closes the period's output/input VAT
 * into `vatPayable` — it is not document VAT, so the VAT control check leaves it (and a reversal of
 * one) out of the ledger side. The payment to the authority (`payVatSettlement`) posts Dr
 * `vatPayable` / Cr cash-bank and never touches either VAT account, so it needs no exclusion.
 * Ported as `vat_settlement_entry_ids` (`src-tauri/src/shared/invariants/ledger.rs`).
 */
function vatSettlementEntryIds(db: MockDb): Set<string> {
  const ids = new Set(db.journalEntries.filter((e) => e.type === 'VAT_SETTLEMENT').map((e) => e.id));
  for (const e of db.journalEntries) if (e.reversalOfId && ids.has(e.reversalOfId)) ids.add(e.id);
  return ids;
}

/** 5. Output/input VAT GL = Σ VAT on documents for all time (no period filter — see scripts/verify/sales.ts),
 * ledger side net of VAT settlements (ACC-0020). */
export function checkVatControl(db: MockDb): InvariantResult[] {
  const settlementIds = vatSettlementEntryIds(db);
  const isSettlement = (e: MockDb['journalEntries'][number]) => settlementIds.has(e.id);
  const invoiceVatBase = (i: MockDb['invoices'][number]) => (i.currency && i.exchangeRate ? round2(i.taxAmount * i.exchangeRate) : i.taxAmount);
  // ACC-0009: an FC invoice's refunds post their VAT in base at the invoice's rate, converted
  // cumulatively per invoice (`refundBaseSplit` in sales.ts) — so Σ refunded VAT in base is the VAT
  // share of `convertLinesToBase([Σ refunded net, Σ refunded VAT], rate)`, exactly.
  const refundVatBase = (invoiceId: string) => {
    const refunds = db.refunds.filter((r) => r.invoiceId === invoiceId);
    const vat = round2(refunds.reduce((a, r) => a + r.taxAmount, 0));
    const inv = db.invoices.find((i) => i.id === invoiceId);
    if (!inv?.currency || !inv.exchangeRate) return vat;
    return convertLinesToBase([round2(refunds.reduce((a, r) => a + r.subTotal, 0)), vat], inv.exchangeRate)[1];
  };
  const refundedInvoiceIds = [...new Set(db.refunds.map((r) => r.invoiceId))];
  const outputVatFromDocs = round2(db.invoices.reduce((a, i) => a + invoiceVatBase(i), 0) - refundedInvoiceIds.reduce((a, id) => a + refundVatBase(id), 0));
  const outputVatLedger = -glBalance(db, 'vatOutput', isSettlement);

  const recoverablePos = db.purchaseOrders.filter((p) => p.status === 'RECEIVED' && !p.vatNotRecoverable);
  const recoverablePoIds = new Set(recoverablePos.map((p) => p.id));
  const inputVatFromDocs = round2(
    recoverablePos.reduce((a, p) => a + p.taxAmount, 0) -
      db.purchaseReturns.filter((r) => recoverablePoIds.has(r.purchaseOrderId)).reduce((a, r) => a + r.taxAmount, 0) +
      db.expenses.reduce((a, e) => a + e.taxAmount, 0),
  );
  const inputVatLedger = glBalance(db, 'vatInput', isSettlement);

  return [
    { ...numericCheck('vat-output', '§4.5', 'output VAT GL = Σ document VAT', outputVatFromDocs, outputVatLedger), doc: '§4.5' },
    { ...numericCheck('vat-input', '§4.5', 'input VAT GL = Σ document VAT', inputVatFromDocs, inputVatLedger), doc: '§4.5' },
  ];
}

/** Entry sources whose party lines the allocation check already reads through their documents
 * (outstanding, refunds/returns settled to the party, payments' unallocated credit). */
const DOCUMENT_SOURCE_KINDS = new Set(['invoice', 'refund', 'payment', 'purchaseOrder', 'purchaseReturn']);

/** 6. For every party: Σ document outstanding − unallocated credit ± opening balance = sub-ledger balance. */
export function checkPartyAllocation(db: MockDb): InvariantResult[] {
  const results: InvariantResult[] = [];

  const custStatementMismatch = db.customers.filter((c) => !closeEnough(partyStatementEnd('customer', c.id), partyBalance('customer', c.id)));
  results.push({
    key: 'customer-statement',
    doc: '§4.6',
    passed: custStatementMismatch.length === 0,
    message: `customer statement running balance = customerBalance() (${custStatementMismatch.length} mismatched: ${custStatementMismatch.map((c) => c.name).join(', ')})`,
  });

  const supStatementMismatch = db.suppliers.filter((s) => !closeEnough(partyStatementEnd('supplier', s.id), partyBalance('supplier', s.id)));
  results.push({
    key: 'supplier-statement',
    doc: '§4.6',
    passed: supStatementMismatch.length === 0,
    message: `supplier statement running balance = supplierBalance() (${supStatementMismatch.length} mismatched: ${supStatementMismatch.map((s) => s.name).join(', ')})`,
  });

  // ACC-0021: party lines with no invoice/refund/purchase/return/payment behind them — an opening
  // balance (and its reversal), a credit expense on a supplier, a manual AR/AP line (B1 write-off or
  // reclassification), an FX revaluation — are part of the party's balance in full, less what a
  // payment allocated to an opening balance (`targetKind: 'opening'`, which also left the payment's
  // unallocated credit). Ported as `non_document_net` (`src-tauri/src/shared/invariants/parties.rs`).
  const nonDocumentNet = (kind: 'customer' | 'supplier', partyId: string): number => {
    const account = roleAccount(kind === 'customer' ? 'receivable' : 'payable');
    if (!account) return 0;
    let net = 0;
    for (const e of db.journalEntries) {
      if (e.sourceRef && DOCUMENT_SOURCE_KINDS.has(e.sourceRef.kind)) continue;
      for (const l of e.lines) {
        if (l.accountId !== account.id || l.partyKind !== kind || l.partyId !== partyId) continue;
        net += kind === 'customer' ? l.debit - l.credit : l.credit - l.debit;
      }
    }
    const openingAllocated = db.payments
      .filter((p) => p.targetType === kind && p.targetId === partyId)
      .reduce((a, p) => a + p.allocations.filter((al) => al.targetKind === 'opening').reduce((s, al) => s + al.amount, 0), 0);
    return round2(net - openingAllocated);
  };

  const arReducedByRefunds = (invoiceId: string) => db.refunds.filter((r) => r.invoiceId === invoiceId).reduce((a, r) => a + r.settledToReceivable, 0);
  // ACC-0009: an FC invoice's outstanding is in its own currency — compare it to the (base-currency)
  // ledger at the invoice's rate, the same conversion `openDocumentsFor` (payments.ts) shows.
  const outstandingBase = (i: MockDb['invoices'][number]) => {
    const fc = i.grandTotal - arReducedByRefunds(i.id) - i.paidAmount;
    return i.currency && i.exchangeRate ? round2(fc * i.exchangeRate) : fc;
  };
  const custAllocationMismatch = db.customers.filter((c) => {
    const outstanding = round2(
      db.invoices
        .filter((i) => i.customerId === c.id && i.status !== 'DRAFT')
        .reduce((a, i) => a + outstandingBase(i), 0),
    );
    const credit = unallocatedCreditFor('customer', c.id);
    return !closeEnough(round2(outstanding + nonDocumentNet('customer', c.id) - credit), partyBalance('customer', c.id));
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
    return !closeEnough(round2(outstanding + nonDocumentNet('supplier', s.id) - credit), partyBalance('supplier', s.id));
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

/**
 * ACC-0015: a payment's "allocate later" realized-FX entry (09 §3.3, `allocationFxLines` in
 * payments.ts) is a designed second entry on the same payment, not a second posting of it: it moves
 * no money — only the party's control account and fxGain/fxLoss. It doesn't count toward
 * `one-active-entry`; the payment's own entry (which always hits a cash/bank account) must still be
 * unique. Ported as `is_allocation_fx_entry` (`src-tauri/src/shared/invariants/documents.rs`).
 */
const ALLOCATION_FX_ROLES = new Set(['receivable', 'payable', 'fxGain', 'fxLoss']);
function isAllocationFxEntry(db: MockDb, e: MockDb['journalEntries'][number]): boolean {
  if (e.sourceRef?.kind !== 'payment') return false;
  const roles = e.lines.map((l) => db.accounts.find((a) => a.id === l.accountId)?.systemRole);
  return roles.some((r) => r === 'fxGain' || r === 'fxLoss') && roles.every((r) => r !== undefined && ALLOCATION_FX_ROLES.has(r));
}

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
  const multiActive = [...bySource.entries()].filter(([, entries]) => entries.filter((e) => !e.reversed && !isAllocationFxEntry(db, e)).length > 1);
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

/**
 * ACC-0019: when each lock date took effect. `saveLockDate` (accountingService.ts; Rust
 * `save_lock_date`) records every change in the activity feed as `تحديد تاريخ القفل {date}` /
 * `إزالة تاريخ القفل` (kind `settings`) — the only record of when a lock was set. Oldest first.
 */
const LOCK_SET_MESSAGE = /^تحديد تاريخ القفل (\d{4}-\d{2}-\d{2})$/;
const LOCK_CLEARED_MESSAGE = 'إزالة تاريخ القفل';
function lockHistory(db: MockDb): { at: number; lockDate: string | undefined }[] {
  const out: { at: number; lockDate: string | undefined }[] = [];
  for (const a of db.activity) {
    if (a.kind !== 'settings') continue;
    const set = LOCK_SET_MESSAGE.exec(a.message);
    if (set) out.push({ at: Date.parse(a.date), lockDate: set[1] });
    else if (a.message === LOCK_CLEARED_MESSAGE) out.push({ at: Date.parse(a.date), lockDate: undefined });
  }
  return out.sort((x, y) => x.at - y.at);
}

/** Postings the period guard lets through on purpose (docs/v2/02 B2 "only `accounting.postToClosedPeriod`
 * can override"): opening and year-close entries and the FX-revaluation auto-reversal always pass
 * `allowClosedPeriod`; a manual entry (post, reversal, draft post) or a VAT settlement passes it when
 * the user who posted it is an admin. */
function lockOverrideAllowed(db: MockDb, e: MockDb['journalEntries'][number]): boolean {
  if (e.type === 'OPENING' || e.type === 'CLOSING' || e.sourceRef?.kind === 'fxReval') return true;
  if (e.type !== 'MANUAL' && e.type !== 'VAT_SETTLEMENT') return false;
  return db.users.find((u) => u.id === (e.postedBy ?? e.createdBy))?.role === 'admin';
}

/** 8. No entry is dated inside a locked period unless its creation time is before the lock (ACC-0019):
 * an offender is an entry posted while a lock covering its date was already in force — the lock in
 * force at posting time comes from the lock history, not today's lock date, so setting or moving a
 * lock over existing history is never an offence. A lock change recorded at the very same instant as
 * the posting could have come before or after it, so the entry counts only if every possible lock
 * state at that instant covers it. With no recorded history the lock's start is unknown and nothing
 * is flagged. */
export function checkLockDate(db: MockDb): InvariantResult {
  const lockDate = db.settings.accounting?.lockDate;
  const history = lockHistory(db);
  const lockedButPosted = history.length
    ? db.journalEntries.filter((e) => {
        if (lockOverrideAllowed(db, e)) return false;
        const at = Date.parse(e.postedAt ?? e.createdAt);
        const before = history.filter((h) => h.at < at).at(-1)?.lockDate;
        const candidates = [before, ...history.filter((h) => h.at === at).map((h) => h.lockDate)];
        const day = e.date.slice(0, 10);
        return candidates.every((c) => c !== undefined && day <= c);
      })
    : [];
  return {
    key: 'lock-date',
    doc: '§4.8',
    passed: lockedButPosted.length === 0,
    message: `no entry posted into a locked period after the lock (lockDate=${lockDate ?? 'none'}, ${lockedButPosted.length} offenders)`,
  };
}

/** 9. openingBalanceEquity = 0 once onboarding is complete (ACC-0023). While the setup wizard is
 * still running (`settings.onboarding` present without `finishedAt`) opening entries legitimately
 * sit in 3900 until the wizard re-closes it, so the check isn't enforced yet. A company with no
 * onboarding record never ran the wizard (the demo seed, an import) and is treated as complete. */
export function checkOpeningBalanceEquity(db: MockDb): InvariantResult {
  const onboarding = db.settings.onboarding;
  if (onboarding && !onboarding.finishedAt) {
    return { key: 'opening-balance-equity', doc: '§4.9', passed: true, message: 'onboarding in progress — openingBalanceEquity (3900) not enforced yet' };
  }
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

/** Allocations ≤ document total: every payment allocation never exceeds the target document's grand
 * total. ACC-0018: compared in the DOCUMENT's currency — a document's `grandTotal` is in its own
 * currency, an allocation's `amount` is the base amount posted to AR/AP at the document's rate and
 * `amountFc` is what it settles of the document, the same figure `applyAllocationToDocument`
 * (payments.ts) adds to `paidAmount` (`amountFc ?? amount`). */
export function checkAllocationsWithinTotal(db: MockDb): InvariantResult {
  const offenders: string[] = [];
  for (const p of db.payments) {
    for (const a of p.allocations) {
      if (a.targetKind === 'opening') continue;
      const invoice = a.targetKind === 'invoice' ? db.invoices.find((i) => i.id === a.targetId) : undefined;
      const po = a.targetKind === 'purchaseOrder' ? db.purchaseOrders.find((o) => o.id === a.targetId) : undefined;
      const total = invoice?.grandTotal ?? po?.grandTotal;
      const settled = a.amountFc ?? a.amount;
      if (total !== undefined && settled > total + 0.01) offenders.push(`${p.id}->${a.targetId} (${settled} > ${total})`);
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
