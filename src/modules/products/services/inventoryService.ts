import { ApiError, clone, db, delay, inDateRange, session } from '@/mocks';
import {
  activeBatchesFor,
  applyStockCount,
  backToCounting,
  completeStockAdjustment,
  draftReturnToSupplier,
  isBatchExpired,
  isBatchNearExpiry,
  recordStockAdjustment,
  setStockCountLine,
  startStockCount,
  submitStockCountForReview,
  writeOffBatches,
} from '@/mocks/backend/inventory';
import { mutate } from '@/mocks/persist';
import type { PagedQuery, PagedResult } from '@/modules/core/types/paging';
import { wrap } from '@/modules/diagnostics/services/defineService';

import type {
  DebitNoteDraft,
  ProductBatch,
  StockAdjustment,
  StockAdjustmentInput,
  StockAdjustmentType,
  StockCount,
  StockCountInput,
  StockMovement,
  StockMovementReason,
} from '../types';

export interface AdjustmentFilter {
  type?: StockAdjustmentType;
  status?: 'DRAFT' | 'COMPLETED';
  from?: string;
  to?: string;
}

/** Value of an adjustment at its snapshot unit costs (gains positive, losses negative). */
export const adjustmentValue = wrap('products.adjustmentValue', function adjustmentValue(adj: StockAdjustment): number {
  return Math.round(adj.lines.reduce((acc, l) => acc + l.qtyChange * (l.unitCost ?? 0), 0) * 100) / 100;
});

export const getStockAdjustments = wrap('products.getStockAdjustments', async function getStockAdjustments(filter: AdjustmentFilter = {}): Promise<StockAdjustment[]> {
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
});

export const getStockAdjustment = wrap('products.getStockAdjustment', async function getStockAdjustment(id: string): Promise<StockAdjustment & { journalEntryId?: string }> {
  await delay();
  const adj = db.stockAdjustments.find((a) => a.id === id);
  if (!adj) throw new ApiError('التسوية غير موجودة', 'NOT_FOUND');
  const je = db.journalEntries.find((e) => e.sourceRef?.kind === 'stockAdjustment' && e.sourceRef.id === id);
  return { ...clone(adj), journalEntryId: je?.id };
});

export const createStockAdjustment = wrap('products.createStockAdjustment', async function createStockAdjustment(input: StockAdjustmentInput, asDraft = false): Promise<StockAdjustment> {
  await delay();
  return clone(recordStockAdjustment(input, session.userId, asDraft));
});

export const completeAdjustment = wrap('products.completeAdjustment', async function completeAdjustment(id: string): Promise<StockAdjustment> {
  await delay();
  return clone(completeStockAdjustment(id, session.userId));
});

export const deleteDraftAdjustment = wrap('products.deleteDraftAdjustment', async function deleteDraftAdjustment(id: string): Promise<void> {
  await delay();
  const adj = db.stockAdjustments.find((a) => a.id === id);
  if (!adj) throw new ApiError('التسوية غير موجودة', 'NOT_FOUND');
  if (adj.status !== 'DRAFT') throw new ApiError('لا يمكن حذف تسوية معتمدة — أنشئ تسوية عكسية بدلاً من ذلك');
  mutate(() => (db.stockAdjustments = db.stockAdjustments.filter((a) => a.id !== id)));
});

export interface MovementFilter {
  productId?: string;
  reason?: StockMovementReason;
  from?: string;
  to?: string;
}

/** Stock ledger, newest first. `refLink` points at the source document screen. */
export const getStockMovements = wrap('products.getStockMovements', async function getStockMovements(filter: MovementFilter = {}): Promise<(StockMovement & { productName: string; refLink?: string })[]> {
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
});

export type StockMovementRow = StockMovement & { productName: string; refLink?: string };

/** Server-mode variant of `getStockMovements` for `DataTable`: paged and sorted server-side. */
export const getStockMovementsPaged = wrap('products.getStockMovementsPaged', async function getStockMovementsPaged(query: PagedQuery<MovementFilter>): Promise<PagedResult<StockMovementRow>> {
  await delay();
  const filter = query.filters ?? {};
  let rows: StockMovementRow[] = db.stockMovements
    .filter(
      (m) =>
        (!filter.productId || m.productId === filter.productId) &&
        (!filter.reason || m.reason === filter.reason) &&
        inDateRange(m.date, filter.from, filter.to),
    )
    .map((m) => ({ ...clone(m), productName: db.products.find((p) => p.id === m.productId)?.name ?? '—', refLink: refLink(m) }));

  const total = rows.length;
  const sort = query.sort;
  rows = [...rows].sort((a, b) => {
    if (sort) {
      const va = (a as any)[sort.key];
      const vb = (b as any)[sort.key];
      const dir = sort.dir === 'asc' ? 1 : -1;
      if (typeof va === 'number' && typeof vb === 'number') return (va - vb) * dir;
      return String(va ?? '').localeCompare(String(vb ?? ''), 'ar') * dir;
    }
    return b.date.localeCompare(a.date);
  });

  const start = (query.page - 1) * query.pageSize;
  return { rows: rows.slice(start, start + query.pageSize), total };
});

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

// ---------------------------------------------------------------------------------------------
// v2 phase 6 §3 — Batches & expiry
// ---------------------------------------------------------------------------------------------

