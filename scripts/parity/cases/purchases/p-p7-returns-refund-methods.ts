/**
 * 07-purchases §8(b) P-P7: purchase returns — each refund method on a paid order (cash / bank
 * transfer hit their account for the cash-back, credit keeps it on AP), `returnedAmount` and
 * `paymentStatus`, every U-9 message (not received, reason, nothing selected, not in the PO, more
 * than remaining, stock CONFLICT). The variance guard is `purchases/p-p7-return-variance-guard`
 * (split out: it exposes a mock sign bug, lane L2 report 2026-09-29).
 */
import { defineCase } from '../../case';
import * as purchaseService from '../../../../src/modules/purchases/services/purchaseService';
import * as paymentService from '../../../../src/modules/payments/services/paymentService';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'purchases/p-p7-returns-refund-methods',
  source: '03-domains/07-purchases.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: 'steps.return-cash.value.taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
    { path: 'steps.return-bank.value.taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
    { path: 'steps.return-credit-default.value.taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
    { path: 'steps.*.value.returns[].taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
  ],
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const sup = s.baseId('sup-1');
    const prd1 = s.baseId('prd-1');
    const po = await s.step('save-and-confirm', () =>
      purchaseService.savePurchaseOrder({ supplierId: sup, date, confirm: true, lines: [{ productId: prd1, qty: 6, costPrice: 45 }] }),
    );
    await s.step('pay-in-full', () =>
      paymentService.createPayment({
        date,
        type: 'PAID',
        targetType: 'supplier',
        targetId: sup,
        amount: po.grandTotal,
        method: 'bank_transfer',
        allocations: [{ targetKind: 'purchaseOrder', targetId: po.id, amount: po.grandTotal }],
      }),
    );
    const journalOf = (id: string) => accountingService.getJournalEntriesForSource('purchaseReturn', id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id))));

    await s.expectError('not-received', () => purchaseService.createPurchaseReturn({ purchaseOrderId: s.baseId('po-35'), reason: 'x', lines: [{ productId: prd1, qty: 1 }] }));
    await s.expectError('missing-po', () => purchaseService.createPurchaseReturn({ purchaseOrderId: s.baseId('cus-1'), reason: 'x', lines: [{ productId: prd1, qty: 1 }] }));
    await s.expectError('no-reason', () => purchaseService.createPurchaseReturn({ purchaseOrderId: po.id, reason: '  ', lines: [{ productId: prd1, qty: 1 }] }));
    await s.expectError('nothing-selected', () => purchaseService.createPurchaseReturn({ purchaseOrderId: po.id, reason: 'x', lines: [{ productId: prd1, qty: 0 }] }));
    await s.expectError('not-in-po', () => purchaseService.createPurchaseReturn({ purchaseOrderId: po.id, reason: 'x', lines: [{ productId: s.baseId('prd-2'), qty: 1 }] }));
    await s.expectError('more-than-remaining', () => purchaseService.createPurchaseReturn({ purchaseOrderId: po.id, reason: 'x', lines: [{ productId: prd1, qty: 7 }] }));

    const rCash = await s.step('return-cash', () =>
      purchaseService.createPurchaseReturn({ purchaseOrderId: po.id, reason: 'مقاس خاطئ', refundMethod: 'cash', lines: [{ productId: prd1, qty: 1 }] }),
    );
    await s.step('return-cash-journal', () => journalOf(rCash.id));
    const rBank = await s.step('return-bank', () =>
      purchaseService.createPurchaseReturn({ purchaseOrderId: po.id, reason: 'لون خاطئ', refundMethod: 'bank_transfer', lines: [{ productId: prd1, qty: 2 }] }),
    );
    await s.step('return-bank-journal', () => journalOf(rBank.id));
    const rCredit = await s.step('return-credit-default', () =>
      purchaseService.createPurchaseReturn({ purchaseOrderId: po.id, reason: 'تالف', lines: [{ productId: prd1, qty: 1 }] }),
    );
    await s.step('return-credit-journal', () => journalOf(rCredit.id));
    await s.expectError('more-than-remaining-after', () => purchaseService.createPurchaseReturn({ purchaseOrderId: po.id, reason: 'x', lines: [{ productId: prd1, qty: 3 }] }));
    await s.step('po-after-returns', () => purchaseService.getPurchaseOrder(po.id));

    // Stock CONFLICT: a received line whose stock has since been sold can't be returned.
    const poSvc = await s.step('po-shoes', () =>
      purchaseService.savePurchaseOrder({ supplierId: sup, date, confirm: true, lines: [{ productId: s.baseId('prd-6'), qty: 1, costPrice: 180 }] }),
    );
    await s.step('sell-all-jackets', () =>
      invoiceService.createSale({ source: 'DESK', lines: [{ productId: s.baseId('prd-6'), qty: 4, price: 389 }], discountRate: 0, paymentMethod: 'cash', paidAmount: 1556 }),
    );
    await s.expectError('stock-conflict', () => purchaseService.createPurchaseReturn({ purchaseOrderId: poSvc.id, reason: 'x', lines: [{ productId: s.baseId('prd-6'), qty: 1 }] }));
  },
});
