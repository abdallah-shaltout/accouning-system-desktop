/**
 * L5 (03-domains/14b-insights.md §8(b)): `dashboard_get_product_inline_hints` for three products —
 * one low-stock, one below-cost and one dead-stock — found dynamically from the seeded demo-sa
 * catalog through the existing reports (`getLowStockReport`, `getInventoryReport`,
 * `getDeadStockReport`) rather than a hardcoded id, since the fixture's stock/price mix can shift.
 * Falls back to the first catalog product (`prd-1`) for any category the seed doesn't happen to
 * produce, so the case still calls the command instead of skipping it (a `[]` result is itself a
 * valid parity point: "not applicable" must diff identically on both backends).
 */
import { defineCase } from '../../case';
import * as insightEngine from '../../../../src/modules/core/services/insightEngine';
import { mirrorsSettled } from '../../../../src/modules/core/services/backendMirror';
import * as reportService from '../../../../src/modules/reports/services/reportService';

export default defineCase({
  name: 'dashboard/product-hints',
  source: '03-domains/14b-insights.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    insightEngine.forceRefresh();
    const lowStock = await s.step('find-low-stock', () => reportService.getLowStockReport());
    const inventory = await s.step('find-inventory', () => reportService.getInventoryReport());
    const deadStock = await s.step('find-dead-stock', () => reportService.getDeadStockReport());

    const fallback = s.baseId('prd-1');
    const belowCostRow = inventory.find((r) => r.price > 0 && r.price <= r.costPrice);

    const lowStockProduct = lowStock[0]?.productId ?? fallback;
    const belowCostProduct = belowCostRow?.productId ?? fallback;
    const deadStockProduct = deadStock[0]?.productId ?? fallback;

    // `getProductInlineHints` is synchronous over a `backendMirror` (G-37) on Rust: the first read
    // starts the IPC load and returns the fallback `[]`, so read, wait for the load, and read again.
    const hints = async (role: 'storekeeper' | 'manager' | 'admin', productId: string) => {
      insightEngine.getProductInlineHints(role, productId);
      await mirrorsSettled();
      return insightEngine.getProductInlineHints(role, productId);
    };
    await s.step('hints-low-stock', () => hints('storekeeper', lowStockProduct));
    await s.step('hints-below-cost', () => hints('manager', belowCostProduct));
    await s.step('hints-dead-stock', () => hints('manager', deadStockProduct));
    // A real id of another entity (a cost center) stands in for "no such product": Rust ids are UUIDs.
    await s.step('hints-missing-product', () => hints('admin', s.baseId('cc-main')));
  },
});
