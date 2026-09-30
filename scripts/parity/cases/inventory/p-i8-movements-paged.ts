/**
 * 06b-inventory §8(b) P-I8: the stock-movement ledger — filters (product, reason, date range),
 * newest-first order, `refLink` per reason, and the paged variant: default order, numeric sorts
 * (exact order), string sorts compared as multisets (Q-I10: `utf8mb4_unicode_ci` vs
 * `localeCompare('ar')`), page 2, totals.
 */
import { defineCase } from '../../case';
import * as inventoryService from '../../../../src/modules/products/services/inventoryService';

export default defineCase({
  name: 'inventory/p-i8-movements-paged',
  source: '03-domains/06b-inventory.md §8(b)',
  base: 'demo-sa',
  unordered: ['steps.paged-sort-reason.value.rows', 'steps.paged-sort-product-name.value.rows'],
  async run(s) {
    const prd3 = s.baseId('prd-3');
    await s.step('by-product', () => inventoryService.getStockMovements({ productId: prd3 }));
    await s.step('by-reason-range', () => inventoryService.getStockMovements({ reason: 'purchase', from: '2026-06-01', to: '2026-06-30' }));
    await s.step('refunds', () => inventoryService.getStockMovements({ reason: 'refund' }));
    await s.step('purchase-returns', () => inventoryService.getStockMovements({ reason: 'purchase_return' }));
    await s.step('transfers', () => inventoryService.getStockMovements({ reason: 'transfer_in' }));
    await s.step('empty-range', () => inventoryService.getStockMovements({ from: '2025-01-01', to: '2025-01-31' }));

    await s.step('paged-default-p1', () => inventoryService.getStockMovementsPaged({ page: 1, pageSize: 15 }));
    await s.step('paged-default-p2', () => inventoryService.getStockMovementsPaged({ page: 2, pageSize: 15 }));
    const productFilter = { productId: prd3 };
    await s.step('paged-qty-asc-p1', () =>
      inventoryService.getStockMovementsPaged({ page: 1, pageSize: 5, sort: { key: 'qtyChange', dir: 'asc' }, filters: productFilter }),
    );
    await s.step('paged-qty-asc-p2', () =>
      inventoryService.getStockMovementsPaged({ page: 2, pageSize: 5, sort: { key: 'qtyChange', dir: 'asc' }, filters: productFilter }),
    );
    await s.step('paged-value-desc', () =>
      inventoryService.getStockMovementsPaged({ page: 1, pageSize: 10, sort: { key: 'valueChange', dir: 'desc' }, filters: productFilter }),
    );
    await s.step('paged-balance-asc', () =>
      inventoryService.getStockMovementsPaged({ page: 1, pageSize: 50, sort: { key: 'balanceAfter', dir: 'asc' }, filters: productFilter }),
    );
    // String sorts: whole filtered set on one page, rows compared as a multiset (Q-I10).
    await s.step('paged-sort-reason', () =>
      inventoryService.getStockMovementsPaged({ page: 1, pageSize: 500, sort: { key: 'reason', dir: 'asc' }, filters: productFilter }),
    );
    await s.step('paged-sort-product-name', () =>
      inventoryService.getStockMovementsPaged({ page: 1, pageSize: 500, sort: { key: 'productName', dir: 'desc' }, filters: { reason: 'stocktake' } }),
    );
    await s.step('paged-past-end', () => inventoryService.getStockMovementsPaged({ page: 99, pageSize: 50, filters: productFilter }));
  },
});
