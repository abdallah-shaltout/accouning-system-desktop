/**
 * 10 §8(b) voucher-payment — a general payment voucher: Dr the picked manual account (with its
 * cost center) / Cr the bank-role account of the bank-transfer method; the journal entry and the
 * voucher read back.
 */
import { defineCase } from '../../case';
import * as voucherService from '../../../../src/modules/vouchers/services/voucherService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'vouchers/voucher-payment',
  source: '03-domains/10-vouchers.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const v = await s.step('create', () =>
      voucherService.createPaymentVoucher({
        date: '2026-06-30T09:00:00.000Z',
        amount: 57.5,
        description: 'رسوم خدمة بنكية',
        paymentMethodId: s.baseId('pm-bank-transfer'),
        debitAccountId: s.baseId('acc-6280'),
        costCenterId: s.baseId('cc-main'),
      }),
    );
    await s.step('get', () => voucherService.getVoucher(v.id));
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('voucher', v.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
  },
});
