import type { PurchaseOrder, PurchaseOrderInput, PurchaseReturn, PurchaseReturnInput } from '@/modules/purchases/types';
import { paymentStatusFor } from '@/modules/invoices/helpers/totals';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, round2, sum, uid } from '../utils';
import { accountFor, settlementAccountFor } from './accounts';
import { applyStockChange, logActivity, postJournal, productById, purchaseTaxRate, type PostingLine } from './core';

/**
 * v2 (E2): the account a non-stock/service purchase line posts to — product override → category
 * fallback → settings default → generic operating-expense role, so a purchase never has to force
 * everything to one hard-coded code.
 */
function purchaseLineAccountId(productId: string): string {
  const product = productById(productId);
  if (product.purchaseAccountId) return product.purchaseAccountId;
  const category = db.categories.find((c) => c.id === product.categoryId);
  if (category?.purchaseAccountId) return category.purchaseAccountId;
  if (db.settings.accounting?.defaultPurchaseAccountId) return db.settings.accounting.defaultPurchaseAccountId;
  return accountFor('freightIn').id;
}

export function computePurchaseTotals(lines: { qty: number; costPrice: number }[], taxRate: number) {
  const subTotal = round2(lines.reduce((acc, l) => acc + l.qty * l.costPrice, 0));
  const taxAmount = round2((subTotal * taxRate) / 100);
  return { subTotal, taxAmount, grandTotal: round2(subTotal + taxAmount) };
}

export function purchaseOutstanding(po: PurchaseOrder): number {
  return Math.max(0, round2(po.grandTotal - po.returnedAmount - po.paidAmount));
}

function validateLines(input: Pick<PurchaseOrderInput, 'supplierId' | 'lines'>) {
  const supplier = db.suppliers.find((s) => s.id === input.supplierId);
  if (!supplier) throw new ApiError('اختر المورد');
  if (!supplier.active) throw new ApiError('المورد غير نشط');
  if (!input.lines.length) throw new ApiError('أضف صنفاً واحداً على الأقل');
  for (const line of input.lines) {
    productById(line.productId);
    if (!(line.qty > 0)) throw new ApiError('الكمية يجب أن تكون أكبر من صفر');
    if (line.costPrice < 0) throw new ApiError('سعر التكلفة لا يمكن أن يكون سالباً');
  }
}

export function savePurchase(input: PurchaseOrderInput, userId: string, existingId?: string): PurchaseOrder {
  validateLines(input);
  const taxRate = purchaseTaxRate();
  const totals = computePurchaseTotals(input.lines, taxRate);

  let po: PurchaseOrder;
  if (existingId) {
    const found = db.purchaseOrders.find((p) => p.id === existingId);
    if (!found) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
    if (found.status !== 'DRAFT') throw new ApiError('لا يمكن تعديل أمر شراء مؤكد أو ملغي');
    mutate(() => Object.assign(found, { supplierId: input.supplierId, date: input.date, lines: input.lines, note: input.note, taxRate, ...totals }));
    po = found;
  } else {
    po = {
      id: uid('po'),
      number: nextNumber('purchaseOrder'),
      supplierId: input.supplierId,
      date: input.date,
      status: 'DRAFT',
      lines: input.lines.map((l) => ({ ...l })),
      taxRate,
      ...totals,
      paymentStatus: 'UNPAID',
      paidAmount: 0,
      returnedAmount: 0,
      note: input.note,
    };
    mutate(() => db.purchaseOrders.push(po));
  }

  if (input.confirm) confirmPurchase(po.id, userId, input.date);
  else logActivity('purchase', `حفظ أمر الشراء ${po.number} كمسودة`, userId, input.date, `/purchases/${po.id}`);
  return po;
}

/** Receive the goods: stock in at weighted-average cost (re-averaged, A1/A2), post Inventory +
 * service-line purchase accounts (E2) + VAT input against AP. */
