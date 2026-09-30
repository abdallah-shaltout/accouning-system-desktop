/**
 * L5 (03-domains/13b-reports-operational.md §8(b)): commands 1, 3–7, 11–14 over the full seeded
 * history (no range bound) — sales, discounts, gross profit, returns, expenses, shifts, transfers,
 * purchases, branch comparison and profit leakage. Epsilon (13b §8(b)) applies only to
 * `SalesByProduct.qty` / `SalesByCategory.qty` / `ReturnsReportRow.qty` (all three groupings).
 */
import { defineCase } from '../../case';
import * as reportService from '../../../../src/modules/reports/services/reportService';

export default defineCase({
  name: 'reports/operational-full',
  source: '03-domains/13b-reports-operational.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  epsilon: ['steps.sales.value.byProduct[].qty', 'steps.sales.value.byCategory[].qty', 'steps.returns.value.byReason[].qty', 'steps.returns.value.byProduct[].qty', 'steps.returns.value.byCashier[].qty'],
  // The FX worked example's free-text line seeds a `productId` literally `'freetext-fx-demo'`
  // (`seed/branches9.ts`), not the mock's own `freetext-<position>` convention; Rust's importer
  // normalizes it to `freetext-<position>` (13b §8(b)) — a known, whitelisted id-mapping exception.
  allow: [{ path: 'steps.sales.value.byProduct[].productId', reason: '13b #8: freetext-fx-demo id-mapping exception' }],
  async run(s) {
    const range = {};
    await s.step('sales', () => reportService.getSalesReport(range));
    await s.step('discounts-cashier', () => reportService.getDiscountsReport(range, 'cashier'));
    await s.step('discounts-product', () => reportService.getDiscountsReport(range, 'product'));
    await s.step('gross-profit-invoice', () => reportService.getGrossProfitReport(range, 'invoice'));
    await s.step('gross-profit-product', () => reportService.getGrossProfitReport(range, 'product'));
    await s.step('gross-profit-category', () => reportService.getGrossProfitReport(range, 'category'));
    await s.step('returns', () => reportService.getReturnsReport(range));
    await s.step('expenses', () => reportService.getExpensesReport(range));
    await s.step('shifts', () => reportService.getShiftsReport(range));
    await s.step('transfers', () => reportService.getTransfersReport(range));
    await s.step('purchases', () => reportService.getPurchasesReport(range));
    await s.step('branch-comparison', () => reportService.getBranchComparison(range));
    await s.step('profit-leakage', () => reportService.getProfitLeakageReport(range));
  },
});
