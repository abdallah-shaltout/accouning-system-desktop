/**
 * 05-parties §8(b): statements end on the computed balance; a new credit sale and a receipt show up
 * as rows in posting order; a supplier statement with purchases and payments; unknown ids return []
 * (Q-6); linked net balance with one side absent → undefined.
 */
import { defineCase } from '../../case';
import * as partyService from '../../../../src/modules/parties/services/partyService';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as paymentService from '../../../../src/modules/payments/services/paymentService';

export default defineCase({
  name: 'parties/parties-statement-and-balance',
  source: '03-domains/05-parties.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const cus = s.baseId('cus-8');
    await s.step('statement-before', () => partyService.getCustomerStatement(cus));
    await s.step('balance-before', () => partyService.getCustomer(cus));
    const inv = await s.step('credit-sale', () =>
      invoiceService.createSale({
        customerId: cus,
        source: 'DESK',
        lines: [{ productId: s.baseId('prd-3'), qty: 2, price: 79 }],
        discountRate: 0,
        paymentMethod: 'credit',
        paidAmount: 0,
      }),
    );
    await s.step('receipt', () =>
      paymentService.createPayment({
        date: '2026-06-30T09:00:00.000Z',
        type: 'RECEIVED',
        targetType: 'customer',
        targetId: cus,
        amount: 100,
        method: 'cash',
        allocations: [{ targetKind: 'invoice', targetId: inv.id, amount: 100 }],
      }),
    );
    await s.step('statement-after', () => partyService.getCustomerStatement(cus));
    await s.step('balance-after', () => partyService.getCustomer(cus));

    const sup = s.baseId('sup-2');
    await s.step('supplier-statement', () => partyService.getSupplierStatement(sup));
    await s.step('supplier-balance', () => partyService.getSupplier(sup));

    await s.step('statement-unknown', () => partyService.getCustomerStatement(s.baseId('sup-3')));
    await s.step('net-one-side', () => partyService.getLinkedNetBalance(cus, undefined));
    await s.step('net-pair', () => partyService.getLinkedNetBalance(cus, sup));
  },
});
