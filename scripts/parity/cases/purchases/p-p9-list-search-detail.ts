/**
 * 07-purchases §8(b) P-P9: the PO list — search over number / supplier name (Arabic-normalized,
 * D-U1: the joined name) / note, status, paymentStatus, supplier and date filters, date-desc order,
 * `outstanding` and `missingSupplierInvoice`; the detail DTO — payments, journal refs (receipt,
 * return, payment), returnedQty, current product info, and the duplicate supplier-invoice warning.
 */
import { defineCase } from '../../case';
import * as purchaseService from '../../../../src/modules/purchases/services/purchaseService';
import * as paymentService from '../../../../src/modules/payments/services/paymentService';

export default defineCase({
  name: 'purchases/p-p9-list-search-detail',
  source: '03-domains/07-purchases.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: 'steps.**.supplierInvoiceDate', dayOfInstantOnly: true, reason: '07-purchases D-U8: supplierInvoiceDate is a calendar day in Rust (YYYY-MM-DD); the mock echoes the ISO instant the UI sent (dateKeyToIso) — same business day' },
    { path: 'steps.return-one.value.taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
    { path: 'steps.*.value.returns[].taxRate', reason: '07-purchases Q-U8: the mock PurchaseReturn object carries an extra taxRate; the TS type and Rust DTO do not' },
  ],
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const sup1 = s.baseId('sup-1');
    await s.step('all', () => purchaseService.getPurchaseOrders());
    await s.step('search-supplier-hamza', () => purchaseService.getPurchaseOrders({ search: 'الأزياء' }));
    await s.step('search-supplier-no-hamza', () => purchaseService.getPurchaseOrders({ search: 'الازياء' }));
    await s.step('search-number', () => purchaseService.getPurchaseOrders({ search: 'po-00003' }));
    await s.step('search-note', () => purchaseService.getPurchaseOrders({ search: 'منخفضة المخزون' }));
    await s.step('status-received-unpaid', () => purchaseService.getPurchaseOrders({ status: 'RECEIVED', paymentStatus: 'UNPAID' }));
    await s.step('partially-paid', () => purchaseService.getPurchaseOrders({ paymentStatus: 'PARTIALLY_PAID' }));
    await s.step('supplier-range', () => purchaseService.getPurchaseOrders({ supplierId: sup1, from: '2026-06-01', to: '2026-06-30' }));

    // A receipt reusing po-37's supplier invoice number, then a payment and a return against it.
    const po = await s.step('save-and-confirm-dup-invoice', () =>
      purchaseService.savePurchaseOrder({
        supplierId: sup1,
        date,
        confirm: true,
        supplierInvoiceNo: ' INV-SUP1-2201 ',
        supplierInvoiceDate: date,
        lines: [{ productId: s.baseId('prd-4'), qty: 5, costPrice: 110 }],
      }),
    );
    await s.step('pay-part', () =>
      paymentService.createPayment({
        date,
        type: 'PAID',
        targetType: 'supplier',
        targetId: sup1,
        amount: 200,
        method: 'cash',
        allocations: [{ targetKind: 'purchaseOrder', targetId: po.id, amount: 200 }],
      }),
    );
    await s.step('return-one', () =>
      purchaseService.createPurchaseReturn({ purchaseOrderId: po.id, reason: 'عيب', lines: [{ productId: s.baseId('prd-4'), qty: 1 }] }),
    );
    await s.step('detail', () => purchaseService.getPurchaseOrder(po.id));
    await s.step('detail-seed-with-landed', () => purchaseService.getPurchaseOrder(s.baseId('po-37')));
    await s.step('detail-seed-draft', () => purchaseService.getPurchaseOrder(s.baseId('po-35')));
    await s.step('supplier-list-after', () => purchaseService.getPurchaseOrders({ supplierId: sup1 }));
  },
});
