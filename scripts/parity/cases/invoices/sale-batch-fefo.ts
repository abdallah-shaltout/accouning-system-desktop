/**
 * 08 §8(b) sale-batch-fefo — a batch-tracked product: a line with a manually picked batch draws
 * that batch first; a line without one draws FEFO and skips the expired batch (PND-24A expired
 * 2026-05-31, before the seed's today).
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as inventoryService from '../../../../src/modules/products/services/inventoryService';
import * as productService from '../../../../src/modules/products/services/productService';

export default defineCase({
  name: 'invoices/sale-batch-fefo',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: 'steps.*.value.prices', emptyArrayOnly: true, reason: '06-products Q-7: Product.prices is [] in Rust for a product that has none; the mock leaves it absent on seed rows never saved through the form ([] ≡ absent for this field)' },
  ],
  async run(s) {
    const productId = s.baseId('prd-panadol');
    const product = await s.step('product-before', () => productService.getProduct(productId));
    await s.step('batches-before', () => inventoryService.getBatches(productId));
    const manual = await s.step('sale-manual-batch', () =>
      invoiceService.createSale({
        lines: [{ productId, qty: 5, price: product.price, batchId: s.baseId('batch-2'), batchNo: 'PND-24B' }],
        discountRate: 0,
        paymentMethod: 'cash',
        paidAmount: 1000,
      }),
    );
    await s.step('detail-manual', () => invoiceService.getInvoice(manual.id));
    await s.step('batches-after-manual', () => inventoryService.getBatches(productId));
    await s.setClock('2026-06-30T09:05:00.000Z');
    await s.step('sale-fefo', () =>
      invoiceService.createSale({ lines: [{ productId, qty: 10, price: product.price }], discountRate: 0, paymentMethod: 'cash', paidAmount: 1000 }),
    );
    await s.step('batches-after-fefo', () => inventoryService.getBatches(productId));
    await s.step('product-after', () => productService.getProduct(productId));
  },
});
