/**
 * 09 §8(b) payment-list-filters-paged — the payment list on the seeded history: each filter on its
 * own (type, method, party, date range, unallocated-only), the Arabic-normalized search on the party
 * name (`احمد` finds `أحمد`) and on the number, and the paged variant: totals over the filtered,
 * unpaged set, a numeric sort (08 Q-7), and a second page.
 */
import { defineCase } from '../../case';
import * as paymentService from '../../../../src/modules/payments/services/paymentService';

export default defineCase({
  name: 'payments/payment-list-filters-paged',
  source: '03-domains/09-payments.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    await s.step('type-paid', () => paymentService.getPayments({ type: 'PAID', from: '2026-06-01', to: '2026-06-30' }));
    await s.step('method-bank', () => paymentService.getPayments({ method: 'bank_transfer', from: '2026-05-01', to: '2026-06-30' }));
    await s.step('party', () => paymentService.getPayments({ targetId: s.baseId('cus-2') }));
    await s.step('date-range', () => paymentService.getPayments({ from: '2026-06-20', to: '2026-06-30' }));
    await s.step('unallocated-only', () => paymentService.getPayments({ unallocatedOnly: true }));
    await s.step('search-hamza', () => paymentService.getPayments({ search: 'أحمد' }));
    await s.step('search-no-hamza', () => paymentService.getPayments({ search: 'احمد' }));
    await s.step('search-number', () => paymentService.getPayments({ search: 'PAY-000135' }));
    const query = { pageSize: 10, sort: { key: 'amount', dir: 'desc' as const }, filters: { type: 'RECEIVED' as const, from: '2026-04-01', to: '2026-06-30' } };
    await s.step('paged-1', () => paymentService.getPaymentsPaged({ page: 1, ...query }));
    await s.step('paged-2', () => paymentService.getPaymentsPaged({ page: 2, ...query }));
    await s.step('paged-default-sort', () => paymentService.getPaymentsPaged({ page: 1, pageSize: 5, filters: { unallocatedOnly: true } }));
  },
});
