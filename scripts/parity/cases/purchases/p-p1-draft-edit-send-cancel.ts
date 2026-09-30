/**
 * 07-purchases §8(b) P-P1: PO draft lifecycle — every U-3 validation message and code (supplier
 * VALIDATION, product NOT_FOUND), totals with a % and a flat line discount, an invoice discount and
 * a zero-rated line; edit while DRAFT; send (→ ORDERED); edit/send after sending refused; cancel of
 * DRAFT/ORDERED; cancel of a RECEIVED order refused with the return hint.
 */
import { defineCase } from '../../case';
import * as purchaseService from '../../../../src/modules/purchases/services/purchaseService';
import * as partyService from '../../../../src/modules/parties/services/partyService';
import type { PurchaseOrderInput } from '../../../../src/modules/purchases/types';

export default defineCase({
  name: 'purchases/p-p1-draft-edit-send-cancel',
  source: '03-domains/07-purchases.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: '**.phones', emptyArrayOnly: true, reason: '05-parties D-6: phones is always an array in Rust; the mock leaves it absent on a party saved without phones (absent ≡ [])' },
  ],
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const sup = s.baseId('sup-1');
    const base: PurchaseOrderInput = {
      supplierId: sup,
      date,
      confirm: false,
      note: 'طلبية تجريبية',
      lines: [
        { productId: s.baseId('prd-1'), qty: 10, costPrice: 45, discount: 10, discountIsPct: true },
        { productId: s.baseId('prd-2'), qty: 3, costPrice: 60.5, discount: 5 },
        { productId: s.baseId('prd-3'), qty: 2, costPrice: 30, taxId: s.baseId('tax-zero-purchase') },
      ],
      invoiceDiscount: { pct: 2 },
    };

    await s.expectError('no-supplier', () => purchaseService.savePurchaseOrder({ ...base, supplierId: s.baseId('cus-1') }));
    const inactive = await s.step('inactive-supplier', () => partyService.saveSupplier({ type: 'company', name: 'مورد موقوف', active: false }));
    await s.expectError('supplier-inactive', () => purchaseService.savePurchaseOrder({ ...base, supplierId: inactive.id }));
    await s.expectError('no-lines', () => purchaseService.savePurchaseOrder({ ...base, lines: [] }));
    await s.expectError('unknown-product', () => purchaseService.savePurchaseOrder({ ...base, lines: [{ productId: s.baseId('cus-1'), qty: 1, costPrice: 1 }] }));
    await s.expectError('zero-qty', () => purchaseService.savePurchaseOrder({ ...base, lines: [{ productId: s.baseId('prd-1'), qty: 0, costPrice: 1 }] }));
    await s.expectError('negative-cost', () => purchaseService.savePurchaseOrder({ ...base, lines: [{ productId: s.baseId('prd-1'), qty: 1, costPrice: -1 }] }));

    const po = await s.step('save-draft', () => purchaseService.savePurchaseOrder(base));
    await s.step('edit-draft', () =>
      purchaseService.savePurchaseOrder({ ...base, lines: base.lines.slice(0, 2), invoiceDiscount: { amount: 25 }, note: 'بعد التعديل' }, po.id),
    );
    await s.step('detail-draft', () => purchaseService.getPurchaseOrder(po.id));
    await s.step('send', () => purchaseService.sendPurchaseOrderToSupplier(po.id));
    await s.expectError('send-twice', () => purchaseService.sendPurchaseOrderToSupplier(po.id));
    await s.expectError('edit-after-send', () => purchaseService.savePurchaseOrder(base, po.id));
    await s.step('cancel-ordered', () => purchaseService.cancelPurchaseOrder(po.id));
    await s.expectError('cancel-twice', () => purchaseService.cancelPurchaseOrder(po.id));

    const po2 = await s.step('save-draft-2', () => purchaseService.savePurchaseOrder({ ...base, invoiceDiscount: undefined }));
    await s.step('cancel-draft', () => purchaseService.cancelPurchaseOrder(po2.id));
    await s.expectError('cancel-received', () => purchaseService.cancelPurchaseOrder(s.baseId('po-1')));
    await s.expectError('send-received', () => purchaseService.sendPurchaseOrderToSupplier(s.baseId('po-1')));
    await s.expectError('edit-missing', () => purchaseService.savePurchaseOrder(base, s.baseId('cus-1')));
    await s.expectError('detail-missing', () => purchaseService.getPurchaseOrder(s.baseId('cus-1')));
    await s.step('list-canceled', () => purchaseService.getPurchaseOrders({ status: 'CANCELED' }));
  },
});
