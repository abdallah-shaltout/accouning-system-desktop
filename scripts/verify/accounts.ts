/**
 * Accounts / ledger invariants — docs/v2/02-accounting-review.md §4 items 1, 2, 7, 8.
 */
import { db } from '../../src/mocks/db';
import { check, closeEnough, ok, todo, type Result } from './shared';

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
  // The trial-balance/balance-sheet report helpers land in Phase 1/2 (accountingService reports).
  results.push(todo('trial balance balanced & balance sheet A = L + E (dedicated report helper)', 1));

  // 7. Every posted document has exactly one active entry (or an entry + reversal pair); every sourceRef resolves.
  const sourceKinds = new Set(['invoice', 'refund', 'purchaseOrder', 'purchaseReturn', 'payment', 'stockAdjustment']);
  const badSourceRefs = db.journalEntries.filter((e) => e.sourceRef && !sourceKinds.has(e.sourceRef.kind));
  results.push(check(badSourceRefs.length === 0, `every entry's sourceRef has a known kind (${badSourceRefs.length} unrecognized)`));
  // Reversal pairs aren't modeled until Phase 2 (journal reversal dialog).
  results.push(todo('exactly one active entry per document, or an entry+reversal pair', 2));

  // 8. No entry is dated inside a locked period unless its creation time is before the lock.
  // Fiscal-year lock dates land in Phase 2.
  results.push(todo('no entry dated inside a locked period', 2));

  return results;
}
