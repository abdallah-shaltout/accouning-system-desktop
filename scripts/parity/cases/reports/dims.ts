/**
 * L5 (03-domains/13-reports.md §8(b)): commands 1 (trial balance), 2 (P&L) and 6 (balance sheet)
 * each with every dimension filter (branch, cost center, currency) — the "dims" case the plan
 * calls out separately from `statements-*`'s own per-dim steps, isolating dim behavior on its own
 * (including a dim value that matches nothing, which both sides treat as "no rows for that filter").
 */
import { defineCase } from '../../case';
import * as reportService from '../../../../src/modules/reports/services/reportService';

export default defineCase({
  name: 'reports/dims',
  source: '03-domains/13-reports.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const range = {};
    const asOf = '2026-06-30';
    const branch = s.baseId('branch-main');
    const cc = s.baseId('cc-main');

    await s.step('tb-branch', () => reportService.getTrialBalance({ ...range, branchId: branch }));
    await s.step('tb-cc', () => reportService.getTrialBalance({ ...range, costCenterId: cc }));
    await s.step('tb-currency-sar', () => reportService.getTrialBalance({ ...range, currency: 'SAR' }));
    await s.step('tb-currency-usd', () => reportService.getTrialBalance({ ...range, currency: 'USD' }));
    await s.step('tb-branch-stale', () => reportService.getTrialBalance({ ...range, branchId: 'branch-nonexistent' }));

    await s.step('pnl-branch', () => reportService.getProfitAndLoss({ ...range, branchId: branch }));
    await s.step('pnl-cc', () => reportService.getProfitAndLoss({ ...range, costCenterId: cc }));
    await s.step('pnl-currency-sar', () => reportService.getProfitAndLoss({ ...range, currency: 'SAR' }));
    await s.step('pnl-currency-usd', () => reportService.getProfitAndLoss({ ...range, currency: 'USD' }));

    await s.step('bs-branch', () => reportService.getBalanceSheet(asOf, { branchId: branch }));
    await s.step('bs-cc', () => reportService.getBalanceSheet(asOf, { costCenterId: cc }));
    await s.step('bs-currency-sar', () => reportService.getBalanceSheet(asOf, { currency: 'SAR' }));
    await s.step('bs-currency-usd', () => reportService.getBalanceSheet(asOf, { currency: 'USD' }));
  },
});
