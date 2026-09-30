import type {
  DebitNoteLine,
  LandedCostLine,
  PurchaseLine,
  PurchaseLineInput,
  PurchaseOrder,
  PurchaseOrderInput,
  PurchaseReturn,
  PurchaseReturnInput,
  ReceiveLineInput,
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
import { applyStockChange, logActivity, postJournal, preflightJournal, productById, purchaseTaxRate, DEFAULT_BRANCH_ID, type PostingLine } from './core';
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

  const found = existingId ? db.purchaseOrders.find((p) => p.id === existingId) : undefined;
  if (existingId && !found) throw new ApiError('أمر الشراء غير موجود', 'NOT_FOUND');
  if (found && found.status !== 'DRAFT') throw new ApiError('لا يمكن تعديل أمر شراء تم إرساله أو استلامه أو إلغاؤه');

  // ACC-0013: build the order as it will be saved, and (confirm) plan + validate its receipt, BEFORE
  // writing anything — a refused receipt (closed period, …) must not leave a saved draft or a
  // consumed document number behind (the Rust backend rolls the whole command back).
  const branchId = found ? found.branchId : (input.branchId ?? DEFAULT_BRANCH_ID);
  const fields = {
    supplierId: input.supplierId,
    date: input.date,
    lines: input.lines.map((l) => ({ ...l })),
    note: input.note,
    invoiceDiscount: input.invoiceDiscount,
    landedCosts: (input.landedCosts ?? []).map((l) => ({ id: uid('lc'), ...l })),
    supplierInvoiceNo: input.supplierInvoiceNo,
    supplierInvoiceDate: input.supplierInvoiceDate,
    attachmentIds: input.attachmentIds,
    costCenterId: defaultCostCenterFor(branchId, input.costCenterId),
    ...totals,
  };
  const candidate: PurchaseOrder = found
    ? { ...found, ...fields }
    : { id: uid('po'), number: '', status: 'DRAFT', paymentStatus: 'UNPAID', paidAmount: 0, returnedAmount: 0, branchId, currency: input.currency, exchangeRate: input.exchangeRate, ...fields };
  const receiptInput: ReceivePurchaseInput | undefined = input.confirm
    ? { date: input.date, lines: candidate.lines.map((l) => ({ productId: l.productId, receivedQty: baseQty(l) })) }
    : undefined;
  const receipt = receiptInput ? planReceipt(candidate, receiptInput) : undefined;

  let po: PurchaseOrder;
  if (found) {
    mutate(() => Object.assign(found, fields));
    po = found;
  } else {
    po = { ...candidate, number: `${branchPrefix(branchId!)}${nextNumber('purchaseOrder')}` };
    const created = po;
    mutate(() => db.purchaseOrders.push(created));
  }

  if (receipt && receiptInput) applyReceipt(po, receipt, receiptInput, userId);
  else logActivity('purchase', `حفظ أمر الشراء ${po.number} كمسودة`, userId, input.date, { name: 'purchase', params: { id: po.id } });
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
  logActivity('purchase', `إرسال أمر الشراء ${po.number} للمورد`, userId, po.sentAt!, { name: 'purchase', params: { id: po.id } });
  return po;
}

/** Spreads landed-cost lines belonging to the purchase's own supplier over the receiving stock lines, by value or qty (docs/v2/09 §1, review E4). Returns each product's per-unit landed-cost share (base unit) plus the AP add-on for the po's own supplier and any other-supplier AP lines. */
function allocateLandedCosts(
  po: PurchaseOrder,
  received: { productId: string; qty: number; value: number }[],
): { shareByProduct: Map<string, number>; ownSupplierTotal: number; otherSupplierLines: { supplierId: string; label: string; amount: number }[] } {
  const shareByProduct = new Map<string, number>();
  let ownSupplierTotal = 0;
  const otherSupplierLines: { supplierId: string; label: string; amount: number }[] = [];
  const stockLines = received.filter((r) => r.qty > 0);
  const totalValue = sum(stockLines, (r) => r.value);
  const totalQty = round2(stockLines.reduce((a, r) => a + r.qty, 0));

  for (const lc of po.landedCosts ?? []) {
    if (!(lc.amount > 0)) continue;
    if (!lc.supplierId || lc.supplierId === po.supplierId) ownSupplierTotal = round2(ownSupplierTotal + lc.amount);
    else otherSupplierLines.push({ supplierId: lc.supplierId, label: lc.label, amount: round2(lc.amount) });

    if (!stockLines.length) continue;
    const weight = lc.spreadBy === 'qty' ? (r: (typeof stockLines)[number]) => r.qty : (r: (typeof stockLines)[number]) => r.value;
    const base = lc.spreadBy === 'qty' ? totalQty : totalValue;
    if (!(base > 0)) continue;
    // Rule (ACC-0004): each share is round2'd and the rounding remainder goes to the largest-weight line (ties → first), so the shares sum exactly to the landed cost.
    const shares = stockLines.map((r) => round2((lc.amount * weight(r)) / base));
    let largest = 0;
    stockLines.forEach((r, i) => {
      if (weight(r) > weight(stockLines[largest])) largest = i;
    });
    shares[largest] = round2(shares[largest] + round2(round2(lc.amount) - shares.reduce((a, s) => a + s, 0)));
    stockLines.forEach((r, i) => shareByProduct.set(r.productId, round2((shareByProduct.get(r.productId) ?? 0) + shares[i])));
  }
  return { shareByProduct, ownSupplierTotal, otherSupplierLines };
}

