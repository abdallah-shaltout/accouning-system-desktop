/**
 * 12b-period-close §8(b): closing a year with no later year creates the next one (§3.3 step 8):
 * `2026` → `2027` (01-01 … 12-31). Then that year is renamed without digits and made to end on
 * 2028-02-28, gets P&L movement, and is closed: the next year starts 2028-02-29 and ends
 * 2029-02-28 (JS `setFullYear` rolls Feb 29 to Mar 1, minus one day), named start year + 1 =
 * `2028`. A year with no P&L movement cannot be closed (quirk Q3: fewer than 2 closing lines).
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'accounting/close-year-next-year-autocreate',
  source: '03-domains/12b-period-close.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const first = await s.step('close-2026', () => accountingService.closeYear(s.baseId('fy-2026')));
    const y2027 = first.nextYear!;
    await s.step('years-after-first', () => accountingService.getFiscalYears());
    await s.expectError('close-empty-year', () => accountingService.closeYear(y2027.id));
    await s.step('reshape-2027', () => accountingService.saveFiscalYear({ name: 'سنة الإطلاق', startDate: '2027-01-01', endDate: '2028-02-28', isClosed: false }, y2027.id));
    await s.step('movement-2027', () =>
      accountingService.createJournalEntry({
        date: '2027-06-15',
        description: 'إيراد ومصروف في السنة الجديدة',
        lines: [
          { accountId: s.baseId('acc-1120'), debit: 2500, credit: 0 },
          { accountId: s.baseId('acc-4300'), debit: 0, credit: 2500 },
          { accountId: s.baseId('acc-6260'), debit: 800, credit: 0 },
          { accountId: s.baseId('acc-1110'), debit: 0, credit: 800 },
        ],
      }),
    );
    const second = await s.step('close-2027', () => accountingService.closeYear(y2027.id));
    await s.step('closing-entry-2027', () => accountingService.getJournalEntry(second.closingEntry.id));
    await s.step('years-final', () => accountingService.getFiscalYears());
  },
});
