/**
 * 10 §8(b) voucher-owner-both-directions — owner vouchers: drawings (Dr drawings / Cr cash) and a
 * capital contribution (Dr bank / Cr drawings), each with its fixed Arabic description suffix.
 */
import { defineCase } from '../../case';
import * as voucherService from '../../../../src/modules/vouchers/services/voucherService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'vouchers/voucher-owner-both-directions',
  source: '03-domains/10-vouchers.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const drawings = await s.step('drawings', () =>
      voucherService.createOwnerVoucher({
        date: '2026-06-30T09:00:00.000Z',
        amount: 750,
        description: 'مسحوبات نهاية الشهر',
        direction: 'drawings',
        cashAccountId: s.baseId('acc-1110'),
      }),
    );
    await s.setClock('2026-06-30T09:05:00.000Z');
    const contribution = await s.step('contribution', () =>
      voucherService.createOwnerVoucher({
        date: '2026-06-30T09:05:00.000Z',
        amount: 5000,
        description: 'زيادة رأس المال العامل',
        direction: 'contribution',
        cashAccountId: s.baseId('acc-1120'),
      }),
    );
    for (const [tag, id] of [['drawings', drawings.id], ['contribution', contribution.id]] as const) {
      await s.step(`get-${tag}`, () => voucherService.getVoucher(id));
      const refs = await s.step(`journal-refs-${tag}`, () => accountingService.getJournalEntriesForSource('voucher', id));
      for (let i = 0; i < refs.length; i++) await s.step(`journal-${tag}-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    }
  },
});
