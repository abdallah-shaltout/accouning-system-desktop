/**
 * Stock transfers (docs/v2/07-products-and-inventory.md §4 "Branch stock & transfers", deferred
 * from Phase 6 to this phase): draft (from-branch, to-branch, lines) → send (stock leaves the
 * source branch into `inventoryInTransit`, the system role Phase 1 seeded and no one has used
 * until now) → receive (stock arrives at the destination; a shortage posts the difference to
 * `inventoryVariance` 5110; reject sends the transit value back to the source branch).
 */
import type { ReceiveTransferInput, StockTransfer, StockTransferInput } from '@/modules/products/types';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, round2, uid } from '../utils';
import { applyStockChange, logActivity, postJournal, productById, type PostingLine } from './core';
import { branchById, branchPrefix } from './branches';

function baseQty(line: { qty: number; unitFactor?: number }): number {
  return round2(line.qty * (line.unitFactor ?? 1));
}

export function listTransfers(): StockTransfer[] {
  return db.stockTransfers;
}

export function transferById(id: string): StockTransfer {
  const t = db.stockTransfers.find((x) => x.id === id);
  if (!t) throw new ApiError('التحويل غير موجود', 'NOT_FOUND');
  return t;
}

/** Product qty currently sitting at a branch (0 if branches/stockByBranch aren't populated yet). */
export function branchStockQty(productId: string, branchId: string): number {
  const product = db.products.find((p) => p.id === productId);
  return product?.stockByBranch?.[branchId]?.qty ?? 0;
}

export function draftTransfer(input: StockTransferInput, userId: string): StockTransfer {
  if (input.fromBranchId === input.toBranchId) throw new ApiError('لا يمكن التحويل لنفس الفرع');
  if (!branchById(input.fromBranchId) || !branchById(input.toBranchId)) throw new ApiError('فرع غير موجود', 'NOT_FOUND');
  if (!input.lines.length) throw new ApiError('أضف صنفاً واحداً على الأقل');
  for (const line of input.lines) {
    const product = productById(line.productId);
    if (product.type === 'service' || product.stockMode === 'none') throw new ApiError(`"${product.name}" لا يُتتبع مخزونها`);
    if (!(line.qty > 0)) throw new ApiError('الكمية يجب أن تكون أكبر من صفر');
  }
  const transfer: StockTransfer = {
    id: uid('trf'),
    number: `${branchPrefix(input.fromBranchId)}${nextNumber('stockTransfer')}`,
    fromBranchId: input.fromBranchId,
    toBranchId: input.toBranchId,
    status: 'DRAFT',
    date: input.date,
    lines: input.lines.map((l) => ({ ...l })),
    note: input.note,
  };
  mutate(() => db.stockTransfers.push(transfer));
  logActivity('stock', `إنشاء تحويل مخزون ${transfer.number} من ${branchById(input.fromBranchId)?.name} إلى ${branchById(input.toBranchId)?.name}`, userId, transfer.date, `/inventory/transfers/${transfer.id}`);
  return transfer;
}

/**
 * Send: stock leaves the source branch's `stockByBranch` and its company-wide qty moves into the
 * `inventoryInTransit` account (Dr transit / Cr inventory — a same-account reclass, so
 * `GL(inventory) + GL(inventoryInTransit)` together always equal `Σ product.stockValue`).
 */