export const getBatches = wrap('products.getBatches', async function getBatches(productId: string): Promise<ProductBatch[]> {
  await delay();
  return clone(activeBatchesFor(productId));
});

export type ExpiryBucket = 'expired' | 'within30' | 'within60' | 'within90' | 'ok';

export interface ExpiryRow extends ProductBatch {
  productName: string;
  productSku: string;
  supplierName?: string;
  bucket: ExpiryBucket;
  daysLeft: number | null;
}

function expiryBucket(batch: ProductBatch, today: string): { bucket: ExpiryBucket; daysLeft: number | null } {
  if (!batch.expiryDate) return { bucket: 'ok', daysLeft: null };
  const days = Math.floor((new Date(batch.expiryDate).getTime() - new Date(today).getTime()) / 86400_000);
  if (days < 0) return { bucket: 'expired', daysLeft: days };
  if (days <= 30) return { bucket: 'within30', daysLeft: days };
  if (days <= 60) return { bucket: 'within60', daysLeft: days };
  if (days <= 90) return { bucket: 'within90', daysLeft: days };
  return { bucket: 'ok', daysLeft: days };
}

/** §4 expiry report: expired / ≤30 / ≤60 / ≤90, grouped by supplier (grouping done client-side by the page). */
export const getExpiryReport = wrap('products.getExpiryReport', async function getExpiryReport(): Promise<ExpiryRow[]> {
  await delay();
  const today = new Date().toISOString().slice(0, 10);
  return db.productBatches
    .filter((b) => b.qty > 0.0001 && b.expiryDate)
    .map((b) => {
      const product = db.products.find((p) => p.id === b.productId);
      const supplier = b.supplierId ? db.suppliers.find((s) => s.id === b.supplierId) : undefined;
      const { bucket, daysLeft } = expiryBucket(b, today);
      return { ...clone(b), productName: product?.name ?? '—', productSku: product?.sku ?? '', supplierName: supplier?.name, bucket, daysLeft };
    })
    .filter((r) => r.bucket !== 'ok')
    .sort((a, b) => (a.daysLeft ?? 0) - (b.daysLeft ?? 0));
});

export const batchAlertTone = wrap('products.batchAlertTone', function batchAlertTone(batch: ProductBatch, alertDays: number): 'danger' | 'warning' | undefined {
  if (isBatchExpired(batch)) return 'danger';
  if (isBatchNearExpiry(batch, alertDays)) return 'warning';
  return undefined;
});

export const writeOffExpiredBatches = wrap('products.writeOffExpiredBatches', async function writeOffExpiredBatches(batchIds: string[], note?: string): Promise<StockAdjustment> {
  await delay();
  return clone(writeOffBatches(batchIds, session.userId, note));
});

/** v2 phase 8: still creates the intermediate DRAFT (backend/inventory.ts `draftReturnToSupplier`) — `ExpiryReportPage.vue` immediately posts it into a real debit note via `purchaseService.postDebitNoteDraft`. */
export const returnBatchesToSupplier = wrap('products.returnBatchesToSupplier', async function returnBatchesToSupplier(
  supplierId: string,
  lines: { productId: string; batchId: string; qty: number; unitCost: number }[],
  note?: string,
): Promise<DebitNoteDraft> {
  await delay();
  return clone(draftReturnToSupplier(supplierId, lines, session.userId, note));
});

export const getDebitNoteDrafts = wrap('products.getDebitNoteDrafts', async function getDebitNoteDrafts(): Promise<DebitNoteDraft[]> {
  await delay();
  return clone(db.debitNoteDrafts);
});

// ---------------------------------------------------------------------------------------------
// v2 phase 6 §5 — Stocktake v2
// ---------------------------------------------------------------------------------------------

export const getStockCounts = wrap('products.getStockCounts', async function getStockCounts(): Promise<StockCount[]> {
  await delay();
  return clone([...db.stockCounts].sort((a, b) => b.startedAt.localeCompare(a.startedAt)));
});

export const getStockCount = wrap('products.getStockCount', async function getStockCount(id: string): Promise<StockCount> {
  await delay();
  const count = db.stockCounts.find((c) => c.id === id);
  if (!count) throw new ApiError('الجرد غير موجود', 'NOT_FOUND');
  return clone(count);
});

export const createStockCount = wrap('products.createStockCount', async function createStockCount(input: StockCountInput): Promise<StockCount> {
  await delay();
  return clone(startStockCount(input, session.userId));
});

export const updateStockCountLine = wrap('products.updateStockCountLine', async function updateStockCountLine(countId: string, productId: string, qty: number, delta = false): Promise<StockCount> {
  await delay(30);
  return clone(setStockCountLine(countId, productId, qty, delta));
});

export const submitCountForReview = wrap('products.submitCountForReview', async function submitCountForReview(countId: string): Promise<StockCount> {
  await delay();
  return clone(submitStockCountForReview(countId));
});

export const resumeCounting = wrap('products.resumeCounting', async function resumeCounting(countId: string): Promise<StockCount> {
  await delay();
  return clone(backToCounting(countId));
});

export const completeStockCount = wrap('products.completeStockCount', async function completeStockCount(countId: string): Promise<StockAdjustment> {
  await delay();
  return clone(applyStockCount(countId, session.userId));
});
