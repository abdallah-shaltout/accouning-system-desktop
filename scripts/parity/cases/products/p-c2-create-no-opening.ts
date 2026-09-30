/**
 * 06-products §8(b) P-C2: create without an opening qty — the normalized DTO (trimmed name/SKU,
 * empty optional strings dropped, service `minStock` cleared, `prices` filtered, unit barcodes
 * filtered), no stock, no adjustment, no journal.
 */
import { defineCase } from '../../case';
import * as productService from '../../../../src/modules/products/services/productService';
import * as inventoryService from '../../../../src/modules/products/services/inventoryService';

export default defineCase({
  name: 'products/p-c2-create-no-opening',
  source: '03-domains/06-products.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const p = await s.step('create-product', () =>
      productService.createProduct({
        name: '  قميص كتان صيفي  ',
        nameEn: '   ',
        sku: '  MEN-008 ',
        barcode: ' 6289990000011 ',
        categoryId: s.baseId('cat-men'),
        unitId: s.baseId('unit-piece'),
        type: 'product',
        costPrice: 40,
        price: 99,
        minStock: 5,
        minPrice: 80,
        active: true,
        tags: ['صيفي', '', 'كتان'],
        prices: [{ priceListId: s.baseId('pl-wholesale'), value: 85 }],
        units: [
          { id: 'u-1', unitId: s.baseId('unit-piece'), factor: 1, barcodes: ['6289990000011', ''], price: 99, priceIsAuto: true, defaultForSale: true, defaultForPurchase: true, active: true },
          { id: 'u-2', unitId: s.baseId('unit-box'), factor: 6, barcodes: ['6289990000028'], price: 560, priceIsAuto: false, defaultForSale: false, defaultForPurchase: false, active: true },
        ],
        openingQty: 0,
      }),
    );
    const svc = await s.step('create-service', () =>
      productService.createProduct({ name: 'كي وتغليف', sku: 'SRV-003', type: 'service', costPrice: 0, price: 10, minStock: 3, active: true, categoryId: s.baseId('cat-services') }),
    );
    // A service with an opening qty never gets stock (only `type: 'product'` does).
    const svc2 = await s.step('create-service-with-opening', () =>
      productService.createProduct({ name: 'توصيل', sku: 'SRV-004', type: 'service', costPrice: 0, price: 20, active: true, openingQty: 5 }),
    );
    // A non-stock item with an opening qty never gets stock either (stockMode 'none').
    const ns = await s.step('create-non-stock-with-opening', () =>
      productService.createProduct({ name: 'كيس تسوق', sku: 'BAG-001', type: 'product', stockMode: 'none', costPrice: 0.5, price: 1, active: true, openingQty: 100 }),
    );
    await s.step('get-product', () => productService.getProduct(p.id));
    await s.step('get-service', () => productService.getProduct(svc.id));
    await s.step('get-service-2', () => productService.getProduct(svc2.id));
    await s.step('get-non-stock', () => productService.getProduct(ns.id));
    await s.step('adjustments', () => inventoryService.getStockAdjustments());
  },
});
