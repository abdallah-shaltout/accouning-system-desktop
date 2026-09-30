/**
 * 07-purchases §8(b) P-P3: short receipt — PO totals shrink to the received part, VAT by the
 * round2'd ratio (Q-U10), and with `createBackorder` a DRAFT backorder for the shortfall
 * (`backorderOfId`, note "أمر متبقٍ من …", default branch — Q-U5); without it no backorder.
 */
import { defineCase } from '../../case';
import * as purchaseService from '../../../../src/modules/purchases/services/purchaseService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'purchases/p-p3-short-receipt-backorder',
  source: '03-domains/07-purchases.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const sup = s.baseId('sup-2');
    const prd8 = s.baseId('prd-8');
    const prd9 = s.baseId('prd-9');
    const po = await s.step('save-draft', () =>
      purchaseService.savePurchaseOrder({
        supplierId: sup,
        date,
        confirm: false,
        lines: [
          { productId: prd8, qty: 10, costPrice: 210, discount: 5, discountIsPct: true },
          { productId: prd9, qty: 3, costPrice: 333.33 },
        ],
      }),
    );
    await s.step('send', () => purchaseService.sendPurchaseOrderToSupplier(po.id));
    await s.step('receive-short', () =>
      purchaseService.receivePurchaseOrder(po.id, {
        date,
        createBackorder: true,
        lines: [
          { productId: prd8, receivedQty: 7 },
          { productId: prd9, receivedQty: 0 },
        ],
      }),
    );
    const detail = await s.step('detail', () => purchaseService.getPurchaseOrder(po.id));
    await s.step('journal', () => accountingService.getJournalEntriesForSource('purchaseOrder', po.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    const drafts = await s.step('drafts', () => purchaseService.getPurchaseOrders({ status: 'DRAFT', supplierId: sup }));
    const bo = drafts.find((d) => d.backorderOfId === detail.id);
    if (bo) await s.step('backorder-detail', () => purchaseService.getPurchaseOrder(bo.id));

    // Short without a backorder.
    const po2 = await s.step('save-draft-2', () =>
      purchaseService.savePurchaseOrder({ supplierId: sup, date, confirm: false, lines: [{ productId: prd8, qty: 5, costPrice: 200 }] }),
    );
    await s.step('receive-short-no-backorder', () => purchaseService.receivePurchaseOrder(po2.id, { date, lines: [{ productId: prd8, receivedQty: 2 }] }));
    await s.step('detail-2', () => purchaseService.getPurchaseOrder(po2.id));
    await s.step('drafts-after', () => purchaseService.getPurchaseOrders({ status: 'DRAFT', supplierId: sup }));
  },
});
