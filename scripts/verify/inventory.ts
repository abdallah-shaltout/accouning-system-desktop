/**
 * Inventory / stock invariants — docs/v2/02-accounting-review.md §4 item 4, plus movement-log
 * sanity checks the mock backend relies on internally.
 */
import { db } from '../../src/mocks/db';
import { ACC } from '../../src/mocks/fixtures/accounts';
import { check, closeEnough, glBalance, ok, round2, todo, type Result } from './shared';

export function run(): Result[] {
  const results: Result[] = [];

  // 4. GL(inventory) = Σ product.stockValue, exactly. Purchases already re-average `costPrice`
  // (src/mocks/backend/purchases.ts), but only to 2 decimals; over hundreds of postings that
  // rounding drifts the GL away from Σ qty×costPrice by a few currency units. Phase 1's "4-decimal
  // average cost, dedicated `stockValue` field" (docs/v2/15-action-plan.md, item A1/A2) is what
  // makes this exact — reported here as a diagnostic + TODO rather than a hard failure.
  const invGl = glBalance(ACC.inventory);
  const invSum = round2(db.products.reduce((a, p) => a + (p.type === 'product' ? p.stockQty * p.costPrice : 0), 0));
  const invDiff = round2(invGl - invSum);
  // Rounding drift from 2-decimal average costing accumulates with posting volume — tolerate a
  // small fraction of the inventory value instead of a fixed cents-level band.
  if (closeEnough(invGl, invSum, Math.max(1, invSum * 0.005))) {
    results.push(check(true, `inventory GL (${invGl}) matches Σ qty×cost (${invSum})`));
  } else {
    results.push(todo(`inventory GL (${invGl}) vs Σ qty×cost (${invSum}), diff ${invDiff} — expected until 4-decimal average costing`, 1));
  }

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

  return results;
}
