/**
 * 09 §8(b) payment-receipt-unallocated — a customer receipt with no allocation: Dr cash /
 * Cr receivable[customer] for the full amount, `unallocated = amount`, status `unallocated`, the
 * customer's open documents unchanged and the credit shown on the party.
 */
import { defineCase } from '../../case';
import * as paymentService from '../../../../src/modules/payments/services/paymentService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'payments/payment-receipt-unallocated',
  source: '03-domains/09-payments.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const customerId = s.baseId('cus-1');
    await s.step('open-docs-before', () => paymentService.getOpenDocuments('customer', customerId));
    const p = await s.step('create', () =>
      paymentService.createPayment({
        date: '2026-06-30T09:00:00.000Z',
        type: 'RECEIVED',
        targetType: 'customer',
        targetId: customerId,
        amount: 500.004,
        method: 'cash',
        note: 'دفعة مقدمة',
      }),
    );
    await s.step('get', () => paymentService.getPayment(p.id));
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('payment', p.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    await s.step('open-docs-after', () => paymentService.getOpenDocuments('customer', customerId));
    await s.step('customer-after', () => partyService.getCustomer(customerId));
  },
});
