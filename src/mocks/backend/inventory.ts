import type {
  DebitNoteDraft,
  ProductBatch,
  StockAdjustment,
  StockAdjustmentInput,
  StockAdjustmentLine,
  StockCount,
  StockCountInput,
  StockInReason,
} from '@/modules/products/types';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, localDateKey, round2, uid } from '../utils';
import { accountById } from './accounts';
import { applyStockChange, logActivity, postJournal, productById, type PostingLine } from './core';

const TYPE_LABEL = { STOCK_IN: 'إدخال مخزون', LOSS: 'إتلاف/فقد', STOCKTAKE: 'جرد' } as const;

const STOCK_IN_REASON_LABEL: Record<StockInReason, string> = {
  opening: 'رصيد افتتاحي',
  owner_contribution: 'مساهمة من المالك',
  gift: 'هدية / بضاعة مجانية من مورد',
  found: 'فائض تم العثور عليه',
  other: 'أخرى',
};

/** Turn user input into adjustment lines against *current* stock levels (or, for a stocktake, the
 * snapshot taken when the count was created — see A5 below). */
function buildLines(input: StockAdjustmentInput, snapshot?: Map<string, number>): StockAdjustmentLine[] {
  if (!input.lines.length) throw new ApiError('أضف صنفاً واحداً على الأقل');
  const seen = new Set<string>();
  return input.lines.map((line) => {
    const product = productById(line.productId);
    if (product.type === 'service' || product.stockMode === 'none') throw new ApiError(`"${product.name}" لا يُتتبع مخزونها`);
    // §3: several batches of the same product are allowed on one STOCK_IN receipt — dedupe by
    // (product, batch) instead of product alone for batch-tracked products; every other type keeps
    // the one-line-per-product rule.
    const dedupeKey = input.type === 'STOCK_IN' && product.trackBatches ? `${product.id}::${line.batchNo ?? ''}` : product.id;
    if (seen.has(dedupeKey)) throw new ApiError(`"${product.name}"${line.batchNo ? ` (تشغيلة ${line.batchNo})` : ''} مكرر في القائمة`);
    seen.add(dedupeKey);
    const systemQty = snapshot?.get(product.id) ?? product.stockQty;

    if (input.type === 'STOCKTAKE') {
      const counted = line.countedQty ?? NaN;
      if (!(counted >= 0)) throw new ApiError(`أدخل الكمية المعدودة لـ "${product.name}"`);
      return { productId: product.id, systemQty, countedQty: counted, qtyChange: round2(counted - systemQty), unitCost: product.costPrice };
    }
    const qty = line.qtyChange ?? NaN;
    if (!(qty > 0)) throw new ApiError(`أدخل كمية صحيحة لـ "${product.name}"`);
    if (input.type === 'LOSS' && qty > product.stockQty) {
      throw new ApiError(`كمية الإتلاف لـ "${product.name}" أكبر من المتوفر (${product.stockQty})`);
    }
    if (input.type === 'STOCK_IN' && product.trackBatches && !line.batchNo?.trim()) {
      throw new ApiError(`أدخل رقم التشغيلة لـ "${product.name}" — هذا الصنف يتتبع التشغيلات وتاريخ الصلاحية`);
    }
    return {
      productId: product.id,
      systemQty,
      qtyChange: input.type === 'LOSS' ? -qty : qty,
      unitCost: product.costPrice,
      batchNo: line.batchNo?.trim() || undefined,
      expiryDate: line.expiryDate || undefined,
    };
  });
}

// ---------------------------------------------------------------------------------------------
// v2 phase 6 §3 — Batches & expiry (FEFO allocation, write-off, near-expiry alerts)
// ---------------------------------------------------------------------------------------------

