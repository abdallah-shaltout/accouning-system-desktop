/**
 * 09 §8(b) payment-receipt-multi-invoice — two credit sales to one customer, then one receipt
 * allocated across both (one GL entry only; each invoice's `paidAmount`/`paymentStatus` updated;
 * `targetRef` unset for two allocations), and a second receipt with a single allocation that sets
 * `targetRef`/`targetRefNumber`.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as paymentService from '../../../../src/modules/payments/services/paymentService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'payments/payment-receipt-multi-invoice',
  source: '03-domains/09-payments.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const customerId = s.baseId('cus-8');
    const sale = (id: string, price: number) => ({
      customerId,
      lines: [{ productId: s.baseId(id), qty: 1, price }],
      discountRate: 0,
      paymentMethod: 'credit' as const,
      paidAmount: 0,
      source: 'DESK' as const,
    });
    const a = await s.step('sale-a', () => invoiceService.createSale(sale('prd-9', 699)));
    await s.setClock('2026-06-30T09:05:00.000Z');
    const b = await s.step('sale-b', () => invoiceService.createSale(sale('prd-8', 449)));
    await s.step('open-docs', () => paymentService.getOpenDocuments('customer', customerId));

    await s.setClock('2026-06-30T09:10:00.000Z');
    const multi = await s.step('receipt-multi', () =>
      paymentService.createPayment({
        date: '2026-06-30T09:10:00.000Z',
        type: 'RECEIVED',
        targetType: 'customer',
        targetId: customerId,
        amount: 1000,
        method: 'bank_transfer',
        allocations: [
          { targetKind: 'invoice', targetId: a.id, amount: 699 },
          { targetKind: 'invoice', targetId: b.id, amount: 200 },
        ],
      }),
    );
    await s.step('get-multi', () => paymentService.getPayment(multi.id));
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('payment', multi.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    await s.step('invoice-a', () => invoiceService.getInvoice(a.id));
    await s.step('invoice-b', () => invoiceService.getInvoice(b.id));

    await s.setClock('2026-06-30T09:15:00.000Z');
    const single = await s.step('receipt-single', () =>
      paymentService.createPayment({
        date: '2026-06-30T09:15:00.000Z',
        type: 'RECEIVED',
        targetType: 'customer',
        targetId: customerId,
        amount: 249,
        method: 'cash',
        allocations: [{ targetKind: 'invoice', targetId: b.id, amount: 249 }],
      }),
    );
    await s.step('get-single', () => paymentService.getPayment(single.id));
    await s.step('invoice-b-paid', () => invoiceService.getInvoice(b.id));
    await s.step('open-docs-after', () => paymentService.getOpenDocuments('customer', customerId));
  },
});
