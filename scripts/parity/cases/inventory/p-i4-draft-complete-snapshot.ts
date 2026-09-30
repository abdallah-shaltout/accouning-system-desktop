/**
 * 06b-inventory §8(b) P-I4: drafts post nothing; completing a STOCKTAKE draft applies
 * counted − snapshot (A5), so a sale made between counting and approving is not counted twice;
 * completing twice / deleting a completed adjustment are refused; deleting a draft works.
 */
import { defineCase } from '../../case';
import * as inventoryService from '../../../../src/modules/products/services/inventoryService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'inventory/p-i4-draft-complete-snapshot',
  source: '03-domains/06b-inventory.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    // The seed's adj-9 is a STOCKTAKE draft on the shoes (prd-19 snapshot 12, counted 11).
    const draftId = s.baseId('adj-9');
    await s.step('draft-before', () => inventoryService.getStockAdjustment(draftId));
    await s.step('drafts', () => inventoryService.getStockAdjustments({ status: 'DRAFT' }));
    // Sell one prd-19 between the count and its approval.
    await s.step('intervening-sale', () =>
      invoiceService.createSale({
        source: 'DESK',
        lines: [{ productId: s.baseId('prd-19'), qty: 1, price: 329 }],
        discountRate: 0,
        paymentMethod: 'cash',
        paidAmount: 329,
      }),
    );
    await s.step('prd-19-after-sale', () => productService.getProduct(s.baseId('prd-19')));
    await s.step('complete', () => inventoryService.completeAdjustment(draftId));
    await s.step('completed-detail', () => inventoryService.getStockAdjustment(draftId));
    await s.step('journal', () => accountingService.getJournalEntriesForSource('stockAdjustment', draftId).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('prd-19-after-complete', () => productService.getProduct(s.baseId('prd-19')));
    await s.expectError('complete-twice', () => inventoryService.completeAdjustment(draftId));
    await s.expectError('delete-completed', () => inventoryService.deleteDraftAdjustment(draftId));

    // A STOCK_IN draft: no stock, no journal; then deleted.
    const d2 = await s.step('stock-in-draft', () =>
      inventoryService.createStockAdjustment(
        { type: 'STOCK_IN', date: '2026-06-30T09:00:00.000Z', reason: 'gift', lines: [{ productId: s.baseId('prd-3'), qtyChange: 4 }] },
        true,
      ),
    );
    await s.step('stock-in-draft-journal', () => accountingService.getJournalEntriesForSource('stockAdjustment', d2.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('prd-3-unchanged', () => productService.getProduct(s.baseId('prd-3')));
    await s.step('delete-draft', () => inventoryService.deleteDraftAdjustment(d2.id));
    await s.expectError('deleted-detail', () => inventoryService.getStockAdjustment(d2.id));
    await s.expectError('delete-missing', () => inventoryService.deleteDraftAdjustment(d2.id));
    await s.expectError('complete-missing', () => inventoryService.completeAdjustment(d2.id));
    // A LOSS draft completed with the stock that is there now.
    const d3 = await s.step('loss-draft', () =>
      inventoryService.createStockAdjustment({ type: 'LOSS', date: '2026-06-30T09:00:00.000Z', lines: [{ productId: s.baseId('prd-4'), qtyChange: 2 }] }, true),
    );
    await s.step('loss-complete', () => inventoryService.completeAdjustment(d3.id));
    await s.step('all', () => inventoryService.getStockAdjustments());
  },
});
