/**
 * Inventory / stock invariants — docs/v2/02-accounting-review.md §4 item 4, plus movement-log
 * sanity checks the mock backend relies on internally.
 *
 * 18.F2: the GL(inventory) = Σ stockValue check is shared with `src/mocks/backend/invariants.ts`;
 * the movement-log/negative-stock/batch checks below are mock-DB structural sanity, not one of
 * the 11 numbered invariants, so they stay here.
 */
import { db } from '../../src/mocks/db';
import { checkInventoryGl } from '../../src/mocks/backend/invariants';
import { check, closeEnough, ok, round2, type Result } from './shared';

export function run(): Result[] {
  const results: Result[] = [];

  const invGlResult = checkInventoryGl(db);
  results.push(check(invGlResult.passed, invGlResult.message));

  const negative = db.products.filter((p) => p.stockQty < 0);
  results.push(check(negative.length === 0, `no product has negative stock (${negative.length}: ${negative.map((p) => p.name).join(', ')})`));

  // Each product's stock movement log is chronological and its running balance matches stockQty.
  let mismatched: string[] = [];
  for (const p of db.products) {
    const mv = db.stockMovements.filter((m) => m.productId === p.id);
    let running = 0;
    let good = true;
    for (const m of mv) {
      running = round2(running + m.qtyChange);
      if (!closeEnough(running, m.balanceAfter ?? 0, 0.001)) good = false;
    }
    if (!good || !closeEnough(running, p.stockQty, 0.001)) mismatched.push(p.name);
  }
  results.push(check(mismatched.length === 0, `every product's movement running balance matches stockQty (${mismatched.length} mismatched: ${mismatched.join(', ')})`));

  const unsorted = db.stockMovements.some((m, i, a) => i > 0 && a[i - 1].date > m.date);
  results.push(check(!unsorted, 'stock movements are in chronological order'));

  const lowStock = db.products.filter((p) => p.type === 'product' && p.stockQty <= (p.minStock ?? 0)).length;
  results.push(ok(`${db.products.length} products, ${lowStock} at/under min stock, ${db.stockMovements.length} movements, ${db.stockAdjustments.length} adjustments`));

  // v2 phase 6 §3: no batch ever goes negative (FEFO consumption in `consumeFefo`/write-off must
  // never take more than a batch holds).
  const negativeBatches = db.productBatches.filter((b) => b.qty < -0.001);
  results.push(check(negativeBatches.length === 0, `no batch has negative qty (${negativeBatches.length})`));
  results.push(ok(`${db.productBatches.length} batches on ${new Set(db.productBatches.map((b) => b.productId)).size} tracked product(s)`));

  return results;
}
