/**
 * Accounts / ledger invariants — docs/v2/02-accounting-review.md §4 items 1, 2, 7, 8, 9.
 *
 * 18.F2: the pure checks live in `src/mocks/backend/invariants.ts` now (shared with the running
 * app's debug-mode watcher) — this file just adapts them to the console `Result` shape and adds a
 * couple of extra descriptive `ok()` lines the original script printed.
 */
import { db } from '../../src/mocks/db';
import {
  checkBalancedEntries,
  checkLockDate,
  checkOpeningBalanceEquity,
  checkSourceRefIntegrity,
  checkTrialBalance,
} from '../../src/mocks/backend/invariants';
import { check, ok, type Result } from './shared';

function toResult(r: { passed: boolean; message: string }): Result {
  return check(r.passed, r.message);
}

export function run(): Result[] {
  const results: Result[] = [];

  results.push(...checkBalancedEntries(db).map(toResult));
  const totalDr = db.journalEntries.reduce((a, e) => a + e.totalDebit, 0);
  const totalCr = db.journalEntries.reduce((a, e) => a + e.totalCredit, 0);
  results.push(ok(`total Dr ${Math.round(totalDr * 100) / 100} / Cr ${Math.round(totalCr * 100) / 100} across ${db.journalEntries.length} entries`));

  results.push(...checkTrialBalance(db).map(toResult));
  results.push(...checkSourceRefIntegrity(db).map(toResult));
  results.push(toResult(checkLockDate(db)));
  results.push(toResult(checkOpeningBalanceEquity(db)));

  return results;
}
