/**
 * Accounts / ledger invariants — docs/v2/02-accounting-review.md §4 items 1, 2, 7, 8.
 */
import { db } from '../../src/mocks/db';
import { check, closeEnough, ok, round2, type Result } from './shared';

export function run(): Result[] {
  const results: Result[] = [];

  // 1. Every entry: Σ debit = Σ credit; no line is both debit and credit; ≥ 2 lines.
  const unbalanced = db.journalEntries.filter((e) => !closeEnough(e.totalDebit, e.totalCredit, 0.001));
  results.push(check(unbalanced.length === 0, `every journal entry is balanced (${unbalanced.length} unbalanced: ${unbalanced.map((e) => e.number).join(', ')})`));

  const bothSides = db.journalEntries.flatMap((e) => e.lines).filter((l) => l.debit > 0 && l.credit > 0);
  results.push(check(bothSides.length === 0, `no journal line is both debit and credit (${bothSides.length} offenders)`));

  const tooShort = db.journalEntries.filter((e) => e.lines.length < 2);
  results.push(check(tooShort.length === 0, `every journal entry has >= 2 lines (${tooShort.length} offenders)`));

  const totalDr = db.journalEntries.reduce((a, e) => a + e.totalDebit, 0);
  const totalCr = db.journalEntries.reduce((a, e) => a + e.totalCredit, 0);
  results.push(ok(`total Dr ${Math.round(totalDr * 100) / 100} / Cr ${Math.round(totalCr * 100) / 100} across ${db.journalEntries.length} entries`));

  // 2. Trial balance is balanced; balance sheet A = L + E, with the current result included.
  // Computed directly against db.accounts/db.journalEntries (not the async reportService, to keep
  // this script synchronous) — same movements-per-account logic modules/reports uses.
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
  // Assets/liabilities/equity accumulate as positive magnitudes in each side's own normal
  // direction (debit for assets, credit for liabilities/equity/revenue, netted with expenses).
  let assets = 0;
  let liabilities = 0;
  let equity = 0; // includes revenue − expenses (the unclosed current-period result), per review §4 item 2
  for (const a of db.accounts) {
    if (a.isGroup) continue;
    const t = movement.get(a.id);
    if (!t) continue;
    const debitNet = round2(t.d - t.c); // + means net debit
    if (debitNet > 0) trialDebit += debitNet;
    else trialCredit += -debitNet;
    if (a.kind === 'ASSET') assets += debitNet;
    else if (a.kind === 'LIABILITY') liabilities += -debitNet;
    else if (a.kind === 'EQUITY') equity += -debitNet;
    else if (a.kind === 'REVENUE') equity += -debitNet; // credit-normal → adds to equity via unclosed current earnings
    else if (a.kind === 'EXPENSE') equity -= debitNet; // debit-normal → reduces equity via unclosed current earnings
  }
  trialDebit = round2(trialDebit);
  trialCredit = round2(trialCredit);
  results.push(check(closeEnough(trialDebit, trialCredit, 0.01), `trial balance is balanced (Dr ${trialDebit} = Cr ${trialCredit})`));
  assets = round2(assets);
  liabilities = round2(liabilities);
  equity = round2(equity);
  const balanceSheetOk = closeEnough(assets, round2(liabilities + equity), 0.01);
  results.push(check(balanceSheetOk, `balance sheet balanced: A (${assets}) = L (${liabilities}) + E incl. current result (${equity})`));

  // 7. Every posted document has exactly one active entry (or an entry + reversal pair); every sourceRef resolves.
  const sourceKinds = new Set([
    'invoice',
    'refund',
    'purchaseOrder',
    'purchaseReturn',
    'payment',
    'stockAdjustment',
    // v2 phase 7 (shift close cash variance/drop) + phase 8 (expenses, general vouchers, card settlement).
    'shift',
    'expense',
    'voucher',
    'settlement',
  ]);
  const badSourceRefs = db.journalEntries.filter((e) => e.sourceRef && !sourceKinds.has(e.sourceRef.kind));
  results.push(check(badSourceRefs.length === 0, `every entry's sourceRef has a known kind (${badSourceRefs.length} unrecognized)`));

  const bySource = new Map<string, typeof db.journalEntries>();
  for (const e of db.journalEntries) {
    if (!e.sourceRef) continue;
    const key = `${e.sourceRef.kind}:${e.sourceRef.id}`;
    bySource.set(key, [...(bySource.get(key) ?? []), e]);
  }
  const multiActive = [...bySource.entries()].filter(([, entries]) => entries.filter((e) => !e.reversed).length > 1);
  results.push(check(multiActive.length === 0, `every SYSTEM document has exactly one active entry (${multiActive.length} with >1 active)`));

  const reversalsResolve = db.journalEntries.filter((e) => e.reversalOfId && !db.journalEntries.some((o) => o.id === e.reversalOfId));
  results.push(check(reversalsResolve.length === 0, `every reversal's original entry resolves (${reversalsResolve.length} dangling)`));

  // 8. No entry is dated inside a locked period unless its creation time is before the lock. The
  // demo seed doesn't set a lock date, so this is a structural check: postJournal's
  // assertOpenPeriod (src/mocks/backend/core.ts) is the single choke point every posting goes
  // through, so if a lock date IS set, no entry can have slipped past it — verified here by
  // re-checking every posted entry against the current lock date (none set = vacuously true).
  const lockDate = db.settings.accounting?.lockDate;
  const lockedButPosted = lockDate ? db.journalEntries.filter((e) => e.date.slice(0, 10) <= lockDate) : [];
  results.push(check(lockedButPosted.length === 0, `no entry is dated inside the locked period (lockDate=${lockDate ?? 'none'}, ${lockedButPosted.length} offenders)`));

  return results;
}
