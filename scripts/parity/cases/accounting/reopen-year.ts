/**
 * 12b-period-close §8(b): reopening a closed year (§3.3) — a non-admin is refused first
 * (FORBIDDEN, before the existence check), an admin reverses the closing entry with a CLOSING
 * mirror (reason on the mirror only, the original flagged `reversed`), the year's closing fields
 * are cleared; reopening an open year is refused, and so is an unknown year. A year flagged closed
 * with no closing entry (the demo's 2025) is simply unflagged.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'accounting/reopen-year',
  source: '03-domains/12b-period-close.md §8(b)',
  base: 'demo-sa',
  user: 'accountant',
  async run(s) {
    const fy2026 = s.baseId('fy-2026');
    const closed = await s.step('close', () => accountingService.closeYear(fy2026));
    await s.expectError('reopen-accountant', () => accountingService.reopenYear(fy2026));
    await s.expectError('reopen-unknown-accountant', () => accountingService.reopenYear('fy-missing'));
    await s.login('admin');
    await s.expectError('reopen-unknown', () => accountingService.reopenYear('fy-missing'));
    await s.step('reopen', () => accountingService.reopenYear(fy2026));
    await s.step('original-closing', () => accountingService.getJournalEntry(closed.closingEntry.id));
    await s.step('closing-entries', () => accountingService.getJournalEntries({ type: 'CLOSING' }));
    await s.expectError('reopen-open-year', () => accountingService.reopenYear(fy2026));
    await s.step('reopen-flag-only-2025', () => accountingService.reopenYear(s.baseId('fy-2025')));
    await s.step('years', () => accountingService.getFiscalYears());
    await s.step('pl-accounts-2026', () => accountingService.getAccounts({ from: '2026-01-01', to: '2026-12-31' }));
  },
});
