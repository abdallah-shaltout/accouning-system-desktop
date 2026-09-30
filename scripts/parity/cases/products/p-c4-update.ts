/**
 * 06-products §8(b) P-C4: update is a full replace of ProductInput (D-P3) — a cost edit is discarded
 * while stock > 0 and applied when stock is 0 (A1/A2), a product with stock can't become a service,
 * a unit's factor can't change after stock moved, `prices` are replaced, the search column follows
 * the new name.
 */
import { defineCase } from '../../case';
import * as productService from '../../../../src/modules/products/services/productService';
import type { Product, ProductInput } from '../../../../src/modules/products/types';

/** The form's full-object resend: every ProductInput field from the stored product. */
function asInput(p: Product): ProductInput {
  const { id: _id, stockQty: _q, stockValue: _v, stockByBranch: _b, ...rest } = p;
  return rest;
}

export default defineCase({
  name: 'products/p-c4-update',
  source: '03-domains/06-products.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: 'steps.**.prices', emptyArrayOnly: true, reason: '06-products Q-7: Product.prices is [] in Rust for a product that has none; the mock leaves it absent on seed rows never saved through the form ([] ≡ absent for this field)' },
  ],
  async run(s) {
    const withStock = await s.step('before-with-stock', () => productService.getProduct(s.baseId('prd-3')));
    await s.step('cost-edit-ignored', () =>
      productService.updateProduct(withStock.id, {
        ...asInput(withStock),
        name: 'تيشيرت بولو رجالي قطن',
        costPrice: 1,
        prices: [{ priceListId: s.baseId('pl-vip'), value: 70 }],
      }),
    );
    await s.step('search-new-name', () => productService.getProducts({ search: 'بولو رجالي قطن' }));
    await s.expectError('to-service-with-stock', () => productService.updateProduct(withStock.id, { ...asInput(withStock), type: 'service' }));

    const empty = await s.step('create-empty', () =>
      productService.createProduct({ name: 'ساعة رقمية', sku: 'ACC-910', type: 'product', costPrice: 50, price: 120, active: true, minPrice: 90, brand: 'X' }),
    );
    // The form always resends the whole object (D-P3), so every update here does too. The cost is
    // applied because stock is 0.
    const e1 = await s.step('cost-edit-applied', () => productService.updateProduct(empty.id, { ...asInput(empty), costPrice: 55, price: 130 }));
    // A service drops `minStock` on normalize.
    await s.step('to-service-without-stock', () =>
      productService.updateProduct(empty.id, { ...asInput(e1), type: 'service', costPrice: 0, minStock: 4 }),
    );

    const panadol = await s.step('panadol', () => productService.getProduct(s.baseId('prd-panadol')));
    // The message embeds the changed unit's `unitId` (06 §3 `validateUnits`, byte-for-byte on both
    // sides), which is `unit-box` on the mock and its UUID on Rust (P4-13: ids are paired, never
    // compared literally). The step swaps that one id for a placeholder so the rest of the message
    // (and the code) still compares byte for byte.
    const boxUnitId = (panadol.units ?? []).find((u) => u.factor === 3)?.unitId ?? '';
    await s.step('factor-change-after-stock', async () => {
      try {
        await productService.updateProduct(panadol.id, {
          ...asInput(panadol),
          units: (panadol.units ?? []).map((u) => (u.factor === 3 ? { ...u, factor: 4 } : u)),
        });
        return { refused: false };
      } catch (e) {
        const err = e as { code?: string; message?: string };
        return { refused: true, code: err.code, message: String(err.message).replace(boxUnitId, '<unitId>') };
      }
    });
    // Adding a new unit (and keeping the old factors) is fine.
    await s.step('add-unit', () =>
      productService.updateProduct(panadol.id, {
        ...asInput(panadol),
        units: [
          ...(panadol.units ?? []),
          { id: 'pu-panadol-carton', unitId: s.baseId('unit-box'), factor: 30, barcodes: ['6281234090034'], price: 230, priceIsAuto: false, defaultForSale: false, defaultForPurchase: false, active: true },
        ],
      }),
    );
    await s.expectError('update-missing', () => productService.updateProduct(s.baseId('cus-1'), asInput(withStock)));
    await s.step('after-with-stock', () => productService.getProduct(withStock.id));
    await s.step('after-empty', () => productService.getProduct(empty.id));
    await s.step('after-panadol', () => productService.getProduct(panadol.id));
  },
});
