/**
 * 12b-period-close §8(b): the lock date (§3.2) — read (none), set, read back, a posting on or
 * before it is refused, after it is allowed, drafts are not period-checked, moving and clearing it
 * (`undefined` and `''` both clear), and posting before the old date once it is cleared. The lock date is the day before the demo history starts: the mock's `lock-date`
 * invariant counts every entry dated on/before the lock date as an offender whenever it was
 * posted, so a later lock date fails the run for a reason outside this case (reported by lane L4,
 * 2026-09-29). For the same reason the admin's override posting under a lock date is not run
 * here (the invariant flags it too); the admin override is covered for closed years in
 * `accounting/manual-entry-post` and `accounting/close-year`. An unparsable lock date is refused
 * only by Rust (decision P-D2), so it is not sent.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import type { JournalEntryInput } from '../../../../src/modules/accounting/types';

export default defineCase({
  name: 'accounting/lock-date',
  source: '03-domains/12b-period-close.md §8(b)',
  base: 'demo-eg',
  user: 'accountant',
  async run(s) {
    const entry = (date: string, amount = 60): JournalEntryInput => ({
      date,
      description: 'قيد اختبار تاريخ القفل',
      lines: [
        { accountId: s.baseId('acc-6260'), debit: amount, credit: 0 },
        { accountId: s.baseId('acc-1110'), debit: 0, credit: amount },
      ],
    });
    await s.step('get-none', () => accountingService.getLockDate());
    await s.step('set', () => accountingService.saveLockDate('2026-04-15'));
    await s.step('get-set', () => accountingService.getLockDate());
    await s.expectError('post-before', () => accountingService.createJournalEntry(entry('2026-03-01')));
    await s.expectError('post-on', () => accountingService.createJournalEntry(entry('2026-04-15T10:00:00.000Z')));
    await s.step('post-after', () => accountingService.createJournalEntry(entry('2026-04-16T08:00:00.000Z')));
    await s.step('draft-before-allowed', () => accountingService.createJournalEntry({ ...entry('2026-04-01'), asDraft: true }));
    await s.step('move', () => accountingService.saveLockDate('2026-04-12'));
    await s.step('get-moved', () => accountingService.getLockDate());
    await s.step('clear', () => accountingService.saveLockDate(undefined));
    await s.step('get-cleared', () => accountingService.getLockDate());
    await s.step('set-empty-string-clears', () => accountingService.saveLockDate(''));
    await s.step('get-after-empty', () => accountingService.getLockDate());
    await s.step('post-before-after-clear', () => accountingService.createJournalEntry(entry('2026-04-01', 15)));
  },
});
