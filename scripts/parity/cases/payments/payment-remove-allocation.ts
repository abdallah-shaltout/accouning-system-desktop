/**
 * 09 §8(b) payment-remove-allocation — removing an allocation restores the invoice's paid amount
 * and status, drops the row, moves `targetRef` to the next remaining allocation (or clears it), and
 * posts no GL entry (the payment keeps its single entry). Unknown payment / allocation ids are
 * NOT_FOUND.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as paymentService from '../../../../src/modules/payments/services/paymentService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

/** An unknown id in the only shape a real id has (a UUID — Rust decodes ids strictly, so a free-form
 * `'x-does-not-exist'` would test id parsing, not the lookup's §3 NOT_FOUND/VALIDATION refusal). */
const MISSING_ID = '00000000-0000-7000-8000-000000000001';

export default defineCase({
  name: 'payments/payment-remove-allocation',
  source: '03-domains/09-payments.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const customerId = s.baseId('cus-3');
    const sale = (id: string, price: number) => ({
      customerId,
      lines: [{ productId: s.baseId(id), qty: 1, price }],
      discountRate: 0,
      paymentMethod: 'credit' as const,
      paidAmount: 0,
      source: 'DESK' as const,
    });
    const a = await s.step('sale-a', () => invoiceService.createSale(sale('prd-12', 229)));
    await s.setClock('2026-06-30T09:05:00.000Z');
    const b = await s.step('sale-b', () => invoiceService.createSale(sale('prd-20', 269)));
    await s.setClock('2026-06-30T09:10:00.000Z');
    const p = await s.step('receipt', () =>
      paymentService.createPayment({
        date: '2026-06-30T09:10:00.000Z',
        type: 'RECEIVED',
        targetType: 'customer',
        targetId: customerId,
        amount: 229,
        method: 'cash',
        allocations: [{ targetKind: 'invoice', targetId: a.id, amount: 229 }],
      }),
    );
    await s.setClock('2026-06-30T09:15:00.000Z');
    await s.expectError('allocate-over-total', () => paymentService.allocateExistingPayment(p.id, [{ targetKind: 'invoice', targetId: b.id, amount: 10 }]));
    await s.step('remove', () => paymentService.removeAllocation(p.id, p.allocations[0].id));
    await s.step('get-after-remove', () => paymentService.getPayment(p.id));
    await s.step('invoice-a-after', () => invoiceService.getInvoice(a.id));
    await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('payment', p.id));
    await s.step('reallocate', () => paymentService.allocateExistingPayment(p.id, [{ targetKind: 'invoice', targetId: b.id, amount: 229 }]));
    await s.step('invoice-b-after', () => invoiceService.getInvoice(b.id));
    await s.expectError('remove-unknown-payment', () => paymentService.removeAllocation(MISSING_ID, MISSING_ID));
    await s.expectError('remove-unknown-allocation', () => paymentService.removeAllocation(p.id, MISSING_ID));
  },
});
