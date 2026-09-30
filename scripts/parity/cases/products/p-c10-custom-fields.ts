/**
 * 06-products §8(b) P-C10: custom field definitions — validation messages, sortOrder = count + 1,
 * update merges `options`, reorder is partial (unknown ids ignored), a field used by a product can't
 * be deleted, an unused one can.
 */
import { defineCase } from '../../case';
import * as catalogService from '../../../../src/modules/products/services/catalogService';
import * as productService from '../../../../src/modules/products/services/productService';

export default defineCase({
  name: 'products/p-c10-custom-fields',
  source: '03-domains/06-products.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    await s.expectError('name-required', () => catalogService.saveCustomFieldDef({ name: '  ', type: 'text', active: true }));
    await s.expectError('list-without-options', () => catalogService.saveCustomFieldDef({ name: 'المقاس', type: 'list', options: [], active: true }));
    const fabric = await s.step('create-text', () => catalogService.saveCustomFieldDef({ name: '  الخامة  ', type: 'text', active: true }));
    const size = await s.step('create-list', () => catalogService.saveCustomFieldDef({ name: 'المقاس', type: 'list', options: ['S', 'M', 'L'], active: true }));
    const year = await s.step('create-number', () => catalogService.saveCustomFieldDef({ name: 'سنة الموديل', type: 'number', active: true }));
    await s.expectError('update-missing', () => catalogService.saveCustomFieldDef({ name: 'س', type: 'text', active: true }, s.baseId('cus-1')));
    await s.step('update-list-options', () => catalogService.saveCustomFieldDef({ name: 'المقاس', type: 'list', options: ['S', 'M', 'L', 'XL'], active: true }, size.id));
    await s.step('deactivate-number', () => catalogService.saveCustomFieldDef({ name: 'سنة الموديل', type: 'number', active: false }, year.id));
    await s.step('reorder-partial', () => catalogService.reorderCustomFieldDefs([year.id, s.baseId('cus-1'), fabric.id]));
    await s.step('list-after-reorder', () => catalogService.getCustomFieldDefs());

    await s.step('product-using-field', () =>
      productService.createProduct({ name: 'قميص بخامة', sku: 'MEN-020', type: 'product', costPrice: 5, price: 9, active: true, customFields: { [fabric.id]: 'قطن' } }),
    );
    await s.expectError('delete-used', () => catalogService.deleteCustomFieldDef(fabric.id));
    await s.step('delete-unused', () => catalogService.deleteCustomFieldDef(year.id));
    await s.step('create-after-delete', () => catalogService.saveCustomFieldDef({ name: 'بلد المنشأ', type: 'text', active: true }));
    await s.step('list-final', () => catalogService.getCustomFieldDefs());
  },
});
