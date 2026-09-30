/**
 * 06-products D-P7 (Part 04 Wave 2 regression): a product's per-unit list prices (`unitPrices`) are
 * keyed by the product's own `ProductUnit.id` — what `PriceMatrix.vue` writes and `PosPage` looks up
 * (`x.unitId === unit.id`) — and must read back verbatim, including a client-made `pu-*` id and
 * `'__base__'`. Rust once decoded that key as a `units` row `Id`, so every per-unit price came back
 * under a hashed stand-in id and never matched again.
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
  name: 'products/p-c11-unit-prices',
  source: '03-domains/06-products.md D-P7',
  base: 'demo-sa',
  allow: [
    { path: 'steps.**.prices', emptyArrayOnly: true, reason: '06-products Q-7: Product.prices is [] in Rust for a product that has none; the mock leaves it absent on seed rows never saved through the form ([] ≡ absent for this field)' },
  ],
  async run(s) {
    const wholesale = s.baseId('pl-wholesale');
    const vip = s.baseId('pl-vip');
    const panadol = await s.step('panadol', () => productService.getProduct(s.baseId('prd-panadol')));
    // A matrix edit: the box unit (`pu-panadol-box`) on two lists, the strip unit on one.
    await s.step('set-unit-prices', () =>
      productService.updateProduct(panadol.id, {
        ...asInput(panadol),
        unitPrices: [
          { priceListId: wholesale, unitId: 'pu-panadol-box', value: 21 },
          { priceListId: vip, unitId: 'pu-panadol-box', value: 22.5 },
          { priceListId: wholesale, unitId: 'pu-panadol-strip', value: 7.25 },
        ],
      }),
    );
    await s.step('panadol-after', () => productService.getProduct(panadol.id));

    // A product with no extra units prices its implicit base unit under `'__base__'`.
    const plain = await s.step('create-plain', () =>
      productService.createProduct({
        name: 'مناديل ورقية',
        sku: 'ACC-920',
        type: 'product',
        costPrice: 3,
        price: 6,
        active: true,
        unitPrices: [{ priceListId: wholesale, unitId: '__base__', value: 5 }],
      }),
    );
    await s.step('plain-after', () => productService.getProduct(plain.id));
  },
});
