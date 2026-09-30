/**
 * 12b-period-close §8(b): closing a fiscal year (§3.3) when the next year already exists — the
 * closing entry (every revenue/expense account to zero, net to retained earnings), the year
 * flagged closed with closer and time, the existing next year returned (not duplicated), then
 * the refusals: already closed (VALIDATION), unknown year (NOT_FOUND), a non-admin posting into
 * the closed year (FORBIDDEN), and the admin's override. Run on the EG base.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import type { JournalEntryInput } from '../../../../src/modules/accounting/types';

export default defineCase({
  name: 'accounting/close-year',
  source: '03-domains/12b-period-close.md §8(b)',
  base: 'demo-eg',
  user: 'accountant',
  async run(s) {
    const fy2026 = s.baseId('fy-2026');
    const next = await s.step('create-2027', () => accountingService.saveFiscalYear({ name: '2027', startDate: '2027-01-01', endDate: '2027-12-31', isClosed: false }));
    await s.step('checks', () => accountingService.getCloseYearPreChecks(fy2026));
    const closed = await s.step('close', () => accountingService.closeYear(fy2026));
    await s.step('closing-entry', () => accountingService.getJournalEntry(closed.closingEntry.id));
    await s.step('years', () => accountingService.getFiscalYears());
    await s.step('pl-accounts-2026', () => accountingService.getAccounts({ from: '2026-01-01', to: '2026-12-31' }));
    await s.expectError('close-again', () => accountingService.closeYear(fy2026));
    await s.expectError('close-unknown', () => accountingService.closeYear('fy-missing'));

    const entry = (date: string): JournalEntryInput => ({
      date,
      description: 'قيد بعد الإقفال',
      lines: [
        { accountId: s.baseId('acc-6260'), debit: 40, credit: 0 },
        { accountId: s.baseId('acc-1110'), debit: 0, credit: 40 },
      ],
    });
    await s.expectError('post-closed-accountant', () => accountingService.createJournalEntry(entry('2026-06-30T10:00:00.000Z')));
    await s.step('post-next-year', () => accountingService.createJournalEntry(entry('2027-01-05')));
    await s.login('admin');
    await s.step('post-closed-admin', () => accountingService.createJournalEntry(entry('2026-06-30T10:00:00.000Z')));
    await s.step('current-year', () => accountingService.getCurrentFiscalYear());
    await s.step('next-year-unchanged', async () => (await accountingService.getFiscalYears()).filter((f) => f.id === next.id));
  },
});
