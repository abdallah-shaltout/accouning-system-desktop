/**
 * 08 §8(b) sale-empties-stock — selling a product's whole remaining stock takes COGS as the exact
 * remaining stock value (not qty × avg), leaving value 0; one more unit is then refused with the
 * CONFLICT availability message.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as productService from '../../../../src/modules/products/services/productService';

export default defineCase({
  name: 'invoices/sale-empties-stock',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const productId = s.baseId('prd-6');
    const before = await s.step('product-before', () => productService.getProduct(productId));
    const sale = await s.step('create', () =>
      invoiceService.createSale({
        lines: [{ productId, qty: before.stockQty, price: 389 }],
        discountRate: 0,
        paymentMethod: 'card',
        paidAmount: 0,
      }),
    );
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('invoice', sale.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    await s.step('product-after', () => productService.getProduct(productId));
    await s.expectError('out-of-stock', () =>
      invoiceService.createSale({ lines: [{ productId, qty: 1, price: 389 }], discountRate: 0, paymentMethod: 'cash', paidAmount: 389 }),
    );
  },
});