function lineRemaining(line: PurchaseLine): number {
  return round2(baseQty(line) - (line.receivedQty ?? 0));
}

/**
 * Everything one receipt writes, computed and validated up front (ACC-0013): `planReceipt` raises
 * every refusal a receipt can hit — status/qty guards, a missing other-supplier, the posting's
 * period/balance check, the backorder's own validation — and mutates nothing; `applyReceipt` then
 * only writes. So a refused receipt leaves no stock, PO-line, counter or row change behind, the
 * same as the Rust backend's rolled-back transaction.
 */
interface ReceiptPlan {
  date: string;
  byProduct: Map<string, ReceiveLineInput>;
  landedCosts: LandedCostLine[];
  received: { productId: string; qty: number; isService: boolean; postedValue: number; landedShare: number }[];
  supplierName: string;
  vatNotRecoverable: boolean;
  vatOnReceipt: number;
  apAmount: number;
  receivedSubTotal: number;
  postingLines: PostingLine[];
  /** ACC-0012: one open payable document per landed-cost line billed by another supplier; `lineIndex` is its `Cr payable` line in `postingLines`. */
  otherSupplierBills: { supplierId: string; supplierName: string; label: string; amount: number; lineIndex: number }[];
  /** Set when a short receipt asked for a backorder — already validated like any new order. */
  backorderLines?: PurchaseLineInput[];
}

