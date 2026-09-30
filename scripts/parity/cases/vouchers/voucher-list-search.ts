/**
 * 10 §8(b) voucher-list-search — the voucher list: all (date desc), by kind, by date range, and the
 * search over number / description / note, Arabic-normalized (`احمد` finds `أحمد`, `ه` vs `ة`).
 */
import { defineCase } from '../../case';
import * as voucherService from '../../../../src/modules/vouchers/services/voucherService';

export default defineCase({
  name: 'vouchers/voucher-list-search',
  source: '03-domains/10-vouchers.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    await s.step('receipt-ahmad', () =>
      voucherService.createReceiptVoucher({
        date: '2026-06-30T09:00:00.000Z',
        amount: 120,
        description: 'استرداد سلفة أحمد',
        note: 'دفعة أولى',
        paymentMethodId: s.baseId('pm-cash'),
        creditAccountId: s.baseId('acc-1115'),
      }),
    );
    await s.step('all', () => voucherService.getVouchers());
    await s.step('kind-receipt', () => voucherService.getVouchers({ kind: 'RECEIPT' }));
    await s.step('kind-owner', () => voucherService.getVouchers({ kind: 'OWNER' }));
    await s.step('date-range', () => voucherService.getVouchers({ from: '2026-06-10', to: '2026-06-20' }));
    await s.step('search-hamza', () => voucherService.getVouchers({ search: 'أحمد' }));
    await s.step('search-no-hamza', () => voucherService.getVouchers({ search: 'احمد' }));
    await s.step('search-note', () => voucherService.getVouchers({ search: 'دفعه اولى' }));
    await s.step('search-number', () => voucherService.getVouchers({ search: 'VCH-000003' }));
    await s.step('search-description', () => voucherService.getVouchers({ search: 'خردة' }));
    await s.step('search-none', () => voucherService.getVouchers({ search: 'لا يوجد شيء كهذا' }));
  },
});