export function confirmPurchase(id: string, userId: string, date = new Date().toISOString()): PurchaseOrder {
  const po = db.purchaseOrders.find((p) => p.id === id);
  if (!po) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
  if (po.status !== 'DRAFT') throw new ApiError('أمر الشراء مؤكد أو ملغي بالفعل');

  let inventoryValue = 0;
  const serviceByAccount = new Map<string, number>();
  for (const line of po.lines) {
    const product = productById(line.productId);
    const value = round2(line.qty * line.costPrice);
    if (product.type === 'service') {
      const accId = purchaseLineAccountId(line.productId);
      serviceByAccount.set(accId, round2((serviceByAccount.get(accId) ?? 0) + value));
      continue;
    }
    inventoryValue += value;
    // Receipt re-averages: value += posted amount, so stockValue stays exactly Σ posted GL amounts.
    applyStockChange(product, line.qty, value, 'purchase', po, date);
  }
  inventoryValue = round2(inventoryValue);

  mutate(() => (po.status = 'CONFIRMED'));
  const lines: PostingLine[] = [
    { role: 'inventory', debit: inventoryValue },
    ...[...serviceByAccount.entries()].map(([accountId, amount]) => ({ accountId, debit: amount })),
    { role: 'vatInput', debit: po.taxAmount },
    { role: 'payable', credit: po.grandTotal, partyKind: 'supplier' as const, partyId: po.supplierId },
  ];
  postJournal({
    date,
    description: `أمر شراء ${po.number}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'purchaseOrder', id: po.id, number: po.number },
    lines,
    createdBy: userId,
  });

  const supplier = db.suppliers.find((s) => s.id === po.supplierId);
  logActivity('purchase', `استلام أمر الشراء ${po.number} من ${supplier?.name ?? ''} بقيمة ${po.grandTotal.toFixed(2)}`, userId, date, `/purchases/${po.id}`);
  emit('parties:changed');
  return po;
}

export function cancelPurchase(id: string, userId: string): PurchaseOrder {
  const po = db.purchaseOrders.find((p) => p.id === id);
  if (!po) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
  if (po.status !== 'DRAFT') throw new ApiError('يمكن إلغاء المسودات فقط — استخدم مرتجع المشتريات للأوامر المؤكدة');
  mutate(() => (po.status = 'CANCELED'));
  logActivity('purchase', `إلغاء أمر الشراء ${po.number}`, userId, new Date().toISOString(), `/purchases/${po.id}`);
  return po;
}

export function returnedQtyByProduct(poId: string): Map<string, number> {
  const map = new Map<string, number>();
  for (const ret of db.purchaseReturns.filter((r) => r.purchaseOrderId === poId)) {
    for (const line of ret.lines) map.set(line.productId, (map.get(line.productId) ?? 0) + line.qty);
  }
  return map;
}

export function recordPurchaseReturn(input: PurchaseReturnInput, userId: string, date = new Date().toISOString()): PurchaseReturn {
  const po = db.purchaseOrders.find((p) => p.id === input.purchaseOrderId);
  if (!po) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
  if (po.status !== 'CONFIRMED') throw new ApiError('يمكن الإرجاع من أوامر الشراء المؤكدة فقط');

  const requested = input.lines.filter((l) => l.qty > 0);
  if (!requested.length) throw new ApiError('اختر صنفاً واحداً على الأقل للإرجاع');

  const returned = returnedQtyByProduct(po.id);
  const lines = requested.map((line) => {
    const poLine = po.lines.find((l) => l.productId === line.productId);
    if (!poLine) throw new ApiError('الصنف غير موجود في أمر الشراء');
    const product = productById(line.productId);
    const remaining = poLine.qty - (returned.get(line.productId) ?? 0);
    if (line.qty > remaining) throw new ApiError(`لا يمكن إرجاع أكثر من ${remaining} من "${product.name}"`);
    if (product.type === 'product' && line.qty > product.stockQty) {
      throw new ApiError(`المخزون الحالي من "${product.name}" (${product.stockQty}) أقل من كمية الإرجاع`, 'CONFLICT');
    }
    return { productId: line.productId, qty: line.qty, costPrice: poLine.costPrice };
  });

  const totals = computePurchaseTotals(lines, po.taxRate);
  const settledToPayable = Math.min(totals.grandTotal, purchaseOutstanding(po));
  const cashBack = round2(totals.grandTotal - settledToPayable);
  // E1: refund method defaults to staying on the supplier's account (credit) — never hard-coded to cash.
  const refundMethod = input.refundMethod ?? 'credit';

  const ret: PurchaseReturn = {
    id: uid('pr'),
    number: nextNumber('purchaseReturn'),
    purchaseOrderId: po.id,
    supplierId: po.supplierId,
    date,
    reason: input.reason,
    lines,
    ...totals,
    settledToPayable,
    cashBack,
    refundMethod,
  };
  mutate(() => {
    db.purchaseReturns.push(ret);
    po.returnedAmount = round2(po.returnedAmount + totals.grandTotal);
    po.paymentStatus = paymentStatusFor(po.grandTotal - po.returnedAmount, po.paidAmount);
  });

  // A1/A2: value −= qty × purchase price. If that would leave a negative value, or a non-zero
  // value with zero quantity, the difference goes to inventoryVariance (5110) instead of silently
  // drifting the GL away from Σ product.stockValue.
  let inventoryValue = 0;
  let variance = 0;
  const serviceByAccount = new Map<string, number>();
  for (const line of lines) {
    const product = productById(line.productId);
    if (product.type === 'service') {
      const accId = purchaseLineAccountId(line.productId);
      const amount = round2(line.qty * line.costPrice);
      serviceByAccount.set(accId, round2((serviceByAccount.get(accId) ?? 0) + amount));
      continue;
    }
    const atPurchasePrice = round2(line.qty * line.costPrice);
    const newQty = round2(product.stockQty - line.qty);
    const newValue = round2(product.stockValue - atPurchasePrice);
    let valueOut = atPurchasePrice;
    if (newValue < 0 || (newQty <= 0.0001 && Math.abs(newValue) > 0.001)) {
      // Guard: don't let the line's own posted value push stockValue negative / leave a stray
      // balance at zero qty — take exactly what's there and book the rest as variance.
      valueOut = product.stockValue;
      variance = round2(variance + (atPurchasePrice - valueOut));
    }
    inventoryValue += valueOut;
    applyStockChange(product, -line.qty, -valueOut, 'purchase_return', ret, date);
  }
  inventoryValue = round2(inventoryValue);

  const postingLines: PostingLine[] = [
    { role: 'payable', debit: settledToPayable, partyKind: 'supplier', partyId: po.supplierId },
    { accountId: settlementAccountFor(refundMethod).id, debit: refundMethod === 'credit' ? 0 : cashBack },
    { role: 'payable', debit: refundMethod === 'credit' ? cashBack : 0, partyKind: 'supplier', partyId: po.supplierId },
    { role: 'inventory', credit: inventoryValue },
    ...[...serviceByAccount.entries()].map(([accountId, amount]) => ({ accountId, credit: amount })),
    { role: 'vatInput', credit: totals.taxAmount },
  ];
  if (variance > 0) postingLines.push({ role: 'inventoryVariance', debit: variance });
  else if (variance < 0) postingLines.push({ role: 'inventoryVariance', credit: -variance });

  postJournal({
    date,
    description: `مرتجع مشتريات ${ret.number} على أمر الشراء ${po.number}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'purchaseReturn', id: ret.id, number: ret.number },
    lines: postingLines,
    createdBy: userId,
  });

  logActivity('purchase_return', `مرتجع مشتريات ${ret.number} بقيمة ${ret.grandTotal.toFixed(2)}`, userId, date, `/purchases/${po.id}`);
  emit('parties:changed');
  return ret;
}

export function supplierOutstandingTotal(supplierId: string): number {
  return sum(
    db.purchaseOrders.filter((p) => p.supplierId === supplierId && p.status === 'CONFIRMED'),
    purchaseOutstanding,
  );
}