export function sendTransfer(id: string, userId: string, date = new Date().toISOString()): StockTransfer {
  const transfer = transferById(id);
  if (transfer.status !== 'DRAFT') throw new ApiError('لا يمكن إرسال تحويل تم إرساله بالفعل');

  let transitValue = 0;
  const postingSourceLines: { productId: string; qty: number; value: number }[] = [];
  for (const line of transfer.lines) {
    const product = productById(line.productId);
    const qty = baseQty(line);
    const available = branchStockQty(product.id, transfer.fromBranchId);
    if (qty > available + 0.0001) throw new ApiError(`الكمية المطلوب تحويلها من "${product.name}" أكبر من المتوفر بالفرع (${available})`, 'CONFLICT');
    // Company-wide average cost stands in for "this branch's cost" (Phase 6/9 keep one weighted-
    // average cost per product, not per branch) — same convention `sales.ts`/`purchases.ts` use.
    const unitCost = product.costPrice;
    const value = round2(qty * unitCost);
    transitValue = round2(transitValue + value);
    postingSourceLines.push({ productId: product.id, qty, value });
  }

  mutate(() => {
    transfer.status = 'SENT';
    transfer.sentAt = date;
    transfer.sentBy = userId;
    for (const l of transfer.lines) {
      const src = postingSourceLines.find((s) => s.productId === l.productId)!;
      l.unitCost = src.qty > 0 ? round4(src.value / src.qty) : 0;
    }
  });

  // Stock leaves the source branch AND the company-wide `inventory` GL role (it becomes
  // `inventoryInTransit` below, a different account) — `applyStockChange` keeps
  // `GL(inventory) = Σ product.stockValue` exact by reducing both together, same as a sale.
  for (const s of postingSourceLines) {
    const product = productById(s.productId);
    applyStockChange(product, -s.qty, -s.value, 'transfer_out', transfer, transfer.sentAt!, transfer.fromBranchId);
  }

  const dim = { branchId: transfer.fromBranchId };
  postJournal({
    date: transfer.sentAt!,
    description: `إرسال تحويل مخزون ${transfer.number} — ${branchById(transfer.fromBranchId)?.name} → ${branchById(transfer.toBranchId)?.name}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'stockAdjustment', id: transfer.id, number: transfer.number },
    lines: [
      { role: 'inventoryInTransit', debit: transitValue, ...dim },
      { role: 'inventory', credit: transitValue, ...dim },
    ] as PostingLine[],
    createdBy: userId,
  });

  logActivity('stock', `إرسال تحويل ${transfer.number} بقيمة ${transitValue.toFixed(2)}`, userId, transfer.sentAt!, `/inventory/transfers/${transfer.id}`);
  emit('ledger:changed');
  return transfer;
}

function round4(n: number): number {
  return Math.round((n + Number.EPSILON) * 10000) / 10000;
}

/**
 * Receive: stock arrives at the destination branch. `receivedQty < qty` (shortage) posts the
 * difference from `inventoryInTransit` to `inventoryVariance` (5110) instead of the destination's
 * inventory — the company-wide stock value is what was actually found, not what was sent.
 */
export function receiveTransfer(id: string, input: ReceiveTransferInput, userId: string, date = new Date().toISOString()): StockTransfer {
  const transfer = transferById(id);
  if (transfer.status !== 'SENT') throw new ApiError('لا يمكن استلام تحويل لم يُرسل بعد');

  const byProduct = new Map(input.lines.map((l) => [l.productId, l]));
  let transitValue = 0;
  let receivedValue = 0;
  let shortageValue = 0;
  const destLines: { productId: string; qty: number; value: number }[] = [];

  for (const line of transfer.lines) {
    const sentQty = baseQty(line);
    const unitCost = line.unitCost ?? 0;
    const sentValue = round2(sentQty * unitCost);
    transitValue = round2(transitValue + sentValue);

    const recv = byProduct.get(line.productId);
    const receivedQty = Math.min(recv?.receivedQty ?? sentQty, sentQty);
    if (receivedQty < 0) throw new ApiError('الكمية المستلمة لا يمكن أن تكون سالبة');
    mutate(() => (line.receivedQty = receivedQty));

    const receivedLineValue = round2(receivedQty * unitCost);
    receivedValue = round2(receivedValue + receivedLineValue);
    if (receivedQty > 0) destLines.push({ productId: line.productId, qty: receivedQty, value: receivedLineValue });
    if (receivedQty < sentQty - 0.0001) shortageValue = round2(shortageValue + round2((sentQty - receivedQty) * unitCost));
  }

  mutate(() => {
    transfer.status = 'RECEIVED';
    transfer.receivedAt = date;
    transfer.receivedBy = userId;
    transfer.shortageValue = shortageValue || undefined;
  });

  for (const d of destLines) {
    const product = productById(d.productId);
    applyStockChange(product, d.qty, d.value, 'transfer_in', transfer, transfer.receivedAt!, transfer.toBranchId);
  }

  const dim = { branchId: transfer.toBranchId };
  const lines: PostingLine[] = [
    { role: 'inventory', debit: receivedValue, ...dim },
    { role: 'inventoryInTransit', credit: transitValue, ...dim },
  ];
  if (shortageValue > 0) lines.push({ role: 'inventoryVariance', debit: shortageValue, description: `عجز تحويل ${transfer.number}`, ...dim });

  postJournal({
    date: transfer.receivedAt!,
    description: `استلام تحويل مخزون ${transfer.number} في ${branchById(transfer.toBranchId)?.name}`,
    type: 'SYSTEM',
    // Distinct sourceRef id from the "send" entry (`${transfer.id}-recv`, not `transfer.id`) — a
    // transfer produces two separate GL events (send, then receive/reject), same pattern as an
    // invoice + its refund each getting their own id under one document number.
    sourceRef: { kind: 'stockAdjustment', id: `${transfer.id}-recv`, number: transfer.number },
    lines,
    createdBy: userId,
  });

  logActivity('stock', `استلام تحويل ${transfer.number}${shortageValue > 0 ? ` — عجز ${shortageValue.toFixed(2)}` : ''}`, userId, transfer.receivedAt!, `/inventory/transfers/${transfer.id}`);
  emit('ledger:changed');
  return transfer;
}

/** Reject: the transfer never arrives — the transit value goes straight back to the source branch's stock. */
export function rejectTransfer(id: string, reason: string, userId: string, date = new Date().toISOString()): StockTransfer {
  const transfer = transferById(id);
  if (transfer.status !== 'SENT') throw new ApiError('لا يمكن رفض تحويل لم يُرسل بعد');
  if (!reason.trim()) throw new ApiError('سبب الرفض مطلوب');

  let transitValue = 0;
  const sourceLines: { productId: string; qty: number; value: number }[] = [];
  for (const line of transfer.lines) {
    const qty = baseQty(line);
    const unitCost = line.unitCost ?? 0;
    const value = round2(qty * unitCost);
    transitValue = round2(transitValue + value);
    sourceLines.push({ productId: line.productId, qty, value });
  }

  mutate(() => {
    transfer.status = 'REJECTED';
    transfer.rejectedAt = date;
    transfer.rejectedBy = userId;
    transfer.rejectReason = reason.trim();
  });

  for (const s of sourceLines) {
    const product = productById(s.productId);
    applyStockChange(product, s.qty, s.value, 'transfer_in', transfer, transfer.rejectedAt!, transfer.fromBranchId);
  }

  const dim = { branchId: transfer.fromBranchId };
  postJournal({
    date: transfer.rejectedAt!,
    description: `رفض تحويل مخزون ${transfer.number} — ${reason.trim()}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'stockAdjustment', id: `${transfer.id}-recv`, number: transfer.number },
    lines: [
      { role: 'inventory', debit: transitValue, ...dim },
      { role: 'inventoryInTransit', credit: transitValue, ...dim },
    ] as PostingLine[],
    createdBy: userId,
  });

  logActivity('stock', `رفض تحويل ${transfer.number} — ${reason.trim()}`, userId, transfer.rejectedAt!, `/inventory/transfers/${transfer.id}`);
  emit('ledger:changed');
  return transfer;
}
