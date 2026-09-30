/**
 * 09 §8(b) payment-allocate-later-fx — "allocate later" on FC documents: a USD receipt recorded
 * unallocated (full cash against AR), later allocated to a USD invoice booked at a lower rate → a
 * second small entry (Dr AR / Cr FX gain) and `fxGainLoss` accumulated on the payment. A second USD
 * receipt allocated later to an invoice at the same rate posts nothing. Removing the FX-realizing
 * allocation is refused (Q-P1 decision, ACC-0001), the zero-FX one is removable.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as paymentService from '../../../../src/modules/payments/services/paymentService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'payments/payment-allocate-later-fx',
  source: '03-domains/09-payments.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const customerId = s.baseId('cus-usd-1');
    const fcSale = (rate: number, price: number) => ({
      customerId,
      lines: [{ productId: 'freetext', qty: 1, price, isFreeText: true, revenueAccountId: s.baseId('acc-4110'), name: 'استشارات' }],
      discountRate: 0,
      paymentMethod: 'credit' as const,
      paidAmount: 0,
      source: 'DESK' as const,
      currency: 'USD',
      exchangeRate: rate,
    });
    const usdReceipt = (at: string, amountFc: number) => ({
      date: at,
      type: 'RECEIVED' as const,
      targetType: 'customer' as const,
      targetId: customerId,
      amount: amountFc * 49.2,
      method: 'bank_transfer' as const,
      currency: 'USD',
      amountFc,
      rate: 49.2,
    });

    const inv1 = await s.step('sale-at-48.5', () => invoiceService.createSale(fcSale(48.5, 500)));
    await s.setClock('2026-06-30T09:05:00.000Z');
    const inv2 = await s.step('sale-at-49.2', () => invoiceService.createSale(fcSale(49.2, 200)));

    await s.setClock('2026-06-30T09:10:00.000Z');
    const p1 = await s.step('receipt-1-unallocated', () => paymentService.createPayment(usdReceipt('2026-06-30T09:10:00.000Z', 500)));
    await s.setClock('2026-06-30T09:15:00.000Z');
    const p1b = await s.step('allocate-1-later', () =>
      paymentService.allocateExistingPayment(p1.id, [{ targetKind: 'invoice', targetId: inv1.id, amount: 24600 }]),
    );
    const refs1 = await s.step('journal-refs-1', () => accountingService.getJournalEntriesForSource('payment', p1.id));
    for (let i = 0; i < refs1.length; i++) await s.step(`journal-1-${i}`, () => accountingService.getJournalEntry(refs1[i].id));

    await s.setClock('2026-06-30T09:20:00.000Z');
    const p2 = await s.step('receipt-2-unallocated', () => paymentService.createPayment(usdReceipt('2026-06-30T09:20:00.000Z', 200)));
    await s.setClock('2026-06-30T09:25:00.000Z');
    const p2b = await s.step('allocate-2-later', () =>
      paymentService.allocateExistingPayment(p2.id, [{ targetKind: 'invoice', targetId: inv2.id, amount: 9840 }]),
    );
    await s.step('journal-refs-2', () => accountingService.getJournalEntriesForSource('payment', p2.id));
    await s.step('invoice-1', () => invoiceService.getInvoice(inv1.id));
    await s.step('invoice-2', () => invoiceService.getInvoice(inv2.id));

    await s.expectError('remove-fx-allocation', () => paymentService.removeAllocation(p1.id, p1b.allocations[0].id));
    await s.step('remove-zero-fx-allocation', () => paymentService.removeAllocation(p2.id, p2b.allocations[0].id));
    await s.step('get-2-after-remove', () => paymentService.getPayment(p2.id));
  },
});
