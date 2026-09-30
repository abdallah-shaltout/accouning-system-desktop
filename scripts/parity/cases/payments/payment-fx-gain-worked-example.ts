/**
 * 09 §8(b) payment-fx-gain-worked-example — the docs/v2/10 §2 numbers: a USD 1,000 invoice at
 * 48.50 (AR 48,500) settled by a USD 1,000 receipt at 49.20 (cash 49,200): one entry Dr bank
 * 49,200 / Cr AR[customer] 48,500 / Cr FX gain 700, the allocation carries `amountFc` 1,000 and
 * `fxGainLoss` 700, the invoice is PAID and the party's balance nets to 0. The credit limit of the
 * USD customer is exceeded by the sale, which admin (accounting write) may override.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as paymentService from '../../../../src/modules/payments/services/paymentService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'payments/payment-fx-gain-worked-example',
  source: '03-domains/09-payments.md §8(b) (docs/v2/10 §2)',
  base: 'demo-sa',
  async run(s) {
    const customerId = s.baseId('cus-usd-1');
    const invoice = await s.step('sale-usd', () =>
      invoiceService.createSale({
        customerId,
        lines: [{ productId: 'freetext', qty: 1, price: 1000, isFreeText: true, revenueAccountId: s.baseId('acc-4110'), name: 'خدمات استشارية' }],
        discountRate: 0,
        paymentMethod: 'credit',
        paidAmount: 0,
        source: 'DESK',
        currency: 'USD',
        exchangeRate: 48.5,
      }),
    );
    await s.setClock('2026-06-30T09:05:00.000Z');
    const p = await s.step('receipt-usd', () =>
      paymentService.createPayment({
        date: '2026-06-30T09:05:00.000Z',
        type: 'RECEIVED',
        targetType: 'customer',
        targetId: customerId,
        amount: 49200,
        method: 'bank_transfer',
        currency: 'USD',
        amountFc: 1000,
        rate: 49.2,
        allocations: [{ targetKind: 'invoice', targetId: invoice.id, amount: 49200 }],
      }),
    );
    await s.step('get', () => paymentService.getPayment(p.id));
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('payment', p.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    await s.step('invoice-after', () => invoiceService.getInvoice(invoice.id));
    await s.step('customer-after', () => partyService.getCustomer(customerId));
  },
});