function planReceipt(po: PurchaseOrder, input: ReceivePurchaseInput): ReceiptPlan {
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
      // Rule (ACC-0005): a non-stock line (service, or a product with stockMode 'none') is expensed to its purchase account, never debited to inventory.
      return { productId: r.productId, qty: r.receivedQty, value: round2(r.receivedQty * unitCost), isService: product.type === 'service' || product.stockMode === 'none' };
    });
  const { shareByProduct, ownSupplierTotal, otherSupplierLines } = allocateLandedCosts({ ...po, landedCosts }, receivedValued.filter((r) => !r.isService));
  // Rule (ACC-0012): the other supplier becomes the owner of a real payable document, so it has to exist.
  const otherSupplierNames = otherSupplierLines.map((o) => {
    const s = db.suppliers.find((x) => x.id === o.supplierId);
    if (!s) throw new ApiError('مورد التكلفة الإضافية غير موجود');
    return s.name;
  });

  let inventoryValue = 0;
  const serviceByAccount = new Map<string, number>();
  const received = receivedValued.map((rv) => {
    if (rv.isService) {
      const accId = purchaseLineAccountId(rv.productId);
      serviceByAccount.set(accId, round2((serviceByAccount.get(accId) ?? 0) + rv.value));
      return { productId: rv.productId, qty: rv.qty, isService: true, postedValue: rv.value, landedShare: 0 };
    }
    const landedShare = shareByProduct.get(rv.productId) ?? 0;
    const postedValue = round2(rv.value + landedShare);
    inventoryValue += postedValue;
    return { productId: rv.productId, qty: rv.qty, isService: false, postedValue, landedShare };
  });
  inventoryValue = round2(inventoryValue);

  // Non-VAT supplier (review E3): input VAT isn't claimed — it's added to inventory/expense cost instead.
  const supplier = db.suppliers.find((s) => s.id === po.supplierId);
  const vatNotRecoverable = input.vatNotRecoverable ?? !supplier?.vatNumber;
  const receivedRatio = po.grandTotal > 0 ? round2Totals(sum(receivedValued, (r) => r.value) / (po.subTotal || 1)) : 1;
  const vatOnReceipt = round2(po.taxAmount * Math.min(1, receivedRatio));

  // When VAT isn't recoverable, its amount is added to cost instead of claimed as vatInput — but the
  // stock is moved via `applyStockChange` at `postedValue` (order price + landed share only, no VAT
  // top-up), so the extra can't be folded into the `inventory` GL line without also drifting
  // `GL(inventory)` away from `Σ product.stockValue` (review A1/A2). Instead it's its own line,
  // straight to the freightIn/COGS-adjacent role that already stands in for "cost, not a dedicated
  // account" elsewhere in this file (`purchaseLineAccountId`'s own fallback) — never routed through
  // `applyStockChange`, so no separate stockValue bookkeeping is needed for it.
  const dim = { branchId: po.branchId ?? DEFAULT_BRANCH_ID, costCenterId: po.costCenterId };
  const postingLines: PostingLine[] = [{ role: 'inventory', debit: inventoryValue, ...dim }, ...[...serviceByAccount.entries()].map(([accountId, amount]) => ({ accountId, debit: amount, ...dim }))];
  if (vatNotRecoverable) {
    if (vatOnReceipt > 0) postingLines.push({ role: 'freightIn', debit: vatOnReceipt, description: 'ضريبة مدخلات غير مستردة (مورد بدون رقم ضريبي) — أُضيفت إلى التكلفة', ...dim });
  } else {
    postingLines.push({ role: 'vatInput', debit: vatOnReceipt, ...dim });
  }
  const apAmount = round2(sum(receivedValued, (r) => r.value) + vatOnReceipt + ownSupplierTotal);
  postingLines.push({ role: 'payable', credit: apAmount, partyKind: 'supplier' as const, partyId: po.supplierId, ...dim });
  const otherSupplierBills = otherSupplierLines.map((other, i) => {
    const lineIndex = postingLines.length;
    postingLines.push({ role: 'payable', credit: other.amount, partyKind: 'supplier' as const, partyId: other.supplierId, ...dim });
    return { ...other, supplierName: otherSupplierNames[i], lineIndex };
  });
  preflightJournal({ date: input.date, lines: postingLines });

  // The short-delivery backorder ("إنشاء أمر متبقٍ") is validated here like any new order, so a
  // refused backorder refuses the whole receipt instead of leaving it posted without one.
  const addedByLine = new Map<PurchaseLine, number>();
  for (const r of received) addedByLine.set(po.lines.find((l) => l.productId === r.productId)!, r.qty);
  const remainingAfter = (l: PurchaseLine) => {
    const added = addedByLine.get(l);
    return added === undefined ? lineRemaining(l) : round2(baseQty(l) - round2((l.receivedQty ?? 0) + added));
  };
  const allReceived = po.lines.every((l) => remainingAfter(l) <= 0.0001);
  let backorderLines: PurchaseLineInput[] | undefined;
  if (input.createBackorder && !allReceived) {
    const shortLines = po.lines
      .filter((l) => remainingAfter(l) > 0.0001)
      .map((l) => ({ productId: l.productId, qty: round2(remainingAfter(l) / (l.unitFactor ?? 1)), costPrice: l.costPrice, unitId: l.unitId, unitFactor: l.unitFactor, taxId: l.taxId }));
    if (shortLines.length) {
      validateLines({ supplierId: po.supplierId, lines: shortLines });
      backorderLines = shortLines;
    }
  }

  return {
    date: input.date,
    byProduct,
    landedCosts,
    received,
    supplierName: supplier?.name ?? '',
    vatNotRecoverable,
    vatOnReceipt,
    apAmount,
    receivedSubTotal: round2(sum(receivedValued, (r) => r.value)),
    postingLines,
    otherSupplierBills,
    backorderLines,
  };
}

/**
 * Rule (ACC-0012): a landed-cost line billed by ANOTHER supplier (a shipper, a customs broker) is
 * that supplier's own bill — a RECEIVED purchase order with no stock lines whose total is the
 * landed amount — so the shipper's `Cr payable` has an open document behind it (open documents,
 * payment allocation, `supplier-allocation`). Its cost is already inside the receipt's inventory
 * posting; this document carries only the liability, and its GL line stays in the receipt's entry.
 */
