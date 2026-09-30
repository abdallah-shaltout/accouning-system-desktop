/**
 * 08 §8(b) quotation-save-convert — quotations never post: save (number, DRAFT, totals with an
 * invoice discount), list and search, flip the status, convert to a real invoice (ACCEPTED + link,
 * the sale posts), and the second conversion is a CONFLICT. Also the empty-lines and NOT_FOUND
 * refusals.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';

/** An unknown id in the only shape a real id has (a UUID — Rust decodes ids strictly, so a free-form
 * `'x-does-not-exist'` would test id parsing, not the lookup's §3 NOT_FOUND/VALIDATION refusal). */
const MISSING_ID = '00000000-0000-7000-8000-000000000001';

export default defineCase({
  name: 'invoices/quotation-save-convert',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const customerId = s.baseId('cus-7');
    await s.expectError('empty-lines', () => invoiceService.saveQuotation({ customerId, lines: [], discountRate: 0 }));
    const q = await s.step('save', () =>
      invoiceService.saveQuotation({
        customerId,
        expiryDate: '2026-07-15T21:00:00.000Z',
        lines: [
          { productId: s.baseId('prd-15'), qty: 10, price: 95 },
          { productId: s.baseId('prd-17'), qty: 10, price: 59, discount: 20 },
        ],
        discountRate: 5,
        note: 'زي مدرسي',
        terms: 'الدفع خلال 45 يوماً',
        poReference: 'PO-SCH-77',
      }),
    );
    await s.step('get', () => invoiceService.getQuotation(q.id));
    await s.step('list-search', () => invoiceService.getQuotations({ search: 'الرواد' }));
    await s.step('set-sent', () => invoiceService.setQuotationStatus(q.id, 'SENT'));
    await s.step('list-sent', () => invoiceService.getQuotations({ status: 'SENT' }));
    await s.setClock('2026-06-30T09:05:00.000Z');
    const invoice = await s.step('convert', () => invoiceService.convertQuotationToInvoice(q.id, { paymentMethod: 'credit', paidAmount: 0 }));
    await s.step('get-after-convert', () => invoiceService.getQuotation(q.id));
    await s.step('invoice-detail', () => invoiceService.getInvoice(invoice.id));
    await s.expectError('convert-twice', () => invoiceService.convertQuotationToInvoice(q.id, { paymentMethod: 'credit', paidAmount: 0 }));
    await s.expectError('get-unknown', () => invoiceService.getQuotation(MISSING_ID));
    await s.expectError('status-unknown', () => invoiceService.setQuotationStatus(MISSING_ID, 'SENT'));
  },
});
