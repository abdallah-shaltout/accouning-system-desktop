import { ApiError, clone, db, delay, inDateRange, session } from '@/mocks';
import { completeStockAdjustment, recordStockAdjustment } from '@/mocks/backend/inventory';
import type { StockAdjustment, StockAdjustmentInput, StockAdjustmentType, StockMovement, StockMovementReason } from '../types';

export interface AdjustmentFilter {
  type?: StockAdjustmentType;
  status?: 'DRAFT' | 'COMPLETED';
  from?: string;
  to?: string;
}

/** Value of an adjustment at its snapshot unit costs (gains positive, losses negative). */
export function adjustmentValue(adj: StockAdjustment): number {
  return Math.round(adj.lines.reduce((acc, l) => acc + l.qtyChange * (l.unitCost ?? 0), 0) * 100) / 100;
}

export async function getStockAdjustments(filter: AdjustmentFilter = {}): Promise<StockAdjustment[]> {
  await delay();
  return clone(
    db.stockAdjustments
      .filter(
        (a) =>
          (!filter.type || a.type === filter.type) &&
          (!filter.status || a.status === filter.status) &&
          inDateRange(a.date, filter.from, filter.to),
      )
      .sort((a, b) => b.date.localeCompare(a.date)),
  );
}

export async function getStockAdjustment(id: string): Promise<StockAdjustment & { journalEntryId?: string }> {
  await delay();
  const adj = db.stockAdjustments.find((a) => a.id === id);
  if (!adj) throw new ApiError('التسوية غير موجودة', 'NOT_FOUND');
  const je = db.journalEntries.find((e) => e.sourceRef?.kind === 'stockAdjustment' && e.sourceRef.id === id);
  return { ...clone(adj), journalEntryId: je?.id };
}

export async function createStockAdjustment(input: StockAdjustmentInput, asDraft = false): Promise<StockAdjustment> {
  await delay();
  return clone(recordStockAdjustment(input, session.userId, asDraft));
}

export async function completeAdjustment(id: string): Promise<StockAdjustment> {
  await delay();
  return clone(completeStockAdjustment(id, session.userId));
}

export async function deleteDraftAdjustment(id: string): Promise<void> {
  await delay();
  const adj = db.stockAdjustments.find((a) => a.id === id);
  if (!adj) throw new ApiError('التسوية غير موجودة', 'NOT_FOUND');
  if (adj.status !== 'DRAFT') throw new ApiError('لا يمكن حذف تسوية معتمدة — أنشئ تسوية عكسية بدلاً من ذلك');
  db.stockAdjustments = db.stockAdjustments.filter((a) => a.id !== id);
}

export interface MovementFilter {
  productId?: string;
  reason?: StockMovementReason;
  from?: string;
  to?: string;
}

/** Stock ledger, newest first. `refLink` points at the source document screen. */
export async function getStockMovements(filter: MovementFilter = {}): Promise<(StockMovement & { productName: string; refLink?: string })[]> {
  await delay();
  return db.stockMovements
    .filter(
      (m) =>
        (!filter.productId || m.productId === filter.productId) &&
        (!filter.reason || m.reason === filter.reason) &&
        inDateRange(m.date, filter.from, filter.to),
    )
    .sort((a, b) => b.date.localeCompare(a.date))
    .map((m) => ({ ...clone(m), productName: db.products.find((p) => p.id === m.productId)?.name ?? '—', refLink: refLink(m) }));
}

function refLink(m: StockMovement): string | undefined {
  switch (m.reason) {
    case 'sale':
      return `/invoices/${m.refId}`;
    case 'refund': {
      const refund = db.refunds.find((r) => r.id === m.refId);
      return refund ? `/invoices/${refund.invoiceId}` : undefined;
    }
    case 'purchase':
      return `/purchases/${m.refId}`;
    case 'purchase_return': {
      const ret = db.purchaseReturns.find((r) => r.id === m.refId);
      return ret ? `/purchases/${ret.purchaseOrderId}` : undefined;
    }
    default:
      return `/inventory/adjustments/${m.refId}`;
  }
}