function newOtherSupplierBill(po: PurchaseOrder, bill: ReceiptPlan['otherSupplierBills'][number], date: string): PurchaseOrder {
  const branchId = po.branchId ?? DEFAULT_BRANCH_ID;
  return {
    id: uid('po'),
    number: `${branchPrefix(branchId)}${nextNumber('purchaseOrder')}`,
    supplierId: bill.supplierId,
    date,
    status: 'RECEIVED',
    lines: [],
    subTotal: bill.amount,
    taxRate: 0,
    taxAmount: 0,
    grandTotal: bill.amount,
    paymentStatus: 'UNPAID',
    paidAmount: 0,
    returnedAmount: 0,
    note: `تكلفة إضافية "${bill.label}" على أمر الشراء ${po.number}`,
    receivedDate: date,
    costCenterId: po.costCenterId,
    branchId,
  };
}

function applyReceipt(po: PurchaseOrder, plan: ReceiptPlan, input: ReceivePurchaseInput, userId: string): PurchaseOrder {
  const { date } = plan;
  for (const r of plan.received) {
    const poLine = po.lines.find((l) => l.productId === r.productId)!;
    const product = productById(r.productId);
    poLine.receivedQty = round2((poLine.receivedQty ?? 0) + r.qty);
    if (r.isService) continue;
    poLine.landedCostShare = round2((poLine.landedCostShare ?? 0) + r.landedShare);

    // Receipt re-averages: value += posted amount (order price + this line's landed-cost share), so
    // stockValue stays exactly Σ posted GL amounts (review A1/A2 + E4).
    applyStockChange(product, r.qty, r.postedValue, 'purchase', po, date, po.branchId ?? DEFAULT_BRANCH_ID);

    if (product.trackBatches) {
      const requested = plan.byProduct.get(r.productId)?.batches?.filter((b) => b.qty > 0) ?? [];
      const unitCostWithLanded = round2(r.postedValue / r.qty);
      if (requested.length) {
        for (const b of requested) receiveBatch(product.id, b.qty, unitCostWithLanded, b.batchNo.trim() || `RCV-${Date.now()}`, b.expiryDate, date, po);
      } else {
        receiveBatch(product.id, r.qty, unitCostWithLanded, `RCV-${po.number}`, undefined, date, po);
      }
    }
  }

  mutate(() => (po.status = 'RECEIVED'));
  mutate(() => (po.receivedDate = date));

  // A PO's totals describe exactly what's billed against it — i.e. what's posted to AP (the
  // `purchaseOutstanding`/supplier-balance invariant depends on this exactly). A short delivery
  // "إنشاء أمر متبقٍ" therefore shrinks THIS order's totals down to the received portion; the
  // unreceived remainder becomes its own fresh DRAFT (below) with its own totals — never double-
  // counted, and one posting per document is preserved either way (short with no backorder simply
  // closes this PO out at less than originally ordered, same as a supplier under-shipping for good).
  mutate(() => {
    po.subTotal = plan.receivedSubTotal;
    po.taxAmount = plan.vatOnReceipt;
    po.grandTotal = plan.apAmount;
    po.vatNotRecoverable = plan.vatNotRecoverable;
    if (input.supplierInvoiceNo !== undefined) po.supplierInvoiceNo = input.supplierInvoiceNo;
    if (input.supplierInvoiceDate !== undefined) po.supplierInvoiceDate = input.supplierInvoiceDate;
    if (plan.landedCosts.length && !po.landedCosts?.length) po.landedCosts = plan.landedCosts;
  });

  // ACC-0012: each other-supplier landed cost gets its own payable document; its number is drawn
  // before the journal's (the Rust port's fixed counter order) and named on its `Cr payable` line.
  const bills = plan.otherSupplierBills.map((b) => ({ ...b, doc: newOtherSupplierBill(po, b, date) }));
  const postingLines = plan.postingLines.map((l, i) => {
    const bill = bills.find((b) => b.lineIndex === i);
    return bill ? { ...l, description: `تكلفة إضافية "${bill.label}" — ${bill.doc.number}` } : l;
  });

  postJournal({
    date,
    description: `استلام أمر شراء ${po.number}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'purchaseOrder', id: po.id, number: po.number },
    lines: postingLines,
    createdBy: userId,
  });

  logActivity('purchase', `استلام أمر الشراء ${po.number} من ${plan.supplierName} بقيمة ${plan.apAmount.toFixed(2)}`, userId, date, { name: 'purchase', params: { id: po.id } });
  for (const bill of bills) {
    mutate(() => db.purchaseOrders.push(bill.doc));
    logActivity('purchase', `مستحق ${bill.doc.number} لـ ${bill.supplierName} بقيمة ${bill.amount.toFixed(2)} — تكلفة إضافية على أمر الشراء ${po.number}`, userId, date, {
      name: 'purchase',
      params: { id: bill.doc.id },
    });
  }
  emit('parties:changed');

  // v2 §1 "short delivery → إنشاء أمر متبقٍ" — a DRAFT backorder PO for exactly the shortfall,
  // linked back via `backorderOfId`. One posting per document is preserved: the backorder is its
  // own fresh DRAFT, received separately later.
  if (plan.backorderLines) {
    const backorder = savePurchase({ supplierId: po.supplierId, date, note: `أمر متبقٍ من ${po.number}`, confirm: false, lines: plan.backorderLines }, userId);
    mutate(() => (backorder.backorderOfId = po.id));
    logActivity('purchase', `إنشاء أمر متبقٍ ${backorder.number} من ${po.number}`, userId, date, { name: 'purchase', params: { id: backorder.id } });
  }

  return po;
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
  const plan = planReceipt(po, input);
  return applyReceipt(po, plan, input, userId);
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
  logActivity('purchase', `إلغاء أمر الشراء ${po.number}`, userId, new Date().toISOString(), { name: 'purchase', params: { id: po.id } });
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

  // A1/A2: value −= qty × purchase price. If that would leave a negative value, or a non-zero
  // value with zero quantity, the difference goes to inventoryVariance (5110) instead of silently
  // drifting the GL away from Σ product.stockValue. Planned against a running per-product
  // qty/value (what each line would see after the earlier lines moved) so nothing is written until
  // the posting below is known to go through (ACC-0013).
  let inventoryValue = 0;
  let variance = 0;
  const serviceByAccount = new Map<string, number>();
  const running = new Map<string, { qty: number; value: number }>();
  const stockMoves: { line: DebitNoteLine; valueOut: number }[] = [];
  for (const line of lines) {
    const product = productById(line.productId);
    if (product.type === 'service') {
      const accId = purchaseLineAccountId(line.productId);
      const amount = round2(line.qty * line.costPrice);
      serviceByAccount.set(accId, round2((serviceByAccount.get(accId) ?? 0) + amount));
      continue;
    }
    const onHand = running.get(product.id) ?? { qty: product.stockQty, value: product.stockValue };
    const atPurchasePrice = round2(line.qty * line.costPrice);
    const newQty = round2(onHand.qty - line.qty);
    const newValue = round2(onHand.value - atPurchasePrice);
    let valueOut = atPurchasePrice;
    if (newValue < 0 || (newQty <= 0.0001 && Math.abs(newValue) > 0.001)) {
      // Guard: don't let the line's own posted value push stockValue negative / leave a stray
      // balance at zero qty — take exactly what's there and book the rest as variance.
      valueOut = onHand.value;
      variance = round2(variance + (atPurchasePrice - valueOut));
    }
    inventoryValue += valueOut;
    // `applyStockChange` leaves a non-stock (stockMode 'none') product untouched — so does the plan.
    if (product.stockMode !== 'none') running.set(product.id, { qty: round2(onHand.qty - line.qty), value: round2(onHand.value - valueOut) });
    stockMoves.push({ line, valueOut });
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
  // Rule (ACC-0011): the supplier's side is always qty × purchase price (Dr payable/refund); the
  // inventory side is what's actually on the books (`valueOut`). A positive variance means the
  // supplier owes back MORE than the stock removed was carried at, so the difference is a CREDIT
  // to inventoryVariance (a gain); a negative one (more value on the books than the price) is a
  // DEBIT (a loss). Either way the entry balances by construction.
  if (variance > 0) postingLines.push({ role: 'inventoryVariance', credit: variance });
  else if (variance < 0) postingLines.push({ role: 'inventoryVariance', debit: -variance });
  preflightJournal({ date, lines: postingLines });

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

  for (const { line, valueOut } of stockMoves) {
    const product = productById(line.productId);
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

  postJournal({
    date,
    description: `مرتجع مشتريات ${ret.number} على أمر الشراء ${po.number} — ${input.reason}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'purchaseReturn', id: ret.id, number: ret.number },
    lines: postingLines,
    createdBy: userId,
  });

  logActivity('purchase_return', `مرتجع مشتريات ${ret.number} بقيمة ${ret.grandTotal.toFixed(2)}`, userId, date, { name: 'purchase', params: { id: po.id } });
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
