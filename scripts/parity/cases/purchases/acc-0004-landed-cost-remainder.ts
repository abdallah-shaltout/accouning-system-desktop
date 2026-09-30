/**
 * ACC-0004 verification (plan 21 Part 04 L2-0, P4-7; docs/diagnostics/issues/ACC-0004-landed-cost-
 * rounding.md, 02-accounting-review E4): 100.00 of landed cost over three equal-qty lines — each
 * share round2'd, the remainder to the largest-weight line (ties → the first): 33.34 / 33.33 / 33.33,
 * shares sum to 100.00, the receipt posts balanced (Dr inventory 130 / Cr payable 130), and the
 * invariants (inventory-gl, supplier sub-ledger) stay green. Also a by-value spread with unequal
 * weights, where the remainder lands on the largest line (not the first).
 */
import { defineCase } from '../../case';
import * as purchaseService from '../../../../src/modules/purchases/services/purchaseService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'purchases/acc-0004-landed-cost-remainder',
  source: 'docs/diagnostics/issues/ACC-0004-landed-cost-rounding.md · 03-domains/07-purchases.md §8(b) P-P4',
  base: 'demo-sa',
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const zero = s.baseId('tax-zero-purchase');
    const [p1, p2, p3] = ['prd-1', 'prd-2', 'prd-3'].map((id) => s.baseId(id));
    // The issue's exact repro (scripts/verify/cases/ACC-0004-landed-cost-rounding.json).
    const po = await s.step('receive-100-over-3', () =>
      purchaseService.savePurchaseOrder({
        supplierId: s.baseId('sup-1'),
        date,
        confirm: true,
        lines: [
          { productId: p1, qty: 1, costPrice: 10, taxId: zero },
          { productId: p2, qty: 1, costPrice: 10, taxId: zero },
          { productId: p3, qty: 1, costPrice: 10, taxId: zero },
        ],
        landedCosts: [{ label: 'شحن', amount: 100, spreadBy: 'qty' }],
      }),
    );
    await s.step('detail', () => purchaseService.getPurchaseOrder(po.id));
    await s.step('journal', () => accountingService.getJournalEntriesForSource('purchaseOrder', po.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('product-1', () => productService.getProduct(p1));
    await s.step('product-2', () => productService.getProduct(p2));
    await s.step('product-3', () => productService.getProduct(p3));

    // By value, weights 10 / 30 / 20: 50.00 → 8.33 / 25.00 / 16.67 (remainder 0); 0.05 → 0.01 / 0.03 /
    // 0.02 = 0.06 → −0.01 to the largest (the 30 line); by qty 0.10 → 0.03 × 3 → +0.01 to the first
    // (equal weights). Shares per line: 8.38 / 25.05 / 16.72 = 50.15.
    const po2 = await s.step('receive-by-value', () =>
      purchaseService.savePurchaseOrder({
        supplierId: s.baseId('sup-1'),
        date,
        confirm: true,
        lines: [
          { productId: p1, qty: 1, costPrice: 10, taxId: zero },
          { productId: p2, qty: 1, costPrice: 30, taxId: zero },
          { productId: p3, qty: 1, costPrice: 20, taxId: zero },
        ],
        landedCosts: [
          { label: 'شحن', amount: 50, spreadBy: 'value' },
          { label: 'تأمين', amount: 0.05, spreadBy: 'value' },
          { label: 'رسوم', amount: 0.1, spreadBy: 'qty' },
        ],
      }),
    );
    await s.step('detail-2', () => purchaseService.getPurchaseOrder(po2.id));
    await s.step('journal-2', () => accountingService.getJournalEntriesForSource('purchaseOrder', po2.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
  },
});
