/**
 * 12b-period-close §8(b): a VAT settlement where output VAT exceeds input VAT (§3.4) — the period
 * totals (business-day range), then the settlement entry dated `to`: Dr output VAT, Cr input VAT,
 * Cr VAT payable (net). The period's totals read zero afterwards. Refusals: a period with no VAT
 * movement (also for the closed year, whose empty totals are checked before the period guard).
 *
 * Known red on the mock (reported by lane L4, 2026-09-29): the mock's §4.5 invariant
 * (`vat-output`/`vat-input` = Σ document VAT, all time) does not exclude VAT-settlement entries,
 * so the successful settlement step is recorded as a new invariant break. The settlement itself
 * is correct (docs/v2/02-accounting-review.md §3 "VAT settlement").
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'accounting/vat-settlement-payable',
  source: '03-domains/12b-period-close.md §8(b)',
  base: 'demo-sa',
  user: 'accountant',
  async run(s) {
    await s.step('totals-q2', () => accountingService.getVatPeriodTotals('2026-04-01', '2026-06-30'));
    await s.step('totals-may', () => accountingService.getVatPeriodTotals('2026-05-01', '2026-05-31'));
    await s.expectError('no-movement', () => accountingService.submitVatSettlement('2026-01-01', '2026-03-31'));
    // The closed year has no VAT movement, and the totals check runs before the period guard.
    await s.expectError('closed-year-no-movement', () => accountingService.submitVatSettlement('2025-10-01', '2025-12-31'));
    const entry = await s.step('settle-may', () => accountingService.submitVatSettlement('2026-05-01', '2026-05-31'));
    await s.step('detail', () => accountingService.getJournalEntry(entry.id));
    await s.step('totals-may-after', () => accountingService.getVatPeriodTotals('2026-05-01', '2026-05-31'));
    await s.step('vat-accounts', async () => (await accountingService.getAccounts()).filter((a) => ['1150', '2150', '2155'].includes(a.code)));
  },
});
