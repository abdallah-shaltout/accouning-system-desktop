/**
 * 07-purchases §8(b) P-P7 (variance-guard part): returning more value than is on hand takes only the
 * stock value out of inventory and books the difference to inventoryVariance (07 U-9 step 6–7).
 * A fresh product is bought at 50 ×2, one is sold, one more is bought at 10 (qty 2, value 60, avg
 * 30); returning both 50-units (net 100 + VAT 15) credits inventory 60 → variance 40.
 *
 * KNOWN MOCK BUG (lane L2 report, 2026-09-29 — not fixed here, report-only): the variance line has
 * the wrong sign. `recordPurchaseReturn` (src/mocks/backend/purchases.ts) posts `variance > 0 → Dr
 * inventoryVariance`, but a positive variance (value due back from the supplier > stock value
 * removed) must be a credit: the entry comes out Dr payable 115 + Dr variance 40 = 155 vs Cr
 * inventory 60 + Cr vatInput 15 = 75, so `postJournal` refuses it ("القيد غير متوازن: المدين 155 ≠
 * الدائن 75"). The mock is not atomic, so the refused return also leaves the return row, stock and
 * returnedAmount changed without a journal (inventory-gl, vat-input, supplier-allocation break).
 * The negative branch is inverted the same way. 07 §3 U-9 step 7 copies the same rule into Rust.
 * This case fails on the mock until the rule is fixed in both backends (P4-7, needs an ACC- issue).
 */
import { defineCase } from '../../case';
import * as purchaseService from '../../../../src/modules/purchases/services/purchaseService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'purchases/p-p7-return-variance-guard',
  source: '03-domains/07-purchases.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: 'steps.return-with-variance.value.taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
    { path: 'steps.*.value.returns[].taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
  ],
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const sup = s.baseId('sup-1');
    const np = await s.step('new-product', () => productService.createProduct({ name: 'حزام قماش', sku: 'ACC-950', type: 'product', costPrice: 0, price: 80, active: true }));
    const poA = await s.step('po-a', () => purchaseService.savePurchaseOrder({ supplierId: sup, date, confirm: true, lines: [{ productId: np.id, qty: 2, costPrice: 50 }] }));
    await s.step('sell-one', () =>
      invoiceService.createSale({ source: 'DESK', lines: [{ productId: np.id, qty: 1, price: 80 }], discountRate: 0, paymentMethod: 'cash', paidAmount: 80 }),
    );
    const poB = await s.step('po-b', () => purchaseService.savePurchaseOrder({ supplierId: sup, date, confirm: true, lines: [{ productId: np.id, qty: 1, costPrice: 10 }] }));
    await s.step('before-return', () => productService.getProduct(np.id));
    const ret = await s.step('return-with-variance', () =>
      purchaseService.createPurchaseReturn({ purchaseOrderId: poA.id, reason: 'إرجاع كامل', lines: [{ productId: np.id, qty: 2 }] }),
    );
    await s.step('variance-journal', () => accountingService.getJournalEntriesForSource('purchaseReturn', ret.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('after-return', () => productService.getProduct(np.id));
    // Stock is 0 now → the other order's line can't be returned (U-9 CONFLICT).
    await s.expectError('stock-conflict', () => purchaseService.createPurchaseReturn({ purchaseOrderId: poB.id, reason: 'x', lines: [{ productId: np.id, qty: 1 }] }));
  },
});
