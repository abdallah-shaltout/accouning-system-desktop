/**
 * 07-purchases §8(b) P-P4 (own-supplier part): the PO supplier's own freight and clearing lines
 * (added to its AP), spread by value and by qty, non-divisible amounts after the ACC-0004 fix
 * (remainder to the largest-weight line), service lines take no share; a receipt that brings its
 * own landed costs spreads those (Q-U4). The other-supplier (shipping company) part is
 * `purchases/p-p4-other-supplier-landed` — split out because it breaks the `supplier-allocation`
 * invariant on the mock (lane L2 report, 2026-09-29).
 */
import { defineCase } from '../../case';
import * as purchaseService from '../../../../src/modules/purchases/services/purchaseService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as partyService from '../../../../src/modules/parties/services/partyService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'purchases/p-p4-landed-costs',
  source: '03-domains/07-purchases.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const sup = s.baseId('sup-4');
    const [p23, p24, p25] = ['prd-23', 'prd-24', 'prd-25'].map((id) => s.baseId(id));
    const po = await s.step('save-and-confirm', () =>
      purchaseService.savePurchaseOrder({
        supplierId: sup,
        date,
        confirm: true,
        lines: [
          { productId: p23, qty: 7, costPrice: 33.33 },
          { productId: p24, qty: 5, costPrice: 47.1 },
          { productId: p25, qty: 2, costPrice: 150 },
          { productId: s.baseId('prd-svc-2'), qty: 1, costPrice: 12 },
        ],
        landedCosts: [
          { label: 'شحن المورد', amount: 100.01, spreadBy: 'value' },
          { label: 'تخليص جمركي', amount: 77.77, spreadBy: 'qty', supplierId: sup },
          { label: 'بلا قيمة', amount: 0, spreadBy: 'qty' },
        ],
      }),
    );
    await s.step('detail', () => purchaseService.getPurchaseOrder(po.id));
    await s.step('journal', () => accountingService.getJournalEntriesForSource('purchaseOrder', po.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('product-23', () => productService.getProduct(p23));
    await s.step('product-25', () => productService.getProduct(p25));
    await s.step('supplier-balance', () => partyService.getSupplier(sup));

    // The order has landed costs, but the receipt brings its own (these are spread; the PO keeps its set).
    const po2 = await s.step('save-draft', () =>
      purchaseService.savePurchaseOrder({
        supplierId: sup,
        date,
        confirm: false,
        lines: [
          { productId: p23, qty: 3, costPrice: 30 },
          { productId: p24, qty: 3, costPrice: 30 },
        ],
        landedCosts: [{ label: 'شحن مقدر', amount: 50, spreadBy: 'value' }],
      }),
    );
    await s.step('receive-with-own-landed', () =>
      purchaseService.receivePurchaseOrder(po2.id, {
        date,
        landedCosts: [{ label: 'شحن فعلي', amount: 10, spreadBy: 'qty' }],
        lines: [
          { productId: p23, receivedQty: 3 },
          { productId: p24, receivedQty: 3 },
        ],
      }),
    );
    await s.step('detail-2', () => purchaseService.getPurchaseOrder(po2.id));
    await s.step('journal-2', () => accountingService.getJournalEntriesForSource('purchaseOrder', po2.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
  },
});
