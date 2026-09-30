/**
 * L5 (phase-b2-parity-cases.md "L5" header: "plus one demo-eg pass each for the statements and
 * home KPIs"). The same statement/ledger commands as `statements-full`, run once against the
 * `demo-eg` base (14% VAT, EGP, Africa/Cairo timezone) instead of `demo-sa`, so the country-
 * dependent formatting/VAT-rate paths get at least one non-SA pass in this lane.
 */
import { defineCase } from '../../case';
import * as reportService from '../../../../src/modules/reports/services/reportService';

export default defineCase({
  name: 'reports/statements-full-eg',
  source: '03-domains/13-reports.md §8(b); phase-b2-parity-cases.md L5 header',
  base: 'demo-eg',
  user: 'admin',
  async run(s) {
    const range = {};
    const asOf = '2026-06-30';

    await s.step('trial-balance', () => reportService.getTrialBalance(range));
    await s.step('pnl', () => reportService.getProfitAndLoss(range));
    await s.step('balance-sheet', () => reportService.getBalanceSheet(asOf));
    await s.step('vat-report', () => reportService.getVatReport(range));
    await s.step('vat-detail', () => reportService.getVatDetail(range));
    await s.step('cash-flow', () => reportService.getCashFlowStatement(range));
    await s.step('day-book', () => reportService.getDayBook(range));
    await s.step('business-health', () => reportService.getBusinessHealthReport(range));
  },
});
