/**
 * 12-accounting §8(b): `reverseJournalEntry` refusals in the order of §3.3 — unknown id, a SYSTEM
 * entry (reversed from its source document), an already-reversed entry, the reversal entry itself,
 * an empty reason (checked after "already reversed", as in the mock), and a reversal dated in the
 * closed year for a non-admin (the admin's override is allowed).
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'accounting/reverse-refusals',
  source: '03-domains/12-accounting.md §8(b)',
  base: 'demo-sa',
  user: 'accountant',
  async run(s) {
    const system = await s.step('system-entries', () => accountingService.getJournalEntries({ type: 'SYSTEM', from: '2026-06-30', to: '2026-06-30' }));
    const closing = await s.step('closing-entries', () => accountingService.getJournalEntries({ type: 'CLOSING' }));
    const own = await s.step('post', () =>
      accountingService.createJournalEntry({
        date: '2026-06-30T06:00:00.000Z',
        description: 'قيد للعكس',
        lines: [
          { accountId: s.baseId('acc-6260'), debit: 90, credit: 0 },
          { accountId: s.baseId('acc-1110'), debit: 0, credit: 90 },
        ],
      }),
    );
    await s.expectError('unknown', () => accountingService.reverseJournalEntry('je-missing', '2026-06-30', 'سبب'));
    await s.expectError('system-entry', () => accountingService.reverseJournalEntry(system[0].id, '2026-06-30', 'سبب'));
    await s.expectError('closing-entry', () => accountingService.reverseJournalEntry(closing[0].id, '2026-06-30', 'سبب'));
    await s.expectError('reason-blank', () => accountingService.reverseJournalEntry(own.id, '2026-06-30', '   '));
    await s.expectError('closed-year-accountant', () => accountingService.reverseJournalEntry(own.id, '2025-12-31', 'عكس في سنة مقفلة'));
    const reversal = await s.step('reverse', () => accountingService.reverseJournalEntry(own.id, '2026-06-30', 'خطأ في الحساب'));
    await s.expectError('already-reversed', () => accountingService.reverseJournalEntry(own.id, '2026-06-30', 'مرة ثانية'));
    await s.expectError('already-reversed-blank-reason', () => accountingService.reverseJournalEntry(own.id, '2026-06-30', ''));
    await s.expectError('reverse-the-reversal', () => accountingService.reverseJournalEntry(reversal.id, '2026-06-30', 'عكس العكس'));
    await s.login('admin');
    const other = await s.step('post-2', () =>
      accountingService.createJournalEntry({
        date: '2026-06-30T06:30:00.000Z',
        description: 'قيد يعكسه المدير في سنة مقفلة',
        lines: [
          { accountId: s.baseId('acc-6260'), debit: 45, credit: 0 },
          { accountId: s.baseId('acc-1110'), debit: 0, credit: 45 },
        ],
      }),
    );
    await s.step('closed-year-admin', () => accountingService.reverseJournalEntry(other.id, '2025-12-31', 'عكس بتاريخ سابق'));
  },
});
