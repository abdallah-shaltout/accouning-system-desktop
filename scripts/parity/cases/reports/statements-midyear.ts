/**
 * L5 (03-domains/13-reports.md §8(b)): the same commands 1–14 as `statements-full`/`statements-
 * month`, but over a range starting **mid-year** (opening balances carry the first half of the
 * seed's fiscal year, per the plan's "starting mid-year (opening balances)" third range).
 */
import { defineCase } from '../../case';
import * as reportService from '../../../../src/modules/reports/services/reportService';

export default defineCase({
  name: 'reports/statements-midyear',
  source: '03-domains/13-reports.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const range = { from: '2026-04-01', to: '2026-06-30' };
    const asOf = '2026-06-30';
    const branch = s.baseId('branch-main');
    const cc = s.baseId('cc-main');
    const cashAccount = s.baseId('acc-1110');
    const customer = s.baseId('cus-1');
    const supplier = s.baseId('sup-1');

    await s.step('trial-balance', () => reportService.getTrialBalance(range));
    await s.step('trial-balance-branch', () => reportService.getTrialBalance({ ...range, branchId: branch }));
    await s.step('trial-balance-cc', () => reportService.getTrialBalance({ ...range, costCenterId: cc }));
    await s.step('trial-balance-currency', () => reportService.getTrialBalance({ ...range, currency: 'SAR' }));

    await s.step('pnl', () => reportService.getProfitAndLoss(range));
    await s.step('pnl-branch', () => reportService.getProfitAndLoss({ ...range, branchId: branch }));
    await s.step('pnl-cc', () => reportService.getProfitAndLoss({ ...range, costCenterId: cc }));
    await s.step('pnl-currency', () => reportService.getProfitAndLoss({ ...range, currency: 'SAR' }));

    await s.step('pnl-comparison', () => reportService.getProfitAndLossComparison(range, { from: '2026-01-01', to: '2026-03-31' }));
    await s.step('cost-center-pnl', () => reportService.getCostCenterProfitAndLoss(range));
    await s.step('budget-vs-actual', () => reportService.getCostCenterBudgetVsActual(s.baseId('fy-2026')));

    await s.step('balance-sheet', () => reportService.getBalanceSheet(asOf));
    await s.step('balance-sheet-branch', () => reportService.getBalanceSheet(asOf, { branchId: branch }));
    await s.step('balance-sheet-cc', () => reportService.getBalanceSheet(asOf, { costCenterId: cc }));
    await s.step('balance-sheet-currency', () => reportService.getBalanceSheet(asOf, { currency: 'SAR' }));

    await s.step('account-ledger-cash', () => reportService.getAccountLedger(cashAccount, range));
    await s.step('party-ledger-customer', () => reportService.getPartyLedger('customer', customer, range));
    await s.step('party-ledger-supplier', () => reportService.getPartyLedger('supplier', supplier, range));

    await s.step('vat-report', () => reportService.getVatReport(range));
    await s.step('vat-detail', () => reportService.getVatDetail(range));
    await s.step('cash-flow', () => reportService.getCashFlowStatement(range));
    await s.step('day-book', () => reportService.getDayBook(range));
    await s.step('period-comparison', () => reportService.getPeriodComparison(range, { from: '2026-01-01', to: '2026-03-31' }));
    await s.step('business-health', () => reportService.getBusinessHealthReport(range));
  },
});
