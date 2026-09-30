/**
 * 08 §8(b) refund-customer-credit — a partly-paid sale to a customer, refunded to customer credit:
 * the refund settles the outstanding first, and the rest is credited to the customer's receivable
 * as one receivable line (no cash paid out), leaving unallocated credit on the party.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'invoices/refund-customer-credit',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const customerId = s.baseId('cus-3');
    await s.step('customer-before', () => partyService.getCustomer(customerId));
    const sale = await s.step('sale', () =>
      invoiceService.createSale({
        customerId,
        lines: [{ productId: s.baseId('prd-9'), qty: 1, price: 699 }],
        discountRate: 0,
        paymentMethod: 'cash',
        paidAmount: 500,
        source: 'DESK',
      }),
    );
    await s.setClock('2026-06-30T09:05:00.000Z');
    const refund = await s.step('refund', () =>
      invoiceService.createRefund({ invoiceId: sale.id, lines: [{ invoiceLineId: sale.lines[0].id, qty: 1 }], refundMethod: 'customer_credit' }),
    );
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('refund', refund.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    await s.step('detail', () => invoiceService.getInvoice(sale.id));
    await s.step('customer-after', () => partyService.getCustomer(customerId));
    await s.step('statement', () => partyService.getCustomerStatement(customerId));
  },
});
