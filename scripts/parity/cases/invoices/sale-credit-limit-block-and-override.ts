/**
 * 08 §8(b) sale-credit-limit-block-and-override — a credit sale that pushes a customer past its
 * credit limit: a cashier (no accounting/parties write) is refused with the exact FORBIDDEN message
 * (balance + this invoice = projected, vs the limit); an accountant (accounting write) may override
 * and the same sale posts. Within-limit sales are never checked.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'invoices/sale-credit-limit-block-and-override',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const customerId = s.baseId('cus-5'); // credit limit 5,000
    await s.step('customer-before', () => partyService.getCustomer(customerId));
    const overLimit = {
      customerId,
      lines: [{ productId: s.baseId('prd-7'), qty: 5, price: 1099 }],
      discountRate: 0,
      paymentMethod: 'credit' as const,
      paidAmount: 0,
      source: 'DESK' as const,
    };
    const withinLimit = { ...overLimit, lines: [{ productId: s.baseId('prd-18'), qty: 1, price: 39 }] };

    await s.login('cashier');
    await s.step('cashier-within-limit', () => invoiceService.createSale(withinLimit));
    await s.expectError('cashier-over-limit', () => invoiceService.createSale(overLimit));

    await s.login('accountant');
    await s.setClock('2026-06-30T09:05:00.000Z');
    const sale = await s.step('accountant-override', () => invoiceService.createSale(overLimit));
    await s.step('detail', () => invoiceService.getInvoice(sale.id));
    await s.step('customer-after', () => partyService.getCustomer(customerId));
  },
});
