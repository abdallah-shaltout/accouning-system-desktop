/**
 * 06-products §8(b) P-C1: list filters (category, type, low stock, inactive) and Arabic-normalized
 * search over name/SKU/barcode — `أحمد`/`احمد` and `٣`/`3`; findByCode (barcode exact, SKU
 * case-insensitive, unit barcodes ignored — Q-1, inactive → null); suggestSku; EAN-13 shape.
 */
import { defineCase } from '../../case';
import * as productService from '../../../../src/modules/products/services/productService';

/** Valid EAN-13 with the internal `628` prefix (the value itself is random on each backend). */
function isInternalEan13(code: string): boolean {
  if (!/^628\d{10}$/.test(code)) return false;
  const d = code.split('').map(Number);
  const sum = d.slice(0, 12).reduce((a, x, i) => a + x * (i % 2 === 0 ? 1 : 3), 0);
  return (10 - (sum % 10)) % 10 === d[12];
}

export default defineCase({
  name: 'products/p-c1-list-filter-search',
  source: '03-domains/06-products.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: 'steps.**.prices', emptyArrayOnly: true, reason: '06-products Q-7: Product.prices is [] in Rust for a product that has none; the mock leaves it absent on seed rows never saved through the form ([] ≡ absent for this field)' },
  ],
  async run(s) {
    const created = await s.step('create-ahmad', () =>
      productService.createProduct({ name: 'عطر أحمد الملكي', sku: 'ACC-900', type: 'product', costPrice: 20, price: 45, active: true, categoryId: s.baseId('cat-acc'), unitId: s.baseId('unit-piece'), minStock: 2 }),
    );
    await s.step('inactive-product', () =>
      productService.createProduct({ name: 'منتج موقوف', sku: 'OLD-001', type: 'product', costPrice: 1, price: 2, active: false }),
    );
    await s.step('all-active', () => productService.getProducts());
    await s.step('include-inactive', () => productService.getProducts({ includeInactive: true }));
    await s.step('search-hamza', () => productService.getProducts({ search: 'أحمد' }));
    await s.step('search-no-hamza', () => productService.getProducts({ search: 'احمد' }));
    await s.step('search-ahmar', () => productService.getProducts({ search: 'احمر' }));
    await s.step('search-indic-digit', () => productService.getProducts({ search: '٠٠٠٠٣٣' }));
    await s.step('search-latin-digit', () => productService.getProducts({ search: '000033' }));
    await s.step('search-sku-lower', () => productService.getProducts({ search: 'men-00' }));
    await s.step('search-blank', () => productService.getProducts({ search: '  ' }));
    await s.step('category', () => productService.getProducts({ categoryId: s.baseId('cat-shoes') }));
    await s.step('services', () => productService.getProducts({ type: 'service' }));
    await s.step('low-stock', () => productService.getProducts({ lowStockOnly: true }));
    await s.step('low-stock-in-category', () => productService.getProducts({ lowStockOnly: true, categoryId: s.baseId('cat-men') }));

    await s.step('find-barcode', () => productService.findByCode('6281234000019'));
    await s.step('find-barcode-padded', () => productService.findByCode('  6281234000026 '));
    await s.step('find-sku-lower', () => productService.findByCode('wom-003'));
    await s.step('find-unit-barcode-ignored', () => productService.findByCode('6281234090027'));
    await s.step('find-inactive', () => productService.findByCode('OLD-001'));
    await s.step('find-miss', () => productService.findByCode('NOPE-1'));
    await s.step('get-created', () => productService.getProduct(created.id));
    await s.expectError('get-missing', () => productService.getProduct(s.baseId('cus-1')));

    await s.step('suggest-men', () => productService.suggestSku('MEN'));
    await s.step('suggest-acc', () => productService.suggestSku('ACC'));
    await s.step('suggest-new-prefix', () => productService.suggestSku('ZZZ'));
    await s.step('ean13-valid', async () => isInternalEan13(await productService.generateEan13()));
  },
});
