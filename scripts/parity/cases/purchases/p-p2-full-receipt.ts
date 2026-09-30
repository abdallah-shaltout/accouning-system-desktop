/**
 * 07-purchases §8(b) P-P2: full receipt — Dr inventory / Dr vatInput / Cr payable, stock qty/value/
 * average cost, PO totals, supplier invoice fields; every receive guard message; confirm of a
 * DRAFT and of the seed's ORDERED order receives everything outstanding at "now".
 */
import { defineCase } from '../../case';
import * as purchaseService from '../../../../src/modules/purchases/services/purchaseService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'purchases/p-p2-full-receipt',
  source: '03-domains/07-purchases.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: 'steps.**.supplierInvoiceDate', dayOfInstantOnly: true, reason: '07-purchases D-U8: supplierInvoiceDate is a calendar day in Rust (YYYY-MM-DD); the mock echoes the ISO instant the UI sent (dateKeyToIso) — same business day' },
  ],
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const sup = s.baseId('sup-3');
    const prd19 = s.baseId('prd-19');
    const prd20 = s.baseId('prd-20');
    const po = await s.step('save-draft', () =>
      purchaseService.savePurchaseOrder({
        supplierId: sup,
        date,
        confirm: false,
        lines: [
          { productId: prd19, qty: 6, costPrice: 150 },
          { productId: prd20, qty: 4, costPrice: 119.99 },
        ],
      }),
    );
    await s.expectError('receive-no-lines', () => purchaseService.receivePurchaseOrder(po.id, { date, lines: [] }));
    await s.expectError('receive-not-in-po', () => purchaseService.receivePurchaseOrder(po.id, { date, lines: [{ productId: s.baseId('prd-1'), receivedQty: 1 }] }));
    await s.expectError('receive-negative', () => purchaseService.receivePurchaseOrder(po.id, { date, lines: [{ productId: prd19, receivedQty: -1 }] }));
    await s.expectError('receive-over-remaining', () => purchaseService.receivePurchaseOrder(po.id, { date, lines: [{ productId: prd19, receivedQty: 7 }] }));
    await s.expectError('receive-all-zero', () => purchaseService.receivePurchaseOrder(po.id, { date, lines: [{ productId: prd19, receivedQty: 0 }] }));

    await s.step('receive', () =>
      purchaseService.receivePurchaseOrder(po.id, {
        date,
        supplierInvoiceNo: 'SH-7781',
        supplierInvoiceDate: '2026-06-29T00:00:00.000Z',
        lines: [
          { productId: prd19, receivedQty: 6 },
          { productId: prd20, receivedQty: 4 },
        ],
      }),
    );
    await s.expectError('receive-twice', () => purchaseService.receivePurchaseOrder(po.id, { date, lines: [{ productId: prd19, receivedQty: 1 }] }));
    await s.step('detail', () => purchaseService.getPurchaseOrder(po.id));
    await s.step('journal', () => accountingService.getJournalEntriesForSource('purchaseOrder', po.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('product-19', () => productService.getProduct(prd19));
    await s.step('product-20', () => productService.getProduct(prd20));
    await s.step('supplier', () => partyService.getSupplier(sup));

    // Confirm a draft: receives everything outstanding, dated now.
    const po2 = await s.step('save-draft-2', () =>
      purchaseService.savePurchaseOrder({ supplierId: sup, date: '2026-06-28T09:00:00.000Z', confirm: false, lines: [{ productId: s.baseId('prd-21'), qty: 3, costPrice: 80 }] }),
    );
    await s.step('confirm-draft', () => purchaseService.confirmPurchaseOrder(po2.id));
    await s.step('confirm-seed-ordered', () => purchaseService.confirmPurchaseOrder(s.baseId('po-38')));
    await s.step('seed-ordered-journal', () => accountingService.getJournalEntriesForSource('purchaseOrder', s.baseId('po-38')).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.expectError('confirm-received', () => purchaseService.confirmPurchaseOrder(s.baseId('po-38')));
    await s.expectError('confirm-missing', () => purchaseService.confirmPurchaseOrder(s.baseId('cus-1')));
    // Save with confirm: true posts in one go.
    await s.step('save-and-confirm', () =>
      purchaseService.savePurchaseOrder({ supplierId: sup, date, confirm: true, lines: [{ productId: s.baseId('prd-22'), qty: 2, costPrice: 60 }] }),
    );
  },
});
