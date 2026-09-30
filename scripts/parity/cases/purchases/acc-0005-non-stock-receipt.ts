/**
 * ACC-0005 verification (plan 21 Part 04 L2-0, P4-7; docs/diagnostics/issues/ACC-0005-non-stock-
 * receipt-inventory.md, 02-accounting-review E1–E4 "Line accounts"): receiving a non-stock product
 * (`type: 'product'`, `stockMode: 'none'`) debits its purchase account (product → category →
 * settings default → freightIn), never `inventory`; it takes no landed-cost share; the invariant
 * `inventory-gl` holds after every step.
 */
import { defineCase } from '../../case';
import * as purchaseService from '../../../../src/modules/purchases/services/purchaseService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as catalogService from '../../../../src/modules/products/services/catalogService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'purchases/acc-0005-non-stock-receipt',
  source: 'docs/diagnostics/issues/ACC-0005-non-stock-receipt-inventory.md · 03-domains/07-purchases.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const zero = s.baseId('tax-zero-purchase');
    // No product/category/settings account → the freightIn fallback.
    const bags = await s.step('non-stock-product', () =>
      productService.createProduct({ name: 'أكياس تسوق', sku: 'BAG-010', type: 'product', stockMode: 'none', costPrice: 10, price: 0, active: true }),
    );
    const po = await s.step('receive-non-stock', () =>
      purchaseService.savePurchaseOrder({ supplierId: s.baseId('sup-2'), date, confirm: true, lines: [{ productId: bags.id, qty: 3, costPrice: 10, taxId: zero }] }),
    );
    await s.step('journal', () => accountingService.getJournalEntriesForSource('purchaseOrder', po.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('bags-after', () => productService.getProduct(bags.id));

    // Category purchase account (6-level resolution: product none → category → …).
    const cat = await s.step('category-with-account', () => catalogService.saveCategory('مستهلكات', undefined, { purchaseAccountId: s.baseId('acc-1170') }));
    const labels = await s.step('non-stock-in-category', () =>
      productService.createProduct({ name: 'ملصقات', sku: 'LBL-001', type: 'product', stockMode: 'none', costPrice: 2, price: 0, active: true, categoryId: cat.id }),
    );
    // Mixed receipt with a landed cost: only the stock line takes the share.
    const po2 = await s.step('receive-mixed-with-landed', () =>
      purchaseService.savePurchaseOrder({
        supplierId: s.baseId('sup-2'),
        date,
        confirm: true,
        lines: [
          { productId: labels.id, qty: 50, costPrice: 2, taxId: zero },
          { productId: s.baseId('prd-10'), qty: 2, costPrice: 55, taxId: zero },
        ],
        landedCosts: [{ label: 'شحن', amount: 20, spreadBy: 'qty' }],
      }),
    );
    await s.step('detail-2', () => purchaseService.getPurchaseOrder(po2.id));
    await s.step('journal-2', () => accountingService.getJournalEntriesForSource('purchaseOrder', po2.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('prd-10-after', () => productService.getProduct(s.baseId('prd-10')));
  },
});
