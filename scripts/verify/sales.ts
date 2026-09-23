/**
 * Sales / purchases / cash invariants — docs/v2/02-accounting-review.md §4 item 5, plus general
 * document-count and cash/bank/VAT diagnostics.
 */
import { db } from '../../src/mocks/db';
import { glBalance, ok, todo, type Result } from './shared';

export function run(): Result[] {
  const results: Result[] = [];

  // 5. Output VAT GL for a period = Σ VAT on invoice lines − Σ VAT on credit-note lines; input VAT
  // the same for purchases/expenses/debit notes. The VAT report (period boxes) is Phase 3.
  results.push(todo('output/input VAT GL reconciles with a period VAT report', 3));

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

  return results;
}
