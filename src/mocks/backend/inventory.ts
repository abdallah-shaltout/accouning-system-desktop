import type { StockAdjustment, StockAdjustmentInput, StockAdjustmentLine } from '@/modules/products/types';
import { db, nextNumber } from '../db';
import { ApiError, round2, uid } from '../utils';
import { applyStockChange, logActivity, postJournal, productById, type PostingLine } from './core';

const TYPE_LABEL = { STOCK_IN: 'إدخال مخزون', LOSS: 'إتلاف/فقد', STOCKTAKE: 'جرد' } as const;

/** Turn user input into adjustment lines against *current* stock levels. */
function buildLines(input: StockAdjustmentInput): StockAdjustmentLine[] {
  if (!input.lines.length) throw new ApiError('أضف صنفاً واحداً على الأقل');
  const seen = new Set<string>();
  return input.lines.map((line) => {
    const product = productById(line.productId);
    if (product.type === 'service') throw new ApiError(`"${product.name}" خدمة ولا يتتبع مخزونها`);
    if (seen.has(product.id)) throw new ApiError(`"${product.name}" مكرر في القائمة`);
    seen.add(product.id);

    if (input.type === 'STOCKTAKE') {
      const counted = line.countedQty ?? NaN;
      if (!(counted >= 0)) throw new ApiError(`أدخل الكمية المعدودة لـ "${product.name}"`);
      return { productId: product.id, systemQty: product.stockQty, countedQty: counted, qtyChange: round2(counted - product.stockQty), unitCost: product.costPrice };
    }
    const qty = line.qtyChange ?? NaN;
    if (!(qty > 0)) throw new ApiError(`أدخل كمية صحيحة لـ "${product.name}"`);
    if (input.type === 'LOSS' && qty > product.stockQty) {
      throw new ApiError(`كمية الإتلاف لـ "${product.name}" أكبر من المتوفر (${product.stockQty})`);
    }
    return { productId: product.id, systemQty: product.stockQty, qtyChange: input.type === 'LOSS' ? -qty : qty, unitCost: product.costPrice };
  });
}

function postAdjustment(adj: StockAdjustment, userId: string) {
  const reason = adj.type === 'STOCK_IN' ? 'stock_in' : adj.type === 'LOSS' ? 'loss' : 'stocktake';
  let gains = 0;
  let losses = 0;
  for (const line of adj.lines) {
    const product = productById(line.productId);
    const value = Math.abs(line.qtyChange) * (line.unitCost ?? product.costPrice);
    if (line.qtyChange > 0) gains += value;
    else losses += value;
    applyStockChange(product, line.qtyChange, reason, adj, adj.date);
  }
  gains = round2(gains);
  losses = round2(losses);

  // STOCK_IN is treated as owner-contributed / opening stock → credited to Capital.
  const lines: PostingLine[] =
    adj.type === 'STOCK_IN'
      ? [{ code: '1140', debit: gains }, { code: '3100', credit: gains }]
      : [
          { code: '1140', debit: gains },
          { code: '4400', credit: gains },
          { code: '5800', debit: losses },
          { code: '1140', credit: losses },
        ];
  if (gains > 0 || losses > 0) {
    postJournal({
      date: adj.date,
      description: `${TYPE_LABEL[adj.type]} ${adj.number}${adj.note ? ` — ${adj.note}` : ''}`,
      type: 'SYSTEM',
      sourceRef: { kind: 'stockAdjustment', id: adj.id, number: adj.number },
      lines,
      createdBy: userId,
    });
  }
}

export function recordStockAdjustment(input: StockAdjustmentInput, userId: string, asDraft = false): StockAdjustment {
  const adj: StockAdjustment = {
    id: uid('adj'),
    number: nextNumber('adjustment'),
    type: input.type,
    date: input.date,
    status: asDraft ? 'DRAFT' : 'COMPLETED',
    lines: buildLines(input),
    note: input.note,
  };
  db.stockAdjustments.push(adj);
  if (!asDraft) postAdjustment(adj, userId);
  logActivity('stock', `${TYPE_LABEL[adj.type]} ${adj.number}${asDraft ? ' (مسودة)' : ''} — ${adj.lines.length} صنف`, userId, adj.date, `/inventory/adjustments/${adj.id}`);
  return adj;
}

/** Complete a draft: quantities are re-read so a stocktake reflects stock at completion time. */
export function completeStockAdjustment(id: string, userId: string): StockAdjustment {
  const adj = db.stockAdjustments.find((a) => a.id === id);
  if (!adj) throw new ApiError('التسوية غير موجودة', 'NOT_FOUND');
  if (adj.status !== 'DRAFT') throw new ApiError('التسوية مكتملة بالفعل');
  adj.lines = buildLines({
    type: adj.type,
    date: adj.date,
    lines: adj.lines.map((l) => ({ productId: l.productId, countedQty: l.countedQty, qtyChange: Math.abs(l.qtyChange) })),
  });
  adj.date = new Date().toISOString();
  adj.status = 'COMPLETED';
  postAdjustment(adj, userId);
  logActivity('stock', `اعتماد ${TYPE_LABEL[adj.type]} ${adj.number}`, userId, adj.date, `/inventory/adjustments/${adj.id}`);
  return adj;
}
