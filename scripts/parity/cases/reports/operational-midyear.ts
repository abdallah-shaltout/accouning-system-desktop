/**
 * L5 (03-domains/13b-reports-operational.md §8(b)): commands 1, 3–7, 11–14 over a range starting
 * mid-year — the third seeded-range variant the plan asks for (full history, current month,
 * mid-year start), same command set as `operational-full`/`operational-month`.
 */
import { defineCase } from '../../case';
import * as reportService from '../../../../src/modules/reports/services/reportService';

export default defineCase({
  name: 'reports/operational-midyear',
  source: '03-domains/13b-reports-operational.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  epsilon: ['steps.sales.value.byProduct[].qty', 'steps.sales.value.byCategory[].qty', 'steps.returns.value.byReason[].qty', 'steps.returns.value.byProduct[].qty', 'steps.returns.value.byCashier[].qty'],
  // See operational-full.ts: the FX worked example's free-text line id ('freetext-fx-demo') maps
  // to Rust's `freetext-<position>` convention (13b §8(b) id-mapping exception).
  allow: [{ path: 'steps.sales.value.byProduct[].productId', reason: '13b #8: freetext-fx-demo id-mapping exception' }],
  async run(s) {
    const range = { from: '2026-04-01', to: '2026-06-30' };
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
