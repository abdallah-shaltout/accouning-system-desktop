/**
 * 07-purchases §8(b) P-P6: receiving a batch-tracked product — the requested batches are created at
 * round2(posted / qty) (landed share included), a receipt without batches gets one default
 * `RCV-{number}` batch; getActiveBatches is FEFO; a unit line (box of 3) receives in base units, and a
 * return against it counts base units too (3 strips at the strip cost, FEFO batch draw).
 */
import { defineCase } from '../../case';
import * as purchaseService from '../../../../src/modules/purchases/services/purchaseService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'purchases/p-p6-tracked-batches',
  source: '03-domains/07-purchases.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: 'steps.panadol.value.prices', emptyArrayOnly: true, reason: '06-products Q-7: Product.prices is [] in Rust for a product that has none; the mock leaves it absent on seed rows never saved through the form ([] ≡ absent for this field)' },
    { path: 'steps.panadol-after-return.value.prices', emptyArrayOnly: true, reason: '06-products Q-7: Product.prices is [] in Rust for a product that has none; the mock leaves it absent on seed rows never saved through the form ([] ≡ absent for this field)' },
    { path: 'steps.return-box-in-strips.value.taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
    { path: 'steps.detail-after-return.value.returns[].taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
  ],
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const sup = s.baseId('sup-5');
    const panadol = s.baseId('prd-panadol');
    // 10 boxes of 3 strips at 18.5 a box → 30 strips; landed 7 by qty.
    const po = await s.step('save-draft', () =>
      purchaseService.savePurchaseOrder({
        supplierId: sup,
        date,
        confirm: false,
        lines: [{ productId: panadol, qty: 10, costPrice: 18.5, unitId: 'pu-panadol-box', unitFactor: 3 }],
        landedCosts: [{ label: 'شحن', amount: 7, spreadBy: 'qty' }],
      }),
    );
    await s.step('receive-with-batches', () =>
      purchaseService.receivePurchaseOrder(po.id, {
        date,
        lines: [
          {
            productId: panadol,
            receivedQty: 30,
            batches: [
              { batchNo: ' PND-27A ', expiryDate: '2027-01-31', qty: 18 },
              { batchNo: 'PND-27B', expiryDate: '2026-11-30', qty: 12 },
              { batchNo: 'PND-ZERO', qty: 0 },
            ],
          },
        ],
      }),
    );
    await s.step('detail', () => purchaseService.getPurchaseOrder(po.id));
    await s.step('active-batches', () => purchaseService.getActiveBatches(panadol));
    await s.step('panadol', () => productService.getProduct(panadol));

    // A return of the box line counts BASE units (strips) — what `returnedQty` and stock hold (the
    // contract `PurchaseReturnPage` shows since ACC-0032's audit): 3 strips = 1 box leave stock at the
    // strip cost (18.5 / 3 + the 7 / 30 landed share), drawn FEFO from the earliest-expiry batch.
    const ret = await s.step('return-box-in-strips', () =>
      purchaseService.createPurchaseReturn({ purchaseOrderId: po.id, reason: 'علبة تالفة', refundMethod: 'credit', lines: [{ productId: panadol, qty: 3 }] }),
    );
    await s.step('return-journal', () =>
      accountingService.getJournalEntriesForSource('purchaseReturn', ret.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))),
    );
    await s.step('detail-after-return', () => purchaseService.getPurchaseOrder(po.id));
    await s.step('active-batches-after-return', () => purchaseService.getActiveBatches(panadol));
    await s.step('panadol-after-return', () => productService.getProduct(panadol));

    // No batches requested → one default batch named after the PO.
    await s.step('save-and-confirm-default-batch', () =>
      purchaseService.savePurchaseOrder({ supplierId: sup, date, confirm: true, lines: [{ productId: panadol, qty: 6, costPrice: 6.25 }] }),
    );
    await s.step('active-batches-after', () => purchaseService.getActiveBatches(panadol));
    await s.step('untracked-product-batches', () => purchaseService.getActiveBatches(s.baseId('prd-1')));
  },
});