/** All batches for a product with remaining qty > 0, earliest expiry first (undated batches last — FEFO). */
export function activeBatchesFor(productId: string): ProductBatch[] {
  return db.productBatches
    .filter((b) => b.productId === productId && b.qty > 0.0001)
    .sort((a, b) => (a.expiryDate ?? '9999-99-99').localeCompare(b.expiryDate ?? '9999-99-99'));
}

export function isBatchExpired(batch: ProductBatch, today = localDateKey(new Date())): boolean {
  return !!batch.expiryDate && batch.expiryDate < today;
}

export function isBatchNearExpiry(batch: ProductBatch, alertDays: number, today = localDateKey(new Date())): boolean {
  if (!batch.expiryDate || isBatchExpired(batch, today)) return false;
  const days = Math.floor((new Date(batch.expiryDate).getTime() - new Date(today).getTime()) / 86400_000);
  return days <= alertDays;
}

/**
 * Receive a batch (STOCK_IN of a tracked product, or a purchase receipt — v2 phase 8 reuses this
 * for the receiving screen's batch capture, docs/v2/09-purchases-payments-expenses.md §1): creates
 * a new lot row. Exported for `backend/purchases.ts`; every other caller in this file still goes
 * through it unqualified (same module).
 */
export function receiveBatch(productId: string, qty: number, unitCost: number, batchNo: string, expiryDate: string | undefined, date: string, ref: { id: string; number: string }): ProductBatch {
  const batch: ProductBatch = {
    id: uid('batch'),
    productId,
    batchNo,
    expiryDate: expiryDate || undefined,
    qty,
    unitCost,
    receivedDate: date,
    sourceRefId: ref.id,
    sourceRefNumber: ref.number,
  };
  db.productBatches.push(batch);
  return batch;
}

/**
 * FEFO consumption (§3 "Selling: FEFO allocation... An expired batch can't be sold"): draws `qty`
 * from the earliest-expiring non-expired batches first. Throws if not enough unexpired stock is
 * batched (callers should already have checked `product.stockQty`, but batch rows can lag it for
 * legacy/unbatched stock — see `allowSellingUnbatchedRemainder`). Returns the batches drawn from,
 * for callers that want to record which batch a sale line came from (Phase 7's job to wire into
 * invoices; exported here so it's ready).
 */
export function consumeFefo(productId: string, qty: number, allowExpired = false): { batchId: string; qty: number }[] {
  let remaining = round2(qty);
  const draws: { batchId: string; qty: number }[] = [];
  const today = localDateKey(new Date());
  for (const batch of activeBatchesFor(productId)) {
    if (remaining <= 0.0001) break;
    if (!allowExpired && isBatchExpired(batch, today)) continue;
    const take = Math.min(batch.qty, remaining);
    if (take <= 0) continue;
    batch.qty = round2(batch.qty - take);
    draws.push({ batchId: batch.id, qty: take });
    remaining = round2(remaining - take);
  }
  return draws;
}

function validateStockIn(input: StockAdjustmentInput): void {
  if (input.type !== 'STOCK_IN') return;
  if (!input.reason) throw new ApiError('اختر سبب إدخال المخزون');
  if (input.reason === 'other') {
    if (!input.offsetAccountId) throw new ApiError('اختر الحساب المقابل لسبب "أخرى"');
    const account = accountById(input.offsetAccountId);
    if (account.systemRole === 'receivable' || account.systemRole === 'payable' || account.systemRole === 'inventory' || !account.allowManual) {
      throw new ApiError(`لا يمكن اختيار "${account.name}" كحساب مقابل — اختر حساباً غير رئيسي`);
    }
  }
}

/**
 * A1/A2: posts each line's value at exactly what moves the GL (COGS-side postings compute their
 * own value elsewhere — this is stock-in/loss/stocktake only), so `applyStockChange` re-averages
 * correctly. A3/A4: the credit/offset side depends on the adjustment's reason (stock-in) or is
 * always `inventoryVariance` for a stocktake, whichever direction the line moves.
 */
