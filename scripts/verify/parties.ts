/**
 * Party (customers/suppliers) invariants — docs/v2/02-accounting-review.md §4 items 3, 6.
 *
 * 18.F2: all four checks below now live in `src/mocks/backend/invariants.ts` (shared with the
 * app's debug-mode watcher) — this file adapts them to the console `Result` shape.
 */
import { db } from '../../src/mocks/db';
import { checkArApControl, checkPartyAllocation } from '../../src/mocks/backend/invariants';
import { check, ok, round2, type Result } from './shared';

export function run(): Result[] {
  const results: Result[] = [];

  for (const r of checkArApControl(db)) results.push(check(r.passed, r.message));
  for (const r of checkPartyAllocation(db)) results.push(check(r.passed, r.message));

  const totalAllocated = round2(db.payments.reduce((a, p) => a + p.allocations.reduce((b, al) => b + al.amount, 0), 0));
  const multiAllocation = db.payments.filter((p) => p.allocations.length > 1).length;
  results.push(ok(`${db.customers.length} customers, ${db.suppliers.length} suppliers, ${db.payments.length} payments (${multiAllocation} multi-document, Σallocated ${totalAllocated})`));

  return results;
}
