/**
 * 06b-inventory §8(b) P-I6: stock count full cycle — scope (category / location / empty scope),
 * snapshot at start (A5), manual entry and `delta` scans, every status guard message, submit with an
 * uncounted line refused, review → resume → review, complete posts one STOCKTAKE adjustment and links
 * `adjustmentId`.
 */
import { defineCase } from '../../case';
import * as inventoryService from '../../../../src/modules/products/services/inventoryService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'inventory/p-i6-stock-count-cycle',
  source: '03-domains/06b-inventory.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    await s.expectError('empty-scope', () => inventoryService.createStockCount({ scope: 'location', location: 'رف غير موجود', blind: false }));
    const count = await s.step('start', () =>
      inventoryService.createStockCount({ scope: 'category', categoryId: s.baseId('cat-shoes'), blind: true, note: 'جرد الأحذية' }),
    );
    const [p19, p20, p21, p22] = ['prd-19', 'prd-20', 'prd-21', 'prd-22'].map((id) => s.baseId(id));
    await s.expectError('out-of-scope', () => inventoryService.updateStockCountLine(count.id, s.baseId('prd-1'), 3));
    await s.step('set-19', () => inventoryService.updateStockCountLine(count.id, p19, 11));
    await s.step('scan-20-a', () => inventoryService.updateStockCountLine(count.id, p20, 1, true));
    await s.step('scan-20-b', () => inventoryService.updateStockCountLine(count.id, p20, 1, true));
    await s.step('scan-20-c', () => inventoryService.updateStockCountLine(count.id, p20, 7.5, true));
    await s.expectError('submit-uncounted', () => inventoryService.submitCountForReview(count.id));
    await s.step('set-21', () => inventoryService.updateStockCountLine(count.id, p21, 10));
    await s.step('set-22', () => inventoryService.updateStockCountLine(count.id, p22, 30));
    await s.expectError('resume-while-open', () => inventoryService.resumeCounting(count.id));
    await s.expectError('complete-while-open', () => inventoryService.completeStockCount(count.id));
    await s.step('submit', () => inventoryService.submitCountForReview(count.id));
    await s.expectError('edit-in-review', () => inventoryService.updateStockCountLine(count.id, p19, 12));
    await s.expectError('submit-twice', () => inventoryService.submitCountForReview(count.id));
    await s.step('resume', () => inventoryService.resumeCounting(count.id));
    await s.step('fix-19', () => inventoryService.updateStockCountLine(count.id, p19, 12));
    await s.step('submit-again', () => inventoryService.submitCountForReview(count.id));
    const adj = await s.step('complete', () => inventoryService.completeStockCount(count.id));
    await s.step('count-after', () => inventoryService.getStockCount(count.id));
    await s.step('adjustment', () => inventoryService.getStockAdjustment(adj.id));
    await s.step('journal', () => accountingService.getJournalEntriesForSource('stockAdjustment', adj.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.expectError('complete-twice', () => inventoryService.completeStockCount(count.id));

    // A whole-store count, left open.
    await s.step('start-all', () => inventoryService.createStockCount({ scope: 'all', blind: false }));
    await s.step('counts', () => inventoryService.getStockCounts());
    await s.expectError('missing', () => inventoryService.getStockCount(s.baseId('cus-1')));
    await s.expectError('update-missing', () => inventoryService.updateStockCountLine(s.baseId('cus-1'), p19, 1));
  },
});
