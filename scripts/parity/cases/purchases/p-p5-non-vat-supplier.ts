/**
 * 07-purchases §8(b) P-P5: a supplier without a VAT number — input VAT is not recoverable (review E3):
 * the receipt debits it to freightIn (not vatInput, not stock value), the PO is flagged
 * `vatNotRecoverable`; the later return credits freightIn back. An explicit
 * `vatNotRecoverable: false` on a receipt overrides the default.
 */
import { defineCase } from '../../case';
import * as purchaseService from '../../../../src/modules/purchases/services/purchaseService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'purchases/p-p5-non-vat-supplier',
  source: '03-domains/07-purchases.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: 'steps.return.value.taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
    { path: 'steps.return-detail.value.taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
    { path: 'steps.*.value.returns[].taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
  ],
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const sup = s.baseId('sup-5'); // no VAT number in the seed
    const prd16 = s.baseId('prd-16');
    const po = await s.step('save-and-confirm', () =>
      purchaseService.savePurchaseOrder({ supplierId: sup, date, confirm: true, lines: [{ productId: prd16, qty: 10, costPrice: 50 }] }),
    );
    await s.step('detail', () => purchaseService.getPurchaseOrder(po.id));
    await s.step('journal', () => accountingService.getJournalEntriesForSource('purchaseOrder', po.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('product-16', () => productService.getProduct(prd16));
    const ret = await s.step('return', () =>
      purchaseService.createPurchaseReturn({ purchaseOrderId: po.id, reason: 'عيب تصنيع', lines: [{ productId: prd16, qty: 4 }] }),
    );
    await s.step('return-detail', () => purchaseService.getPurchaseReturn(ret.id));
    await s.step('return-journal', () => accountingService.getJournalEntriesForSource('purchaseReturn', ret.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('detail-after-return', () => purchaseService.getPurchaseOrder(po.id));

    // Same supplier, but the receiver says VAT is recoverable this time.
    const po2 = await s.step('save-draft', () =>
      purchaseService.savePurchaseOrder({ supplierId: sup, date, confirm: false, lines: [{ productId: prd16, qty: 2, costPrice: 50 }] }),
    );
    await s.step('receive-recoverable', () =>
      purchaseService.receivePurchaseOrder(po2.id, { date, vatNotRecoverable: false, lines: [{ productId: prd16, receivedQty: 2 }] }),
    );
    await s.step('journal-2', () => accountingService.getJournalEntriesForSource('purchaseOrder', po2.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.expectError('return-missing', () => purchaseService.getPurchaseReturn(s.baseId('cus-1')));
  },
});
