/**
 * 12-accounting §8(b): reversing a MANUAL entry (B3) — the mirror entry (swapped sides, same
 * accounts, branch and cost center), the caller-chosen date, the reason stored on both entries,
 * `reversed` on the original, and `reversedById/Number` on the original's detail. One reversal of a
 * seeded entry and one of an entry posted in the story with a cost-center split.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'accounting/reverse-manual',
  source: '03-domains/12-accounting.md §8(b)',
  base: 'demo-sa',
  user: 'accountant',
  async run(s) {
    const manual = await s.step('manual-june', () => accountingService.getJournalEntries({ type: 'MANUAL', from: '2026-06-27', to: '2026-06-30' }));
    const split = manual.find((e) => e.description.includes('مركز تكلفة'))!;
    const reversal = await s.step('reverse-seeded', () => accountingService.reverseJournalEntry(split.id, '2026-06-30T10:00:00.000Z', '  توزيع خاطئ على الفروع  '));
    await s.step('detail-original', () => accountingService.getJournalEntry(split.id));
    await s.step('detail-reversal', () => accountingService.getJournalEntry(reversal.id));

    const own = await s.step('post-own', () =>
      accountingService.createJournalEntry({
        date: '2026-06-20T09:00:00.000Z',
        description: 'تسوية عهدة نقدية',
        lines: [
          { accountId: s.baseId('acc-1115'), debit: 700, credit: 0, costCenterId: s.baseId('cc-main') },
          { accountId: s.baseId('acc-1110'), debit: 0, credit: 700 },
        ],
      }),
    );
    // A date-only key (the detail page sends an instant; both are accepted).
    const ownReversal = await s.step('reverse-own', () => accountingService.reverseJournalEntry(own.id, '2026-06-21', 'قيد مكرر'));
    await s.step('detail-own', () => accountingService.getJournalEntry(own.id));
    await s.step('detail-own-reversal', () => accountingService.getJournalEntry(ownReversal.id));
    await s.step('list-reversed', () => accountingService.getJournalEntries({ reversed: true }));
    await s.step('accounts-after', () => accountingService.getAccounts({ from: '2026-06-20', to: '2026-06-30' }));
  },
});
