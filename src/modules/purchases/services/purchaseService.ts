import { ApiError, clone, db, delay, inDateRange, includesText, session } from '@/mocks';
import {
  cancelPurchase,
  confirmPurchase,
  purchaseOutstanding,
  recordPurchaseReturn,
  returnedQtyByProduct,
  savePurchase,
} from '@/mocks/backend/purchases';
import type { Supplier } from '@/modules/parties/types';
import type { Payment } from '@/modules/payments/types';
import type { PurchaseFilter, PurchaseOrder, PurchaseOrderInput, PurchaseReturn, PurchaseReturnInput } from '../types';

export type PurchaseRow = PurchaseOrder & { supplierName: string; outstanding: number };

export interface PurchaseDetail extends PurchaseRow {
  supplier?: Supplier;
  /** productId → display info (names aren't snapshotted on PO lines). */
  products: Record<string, { name: string; sku: string; stockQty: number; type: 'product' | 'service' }>;
  returns: PurchaseReturn[];
  payments: Payment[];
  journalEntries: { id: string; number: string; description: string }[];
  /** productId → quantity already returned to the supplier. */
  returnedQty: Record<string, number>;
}

function toRow(po: PurchaseOrder): PurchaseRow {
  return {
    ...clone(po),
    supplierName: db.suppliers.find((s) => s.id === po.supplierId)?.name ?? '—',
    outstanding: po.status === 'CONFIRMED' ? purchaseOutstanding(po) : 0,
  };
}

export async function getPurchaseOrders(filter: PurchaseFilter & { from?: string; to?: string } = {}): Promise<PurchaseRow[]> {
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
}

export async function getPurchaseOrder(id: string): Promise<PurchaseDetail> {
  await delay();
  const po = db.purchaseOrders.find((p) => p.id === id);
  if (!po) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
  const returns = db.purchaseReturns.filter((r) => r.purchaseOrderId === id);
  const payments = db.payments.filter((p) => p.type === 'PAID' && p.allocations.some((a) => a.targetKind === 'purchaseOrder' && a.targetId === id));
  const sourceIds = new Set([id, ...returns.map((r) => r.id), ...payments.map((p) => p.id)]);
  const products: PurchaseDetail['products'] = {};
  for (const line of po.lines) {
    const p = db.products.find((x) => x.id === line.productId);
    if (p) products[p.id] = { name: p.name, sku: p.sku, stockQty: p.stockQty, type: p.type };
  }
  return {
    ...toRow(po),
    supplier: clone(db.suppliers.find((s) => s.id === po.supplierId)),
    products,
    returns: clone(returns),
    payments: clone(payments),
    journalEntries: db.journalEntries
      .filter((e) => e.sourceRef && sourceIds.has(e.sourceRef.id))
      .map((e) => ({ id: e.id, number: e.number, description: e.description })),
    returnedQty: Object.fromEntries(returnedQtyByProduct(id)),
  };
}

export async function savePurchaseOrder(input: PurchaseOrderInput, id?: string): Promise<PurchaseOrder> {
  await delay(300);
  return clone(savePurchase(input, session.userId, id));
}

export async function confirmPurchaseOrder(id: string): Promise<PurchaseOrder> {
  await delay(300);
  return clone(confirmPurchase(id, session.userId));
}

export async function cancelPurchaseOrder(id: string): Promise<PurchaseOrder> {
  await delay();
  return clone(cancelPurchase(id, session.userId));
}

export async function createPurchaseReturn(input: PurchaseReturnInput): Promise<PurchaseReturn> {
  await delay();
  return clone(recordPurchaseReturn(input, session.userId));
}
