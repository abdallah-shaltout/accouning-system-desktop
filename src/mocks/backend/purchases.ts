import type { PurchaseOrder, PurchaseOrderInput, PurchaseReturn, PurchaseReturnInput } from '@/modules/purchases/types';
import { paymentStatusFor } from '@/modules/invoices/helpers/totals';
import { db, nextNumber } from '../db';
import { ApiError, round2, sum, uid } from '../utils';
import { applyStockChange, logActivity, postJournal, productById, purchaseTaxRate } from './core';

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
    Object.assign(found, { supplierId: input.supplierId, date: input.date, lines: input.lines, note: input.note, taxRate, ...totals });
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
    db.purchaseOrders.push(po);
  }

  if (input.confirm) confirmPurchase(po.id, userId, input.date);
  else logActivity('purchase', `حفظ أمر الشراء ${po.number} كمسودة`, userId, input.date, `/purchases/${po.id}`);
  return po;
}

/** Receive the goods: stock in at weighted-average cost, post Inventory + VAT input against AP. */
export function confirmPurchase(id: string, userId: string, date = new Date().toISOString()): PurchaseOrder {
  const po = db.purchaseOrders.find((p) => p.id === id);
  if (!po) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
  if (po.status !== 'DRAFT') throw new ApiError('أمر الشراء مؤكد أو ملغي بالفعل');

  let inventoryValue = 0;
  for (const line of po.lines) {
    const product = productById(line.productId);
    const value = line.qty * line.costPrice;
    if (product.type === 'service') continue;
    inventoryValue += value;
    const onHand = Math.max(0, product.stockQty);
    product.costPrice = onHand > 0 ? round2((onHand * product.costPrice + value) / (onHand + line.qty)) : line.costPrice;
    applyStockChange(product, line.qty, 'purchase', po, date);
  }

  po.status = 'CONFIRMED';
  postJournal({
    date,
    description: `أمر شراء ${po.number}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'purchaseOrder', id: po.id, number: po.number },
    lines: [
      { code: '1140', debit: round2(inventoryValue) },
      { code: '5300', debit: round2(po.subTotal - round2(inventoryValue)) },
      { code: '1150', debit: po.taxAmount },
      { code: '2100', credit: po.grandTotal },
    ],
    createdBy: userId,
  });

  const supplier = db.suppliers.find((s) => s.id === po.supplierId);
  logActivity('purchase', `استلام أمر الشراء ${po.number} من ${supplier?.name ?? ''} بقيمة ${po.grandTotal.toFixed(2)}`, userId, date, `/purchases/${po.id}`);
  return po;
}

export function cancelPurchase(id: string, userId: string): PurchaseOrder {
  const po = db.purchaseOrders.find((p) => p.id === id);
  if (!po) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
  if (po.status !== 'DRAFT') throw new ApiError('يمكن إلغاء المسودات فقط — استخدم مرتجع المشتريات للأوامر المؤكدة');
  po.status = 'CANCELED';
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
  };
  db.purchaseReturns.push(ret);

  po.returnedAmount = round2(po.returnedAmount + totals.grandTotal);
  po.paymentStatus = paymentStatusFor(po.grandTotal - po.returnedAmount, po.paidAmount);

  let inventoryValue = 0;
  for (const line of lines) {
    const product = productById(line.productId);
    if (product.type !== 'service') inventoryValue += line.qty * line.costPrice;
    applyStockChange(product, -line.qty, 'purchase_return', ret, date);
  }

  postJournal({
    date,
    description: `مرتجع مشتريات ${ret.number} على أمر الشراء ${po.number}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'purchaseReturn', id: ret.id, number: ret.number },
    lines: [
      { code: '2100', debit: settledToPayable },
      { code: '1110', debit: cashBack },
      { code: '1140', credit: round2(inventoryValue) },
      { code: '5300', credit: round2(totals.subTotal - round2(inventoryValue)) },
      { code: '1150', credit: totals.taxAmount },
    ],
    createdBy: userId,
  });

  logActivity('purchase_return', `مرتجع مشتريات ${ret.number} بقيمة ${ret.grandTotal.toFixed(2)}`, userId, date, `/purchases/${po.id}`);
  return ret;
}

export function supplierOutstandingTotal(supplierId: string): number {
  return sum(
    db.purchaseOrders.filter((p) => p.supplierId === supplierId && p.status === 'CONFIRMED'),
    purchaseOutstanding,
  );
}
