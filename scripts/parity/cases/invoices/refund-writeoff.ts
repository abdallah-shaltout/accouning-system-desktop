/**
 * 08 §8(b) refund-writeoff — a card sale refunded with one line restocked and one line written off
 * (`restock: false`, damaged): the restocked line returns to stock at its sale-time cost, the
 * written-off line's cost goes to inventory write-off (5120) instead and stock is unchanged; the
 * card refund settles to the bank.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as productService from '../../../../src/modules/products/services/productService';

export default defineCase({
  name: 'invoices/refund-writeoff',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const shirt = s.baseId('prd-19');
    const sandal = s.baseId('prd-21');
    const sale = await s.step('sale', () =>
      invoiceService.createSale({
        lines: [
          { productId: shirt, qty: 2, price: 329 },
          { productId: sandal, qty: 1, price: 159 },
        ],
        discountRate: 0,
        paymentMethod: 'card',
        paidAmount: 0,
      }),
    );
    await s.setClock('2026-06-30T09:05:00.000Z');
    const refund = await s.step('refund', () =>
      invoiceService.createRefund({
        invoiceId: sale.id,
        reason: 'تالف',
        lines: [
          { invoiceLineId: sale.lines[0].id, qty: 1, restock: true },
          { invoiceLineId: sale.lines[1].id, qty: 1, restock: false },
        ],
      }),
    );
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('refund', refund.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    await s.step('restocked-product', () => productService.getProduct(shirt));
    await s.step('written-off-product', () => productService.getProduct(sandal));
    await s.step('detail', () => invoiceService.getInvoice(sale.id));
  },
});
