/**
 * 06b-inventory §8(b) P-I9: the expiry report (UTC "today", Q-I11/D-I8; buckets expired / ≤30 /
 * ≤60 / ≤90, `ok` dropped; sorted by days left) and writing off selected batches as one LOSS per
 * product that consumes FEFO (Q-I3), plus the empty-selection and nothing-left messages.
 */
import { defineCase } from '../../case';
import * as inventoryService from '../../../../src/modules/products/services/inventoryService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'inventory/p-i9-expiry-write-off',
  source: '03-domains/06b-inventory.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: 'steps.**.prices', emptyArrayOnly: true, reason: '06-products Q-7: Product.prices is [] in Rust for a product that has none; the mock leaves it absent on seed rows never saved through the form ([] ≡ absent for this field)' },
  ],
  async run(s) {
    const panadol = s.baseId('prd-panadol');
    // Add batches that land in each bucket relative to 2026-06-30.
    await s.step('receive-batches', () =>
      inventoryService.createStockAdjustment({
        type: 'STOCK_IN',
        date: '2026-06-30T09:00:00.000Z',
        reason: 'found',
        lines: [
          { productId: panadol, qtyChange: 5, batchNo: 'PND-D20', expiryDate: '2026-07-20' },
          { productId: panadol, qtyChange: 4, batchNo: 'PND-D50', expiryDate: '2026-08-19' },
          { productId: panadol, qtyChange: 3, batchNo: 'PND-D80', expiryDate: '2026-09-18' },
          { productId: panadol, qtyChange: 2, batchNo: 'PND-NODATE' },
        ],
      }),
    );
    await s.step('batches', () => inventoryService.getBatches(panadol));
    const report = await s.step('expiry-report', () => inventoryService.getExpiryReport());
    await s.expectError('write-off-none', () => inventoryService.writeOffExpiredBatches([]));
    const expired = report.filter((r) => r.bucket === 'expired').map((r) => r.id);
    const adj = await s.step('write-off-expired', () => inventoryService.writeOffExpiredBatches(expired));
    await s.step('write-off-journal', () => accountingService.getJournalEntriesForSource('stockAdjustment', adj.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.expectError('write-off-again', () => inventoryService.writeOffExpiredBatches(expired));
    await s.step('write-off-with-note', () => inventoryService.writeOffExpiredBatches([report.find((r) => r.bucket === 'within30')!.id], 'تالف بالتخزين'));
    await s.step('report-after', () => inventoryService.getExpiryReport());
    await s.step('batches-after', () => inventoryService.getBatches(panadol));
    await s.step('panadol-after', () => productService.getProduct(panadol));
  },
});
