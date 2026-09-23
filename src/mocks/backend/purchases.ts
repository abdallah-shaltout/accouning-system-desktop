import type {
  DebitNoteLine,
  LandedCostLine,
  PurchaseLine,
  PurchaseOrder,
  PurchaseOrderInput,
  PurchaseReturn,
  PurchaseReturnInput,
  ReceivePurchaseInput,
} from '@/modules/purchases/types';
import { computeInvoiceTotals, paymentStatusFor, round2 as round2Totals } from '@/modules/invoices/helpers/totals';
import type { DebitNoteDraft } from '@/modules/products/types';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, round2, sum, uid } from '../utils';
import { accountFor, settlementAccountFor } from './accounts';
import { activeBatchesFor, receiveBatch } from './inventory';
import { applyStockChange, logActivity, postJournal, productById, purchaseTaxRate, DEFAULT_BRANCH_ID, type PostingLine } from './core';
import { branchPrefix, defaultCostCenterFor } from './branches';

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

function taxRateFor(taxId: string | undefined, fallback: number): { rate: number; id?: string } {
  if (!taxId) return { rate: fallback };
  const tax = db.taxes.find((t) => t.id === taxId && t.active);
  return tax ? { rate: tax.rate, id: tax.id } : { rate: fallback };
}

/**
 * v2 §1 totals — reuses the same discount/VAT engine as sales (`computeInvoiceTotals`,
 * docs/v2/06-sales-and-pos.md §3), applied to purchase lines with the purchase tax direction.
 * Purchase prices are always tax-exclusive (no "prices include tax" concept on the supplier side).
 */
export function computePurchaseTotals(
  lines: { qty: number; costPrice: number; discount?: number; discountIsPct?: boolean; taxId?: string }[],
  invoiceDiscount: { pct?: number; amount?: number } | undefined,
  defaultTaxRate: number,
) {
  const result = computeInvoiceTotals(
    lines.map((l) => {
      const { rate, id } = taxRateFor(l.taxId, defaultTaxRate);
      return {
        qty: l.qty,
        unitPrice: l.costPrice,
        discount: l.discount ?? 0,
        discountIsPct: !!l.discountIsPct,
        tax: { rate, category: db.taxes.find((t) => t.id === id)?.category ?? 'S' },
      };
    }),
    invoiceDiscount?.pct ? { pct: invoiceDiscount.pct } : invoiceDiscount?.amount ? { amount: invoiceDiscount.amount } : undefined,
    false,
  );
  return {
    subTotal: result.net,
    taxAmount: result.vat,
    grandTotal: result.gross,
    taxRate: lines.length ? round2Totals((result.vat / (result.net || 1)) * 100) : defaultTaxRate,
  };
}

export function purchaseOutstanding(po: PurchaseOrder): number {
  return Math.max(0, round2(po.grandTotal - po.returnedAmount - po.paidAmount));
}

/** True while the posted order is missing the supplier's invoice number/date (v2 §1 "warning banner"). */
export function missingSupplierInvoice(po: PurchaseOrder): boolean {
  return po.status === 'RECEIVED' && (!po.supplierInvoiceNo?.trim() || !po.supplierInvoiceDate);
}

