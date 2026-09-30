/**
 * 10 §8(b) voucher-receipt — a general receipt voucher: Dr the cash-role account of the chosen
 * method / Cr the picked manual account with its cost center; VCH-000005; the journal entry and the
 * voucher read back.
 */
import { defineCase } from '../../case';
import * as voucherService from '../../../../src/modules/vouchers/services/voucherService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'vouchers/voucher-receipt',
  source: '03-domains/10-vouchers.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const v = await s.step('create', () =>
      voucherService.createReceiptVoucher({
        date: '2026-06-30T09:00:00.000Z',
        amount: 350.5,
        description: 'بيع أرفف عرض مستعملة',
        note: 'نقداً من المشتري',
        paymentMethodId: s.baseId('pm-cash'),
        creditAccountId: s.baseId('acc-4300'),
        costCenterId: s.baseId('cc-main'),
      }),
    );
    await s.step('get', () => voucherService.getVoucher(v.id));
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('voucher', v.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
  },
});
