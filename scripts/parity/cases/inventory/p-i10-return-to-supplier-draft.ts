/**
 * 06b-inventory §8(b) P-I10: the expiry report's "return to supplier" first step — a DebitNoteDraft
 * (no posting, no stock change; Q-I7: supplier and batches not validated, client unitCost stored),
 * the empty-lines message, and the drafts list. Continued by purchases/p-p8 (post the draft).
 */
import { defineCase } from '../../case';
import * as inventoryService from '../../../../src/modules/products/services/inventoryService';
import * as purchaseService from '../../../../src/modules/purchases/services/purchaseService';
import * as productService from '../../../../src/modules/products/services/productService';

export default defineCase({
  name: 'inventory/p-i10-return-to-supplier-draft',
  source: '03-domains/06b-inventory.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: 'steps.**.prices', emptyArrayOnly: true, reason: '06-products Q-7: Product.prices is [] in Rust for a product that has none; the mock leaves it absent on seed rows never saved through the form ([] ≡ absent for this field)' },
  ],
  async run(s) {
    const panadol = s.baseId('prd-panadol');
    const sup = s.baseId('sup-5');
    await s.expectError('no-lines', () => inventoryService.returnBatchesToSupplier(sup, []));
    const d1 = await s.step('draft-1', () =>
      inventoryService.returnBatchesToSupplier(sup, [{ productId: panadol, batchId: s.baseId('batch-1'), qty: 60, unitCost: 6.5 }], 'منتهي الصلاحية'),
    );
    await s.step('draft-2', () =>
      inventoryService.returnBatchesToSupplier(s.baseId('sup-1'), [
        { productId: panadol, batchId: s.baseId('batch-2'), qty: 4, unitCost: 6.25 },
        { productId: panadol, batchId: s.baseId('batch-1'), qty: 1, unitCost: 6.5 },
      ]),
    );
    await s.step('drafts-inventory', () => inventoryService.getDebitNoteDrafts());
    await s.step('drafts-purchases', () => purchaseService.getDebitNoteDrafts());
    await s.step('panadol-unchanged', () => productService.getProduct(panadol));
    await s.step('batches-unchanged', () => inventoryService.getBatches(panadol));
    // The seed batches came from a stock adjustment, not a received PO → posting is refused and the
    // draft stays (the success path is purchases/p-p8-debit-note-from-draft).
    await s.expectError('post-non-po-batch', () => purchaseService.postDebitNoteDraft(d1.id, 'credit'));
    await s.expectError('post-missing-draft', () => purchaseService.postDebitNoteDraft(s.baseId('cus-1'), 'credit'));
    await s.step('drafts-after', () => purchaseService.getDebitNoteDrafts());
  },
});
