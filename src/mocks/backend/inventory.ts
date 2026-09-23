import type { StockAdjustment, StockAdjustmentInput, StockAdjustmentLine, StockInReason } from '@/modules/products/types';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, round2, uid } from '../utils';
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
    if (product.type === 'service') throw new ApiError(`"${product.name}" خدمة ولا يتتبع مخزونها`);
    if (seen.has(product.id)) throw new ApiError(`"${product.name}" مكرر في القائمة`);
    seen.add(product.id);
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
    return { productId: product.id, systemQty, qtyChange: input.type === 'LOSS' ? -qty : qty, unitCost: product.costPrice };
  });
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

export function recordStockAdjustment(input: StockAdjustmentInput, userId: string, asDraft = false): StockAdjustment {
  validateStockIn(input);
  const adj: StockAdjustment = {
    id: uid('adj'),
    number: nextNumber('adjustment'),
    type: input.type,
    date: input.date,
    status: asDraft ? 'DRAFT' : 'COMPLETED',
    lines: buildLines(input),
    note: input.note,
    reason: input.reason,
    offsetAccountId: input.offsetAccountId,
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
      lines: adj.lines.map((l) => ({ productId: l.productId, countedQty: l.countedQty, qtyChange: Math.abs(l.qtyChange) })),
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
