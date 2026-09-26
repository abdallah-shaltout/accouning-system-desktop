import { ApiError, clone, db, delay, inDateRange, includesText, session } from '@/mocks';
import { activeBatchesFor } from '@/mocks/backend/inventory';
import {
  cancelPurchase,
  computePurchaseTotals,
  duplicateSupplierInvoice,
  getDebitNoteDrafts as getDebitNoteDraftsBackend,
  missingSupplierInvoice,
  postDebitNoteFromDraft,
  purchaseOutstanding,
  receivePurchase,
  recordPurchaseReturn,
  returnedQtyByProduct,
  savePurchase,
  sendPurchaseToSupplier,
} from '@/mocks/backend/purchases';
import type { ProductBatch } from '@/modules/products/types';
import type { Supplier } from '@/modules/parties/types';
import type { Payment } from '@/modules/payments/types';
import { wrap } from '@/modules/diagnostics/services/defineService';

/** Pure line/VAT totals math (docs/v2/06 §3 engine, applied to purchase-tax direction) — used
 * reactively by `PurchaseFormPage`'s totals preview, so exposed as a plain sync re-export rather
 * than an async `wrap()`ed call. */
export { computePurchaseTotals };

import type {
  PurchaseFilter,
  PurchaseOrder,
  PurchaseOrderInput,
  PurchaseReturn,
  PurchaseReturnInput,
  ReceivePurchaseInput,
} from '../types';

export type PurchaseRow = PurchaseOrder & { supplierName: string; outstanding: number; missingSupplierInvoice: boolean };

export interface PurchaseDetail extends PurchaseRow {
  supplier?: Supplier;
  /** productId → display info (names aren't snapshotted on PO lines). */
  products: Record<string, { name: string; sku: string; stockQty: number; type: 'product' | 'service'; trackBatches?: boolean }>;
  returns: PurchaseReturn[];
  payments: Payment[];
  journalEntries: { id: string; number: string; description: string }[];
  /** productId → quantity already returned to the supplier. */
  returnedQty: Record<string, number>;
  duplicateInvoiceWarning?: string;
}

function toRow(po: PurchaseOrder): PurchaseRow {
  return {
    ...clone(po),
    supplierName: db.suppliers.find((s) => s.id === po.supplierId)?.name ?? '—',
    outstanding: po.status === 'RECEIVED' ? purchaseOutstanding(po) : 0,
    missingSupplierInvoice: missingSupplierInvoice(po),
  };
}

export const getPurchaseOrders = wrap('purchases.getPurchaseOrders', async function getPurchaseOrders(filter: PurchaseFilter & { from?: string; to?: string } = {}): Promise<PurchaseRow[]> {
  await delay();
  return db.purchaseOrders
    .filter(
      (p) =>
        (!filter.status || p.status === filter.status) &&
        (!filter.paymentStatus || p.paymentStatus === filter.paymentStatus) &&
        (!filter.supplierId || p.supplierId === filter.supplierId) &&
        inDateRange(p.date, filter.from, filter.to),
    )
    .map(toRow)
    .filter((r) => includesText([r.number, r.supplierName, r.note], filter.search))
    .sort((a, b) => b.date.localeCompare(a.date));
});

export const getPurchaseOrder = wrap('purchases.getPurchaseOrder', async function getPurchaseOrder(id: string): Promise<PurchaseDetail> {
  await delay();
  const po = db.purchaseOrders.find((p) => p.id === id);
  if (!po) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
  const returns = db.purchaseReturns.filter((r) => r.purchaseOrderId === id);
  const payments = db.payments.filter((p) => p.type === 'PAID' && p.allocations.some((a) => a.targetKind === 'purchaseOrder' && a.targetId === id));
  const sourceIds = new Set([id, ...returns.map((r) => r.id), ...payments.map((p) => p.id)]);
  const products: PurchaseDetail['products'] = {};
  for (const line of po.lines) {
    const p = db.products.find((x) => x.id === line.productId);
    if (p) products[p.id] = { name: p.name, sku: p.sku, stockQty: p.stockQty, type: p.type, trackBatches: p.trackBatches };
  }
  const supplier = db.suppliers.find((s) => s.id === po.supplierId);
  const dup = po.supplierInvoiceNo && supplier ? duplicateSupplierInvoice(supplier.id, po.supplierInvoiceNo, po.id) : undefined;
  return {
    ...toRow(po),
    supplier: clone(supplier),
    products,
    returns: clone(returns),
    payments: clone(payments),
    journalEntries: db.journalEntries
      .filter((e) => e.sourceRef && sourceIds.has(e.sourceRef.id))
      .map((e) => ({ id: e.id, number: e.number, description: e.description })),
    returnedQty: Object.fromEntries(returnedQtyByProduct(id)),
    duplicateInvoiceWarning: dup ? `رقم الفاتورة مستخدم من قبل في أمر الشراء ${dup.number} من نفس المورد` : undefined,
  };
});

