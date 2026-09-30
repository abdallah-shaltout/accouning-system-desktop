/**
 * 09 §8(b) payment-supplier-po — a supplier payment against a received, unpaid purchase order (the
 * mirror of a receipt): Dr payable[supplier] / Cr bank, the PO's `paidAmount`/`paymentStatus`
 * updated, the supplier's open documents before and after.
 */
import { defineCase } from '../../case';
import * as paymentService from '../../../../src/modules/payments/services/paymentService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'payments/payment-supplier-po',
  source: '03-domains/09-payments.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const supplierId = s.baseId('sup-1');
    const po = s.baseId('po-30');
    await s.step('open-docs-before', () => paymentService.getOpenDocuments('supplier', supplierId));
    const p = await s.step('create', () =>
      paymentService.createPayment({
        date: '2026-06-30T09:00:00.000Z',
        type: 'PAID',
        targetType: 'supplier',
        targetId: supplierId,
        amount: 2000,
        method: 'bank_transfer',
        note: 'دفعة من أمر الشراء',
        allocations: [{ targetKind: 'purchaseOrder', targetId: po, amount: 2000 }],
      }),
    );
    await s.step('get', () => paymentService.getPayment(p.id));
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('payment', p.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    await s.step('open-docs-after', () => paymentService.getOpenDocuments('supplier', supplierId));
  },
});
