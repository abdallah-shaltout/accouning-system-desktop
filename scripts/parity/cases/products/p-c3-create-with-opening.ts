/**
 * 06-products §8(b) P-C3: create with `openingQty` → one opening STOCK_IN adjustment (Dr inventory /
 * Cr openingBalanceEquity at the product's cost, A3), a stock movement, qty/value/cost on the
 * product; then an opening above `inventoryApprovalThreshold` → the FORBIDDEN approval message.
 *
 * Invariant 9 (02-accounting-review §4.9) wants `openingBalanceEquity` at 0 after onboarding, and an
 * opening posted from the product form credits it. Each opening step therefore also posts the
 * documented "close opening equity" entry (Dr openingBalanceEquity / Cr capital, 02 §3) inside the
 * same step, so the invariants checked after every step stay green on both backends.
 *
 * Q-5: the mock keeps the refused product row (it pushes before the opening adjustment throws);
 * Rust rolls the whole create back. That product is created `active: false` so it stays out of
 * every list and the books (the inventory report lists active products only) — the error itself is
 * what this case compares.
 */
import { defineCase } from '../../case';
import * as productService from '../../../../src/modules/products/services/productService';
import * as inventoryService from '../../../../src/modules/products/services/inventoryService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as settingsService from '../../../../src/modules/settings/services/settingsService';
import type { ProductInput } from '../../../../src/modules/products/types';

export default defineCase({
  name: 'products/p-c3-create-with-opening',
  source: '03-domains/06-products.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const obe = s.baseId('acc-3900');
    const capital = s.baseId('acc-3100');
    /** createProduct with an opening, then close the opening equity it created (same step). */
    const createAndClose = async (input: ProductInput) => {
      const product = await productService.createProduct(input);
      const value = product.stockValue;
      if (value > 0) {
        await accountingService.createJournalEntry({
          date: '2026-06-30T09:00:00.000Z',
          description: `إقفال الأرصدة الافتتاحية — ${product.name}`,
          lines: [
            { accountId: obe, debit: value, credit: 0 },
            { accountId: capital, debit: 0, credit: value },
          ],
        });
      }
      return product;
    };

    const p = await s.step('create-with-opening', () =>
      createAndClose({
        name: 'حقيبة ظهر',
        sku: 'ACC-901',
        type: 'product',
        costPrice: 10,
        price: 25,
        active: true,
        categoryId: s.baseId('cat-acc'),
        unitId: s.baseId('unit-piece'),
        openingQty: 5,
      }),
    );
    await s.step('product', () => productService.getProduct(p.id));
    const adjs = await s.step('adjustments', () => inventoryService.getStockAdjustments({ type: 'STOCK_IN' }));
    const opening = adjs.find((a) => a.lines.some((l) => l.productId === p.id));
    if (opening) {
      await s.step('adjustment-detail', () => inventoryService.getStockAdjustment(opening.id));
      await s.step('opening-journal', () => accountingService.getJournalEntriesForSource('stockAdjustment', opening.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    }
    await s.step('movements', () => inventoryService.getStockMovements({ productId: p.id }));

    await s.step('set-threshold', () => settingsService.updateSettings({ inventoryApprovalThreshold: 1000 }));
    await s.expectError('opening-over-threshold', () =>
      productService.createProduct({ name: 'مخزون افتتاحي كبير', sku: 'ACC-902', type: 'product', costPrice: 10, price: 25, active: false, openingQty: 200 }),
    );
    // Below the threshold still posts without an approver.
    const small = await s.step('opening-under-threshold', () =>
      createAndClose({ name: 'مخزون افتتاحي صغير', sku: 'ACC-903', type: 'product', costPrice: 10, price: 25, active: true, openingQty: 99 }),
    );
    await s.step('small-product', () => productService.getProduct(small.id));
  },
});