export const savePurchaseOrder = wrap('purchases.savePurchaseOrder', async function savePurchaseOrder(input: PurchaseOrderInput, id?: string): Promise<PurchaseOrder> {
  await delay(300);
  return clone(savePurchase(input, session.userId, id));
});

export const sendPurchaseOrderToSupplier = wrap('purchases.sendPurchaseOrderToSupplier', async function sendPurchaseOrderToSupplier(id: string): Promise<PurchaseOrder> {
  await delay();
  return clone(sendPurchaseToSupplier(id, session.userId));
});

/** v2 §2 receiving screen: "confirm receipt" posts stock + AP at the ORDER prices. */
export const receivePurchaseOrder = wrap('purchases.receivePurchaseOrder', async function receivePurchaseOrder(id: string, input: ReceivePurchaseInput): Promise<PurchaseOrder> {
  await delay(300);
  return clone(receivePurchase(id, input, session.userId));
});

/** Legacy one-step path (save-as-draft, then immediately receive in full at order prices) — still used by the "quick" flow / seed data. */
export const confirmPurchaseOrder = wrap('purchases.confirmPurchaseOrder', async function confirmPurchaseOrder(id: string): Promise<PurchaseOrder> {
  await delay(300);
  const po = db.purchaseOrders.find((p) => p.id === id);
  if (!po) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
  return clone(receivePurchase(id, { date: new Date().toISOString(), lines: po.lines.map((l) => ({ productId: l.productId, receivedQty: l.qty * (l.unitFactor ?? 1) - (l.receivedQty ?? 0) })) }, session.userId));
});

export const cancelPurchaseOrder = wrap('purchases.cancelPurchaseOrder', async function cancelPurchaseOrder(id: string): Promise<PurchaseOrder> {
  await delay();
  return clone(cancelPurchase(id, session.userId));
});

export const createPurchaseReturn = wrap('purchases.createPurchaseReturn', async function createPurchaseReturn(input: PurchaseReturnInput): Promise<PurchaseReturn> {
  await delay();
  return clone(recordPurchaseReturn(input, session.userId));
});

/** v2 phase 11b (docs/v2/12-documents-pdf-excel.md §3 "debit note"): looks up a single purchase
 * return by id for `pdfService`'s debit-note payload — returns don't have their own detail
 * route/page (shown inline on the purchase order they belong to). */
export const getPurchaseReturn = wrap('purchases.getPurchaseReturn', async function getPurchaseReturn(id: string): Promise<PurchaseReturn> {
  await delay();
  const ret = db.purchaseReturns.find((r) => r.id === id);
  if (!ret) throw new ApiError('إشعار المدين غير موجود', 'NOT_FOUND');
  return clone(ret);
});

/** Batches remaining for a tracked product — the debit-note form's batch picker (v2 §4). */
export const getActiveBatches = wrap('purchases.getActiveBatches', async function getActiveBatches(productId: string): Promise<ProductBatch[]> {
  await delay();
  return clone(activeBatchesFor(productId));
});

/** v2 §4 "return expiring batch" shortcut — posts Phase 6's stubbed draft into a real debit note. */
export const getDebitNoteDrafts = wrap('purchases.getDebitNoteDrafts', async function getDebitNoteDrafts() {
  await delay();
  return clone(getDebitNoteDraftsBackend());
});

export const postDebitNoteDraft = wrap('purchases.postDebitNoteDraft', async function postDebitNoteDraft(draftId: string, refundMethod: PurchaseReturnInput['refundMethod']): Promise<PurchaseReturn> {
  await delay();
  return clone(postDebitNoteFromDraft(draftId, refundMethod, session.userId));
});
