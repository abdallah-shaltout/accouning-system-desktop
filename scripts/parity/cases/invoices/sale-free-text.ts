/**
 * 08 §8(b) sale-free-text — a desk sale mixing a catalog line with three free-text lines on two
 * revenue accounts: free-text lines carry no product (`freetext-<position>`), no stock and no COGS,
 * and credit their own revenue account (grouped per account, D-I2).
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'invoices/sale-free-text',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const services = s.baseId('acc-4110');
    const other = s.baseId('acc-4300');
    const sale = await s.step('create', () =>
      invoiceService.createSale({
        customerId: s.baseId('cus-1'),
        lines: [
          { productId: s.baseId('prd-8'), qty: 1, price: 449 },
          { productId: 'freetext', qty: 1, price: 230, isFreeText: true, revenueAccountId: services, name: 'تفصيل وخياطة' },
          { productId: 'freetext', qty: 2, price: 57.5, isFreeText: true, revenueAccountId: other, name: 'تغليف هدايا' },
          { productId: 'freetext', qty: 1, price: 20, isFreeText: true, revenueAccountId: services },
        ],
        discountRate: 0,
        paymentMethod: 'cash',
        paidAmount: 814,
        source: 'DESK',
      }),
    );
    await s.step('detail', () => invoiceService.getInvoice(sale.id));
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('invoice', sale.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
  },
});
