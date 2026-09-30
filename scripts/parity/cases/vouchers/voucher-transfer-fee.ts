/**
 * 10 §8(b) voucher-transfer-fee — a transfer between accounts with a fee: Dr destination `amount`,
 * Cr source `amount + fee`, Dr fee account `fee` (fee stored); and a transfer without a fee
 * (`feeAmount` absent, two lines).
 */
import { defineCase } from '../../case';
import * as voucherService from '../../../../src/modules/vouchers/services/voucherService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'vouchers/voucher-transfer-fee',
  source: '03-domains/10-vouchers.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const withFee = await s.step('transfer-fee', () =>
      voucherService.createTransferVoucher({
        date: '2026-06-30T09:00:00.000Z',
        amount: 1200,
        description: 'تعزيز الصندوق من البنك',
        sourceAccountId: s.baseId('acc-1120'),
        destinationAccountId: s.baseId('acc-1110'),
        feeAmount: 5.75,
        feeAccountId: s.baseId('acc-6280'),
      }),
    );
    await s.step('get-fee', () => voucherService.getVoucher(withFee.id));
    const refs = await s.step('journal-refs-fee', () => accountingService.getJournalEntriesForSource('voucher', withFee.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-fee-${i}`, () => accountingService.getJournalEntry(refs[i].id));

    await s.setClock('2026-06-30T09:05:00.000Z');
    const noFee = await s.step('transfer-no-fee', () =>
      voucherService.createTransferVoucher({
        date: '2026-06-30T09:05:00.000Z',
        amount: 300,
        description: 'إيداع نقدية بالبنك',
        sourceAccountId: s.baseId('acc-1110'),
        destinationAccountId: s.baseId('acc-1120'),
      }),
    );
    await s.step('get-no-fee', () => voucherService.getVoucher(noFee.id));
    const refs2 = await s.step('journal-refs-no-fee', () => accountingService.getJournalEntriesForSource('voucher', noFee.id));
    for (let i = 0; i < refs2.length; i++) await s.step(`journal-no-fee-${i}`, () => accountingService.getJournalEntry(refs2[i].id));
  },
});
