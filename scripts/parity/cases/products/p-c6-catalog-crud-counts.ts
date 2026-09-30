/**
 * 06-products §8(b) P-C6: category / unit / price-list CRUD — exact-name conflicts (case-sensitive:
 * "Food" and "food" coexist), blank names, not-found updates, delete guards for linked rows, and the
 * `productCount` columns (Q-2: counts inactive products, base unit only).
 */
import { defineCase } from '../../case';
import * as catalogService from '../../../../src/modules/products/services/catalogService';
import * as productService from '../../../../src/modules/products/services/productService';

export default defineCase({
  name: 'products/p-c6-catalog-crud-counts',
  source: '03-domains/06-products.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const missing = s.baseId('cus-1'); // an id that is no catalog row on either backend

    // Categories
    await s.step('categories-before', () => catalogService.getCategories());
    const food = await s.step('category-create', () => catalogService.saveCategory('  Food  '));
    await s.step('category-create-other-case', () => catalogService.saveCategory('food'));
    await s.expectError('category-blank', () => catalogService.saveCategory('   '));
    await s.expectError('category-duplicate', () => catalogService.saveCategory('Food'));
    await s.expectError('category-rename-to-existing', () => catalogService.saveCategory('أحذية', food.id));
    await s.expectError('category-update-missing', () => catalogService.saveCategory('جديد', missing));
    await s.step('category-rename-with-defaults', () =>
      catalogService.saveCategory('أغذية', food.id, { purchaseAccountId: s.baseId('acc-5130'), saleTaxId: s.baseId('tax-zero-sales') }),
    );
    // A later rename without defaults keeps them (merge).
    await s.step('category-rename-keep-defaults', () => catalogService.saveCategory('أغذية ومشروبات', food.id));
    await s.step('product-in-category', () =>
      productService.createProduct({ name: 'عصير', sku: 'FD-001', type: 'product', costPrice: 2, price: 4, active: false, categoryId: food.id, unitId: s.baseId('unit-box') }),
    );
    await s.expectError('category-delete-linked', () => catalogService.deleteCategory(s.baseId('cat-men')));
    await s.expectError('category-delete-linked-inactive-product', () => catalogService.deleteCategory(food.id));
    const empty = await s.step('category-create-empty', () => catalogService.saveCategory('مؤقت'));
    await s.step('category-delete', () => catalogService.deleteCategory(empty.id));
    await s.step('category-name-free-again', () => catalogService.saveCategory('مؤقت'));
    await s.step('categories-after', () => catalogService.getCategories());

    // Units
    await s.step('units-before', () => catalogService.getUnits());
    const kg = await s.step('unit-create', () => catalogService.saveUnit('كيلو', undefined, { symbol: 'kg', allowsDecimals: true }));
    await s.expectError('unit-blank', () => catalogService.saveUnit(''));
    await s.expectError('unit-duplicate', () => catalogService.saveUnit('قطعة'));
    await s.expectError('unit-update-missing', () => catalogService.saveUnit('س', missing));
    await s.step('unit-rename-keep-extra', () => catalogService.saveUnit('كيلوجرام', kg.id));
    await s.expectError('unit-delete-linked', () => catalogService.deleteUnit(s.baseId('unit-piece')));
    await s.step('unit-delete', () => catalogService.deleteUnit(kg.id));
    await s.step('units-after', () => catalogService.getUnits());

    // Price lists
    await s.step('price-lists-before', () => catalogService.getPriceLists());
    const pl = await s.step('price-list-create', () => catalogService.savePriceList({ name: ' موسمي ', active: true }));
    await s.expectError('price-list-blank', () => catalogService.savePriceList({ name: ' ', active: true }));
    await s.expectError('price-list-duplicate', () => catalogService.savePriceList({ name: 'سعر الجملة', active: true }));
    await s.expectError('price-list-update-missing', () => catalogService.savePriceList({ name: 'س', active: true }, missing));
    await s.step('price-list-deactivate', () => catalogService.savePriceList({ name: 'موسمي', active: false }, pl.id));
    await s.step('price-lists-after', () => catalogService.getPriceLists());
  },
});
