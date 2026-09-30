/**
 * 06-products §8(b) P-C5: every C-5 validation message, in the documented order, on create and on
 * update (the update excludes the product itself from the uniqueness checks).
 */
import { defineCase } from '../../case';
import * as productService from '../../../../src/modules/products/services/productService';
import type { ProductInput, ProductUnit } from '../../../../src/modules/products/types';

function unit(id: string, unitId: string, factor: number, barcodes: string[]): ProductUnit {
  return { id, unitId, factor, barcodes, price: 10 * factor, priceIsAuto: true, defaultForSale: factor === 1, defaultForPurchase: factor === 1, active: true };
}

export default defineCase({
  name: 'products/p-c5-validation-each',
  source: '03-domains/06-products.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const piece = s.baseId('unit-piece');
    const box = s.baseId('unit-box');
    const ok: ProductInput = { name: 'منتج اختبار', sku: 'TST-001', type: 'product', costPrice: 5, price: 10, active: true };
    await s.expectError('name-required', () => productService.createProduct({ ...ok, name: '  ' }));
    await s.expectError('sku-required', () => productService.createProduct({ ...ok, sku: ' ' }));
    await s.expectError('sku-duplicate-case-insensitive', () => productService.createProduct({ ...ok, sku: ' men-001 ' }));
    await s.expectError('barcode-duplicate', () => productService.createProduct({ ...ok, barcode: '6281234000019' }));
    await s.expectError('unit-barcode-duplicate', () =>
      productService.createProduct({ ...ok, units: [unit('u1', piece, 1, ['6281234090027'])] }),
    );
    await s.expectError('negative-price', () => productService.createProduct({ ...ok, price: -1 }));
    await s.expectError('negative-cost', () => productService.createProduct({ ...ok, costPrice: -0.01 }));
    await s.expectError('min-price-above-price', () => productService.createProduct({ ...ok, minPrice: 10.5 }));
    await s.expectError('factor-zero', () =>
      productService.createProduct({ ...ok, units: [unit('u1', piece, 1, []), unit('u2', box, 0, [])] }),
    );
    await s.expectError('two-base-units', () =>
      productService.createProduct({ ...ok, units: [unit('u1', piece, 1, []), unit('u2', box, 1, [])] }),
    );
    await s.expectError('no-base-unit', () => productService.createProduct({ ...ok, units: [unit('u2', box, 6, [])] }));
    await s.expectError('own-barcode-duplicate', () =>
      productService.createProduct({ ...ok, units: [unit('u1', piece, 1, ['6289991111110']), unit('u2', box, 6, ['6289991111110'])] }),
    );
    // Order: SKU conflict wins over a negative price in the same input.
    await s.expectError('sku-before-price', () => productService.createProduct({ ...ok, sku: 'MEN-001', price: -5 }));

    const created = await s.step('create-ok', () => productService.createProduct({ ...ok, barcode: '6289992222229' }));
    // Update: its own SKU/barcode don't conflict with itself…
    await s.step('update-self-codes', () => productService.updateProduct(created.id, { ...ok, sku: 'tst-001', barcode: '6289992222229', price: 12 }));
    // …but another product's do.
    await s.expectError('update-sku-of-other', () => productService.updateProduct(created.id, { ...ok, sku: 'MEN-002' }));
    await s.expectError('update-barcode-of-other', () => productService.updateProduct(created.id, { ...ok, barcode: '6281234000026' }));
    await s.expectError('update-name-required', () => productService.updateProduct(created.id, { ...ok, name: '' }));
    await s.step('after', () => productService.getProduct(created.id));
  },
});
