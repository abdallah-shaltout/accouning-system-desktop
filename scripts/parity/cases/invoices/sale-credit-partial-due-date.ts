/**
 * 08 §8(b) sale-credit-partial-due-date — on demo-eg (VAT 14 %, Africa/Cairo, because the due date
 * is a wall-clock "+N days" in the business zone): a partly-paid sale to a customer with 30-day
 * terms (PARTIALLY_PAID, receivable tagged with the customer, due date from the terms), a credit
 * sale whose `dueDateOverride` wins, and a fully-paid sale that carries no due date.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'invoices/sale-credit-partial-due-date',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-eg',
  async run(s) {
    const customerId = s.baseId('cus-2');
    const partial = await s.step('sale-partial', () =>
      invoiceService.createSale({
        customerId,
        lines: [{ productId: s.baseId('prd-4'), qty: 1, price: 249 }],
        discountRate: 0,
        paymentMethod: 'cash',
        paidAmount: 100,
        source: 'DESK',
      }),
    );
    await s.step('detail-partial', () => invoiceService.getInvoice(partial.id));
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('invoice', partial.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));

    await s.setClock('2026-06-30T09:05:00.000Z');
    const override = await s.step('sale-credit-override', () =>
      invoiceService.createSale({
        customerId,
        lines: [{ productId: s.baseId('prd-5'), qty: 2, price: 145 }],
        discountRate: 0,
        paymentMethod: 'credit',
        paidAmount: 0,
        source: 'DESK',
        dueDateOverride: '2026-08-15T10:00:00.000Z',
      }),
    );
    await s.step('detail-override', () => invoiceService.getInvoice(override.id));

    await s.setClock('2026-06-30T09:10:00.000Z');
    const paid = await s.step('sale-paid', () =>
      invoiceService.createSale({
        customerId,
        lines: [{ productId: s.baseId('prd-18'), qty: 1, price: 39 }],
        discountRate: 0,
        paymentMethod: 'cash',
        paidAmount: 39,
        source: 'DESK',
      }),
    );
    await s.step('detail-paid', () => invoiceService.getInvoice(paid.id));
  },
});
