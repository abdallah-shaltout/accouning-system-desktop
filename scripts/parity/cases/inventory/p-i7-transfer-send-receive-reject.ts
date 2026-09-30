/**
 * 06b-inventory §8(b) P-I7: branch transfers — create validations, send (Dr inventoryInTransit /
 * Cr inventory at average cost, branch = from), receive short (Dr inventory received + Dr
 * inventoryVariance shortage / Cr transit, branch = to), reject (full value back to the source),
 * every status guard, and the over-branch-stock CONFLICT.
 */
import { defineCase } from '../../case';
import * as transferService from '../../../../src/modules/products/services/transferService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'inventory/p-i7-transfer-send-receive-reject',
  source: '03-domains/06b-inventory.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const main = s.baseId('branch-main');
    const jed = s.baseId('branch-1');
    const date = '2026-06-30T09:00:00.000Z';
    const prd1 = s.baseId('prd-1');
    const prd2 = s.baseId('prd-2');
    const journal = (id: string) => accountingService.getJournalEntriesForSource('stockAdjustment', id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id))));

    await s.expectError('same-branch', () => transferService.createTransfer({ fromBranchId: main, toBranchId: main, date, lines: [{ productId: prd1, qty: 1 }] }));
    await s.expectError('missing-branch', () => transferService.createTransfer({ fromBranchId: main, toBranchId: s.baseId('cus-1'), date, lines: [{ productId: prd1, qty: 1 }] }));
    await s.expectError('no-lines', () => transferService.createTransfer({ fromBranchId: main, toBranchId: jed, date, lines: [] }));
    await s.expectError('service-line', () => transferService.createTransfer({ fromBranchId: main, toBranchId: jed, date, lines: [{ productId: s.baseId('prd-svc-1'), qty: 1 }] }));
    await s.expectError('zero-qty', () => transferService.createTransfer({ fromBranchId: main, toBranchId: jed, date, lines: [{ productId: prd1, qty: 0 }] }));

    // Send → receive short.
    const t1 = await s.step('create-1', () =>
      transferService.createTransfer({ fromBranchId: main, toBranchId: jed, date, note: 'دعم فرع جدة', lines: [{ productId: prd1, qty: 10 }, { productId: prd2, qty: 4 }] }),
    );
    await s.expectError('receive-draft', () => transferService.receiveTransfer(t1.id, { lines: [] }));
    await s.expectError('reject-draft', () => transferService.rejectTransfer(t1.id, 'x'));
    await s.step('send-1', () => transferService.sendTransfer(t1.id));
    await s.expectError('send-twice', () => transferService.sendTransfer(t1.id));
    await s.step('send-1-journal', () => journal(t1.id));
    await s.step('prd-1-in-transit', () => productService.getProduct(prd1));
    await s.expectError('receive-negative', () => transferService.receiveTransfer(t1.id, { lines: [{ productId: prd1, receivedQty: -1 }] }));
    await s.step('receive-1-short', () => transferService.receiveTransfer(t1.id, { lines: [{ productId: prd1, receivedQty: 8 }] }));
    await s.expectError('receive-twice', () => transferService.receiveTransfer(t1.id, { lines: [] }));
    await s.step('prd-1-after-receive', () => productService.getProduct(prd1));
    await s.step('prd-2-after-receive', () => productService.getProduct(prd2));

    // Send → reject.
    const t2 = await s.step('create-2', () => transferService.createTransfer({ fromBranchId: main, toBranchId: jed, date, lines: [{ productId: s.baseId('prd-3'), qty: 5 }] }));
    await s.step('send-2', () => transferService.sendTransfer(t2.id));
    await s.expectError('reject-blank-reason', () => transferService.rejectTransfer(t2.id, '  '));
    await s.step('reject-2', () => transferService.rejectTransfer(t2.id, '  الفرع لا يحتاجها  '));
    await s.step('prd-3-after-reject', () => productService.getProduct(s.baseId('prd-3')));

    // More than the source branch holds.
    const t3 = await s.step('create-3', () => transferService.createTransfer({ fromBranchId: jed, toBranchId: main, date, lines: [{ productId: prd2, qty: 5 }] }));
    await s.expectError('send-over-branch-stock', () => transferService.sendTransfer(t3.id));

    await s.step('transfers', () => transferService.getTransfers());
    await s.step('transfer-1', () => transferService.getTransfer(t1.id));
    await s.expectError('transfer-missing', () => transferService.getTransfer(s.baseId('cus-1')));
  },
});
