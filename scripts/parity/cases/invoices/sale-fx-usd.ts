/**
 * 08 §8(b) sale-fx-usd — a USD credit sale to the USD customer with no explicit rate (the latest
 * seeded rate is used): credit lines converted to base with the gap on the largest line, the
 * receivable tagged with `amountFc`/`rate`, COGS unconverted. Then a sale in a currency with no
 * rate at all fails with the `requireRate` message.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'invoices/sale-fx-usd',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const input = {
      customerId: s.baseId('cus-usd-1'),
      lines: [
        { productId: s.baseId('prd-1'), qty: 3, price: 33.33 },
        { productId: s.baseId('prd-2'), qty: 1, price: 47.1 },
      ],
      discountRate: 0,
      paymentMethod: 'credit' as const,
      paidAmount: 0,
      source: 'DESK' as const,
      currency: 'USD',
    };
    await s.step('preview', () => invoiceService.previewSale(input));
    const sale = await s.step('create', () => invoiceService.createSale(input));
    await s.step('detail', () => invoiceService.getInvoice(sale.id));
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('invoice', sale.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    await s.expectError('no-rate', () => invoiceService.createSale({ ...input, currency: 'EUR' }));
  },
});
