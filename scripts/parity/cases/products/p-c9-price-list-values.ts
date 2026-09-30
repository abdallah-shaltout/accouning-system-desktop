/**
 * 06-products §8(b) P-C9: setPriceListValues — `null` removes a product's entry, a value replaces it
 * (appended last), unknown product ids are skipped, one negative value refuses the call (Q-4: a
 * single negative key, so JS-vs-BTreeMap key order can't matter), an unknown list is NOT_FOUND.
 */
import { defineCase } from '../../case';
import * as catalogService from '../../../../src/modules/products/services/catalogService';
import * as productService from '../../../../src/modules/products/services/productService';

export default defineCase({
  name: 'products/p-c9-price-list-values',
  source: '03-domains/06-products.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const wholesale = s.baseId('pl-wholesale');
    const prd1 = s.baseId('prd-1');
    const prd2 = s.baseId('prd-2');
    await s.step('before-prd-1', () => productService.getProduct(prd1));
    await s.step('before-prd-2', () => productService.getProduct(prd2));
    // prd-1: remove its wholesale price; prd-2: set a new one.
    await s.step('null-and-value', () => catalogService.setPriceListValues(wholesale, { [prd1]: null, [prd2]: 150 }));
    await s.step('after-prd-1', () => productService.getProduct(prd1));
    await s.step('after-prd-2', () => productService.getProduct(prd2));
    // Re-setting prd-1 appends the entry after its other lists' entries.
    await s.step('re-set', () => catalogService.setPriceListValues(wholesale, { [prd1]: 111.5 }));
    await s.step('after-re-set', () => productService.getProduct(prd1));
    await s.step('unknown-product-skipped', () => catalogService.setPriceListValues(wholesale, { [s.baseId('cus-1')]: 5 }));
    await s.expectError('negative', () => catalogService.setPriceListValues(wholesale, { [s.baseId('prd-3')]: -1 }));
    await s.step('after-negative', () => productService.getProduct(s.baseId('prd-3')));
    await s.expectError('unknown-list', () => catalogService.setPriceListValues(s.baseId('cus-1'), { [prd1]: 5 }));
  },
});
