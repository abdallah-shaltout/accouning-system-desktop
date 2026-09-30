/**
 * 06b-inventory §8(b) P-I3: a direct STOCKTAKE with gains and losses — one journal with both pairs
 * (Dr inventory / Cr inventoryVariance, then Dr inventoryVariance / Cr inventory), each line valued
 * at round2(|Δ| × average cost) and the totals round2'd again (analysis §7 row 247); counted qty
 * missing/negative refused; STOCKTAKE is never checked against the approval threshold.
 */
import { defineCase } from '../../case';
import * as inventoryService from '../../../../src/modules/products/services/inventoryService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as settingsService from '../../../../src/modules/settings/services/settingsService';

export default defineCase({
  name: 'inventory/p-i3-stocktake-rounding',
  source: '03-domains/06b-inventory.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    await s.expectError('counted-missing', () =>
      inventoryService.createStockAdjustment({ type: 'STOCKTAKE', date, lines: [{ productId: s.baseId('prd-1') }] }),
    );
    await s.expectError('counted-negative', () =>
      inventoryService.createStockAdjustment({ type: 'STOCKTAKE', date, lines: [{ productId: s.baseId('prd-1'), countedQty: -1 }] }),
    );
    // A threshold far below the stocktake's value — STOCKTAKE is exempt.
    await s.step('set-threshold', () => settingsService.updateSettings({ inventoryApprovalThreshold: 10 }));
    const adj = await s.step('stocktake', () =>
      inventoryService.createStockAdjustment({
        type: 'STOCKTAKE',
        date,
        note: 'جرد مفاجئ',
        lines: [
          { productId: s.baseId('prd-1'), countedQty: 53 }, // +1 × 52.6327
          { productId: s.baseId('prd-2'), countedQty: 45 }, // −1 × 72.0496
          { productId: s.baseId('prd-5'), countedQty: 26 }, // +1 × 69.9464
          { productId: s.baseId('prd-10'), countedQty: 9.5 }, // −2.5 × 56.405
          { productId: s.baseId('prd-11'), countedQty: 12 }, // no change
        ],
      }),
    );
    await s.step('detail', () => inventoryService.getStockAdjustment(adj.id));
    await s.step('journal', () => accountingService.getJournalEntriesForSource('stockAdjustment', adj.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('product-1', () => productService.getProduct(s.baseId('prd-1')));
    await s.step('product-10', () => productService.getProduct(s.baseId('prd-10')));
    await s.step('movements', () => inventoryService.getStockMovements({ reason: 'stocktake', from: '2026-06-30', to: '2026-06-30' }));
  },
});
