/**
 * 08 §8(b) sale-cash-inclusive — a tax-inclusive cash POS sale (SA, VAT 15 %): preview, post, then
 * read the invoice, its one cash tender, the posting (tender / sales / VAT / COGS / inventory lines,
 * zero lines dropped) and the product's stock after the sale.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as productService from '../../../../src/modules/products/services/productService';

export default defineCase({
  name: 'invoices/sale-cash-inclusive',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const input = {
      lines: [
        { productId: s.baseId('prd-1'), qty: 2, price: 129 },
        { productId: s.baseId('prd-14'), qty: 3, price: 39, discount: 5 },
      ],
      discountRate: 0,
      paymentMethod: 'cash' as const,
      paidAmount: 400,
      tenderedAmount: 400,
    };
    await s.step('preview', () => invoiceService.previewSale(input));
    const sale = await s.step('create', () => invoiceService.createSale(input));
    await s.step('detail', () => invoiceService.getInvoice(sale.id));
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('invoice', sale.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    await s.step('product-after', () => productService.getProduct(s.baseId('prd-1')));
  },
});
