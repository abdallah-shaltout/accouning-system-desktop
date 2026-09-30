/**
 * 06b-inventory §8(b) P-I2: LOSS (write-off) — Dr inventoryWriteOff / Cr inventory at the average
 * cost; over-stock refused with the available qty in the message; on a batch-tracked product the
 * loss consumes batches FEFO, expired ones included.
 */
import { defineCase } from '../../case';
import * as inventoryService from '../../../../src/modules/products/services/inventoryService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'inventory/p-i2-loss-batch-fefo',
  source: '03-domains/06b-inventory.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: 'steps.**.prices', emptyArrayOnly: true, reason: '06-products Q-7: Product.prices is [] in Rust for a product that has none; the mock leaves it absent on seed rows never saved through the form ([] ≡ absent for this field)' },
  ],
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const panadol = s.baseId('prd-panadol');
    await s.expectError('loss-over-stock', () =>
      inventoryService.createStockAdjustment({ type: 'LOSS', date, lines: [{ productId: s.baseId('prd-6'), qtyChange: 4 }] }),
    );
    await s.expectError('loss-negative-qty', () =>
      inventoryService.createStockAdjustment({ type: 'LOSS', date, lines: [{ productId: s.baseId('prd-6'), qtyChange: -1 }] }),
    );
    const loss = await s.step('loss-two-lines', () =>
      inventoryService.createStockAdjustment({
        type: 'LOSS',
        date,
        note: 'تلف أثناء العرض',
        lines: [
          { productId: s.baseId('prd-5'), qtyChange: 1 },
          { productId: s.baseId('prd-9'), qtyChange: 2 },
        ],
      }),
    );
    await s.step('loss-journal', () => accountingService.getJournalEntriesForSource('stockAdjustment', loss.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('prd-5-after', () => productService.getProduct(s.baseId('prd-5')));

    await s.step('panadol-batches-before', () => inventoryService.getBatches(panadol));
    // 70 > the expired batch (60): FEFO takes all of it, then 10 from the next batch.
    const fefo = await s.step('loss-panadol-fefo', () =>
      inventoryService.createStockAdjustment({ type: 'LOSS', date, lines: [{ productId: panadol, qtyChange: 70 }] }),
    );
    await s.step('panadol-batches-after', () => inventoryService.getBatches(panadol));
    await s.step('panadol-after', () => productService.getProduct(panadol));
    await s.step('fefo-journal', () => accountingService.getJournalEntriesForSource('stockAdjustment', fefo.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('loss-list', () => inventoryService.getStockAdjustments({ type: 'LOSS' }));
  },
});
