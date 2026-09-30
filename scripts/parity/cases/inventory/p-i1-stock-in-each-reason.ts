/**
 * 06b-inventory §8(b) P-I1: STOCK_IN with each reason → its credit account (A3): owner_contribution →
 * ownerCurrent, gift → otherIncome, found → inventoryVariance, other → the chosen offset account,
 * opening → openingBalanceEquity (closed again inside the same step, see below); plus every S-2/S-3
 * validation message (reason, other without/with a control account, unknown account, bad qty,
 * duplicate line, untracked product, tracked product without a batch) and a batch receipt.
 *
 * Invariant 9 (02 §4.9: openingBalanceEquity = 0 after onboarding): the `opening` step also posts the
 * documented close-out (Dr openingBalanceEquity / Cr capital, 02 §3) so invariants stay green.
 */
import { defineCase } from '../../case';
import * as inventoryService from '../../../../src/modules/products/services/inventoryService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import { round2 } from '../../../../src/modules/core/helpers/numbers';
import type { StockAdjustmentInput } from '../../../../src/modules/products/types';

export default defineCase({
  name: 'inventory/p-i1-stock-in-each-reason',
  source: '03-domains/06b-inventory.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const prd = (id: string) => s.baseId(id);
    const stockIn = (reason: StockAdjustmentInput['reason'], productId: string, qty: number, extra: Partial<StockAdjustmentInput> = {}): StockAdjustmentInput => ({
      type: 'STOCK_IN',
      date,
      reason,
      note: `إدخال ${reason ?? ''}`,
      lines: [{ productId, qtyChange: qty }],
      ...extra,
    });
    const journalOf = (adjId: string) => accountingService.getJournalEntriesForSource('stockAdjustment', adjId).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id))));

    // Validation (S-3 before S-2).
    await s.expectError('no-reason', () => inventoryService.createStockAdjustment(stockIn(undefined, prd('prd-1'), 1)));
    await s.expectError('other-without-account', () => inventoryService.createStockAdjustment(stockIn('other', prd('prd-1'), 1)));
    await s.expectError('other-control-account', () =>
      inventoryService.createStockAdjustment(stockIn('other', prd('prd-1'), 1, { offsetAccountId: s.baseId('acc-1130') })),
    );
    await s.expectError('other-group-account', () =>
      inventoryService.createStockAdjustment(stockIn('other', prd('prd-1'), 1, { offsetAccountId: s.baseId('acc-11') })),
    );
    await s.expectError('other-unknown-account', () =>
      inventoryService.createStockAdjustment(stockIn('other', prd('prd-1'), 1, { offsetAccountId: s.baseId('cus-1') })),
    );
    await s.expectError('no-lines', () => inventoryService.createStockAdjustment({ type: 'STOCK_IN', date, reason: 'gift', lines: [] }));
    await s.expectError('zero-qty', () => inventoryService.createStockAdjustment(stockIn('gift', prd('prd-1'), 0)));
    await s.expectError('unknown-product', () => inventoryService.createStockAdjustment(stockIn('gift', s.baseId('cus-1'), 1)));
    await s.expectError('service-product', () => inventoryService.createStockAdjustment(stockIn('gift', prd('prd-svc-1'), 1)));
    await s.expectError('duplicate-line', () =>
      inventoryService.createStockAdjustment({
        type: 'STOCK_IN',
        date,
        reason: 'gift',
        lines: [
          { productId: prd('prd-1'), qtyChange: 1 },
          { productId: prd('prd-1'), qtyChange: 2 },
        ],
      }),
    );
    await s.expectError('tracked-without-batch', () => inventoryService.createStockAdjustment(stockIn('gift', prd('prd-panadol'), 5)));

    const owner = await s.step('owner-contribution', () => inventoryService.createStockAdjustment(stockIn('owner_contribution', prd('prd-1'), 3)));
    await s.step('owner-journal', () => journalOf(owner.id));
    const gift = await s.step('gift', () => inventoryService.createStockAdjustment(stockIn('gift', prd('prd-2'), 2)));
    await s.step('gift-journal', () => journalOf(gift.id));
    const found = await s.step('found', () => inventoryService.createStockAdjustment(stockIn('found', prd('prd-6'), 1)));
    await s.step('found-journal', () => journalOf(found.id));
    const other = await s.step('other', () =>
      inventoryService.createStockAdjustment(stockIn('other', prd('prd-7'), 1, { offsetAccountId: s.baseId('acc-1115') })),
    );
    await s.step('other-journal', () => journalOf(other.id));
    const opening = await s.step('opening-and-close', async () => {
      const adj = await inventoryService.createStockAdjustment(stockIn('opening', prd('prd-8'), 2));
      const value = round2(adj.lines.reduce((a, l) => a + round2(Math.abs(l.qtyChange) * (l.unitCost ?? 0)), 0)); // as posted (S-5)
      await accountingService.createJournalEntry({
        date,
        description: 'إقفال الأرصدة الافتتاحية',
        lines: [
          { accountId: s.baseId('acc-3900'), debit: value, credit: 0 },
          { accountId: s.baseId('acc-3100'), debit: 0, credit: value },
        ],
      });
      return adj;
    });
    await s.step('opening-journal', () => journalOf(opening.id));

    // Batch-tracked product: several batches of the same product on one receipt are allowed.
    const batches = await s.step('batch-receipt', () =>
      inventoryService.createStockAdjustment({
        type: 'STOCK_IN',
        date,
        reason: 'found',
        lines: [
          { productId: prd('prd-panadol'), qtyChange: 6, batchNo: ' PND-26X ', expiryDate: '2027-03-31' },
          { productId: prd('prd-panadol'), qtyChange: 4, batchNo: 'PND-26Y' },
        ],
      }),
    );
    await s.step('batch-detail', () => inventoryService.getStockAdjustment(batches.id));
    await s.step('panadol-batches', () => inventoryService.getBatches(prd('prd-panadol')));
    await s.step('list-stock-in', () => inventoryService.getStockAdjustments({ type: 'STOCK_IN', status: 'COMPLETED' }));
    await s.expectError('detail-missing', () => inventoryService.getStockAdjustment(s.baseId('cus-1')));
  },
});
