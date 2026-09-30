/**
 * L5 (03-domains/13b-reports-operational.md §8(b)): commands 2, 8–10, 15, 16 — the reports that
 * take no date range (inventory, low stock, dead stock, stocktake variances, aging, overdue), run
 * once each. Epsilon (13b §8(b)) applies to `LowStockRow.suggestedQty`.
 */
import { defineCase } from '../../case';
import * as reportService from '../../../../src/modules/reports/services/reportService';

export default defineCase({
  name: 'reports/operational-once',
  source: '03-domains/13b-reports-operational.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  epsilon: ['steps.low-stock.value[].suggestedQty'],
  async run(s) {
    await s.step('inventory', () => reportService.getInventoryReport());
    await s.step('low-stock', () => reportService.getLowStockReport());
    await s.step('dead-stock-default', () => reportService.getDeadStockReport());
    await s.step('dead-stock-30', () => reportService.getDeadStockReport(30));
    // Decision R-8 (13b-reports-operational.md): a negative `days` is `VALIDATION "عدد الأيام غير صالح"`.
    // Rust validates it at the IPC boundary; the mock branch of `getDeadStockReport` does the same
    // (aligned in Part 04 Wave 2), so both sides record the same expected error.
    await s.expectError('dead-stock-negative', () => reportService.getDeadStockReport(-1));
    await s.step('stocktake-variances', () => reportService.getStocktakeVariances());
    await s.step('aging-customer', () => reportService.getAgingReport('customer'));
    await s.step('aging-supplier', () => reportService.getAgingReport('supplier'));
    await s.step('overdue-customer', () => reportService.getOverdueReport('customer'));
    await s.step('overdue-supplier', () => reportService.getOverdueReport('supplier'));
  },
});