function postAdjustment(adj: StockAdjustment, userId: string) {
  const reason = adj.type === 'STOCK_IN' ? 'stock_in' : adj.type === 'LOSS' ? 'loss' : 'stocktake';
  let gains = 0;
  let losses = 0;
  for (const line of adj.lines) {
    const product = productById(line.productId);
    const value = round2(Math.abs(line.qtyChange) * (line.unitCost ?? product.costPrice));
    if (line.qtyChange > 0) gains += value;
    else losses += value;
    applyStockChange(product, line.qtyChange, line.qtyChange > 0 ? value : -value, reason, adj, adj.date);
    // §3 batches: a STOCK_IN of a tracked product opens a new lot; a LOSS (write-off) consumes the
    // oldest-expiring batches first (FEFO applies to write-offs too — expired stock goes first).
    if (product.trackBatches) {
      if (adj.type === 'STOCK_IN' && line.batchNo) {
        receiveBatch(product.id, line.qtyChange, line.unitCost ?? product.costPrice, line.batchNo, line.expiryDate, adj.date, adj);
      } else if (adj.type === 'LOSS') {
        consumeFefo(product.id, Math.abs(line.qtyChange), true);
      }
    }
  }
  gains = round2(gains);
  losses = round2(losses);
  if (gains <= 0 && losses <= 0) return;

  let lines: PostingLine[];
  if (adj.type === 'STOCK_IN') {
    // A3: the credit side depends on the reason, not always Capital.
    const creditLine: PostingLine =
      adj.reason === 'opening'
        ? { role: 'openingBalanceEquity', credit: gains }
        : adj.reason === 'owner_contribution'
          ? { role: 'ownerCurrent', credit: gains }
          : adj.reason === 'gift'
            ? { role: 'otherIncome', credit: gains }
            : adj.reason === 'found'
              ? { role: 'inventoryVariance', credit: gains }
              : { accountId: adj.offsetAccountId!, credit: gains };
    lines = [{ role: 'inventory', debit: gains }, creditLine];
  } else if (adj.type === 'LOSS') {
    // Write-off (damaged/expired): its own account (5120) so the owner sees it separately from COGS drift.
    lines = [{ role: 'inventoryWriteOff', debit: losses }, { role: 'inventory', credit: losses }];
  } else {
    // A4: stocktake gains AND losses both post to inventoryVariance (5110) — corrections of COGS, not revenue.
    lines = [];
    if (gains > 0) lines.push({ role: 'inventory', debit: gains }, { role: 'inventoryVariance', credit: gains });
    if (losses > 0) lines.push({ role: 'inventoryVariance', debit: losses }, { role: 'inventory', credit: losses });
  }

  postJournal({
    date: adj.date,
    description: `${TYPE_LABEL[adj.type]} ${adj.number}${adj.note ? ` — ${adj.note}` : ''}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'stockAdjustment', id: adj.id, number: adj.number },
    lines,
    createdBy: userId,
  });
}

/**
 * v2 §5 (docs/v2/07-products-and-inventory.md, docs/v2/01-personas.md storekeeper): a stock-in or
 * write-off whose absolute value is at or above `settings.inventoryApprovalThreshold` needs a
 * manager PIN before it can post. A simple settings value + PIN-style confirm dialog (not a full
 * approvals inbox — that's Phase 13b). Stocktake lines are approved as a whole through its own
 * review-screen flow (`applyStockCount`), so this guard only applies to STOCK_IN/LOSS.
 */
function assertApproval(input: StockAdjustmentInput, lines: StockAdjustmentLine[]): void {
  if (input.type === 'STOCKTAKE') return;
  const threshold = db.settings.inventoryApprovalThreshold;
  if (!threshold || threshold <= 0) return;
  const value = round2(lines.reduce((acc, l) => acc + Math.abs(l.qtyChange) * (l.unitCost ?? 0), 0));
  if (value >= threshold && !input.approvedBy) {
    throw new ApiError(`قيمة هذه الحركة (${value.toFixed(2)}) تتجاوز حد الاعتماد (${threshold.toFixed(2)}) — يلزم تأكيد المدير`, 'FORBIDDEN');
  }
}

export function recordStockAdjustment(input: StockAdjustmentInput, userId: string, asDraft = false): StockAdjustment {
  validateStockIn(input);
  const builtLines = buildLines(input);
  if (!asDraft) assertApproval(input, builtLines);
  const adj: StockAdjustment = {
    id: uid('adj'),
    number: nextNumber('adjustment'),
    type: input.type,
    date: input.date,
    status: asDraft ? 'DRAFT' : 'COMPLETED',
    lines: builtLines,
    note: input.note,
    reason: input.reason,
    offsetAccountId: input.offsetAccountId,
    approvedBy: input.approvedBy,
    approvedAt: input.approvedBy ? new Date().toISOString() : undefined,
  };
  mutate(() => db.stockAdjustments.push(adj));
  if (!asDraft) postAdjustment(adj, userId);
  logActivity('stock', `${TYPE_LABEL[adj.type]}${adj.reason ? ` — ${STOCK_IN_REASON_LABEL[adj.reason]}` : ''} ${adj.number}${asDraft ? ' (مسودة)' : ''} — ${adj.lines.length} صنف`, userId, adj.date, `/inventory/adjustments/${adj.id}`);
  if (!asDraft) emit('catalog:changed');
  return adj;
}

/**
 * Complete a draft (A5 — count snapshot): a stocktake's `systemQtyAtCount` was captured when the
 * count was *created*, not re-read here. Completing it applies `countedQty − systemQtyAtCount` as
 * the delta, so sales made between counting and approving aren't counted twice. STOCK_IN/LOSS
 * drafts don't have this concern (their qty is a direct movement, not a count), but still honor
 * whatever `systemQty` was captured at creation for the same reason.
 */
export function completeStockAdjustment(id: string, userId: string): StockAdjustment {
  const adj = db.stockAdjustments.find((a) => a.id === id);
  if (!adj) throw new ApiError('التسوية غير موجودة', 'NOT_FOUND');
  if (adj.status !== 'DRAFT') throw new ApiError('التسوية مكتملة بالفعل');
  const snapshot = new Map(adj.lines.map((l) => [l.productId, l.systemQty ?? 0]));
  adj.lines = buildLines(
    {
      type: adj.type,
      date: adj.date,
      reason: adj.reason,
      offsetAccountId: adj.offsetAccountId,
      lines: adj.lines.map((l) => ({ productId: l.productId, countedQty: l.countedQty, qtyChange: Math.abs(l.qtyChange), batchNo: l.batchNo, expiryDate: l.expiryDate })),
    },
    snapshot,
  );
  mutate(() => {
    adj.date = new Date().toISOString();
    adj.status = 'COMPLETED';
  });
  postAdjustment(adj, userId);
  logActivity('stock', `اعتماد ${TYPE_LABEL[adj.type]} ${adj.number}`, userId, adj.date, `/inventory/adjustments/${adj.id}`);
  emit('catalog:changed');
  return adj;
}

// ---------------------------------------------------------------------------------------------
// v2 phase 6 §5 — Stocktake v2: scope selection, systemQty snapshot at count-start, blind count,
// scan counting, review screen before applying. Reuses Phase 1's snapshot mechanic (A5): `StockCount`
// captures `systemQty` per line the moment the count STARTS, exactly like a STOCKTAKE
// StockAdjustment's `systemQty` today — `applyStockCount` below simply builds and posts that same
// STOCKTAKE adjustment from the count's lines once reviewed, so the A4/A5 posting rule (gains AND
// losses → inventoryVariance 5110) is not duplicated, only reused.
// ---------------------------------------------------------------------------------------------

function scopedProductIds(input: Pick<StockCountInput, 'scope' | 'categoryId' | 'location'>): string[] {
  return db.products
    .filter((p) => {
      if (p.type !== 'product' || p.stockMode === 'none' || !p.active) return false;
      if (input.scope === 'category') return p.categoryId === input.categoryId;
      if (input.scope === 'location') return (p.shelfLocation ?? '') === (input.location ?? '');
      return true;
    })
    .map((p) => p.id);
}

/** Starts a new count: snapshots `systemQty` for every in-scope product right now (A5). */
export function startStockCount(input: StockCountInput, userId: string): StockCount {
  const productIds = scopedProductIds(input);
  if (!productIds.length) throw new ApiError('لا توجد أصناف ضمن نطاق الجرد المحدد');
  const count: StockCount = {
    id: uid('cnt'),
    number: nextNumber('stockCount'),
    status: 'OPEN',
    scope: input.scope,
    categoryId: input.categoryId,
    location: input.location,
    blind: input.blind,
    startedAt: new Date().toISOString(),
    startedBy: userId,
    lines: productIds.map((productId) => {
      const p = productById(productId);
      return { productId, systemQty: p.stockQty, unitCost: p.costPrice };
    }),
    note: input.note,
  };
  mutate(() => db.stockCounts.push(count));
  logActivity('stock', `بدء جرد ${count.number}${count.blind ? ' (أعمى)' : ''} — ${count.lines.length} صنف`, userId, count.startedAt, `/inventory/counts/${count.id}`);
  return count;
}

/** Records/updates a counted qty for one product — used by manual entry and by barcode scanning (each scan calls this with `delta: true`). */
export function setStockCountLine(countId: string, productId: string, qty: number, delta: boolean): StockCount {
  const count = db.stockCounts.find((c) => c.id === countId);
  if (!count) throw new ApiError('الجرد غير موجود', 'NOT_FOUND');
  if (count.status !== 'OPEN') throw new ApiError('لا يمكن تعديل جرد غير مفتوح');
  const line = count.lines.find((l) => l.productId === productId);
  if (!line) throw new ApiError('الصنف خارج نطاق هذا الجرد');
  mutate(() => {
    line.countedQty = round2(delta ? (line.countedQty ?? 0) + qty : qty);
  });
  return count;
}

/** Moves an OPEN count to REVIEW (variance computed, nothing posted yet). */
export function submitStockCountForReview(countId: string): StockCount {
  const count = db.stockCounts.find((c) => c.id === countId);
  if (!count) throw new ApiError('الجرد غير موجود', 'NOT_FOUND');
  if (count.status !== 'OPEN') throw new ApiError('الجرد ليس في حالة مفتوحة');
  if (count.lines.some((l) => l.countedQty === undefined)) throw new ApiError('أكمل عدّ جميع الأصناف قبل المتابعة للمراجعة');
  mutate(() => (count.status = 'REVIEW'));
  return count;
}

export function backToCounting(countId: string): StockCount {
  const count = db.stockCounts.find((c) => c.id === countId);
  if (!count) throw new ApiError('الجرد غير موجود', 'NOT_FOUND');
  if (count.status !== 'REVIEW') throw new ApiError('الجرد ليس قيد المراجعة');
  mutate(() => (count.status = 'OPEN'));
  return count;
}

/** Applies a reviewed count: builds a STOCKTAKE StockAdjustment from its lines and posts it (A4/A5), using the count's own snapshot instead of re-reading current stock. */
export function applyStockCount(countId: string, userId: string): StockAdjustment {
  const count = db.stockCounts.find((c) => c.id === countId);
  if (!count) throw new ApiError('الجرد غير موجود', 'NOT_FOUND');
  if (count.status !== 'REVIEW') throw new ApiError('يجب إرسال الجرد للمراجعة أولاً');

  const snapshot = new Map(count.lines.map((l) => [l.productId, l.systemQty]));
  const adj: StockAdjustment = {
    id: uid('adj'),
    number: nextNumber('adjustment'),
    type: 'STOCKTAKE',
    date: new Date().toISOString(),
    status: 'COMPLETED',
    lines: buildLines(
      { type: 'STOCKTAKE', date: new Date().toISOString(), lines: count.lines.map((l) => ({ productId: l.productId, countedQty: l.countedQty })) },
      snapshot,
    ),
    note: `جرد ${count.number}${count.note ? ` — ${count.note}` : ''}`,
  };
  mutate(() => {
    db.stockAdjustments.push(adj);
    count.status = 'COMPLETED';
    count.adjustmentId = adj.id;
  });
  postAdjustment(adj, userId);
  logActivity('stock', `اعتماد نتيجة الجرد ${count.number}`, userId, adj.date, `/inventory/adjustments/${adj.id}`);
  emit('catalog:changed');
  return adj;
}

// ---------------------------------------------------------------------------------------------
// v2 phase 6 §4 — Expiry report actions: write-off (wired to Phase 1's LOSS/5120 posting above) and
// return-to-supplier (a minimal DRAFT stub — TODO(phase 8) owns real debit-note posting).
// ---------------------------------------------------------------------------------------------

/** Write off one or more expired/near-expired batches — a regular LOSS StockAdjustment (§4 "write off"). */
export function writeOffBatches(batchIds: string[], userId: string, note?: string): StockAdjustment {
  if (!batchIds.length) throw new ApiError('اختر تشغيلة واحدة على الأقل');
  const byProduct = new Map<string, { qty: number; batchIds: string[] }>();
  for (const id of batchIds) {
    const batch = db.productBatches.find((b) => b.id === id);
    if (!batch || batch.qty <= 0) continue;
    const entry = byProduct.get(batch.productId) ?? { qty: 0, batchIds: [] };
    entry.qty = round2(entry.qty + batch.qty);
    entry.batchIds.push(id);
    byProduct.set(batch.productId, entry);
  }
  if (!byProduct.size) throw new ApiError('لا توجد كميات متبقية في التشغيلات المحددة');
  const adj = recordStockAdjustment(
    {
      type: 'LOSS',
      date: new Date().toISOString(),
      note: note ?? 'إتلاف — بضاعة منتهية الصلاحية',
      lines: [...byProduct.entries()].map(([productId, { qty }]) => ({ productId, qtyChange: qty })),
    },
    userId,
  );
  // The LOSS above already consumed batches FEFO-first; since we asked for exactly each expired
  // batch's own qty, that draw lands on these same batches for a normal (oldest-first) product.
  return adj;
}

/**
 * TODO(phase 8): minimal draft-only stub (docs/v2/07-products-and-inventory.md §4 "return to
 * supplier"). Records the intent as a DRAFT `DebitNoteDraft` — no GL posting, no supplier AP
 * effect. Phase 8 (docs/v2/09-purchases-payments-expenses.md "Debit notes v2") replaces this with
 * a real debit note that reduces stock and AP; until then this is just a worklist entry so nothing
 * is lost when a storekeeper flags expired batches for return.
 */
export function draftReturnToSupplier(supplierId: string, lines: { productId: string; batchId: string; qty: number; unitCost: number }[], userId: string, note?: string): DebitNoteDraft {
  if (!lines.length) throw new ApiError('اختر تشغيلة واحدة على الأقل للإرجاع');
  const draft: DebitNoteDraft = {
    id: uid('dnd'),
    number: nextNumber('debitNoteDraft'),
    supplierId,
    date: new Date().toISOString(),
    status: 'DRAFT',
    lines,
    note,
  };
  mutate(() => db.debitNoteDrafts.push(draft));
  logActivity('stock', `مسودة إرجاع للمورد ${draft.number} — بانتظار مرحلة المشتريات (Phase 8)`, userId, draft.date, `/products`);
  return draft;
}
