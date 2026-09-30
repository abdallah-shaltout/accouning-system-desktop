/**
 * ACC-0008 verification (P4-7, 12b D-A2 / quirk Q4): reopening a year dates the closing entry's
 * mirror at the closing entry's own date (the year end), not "now" — so the reopened year's P&L
 * comes back, the following year is untouched, and the year can be closed again with the same net.
 * The clock is moved into the next year before reopening, which is exactly when the old behaviour
 * put the mirror in the wrong year.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'accounting/acc-0008-reopen-mirror-date',
  source: 'docs/diagnostics/issues/ACC-0008-reopen-mirror-date.md; 03-domains/12b-period-close.md §7 Q4',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const fy2026 = s.baseId('fy-2026');
    const first = await s.step('close', () => accountingService.closeYear(fy2026));
    await s.step('pl-2026-after-close', () => accountingService.getAccounts({ from: '2026-01-01', to: '2026-12-31' }));
    await s.setClock('2027-02-10T09:00:00.000Z');
    await s.step('reopen', () => accountingService.reopenYear(fy2026));
    const mirrors = await s.step('mirror', async () =>
      (await accountingService.getJournalEntries({ type: 'CLOSING', reversed: false, from: '2026-01-01' })).filter((e) => e.reversalOfId === first.closingEntry.id),
    );
    await s.step('mirror-detail', () => accountingService.getJournalEntry(mirrors[0].id));
    await s.step('pl-2026-after-reopen', () => accountingService.getAccounts({ from: '2026-01-01', to: '2026-12-31' }));
    await s.step('pl-2027-untouched', () => accountingService.getAccounts({ from: '2027-01-01', to: '2027-12-31' }));
    const again = await s.step('close-again', () => accountingService.closeYear(fy2026));
    await s.step('same-net', () => ({
      first: first.closingEntry.totalDebit,
      again: again.closingEntry.totalDebit,
    }));
    await s.step('years', () => accountingService.getFiscalYears());
  },
});