/** v2 §1 duplicate-invoice-number warning (review E3): another RECEIVED order from the same supplier already used this number. */
export function duplicateSupplierInvoice(supplierId: string, supplierInvoiceNo: string, excludePoId?: string): PurchaseOrder | undefined {
  const no = supplierInvoiceNo.trim();
  if (!no) return undefined;
  return db.purchaseOrders.find(
    (p) => p.id !== excludePoId && p.supplierId === supplierId && p.status === 'RECEIVED' && p.supplierInvoiceNo?.trim() === no,
  );
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

/** Base-unit qty for a purchase line (qty × unitFactor, defaulting factor to 1 for legacy/base-unit lines). */
export function baseQty(line: { qty: number; unitFactor?: number }): number {
  return round2(line.qty * (line.unitFactor ?? 1));
}

/** Base-unit unit cost for a purchase line (cost is per the line's own unit; base cost = cost / factor). */
export function baseUnitCost(line: { costPrice: number; unitFactor?: number }): number {
  const factor = line.unitFactor ?? 1;
  return factor > 0 ? line.costPrice / factor : line.costPrice;
}

export function savePurchase(input: PurchaseOrderInput, userId: string, existingId?: string): PurchaseOrder {
  validateLines(input);
  const taxRate = purchaseTaxRate();
  const totals = computePurchaseTotals(input.lines, input.invoiceDiscount, taxRate);

  let po: PurchaseOrder;
  if (existingId) {
    const found = db.purchaseOrders.find((p) => p.id === existingId);
    if (!found) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
    if (found.status !== 'DRAFT') throw new ApiError('لا يمكن تعديل أمر شراء تم إرساله أو استلامه أو إلغاؤه');
    mutate(() =>
      Object.assign(found, {
        supplierId: input.supplierId,
        date: input.date,
        lines: input.lines.map((l) => ({ ...l })),
        note: input.note,
        invoiceDiscount: input.invoiceDiscount,
        landedCosts: (input.landedCosts ?? []).map((l) => ({ id: uid('lc'), ...l })),
        supplierInvoiceNo: input.supplierInvoiceNo,
        supplierInvoiceDate: input.supplierInvoiceDate,
        attachmentIds: input.attachmentIds,
        costCenterId: defaultCostCenterFor(found.branchId, input.costCenterId),
        ...totals,
      }),
    );
    po = found;
  } else {
    const branchId = input.branchId ?? DEFAULT_BRANCH_ID;
    po = {
      id: uid('po'),
      number: `${branchPrefix(branchId)}${nextNumber('purchaseOrder')}`,
      supplierId: input.supplierId,
      date: input.date,
      status: 'DRAFT',
      lines: input.lines.map((l) => ({ ...l })),
      ...totals,
      paymentStatus: 'UNPAID',
      paidAmount: 0,
      returnedAmount: 0,
      note: input.note,
      invoiceDiscount: input.invoiceDiscount,
      landedCosts: (input.landedCosts ?? []).map((l) => ({ id: uid('lc'), ...l })),
      supplierInvoiceNo: input.supplierInvoiceNo,
      supplierInvoiceDate: input.supplierInvoiceDate,
      attachmentIds: input.attachmentIds,
      costCenterId: defaultCostCenterFor(branchId, input.costCenterId),
      branchId,
      currency: input.currency,
      exchangeRate: input.exchangeRate,
    };
    mutate(() => db.purchaseOrders.push(po));
  }

  if (input.confirm) receivePurchase(po.id, { date: input.date, lines: po.lines.map((l) => ({ productId: l.productId, receivedQty: baseQty(l) })) }, userId);
  else logActivity('purchase', `حفظ أمر الشراء ${po.number} كمسودة`, userId, input.date, `/purchases/${po.id}`);
  return po;
}

/** v2 §1 "إرسال للمورد" — DRAFT → ORDERED, stamps `sentAt`. The PO PDF/print is the caller's job (pdfService/print route). */
export function sendPurchaseToSupplier(id: string, userId: string): PurchaseOrder {
  const po = db.purchaseOrders.find((p) => p.id === id);
  if (!po) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
  if (po.status !== 'DRAFT') throw new ApiError('لا يمكن إرسال إلا مسودة');
  mutate(() => {
    po.status = 'ORDERED';
    po.sentAt = new Date().toISOString();
  });
  logActivity('purchase', `إرسال أمر الشراء ${po.number} للمورد`, userId, po.sentAt!, `/purchases/${po.id}`);
  return po;
}

/** Spreads landed-cost lines belonging to the purchase's own supplier over the receiving stock lines, by value or qty (docs/v2/09 §1, review E4). Returns each product's per-unit landed-cost share (base unit) plus the AP add-on for the po's own supplier and any other-supplier AP lines. */
function allocateLandedCosts(
  po: PurchaseOrder,
  received: { productId: string; qty: number; value: number }[],
): { shareByProduct: Map<string, number>; ownSupplierTotal: number; otherSupplierLines: { supplierId: string; amount: number }[] } {
  const shareByProduct = new Map<string, number>();
  let ownSupplierTotal = 0;
  const otherSupplierLines: { supplierId: string; amount: number }[] = [];
  const stockLines = received.filter((r) => r.qty > 0);
  const totalValue = sum(stockLines, (r) => r.value);
  const totalQty = round2(stockLines.reduce((a, r) => a + r.qty, 0));

  for (const lc of po.landedCosts ?? []) {
    if (!(lc.amount > 0)) continue;
    if (!lc.supplierId || lc.supplierId === po.supplierId) ownSupplierTotal = round2(ownSupplierTotal + lc.amount);
    else otherSupplierLines.push({ supplierId: lc.supplierId, amount: round2(lc.amount) });

    if (!stockLines.length) continue;
    const weight = lc.spreadBy === 'qty' ? (r: (typeof stockLines)[number]) => r.qty : (r: (typeof stockLines)[number]) => r.value;
    const base = lc.spreadBy === 'qty' ? totalQty : totalValue;
    if (!(base > 0)) continue;
    for (const r of stockLines) {
      const share = round2((lc.amount * weight(r)) / base);
      shareByProduct.set(r.productId, round2((shareByProduct.get(r.productId) ?? 0) + share));
    }
  }
  return { shareByProduct, ownSupplierTotal, otherSupplierLines };
}

function lineRemaining(line: PurchaseLine): number {
  return round2(baseQty(line) - (line.receivedQty ?? 0));
}

/**
 * v2 §2 receiving flow (docs/v2/09-purchases-payments-expenses.md §1-§2): "Confirm receipt" posts
 * stock + AP at the ORDER prices (never the storekeeper's own guess). One posting per document —
 * a PO can only be received once (short delivery uses "إنشاء أمر متبقٍ" for the difference instead
 * of a second receipt on the same document). Landed costs (own-supplier AP add-on + other-supplier
 * AP lines) are folded into this same posting; each stock line's unit cost includes its landed-cost
 * share (review E4), so `applyStockChange`'s `valueChange` — and therefore `GL(inventory)` — already
 * reflects it exactly.
 */
export function receivePurchase(id: string, input: ReceivePurchaseInput, userId: string): PurchaseOrder {
  const po = db.purchaseOrders.find((p) => p.id === id);
  if (!po) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
  if (po.status !== 'DRAFT' && po.status !== 'ORDERED') throw new ApiError('تم استلام أمر الشراء هذا بالفعل أو تم إلغاؤه');
  if (!input.lines.length) throw new ApiError('لا توجد كميات للاستلام');

  const byProduct = new Map(input.lines.map((l) => [l.productId, l]));
  for (const rl of input.lines) {
    const poLine = po.lines.find((l) => l.productId === rl.productId);
    if (!poLine) throw new ApiError('صنف غير موجود في أمر الشراء');
    if (rl.receivedQty < 0) throw new ApiError('الكمية المستلمة لا يمكن أن تكون سالبة');
    if (rl.receivedQty > lineRemaining(poLine) + 0.0001) throw new ApiError(`الكمية المستلمة لـ "${productById(rl.productId).name}" أكبر من المتبقي في الأمر`);
  }
  if (![...byProduct.values()].some((l) => l.receivedQty > 0)) throw new ApiError('أدخل كمية استلام واحدة على الأقل');

  // Landed costs (this receipt's own, or the PO's, when the caller passes none — receiving reuses
  // whatever was entered on the order form unless it hands over its own set at receiving time).
  const landedCosts: LandedCostLine[] = input.landedCosts?.length ? input.landedCosts.map((l) => ({ id: uid('lc'), ...l })) : (po.landedCosts ?? []);

  // Pass 1: value at order price, per receiving line, to weight the landed-cost spread.
  const receivedValued = [...byProduct.values()]
    .filter((r) => r.receivedQty > 0)
    .map((r) => {
      const poLine = po.lines.find((l) => l.productId === r.productId)!;
      const product = productById(r.productId);
      const unitCost = baseUnitCost(poLine);
      return { productId: r.productId, qty: r.receivedQty, value: round2(r.receivedQty * unitCost), isService: product.type === 'service' };
    });
  const { shareByProduct, ownSupplierTotal, otherSupplierLines } = allocateLandedCosts({ ...po, landedCosts }, receivedValued.filter((r) => !r.isService));

  const date = input.date;
  let inventoryValue = 0;
  const serviceByAccount = new Map<string, number>();
  const receivedLineDetails: { productId: string; qty: number }[] = [];

  for (const rv of receivedValued) {
    const poLine = po.lines.find((l) => l.productId === rv.productId)!;
    const product = productById(rv.productId);
    if (product.type === 'service') {
      const accId = purchaseLineAccountId(rv.productId);
      serviceByAccount.set(accId, round2((serviceByAccount.get(accId) ?? 0) + rv.value));
      poLine.receivedQty = round2((poLine.receivedQty ?? 0) + rv.qty);
      continue;
    }
    const landedShare = shareByProduct.get(rv.productId) ?? 0;
    const postedValue = round2(rv.value + landedShare);
    inventoryValue += postedValue;
    poLine.receivedQty = round2((poLine.receivedQty ?? 0) + rv.qty);
    poLine.landedCostShare = round2((poLine.landedCostShare ?? 0) + landedShare);
    receivedLineDetails.push({ productId: rv.productId, qty: rv.qty });

    // Receipt re-averages: value += posted amount (order price + this line's landed-cost share), so
    // stockValue stays exactly Σ posted GL amounts (review A1/A2 + E4).
    applyStockChange(product, rv.qty, postedValue, 'purchase', po, date, po.branchId ?? DEFAULT_BRANCH_ID);

    if (product.trackBatches) {
      const requested = byProduct.get(rv.productId)?.batches?.filter((b) => b.qty > 0) ?? [];
      const unitCostWithLanded = round2(postedValue / rv.qty);
      if (requested.length) {
        for (const b of requested) receiveBatch(product.id, b.qty, unitCostWithLanded, b.batchNo.trim() || `RCV-${Date.now()}`, b.expiryDate, date, po);
      } else {
        receiveBatch(product.id, rv.qty, unitCostWithLanded, `RCV-${po.number}`, undefined, date, po);
      }
    }
  }
  inventoryValue = round2(inventoryValue);

  const allReceived = po.lines.every((l) => lineRemaining(l) <= 0.0001);
  mutate(() => (po.status = 'RECEIVED'));
  mutate(() => (po.receivedDate = date));

  // Non-VAT supplier (review E3): input VAT isn't claimed — it's added to inventory/expense cost instead.
  const supplier = db.suppliers.find((s) => s.id === po.supplierId);
  const vatNotRecoverable = input.vatNotRecoverable ?? !supplier?.vatNumber;
  const receivedRatio = po.grandTotal > 0 ? round2Totals(sum(receivedValued, (r) => r.value) / (po.subTotal || 1)) : 1;
  const vatOnReceipt = round2(po.taxAmount * Math.min(1, receivedRatio));

  // When VAT isn't recoverable, its amount is added to cost instead of claimed as vatInput — but the
  // stock has already been moved via `applyStockChange` above at `postedValue` (order price + landed
  // share only, no VAT top-up), so the extra can't be folded into the `inventory` GL line without
  // also drifting `GL(inventory)` away from `Σ product.stockValue` (review A1/A2). Instead it's its
  // own line, straight to the freightIn/COGS-adjacent role that already stands in for "cost, not a
  // dedicated account" elsewhere in this file (`purchaseLineAccountId`'s own fallback) — never routed
  // through `applyStockChange`, so no separate stockValue bookkeeping is needed for it.
  const dim = { branchId: po.branchId ?? DEFAULT_BRANCH_ID, costCenterId: po.costCenterId };
  const lines: PostingLine[] = [{ role: 'inventory', debit: inventoryValue, ...dim }, ...[...serviceByAccount.entries()].map(([accountId, amount]) => ({ accountId, debit: amount, ...dim }))];
  if (vatNotRecoverable) {
    if (vatOnReceipt > 0) lines.push({ role: 'freightIn', debit: vatOnReceipt, description: 'ضريبة مدخلات غير مستردة (مورد بدون رقم ضريبي) — أُضيفت إلى التكلفة', ...dim });
  } else {
    lines.push({ role: 'vatInput', debit: vatOnReceipt, ...dim });
  }
  const apAmount = round2(sum(receivedValued, (r) => r.value) + vatOnReceipt + ownSupplierTotal);
  lines.push({ role: 'payable', credit: apAmount, partyKind: 'supplier' as const, partyId: po.supplierId, ...dim });
  for (const other of otherSupplierLines) {
    lines.push({ role: 'payable', credit: other.amount, partyKind: 'supplier' as const, partyId: other.supplierId, ...dim });
  }

  // A PO's totals describe exactly what's billed against it — i.e. what's posted to AP (the
  // `purchaseOutstanding`/supplier-balance invariant depends on this exactly). A short delivery
  // "إنشاء أمر متبقٍ" therefore shrinks THIS order's totals down to the received portion; the
  // unreceived remainder becomes its own fresh DRAFT (below) with its own totals — never double-
  // counted, and one posting per document is preserved either way (short with no backorder simply
  // closes this PO out at less than originally ordered, same as a supplier under-shipping for good).
  const receivedSubTotal = round2(sum(receivedValued, (r) => r.value));
  mutate(() => {
    po.subTotal = receivedSubTotal;
    po.taxAmount = vatOnReceipt;
    po.grandTotal = apAmount;
    po.vatNotRecoverable = vatNotRecoverable;
    if (input.supplierInvoiceNo !== undefined) po.supplierInvoiceNo = input.supplierInvoiceNo;
    if (input.supplierInvoiceDate !== undefined) po.supplierInvoiceDate = input.supplierInvoiceDate;
    if (landedCosts.length && !po.landedCosts?.length) po.landedCosts = landedCosts;
  });

  postJournal({
    date,
    description: `استلام أمر شراء ${po.number}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'purchaseOrder', id: po.id, number: po.number },
    lines,
    createdBy: userId,
  });

  logActivity('purchase', `استلام أمر الشراء ${po.number} من ${supplier?.name ?? ''} بقيمة ${apAmount.toFixed(2)}`, userId, date, `/purchases/${po.id}`);
  emit('parties:changed');

  // v2 §1 "short delivery → إنشاء أمر متبقٍ" — a DRAFT backorder PO for exactly the shortfall,
  // linked back via `backorderOfId`. One posting per document is preserved: the backorder is its
  // own fresh DRAFT, received separately later.
  if (input.createBackorder && !allReceived) {
    const shortLines = po.lines
      .filter((l) => lineRemaining(l) > 0.0001)
      .map((l) => ({ productId: l.productId, qty: round2(lineRemaining(l) / (l.unitFactor ?? 1)), costPrice: l.costPrice, unitId: l.unitId, unitFactor: l.unitFactor, taxId: l.taxId }));
    if (shortLines.length) {
      const backorder = savePurchase({ supplierId: po.supplierId, date, note: `أمر متبقٍ من ${po.number}`, confirm: false, lines: shortLines }, userId);
      mutate(() => (backorder.backorderOfId = po.id));
      logActivity('purchase', `إنشاء أمر متبقٍ ${backorder.number} من ${po.number}`, userId, date, `/purchases/${backorder.id}`);
    }
  }

  return po;
}

/** Legacy alias — the v1 "confirm" name, now meaning "receive everything ordered, at order prices, today". Kept for callers (seed history, tests) that post a PO in one step. */
export function confirmPurchase(id: string, userId: string, date = new Date().toISOString()): PurchaseOrder {
  const po = db.purchaseOrders.find((p) => p.id === id);
  if (!po) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
  return receivePurchase(id, { date, lines: po.lines.map((l) => ({ productId: l.productId, receivedQty: lineRemaining(l) })) }, userId);
}

export function cancelPurchase(id: string, userId: string): PurchaseOrder {
  const po = db.purchaseOrders.find((p) => p.id === id);
  if (!po) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
  if (po.status !== 'DRAFT' && po.status !== 'ORDERED') throw new ApiError('يمكن إلغاء المسودات والأوامر المرسلة فقط — استخدم مرتجع المشتريات للأوامر المستلمة');
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

/** v2 §4 debit notes — reason required, refund method, batch picking for tracked products. */
export function recordPurchaseReturn(input: PurchaseReturnInput, userId: string, date = new Date().toISOString()): PurchaseReturn {
  const po = db.purchaseOrders.find((p) => p.id === input.purchaseOrderId);
  if (!po) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
  if (po.status !== 'RECEIVED') throw new ApiError('يمكن الإرجاع من أوامر الشراء المستلمة فقط');
  if (!input.reason?.trim()) throw new ApiError('سبب الإرجاع مطلوب');

  const requested = input.lines.filter((l) => l.qty > 0);
  if (!requested.length) throw new ApiError('اختر صنفاً واحداً على الأقل للإرجاع');

  const returned = returnedQtyByProduct(po.id);
  const lines: DebitNoteLine[] = requested.map((line) => {
    const poLine = po.lines.find((l) => l.productId === line.productId);
    if (!poLine) throw new ApiError('الصنف غير موجود في أمر الشراء');
    const product = productById(line.productId);
    const remaining = round2(baseQty(poLine) - (returned.get(line.productId) ?? 0));
    if (line.qty > remaining) throw new ApiError(`لا يمكن إرجاع أكثر من ${remaining} من "${product.name}"`);
    if (product.type === 'product' && line.qty > product.stockQty) {
      throw new ApiError(`المخزون الحالي من "${product.name}" (${product.stockQty}) أقل من كمية الإرجاع`, 'CONFLICT');
    }
    const unitCost = baseUnitCost(poLine) + (poLine.landedCostShare && baseQty(poLine) > 0 ? poLine.landedCostShare / baseQty(poLine) : 0);
    return { productId: line.productId, qty: line.qty, costPrice: round2(unitCost), batchId: line.batchId };
  });

  const totals = computePurchaseTotals(lines, undefined, po.taxRate);
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
    fromDraftId: input.fromDraftId,
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
    applyStockChange(product, -line.qty, -valueOut, 'purchase_return', ret, date, po.branchId ?? DEFAULT_BRANCH_ID);

    // v2 §4 batch picking: draw the returned qty from the chosen batch (or FEFO-oldest if unspecified).
    if (product.trackBatches) {
      if (line.batchId) {
        const batch = db.productBatches.find((b) => b.id === line.batchId && b.productId === product.id);
        if (batch) batch.qty = round2(Math.max(0, batch.qty - line.qty));
      } else {
        let remaining = line.qty;
        for (const batch of activeBatchesFor(product.id)) {
          if (remaining <= 0.0001) break;
          const take = Math.min(batch.qty, remaining);
          batch.qty = round2(batch.qty - take);
          remaining = round2(remaining - take);
        }
      }
    }
  }
  inventoryValue = round2(inventoryValue);

  const retDim = { branchId: po.branchId ?? DEFAULT_BRANCH_ID, costCenterId: po.costCenterId };
  const postingLines: PostingLine[] = [
    { role: 'payable', debit: settledToPayable, partyKind: 'supplier', partyId: po.supplierId, ...retDim },
    { accountId: settlementAccountFor(refundMethod).id, debit: refundMethod === 'credit' ? 0 : cashBack, ...retDim },
    { role: 'payable', debit: refundMethod === 'credit' ? cashBack : 0, partyKind: 'supplier', partyId: po.supplierId, ...retDim },
    { role: 'inventory', credit: inventoryValue, ...retDim },
    ...[...serviceByAccount.entries()].map(([accountId, amount]) => ({ accountId, credit: amount, ...retDim })),
  ];
  // E3: a non-VAT supplier's receipt never debited vatInput (the VAT was added to cost instead), so
  // the debit note must not credit vatInput either — it credits the same freightIn "cost, not a
  // dedicated account" line the receipt used, matching what was actually debited there.
  if (po.vatNotRecoverable) {
    if (totals.taxAmount > 0) postingLines.push({ role: 'freightIn', credit: totals.taxAmount });
  } else {
    postingLines.push({ role: 'vatInput', credit: totals.taxAmount });
  }
  if (variance > 0) postingLines.push({ role: 'inventoryVariance', debit: variance });
  else if (variance < 0) postingLines.push({ role: 'inventoryVariance', credit: -variance });

  postJournal({
    date,
    description: `مرتجع مشتريات ${ret.number} على أمر الشراء ${po.number} — ${input.reason}`,
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
    db.purchaseOrders.filter((p) => p.supplierId === supplierId && p.status === 'RECEIVED'),
    purchaseOutstanding,
  );
}

// ---------------------------------------------------------------------------------------------
// v2 §4 "return expiring batch" shortcut — completes Phase 6's `draftReturnToSupplier` stub
// (src/mocks/backend/inventory.ts, db.debitNoteDrafts) into a real posted debit note.
// ---------------------------------------------------------------------------------------------

/** Every draft still awaiting a real debit note (Phase 6's expiry-report shortcut). */
export function getDebitNoteDrafts(): DebitNoteDraft[] {
  return db.debitNoteDrafts;
}

/**
 * Posts a real debit note from one of Phase 6's `DebitNoteDraft` rows: finds the purchase order the
 * batch was received on (`ProductBatch.sourceRefId`) and calls `recordPurchaseReturn` against it,
 * then removes the draft. Requires every draft line to trace back to a RECEIVED purchase order —
 * batches from opening stock (no purchase order) can't become a debit note and are reported so the
 * caller can fall back to a write-off instead.
 */
export function postDebitNoteFromDraft(draftId: string, refundMethod: PurchaseReturnInput['refundMethod'], userId: string): PurchaseReturn {
  const draft = db.debitNoteDrafts.find((d) => d.id === draftId);
  if (!draft) throw new ApiError('مسودة الإرجاع غير موجودة', 'NOT_FOUND');
  if (!draft.lines.length) throw new ApiError('لا توجد أصناف في المسودة');

  const poIds = new Set<string>();
  for (const line of draft.lines) {
    const batch = db.productBatches.find((b) => b.id === line.batchId);
    const poId = batch?.sourceRefId && db.purchaseOrders.find((p) => p.id === batch.sourceRefId)?.status === 'RECEIVED' ? batch.sourceRefId : undefined;
    if (!poId) throw new ApiError('إحدى التشغيلات ليست من أمر شراء مستلم — استخدم الإتلاف بدلاً من الإرجاع لهذه التشغيلة', 'CONFLICT');
    poIds.add(poId);
  }
  if (poIds.size > 1) throw new ApiError('تشغيلات المسودة من أوامر شراء مختلفة — أنشئ مرتجعاً منفصلاً لكل أمر شراء', 'CONFLICT');
  const purchaseOrderId = [...poIds][0];

  const ret = recordPurchaseReturn(
    {
      purchaseOrderId,
      reason: draft.note || 'بضاعة منتهية/قاربت على انتهاء الصلاحية',
      refundMethod,
      lines: draft.lines.map((l) => ({ productId: l.productId, qty: l.qty, batchId: l.batchId })),
      fromDraftId: draft.id,
    },
    userId,
  );

  mutate(() => (db.debitNoteDrafts = db.debitNoteDrafts.filter((d) => d.id !== draft.id)));
  return ret;
}
