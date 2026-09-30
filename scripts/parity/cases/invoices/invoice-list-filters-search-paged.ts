/**
 * 08 §8(b) invoice-list-filters-search-paged — the invoice list reads on the seeded history: each
 * filter on its own (status, payment status, customer, open-only, overdue-only, source, cashier,
 * amount range, date range), the Arabic-normalized search (`احمد` finds `أحمد`), and the paged
 * variant: totals over the filtered, unpaged set, a numeric sort (Q-7: text sorts are not compared),
 * and a second page.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';

export default defineCase({
  name: 'invoices/invoice-list-filters-search-paged',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const day = { from: '2026-06-28', to: '2026-06-29' };
    await s.step('date-range', () => invoiceService.getInvoices(day));
    await s.step('status-refunded', () => invoiceService.getInvoices({ status: 'REFUNDED', from: '2026-06-01', to: '2026-06-30' }));
    await s.step('payment-partial', () => invoiceService.getInvoices({ paymentStatus: 'PARTIALLY_PAID' }));
    await s.step('customer', () => invoiceService.getInvoices({ customerId: s.baseId('cus-2') }));
    await s.step('open-only', () => invoiceService.getInvoices({ openOnly: true }));
    await s.step('overdue-only', () => invoiceService.getInvoices({ overdueOnly: true }));
    await s.step('source-desk', () => invoiceService.getInvoices({ source: 'DESK', from: '2026-06-01', to: '2026-06-30' }));
    await s.step('cashier', () => invoiceService.getInvoices({ cashierId: s.baseId('usr-5'), ...day }));
    await s.step('amount-range', () => invoiceService.getInvoices({ minAmount: 1000, maxAmount: 1500, from: '2026-06-01', to: '2026-06-30' }));
    await s.step('search-hamza', () => invoiceService.getInvoices({ search: 'أحمد' }));
    await s.step('search-no-hamza', () => invoiceService.getInvoices({ search: 'احمد' }));
    await s.step('search-number', () => invoiceService.getInvoices({ search: '000634' }));

    await s.step('paged-1', () =>
      invoiceService.getInvoicesPaged({ page: 1, pageSize: 10, sort: { key: 'grandTotal', dir: 'desc' }, filters: { from: '2026-06-01', to: '2026-06-30' } }),
    );
    await s.step('paged-2', () =>
      invoiceService.getInvoicesPaged({ page: 2, pageSize: 10, sort: { key: 'grandTotal', dir: 'desc' }, filters: { from: '2026-06-01', to: '2026-06-30' } }),
    );
    await s.step('paged-open-default-sort', () => invoiceService.getInvoicesPaged({ page: 1, pageSize: 5, filters: { openOnly: true } }));
    await s.step('paged-search', () => invoiceService.getInvoicesPaged({ page: 1, pageSize: 5, sort: { key: 'outstanding', dir: 'asc' }, filters: { search: 'احمد' } }));
  },
});
