/**
 * 07-purchases §8(b) P-P8 (continues 06b P-I10): the expiry-report flow — batches received on a PO
 * are drafted back to the supplier (`returnBatchesToSupplier`), then `postDebitNoteDraft` turns the
 * draft into a real purchase return against that PO (reason from the draft note, batch drawn by id,
 * `fromDraftId`) and deletes the draft; drafts spanning two POs and a draft with no lines are refused.
 */
import { defineCase } from '../../case';
import * as purchaseService from '../../../../src/modules/purchases/services/purchaseService';
import * as inventoryService from '../../../../src/modules/products/services/inventoryService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'purchases/p-p8-debit-note-from-draft',
  source: '03-domains/07-purchases.md §8(b) · 03-domains/06b-inventory.md §8(b) P-I10',
  base: 'demo-sa',
  allow: [
    { path: 'steps.post-draft.value.taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
    { path: 'steps.return-detail.value.taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
    { path: 'steps.post-draft-b.value.taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
    { path: 'steps.*.value.returns[].taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
  ],
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const sup = s.baseId('sup-4');
    const panadol = s.baseId('prd-panadol');
    const receive = (batchNo: string, expiryDate: string) =>
      purchaseService.savePurchaseOrder({ supplierId: sup, date, confirm: false, lines: [{ productId: panadol, qty: 10, costPrice: 6 }] }).then((po) =>
        purchaseService.receivePurchaseOrder(po.id, { date, lines: [{ productId: panadol, receivedQty: 10, batches: [{ batchNo, expiryDate, qty: 10 }] }] }),
      );
    const poA = await s.step('receive-a', () => receive('PND-EXP-A', '2026-07-10'));
    const poB = await s.step('receive-b', () => receive('PND-EXP-B', '2026-07-15'));
    const batches = await s.step('batches', () => inventoryService.getBatches(panadol));
    const batchA = batches.find((b) => b.sourceRefId === poA.id)!;
    const batchB = batches.find((b) => b.sourceRefId === poB.id)!;
    await s.step('expiry-report', () => inventoryService.getExpiryReport());

    const mixed = await s.step('draft-two-pos', () =>
      inventoryService.returnBatchesToSupplier(sup, [
        { productId: panadol, batchId: batchA.id, qty: 2, unitCost: 6 },
        { productId: panadol, batchId: batchB.id, qty: 2, unitCost: 6 },
      ]),
    );
    await s.expectError('post-two-pos', () => purchaseService.postDebitNoteDraft(mixed.id, 'credit'));

    const draft = await s.step('draft-a', () =>
      inventoryService.returnBatchesToSupplier(sup, [{ productId: panadol, batchId: batchA.id, qty: 10, unitCost: 5.5 }], 'قاربت على انتهاء الصلاحية'),
    );
    const ret = await s.step('post-draft', () => purchaseService.postDebitNoteDraft(draft.id, 'credit'));
    await s.step('return-detail', () => purchaseService.getPurchaseReturn(ret.id));
    await s.step('return-journal', () => accountingService.getJournalEntriesForSource('purchaseReturn', ret.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('drafts-after', () => purchaseService.getDebitNoteDrafts());
    await s.step('batches-after', () => inventoryService.getBatches(panadol));
    await s.step('po-a-after', () => purchaseService.getPurchaseOrder(poA.id));
    await s.expectError('post-again', () => purchaseService.postDebitNoteDraft(draft.id, 'credit'));

    // A draft without a note posts with the default reason, and the default refund method.
    const d2 = await s.step('draft-b', () => inventoryService.returnBatchesToSupplier(sup, [{ productId: panadol, batchId: batchB.id, qty: 3, unitCost: 6 }]));
    await s.step('post-draft-b', () => purchaseService.postDebitNoteDraft(d2.id, undefined));
  },
});
