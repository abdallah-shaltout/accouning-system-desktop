/**
 * Sales / purchases / cash invariants — docs/v2/02-accounting-review.md §4 items 5, 10, 11, plus
 * general document-count and cash/bank/VAT diagnostics.
 *
 * 18.F2: the VAT, clearing-account and shift-variance checks now live in
 * `src/mocks/backend/invariants.ts` (shared with the app's debug-mode watcher).
 */
import { db } from '../../src/mocks/db';
import { checkClearingAccounts, checkShiftVariance, checkVatControl } from '../../src/mocks/backend/invariants';
import { check, glBalance, ok, todo, type Result } from './shared';

export function run(): Result[] {
  const results: Result[] = [];

  for (const r of checkVatControl(db)) results.push(check(r.passed, r.message));
  for (const r of checkClearingAccounts(db)) results.push(check(r.passed, r.message));

  const todayKey = new Date().toDateString();
  const todayInvoices = db.invoices.filter((i) => new Date(i.date).toDateString() === todayKey).length;
  const statusCounts = Object.entries(
    db.invoices.reduce((a: Record<string, number>, i) => {
      const key = `${i.status}/${i.paymentStatus}`;
      a[key] = (a[key] ?? 0) + 1;
      return a;
    }, {}),
  );
  results.push(
    ok(
      `${db.invoices.length} invoices (${todayInvoices} today), ${db.refunds.length} refunds, ${db.purchaseOrders.length} POs — statuses: ${statusCounts.map(([k, v]) => `${k}=${v}`).join(', ')}`,
    ),
  );

  const poStatuses = Object.entries(
    db.purchaseOrders.reduce((a: Record<string, number>, p) => {
      a[p.status] = (a[p.status] ?? 0) + 1;
      return a;
    }, {}),
  );
  results.push(ok(`PO statuses: ${poStatuses.map(([k, v]) => `${k}=${v}`).join(', ') || 'none'}`));

  const cash = glBalance('cash');
  const bank = glBalance('bank');
  const vatOut = -glBalance('vatOutput');
  const vatIn = glBalance('vatInput');
  results.push(ok(`cash ${cash}, bank ${bank}, VAT out ${vatOut}, VAT in ${vatIn}`));

  // 11: `checkShiftVariance` reports "vacuously holds" (passed: true) when there are no closed
  // shifts, but the original verify script printed that case as `todo` (nothing to check yet),
  // not `ok` — preserved here so the ok/todo/failed counts match exactly.
  if (!db.shifts.some((s) => s.status === 'CLOSED')) {
    results.push(todo('invariant 11 (shift variance): no closed shifts yet — nothing to check', 7));
  } else {
    const shiftResult = checkShiftVariance(db);
    results.push(check(shiftResult.passed, shiftResult.message));
  }

  return results;
}
