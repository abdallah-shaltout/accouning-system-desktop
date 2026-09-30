/**
 * 08 §8(b) invoice-detail-joins — `getInvoice` joins: a credit sale to a customer, a receipt
 * allocated to it and a partial refund, then the detail carries the customer, the refund, the
 * allocated RECEIVED payment, the journal refs of the invoice + refund + payment, and `returnedQty`
 * per line. Plus the NOT_FOUND for an unknown id.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as paymentService from '../../../../src/modules/payments/services/paymentService';

/** An unknown id in the only shape a real id has (a UUID — Rust decodes ids strictly, so a free-form
 * `'x-does-not-exist'` would test id parsing, not the lookup's §3 NOT_FOUND/VALIDATION refusal). */
const MISSING_ID = '00000000-0000-7000-8000-000000000001';

export default defineCase({
  name: 'invoices/invoice-detail-joins',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const customerId = s.baseId('cus-6');
    const sale = await s.step('sale', () =>
      invoiceService.createSale({
        customerId,
        lines: [
          { productId: s.baseId('prd-11'), qty: 2, price: 129 },
          { productId: s.baseId('prd-16'), qty: 1, price: 119 },
        ],
        discountRate: 0,
        paymentMethod: 'credit',
        paidAmount: 0,
        source: 'DESK',
      }),
    );
    await s.setClock('2026-06-30T09:05:00.000Z');
    await s.step('payment', () =>
      paymentService.createPayment({
        date: '2026-06-30T09:05:00.000Z',
        type: 'RECEIVED',
        targetType: 'customer',
        targetId: customerId,
        amount: 150,
        method: 'cash',
        allocations: [{ targetKind: 'invoice', targetId: sale.id, amount: 150 }],
      }),
    );
    await s.setClock('2026-06-30T09:10:00.000Z');
    await s.step('refund', () => invoiceService.createRefund({ invoiceId: sale.id, lines: [{ invoiceLineId: sale.lines[1].id, qty: 1 }] }));
    await s.step('detail', () => invoiceService.getInvoice(sale.id));
    await s.expectError('unknown', () => invoiceService.getInvoice(MISSING_ID));
  },
});
