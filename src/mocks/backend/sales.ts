import type { Invoice, JournalPreviewLine, Refund, RefundInput, SaleInput } from '@/modules/invoices/types';
import { computeSaleTotals, invoiceOutstanding, paymentStatusFor } from '@/modules/invoices/helpers/totals';
import { db, nextNumber } from '../db';
import { ApiError, round2, sum, uid } from '../utils';
import {
  accountByCode,
  applyStockChange,
  logActivity,
  postJournal,
  productById,
  salesTaxRate,
  settlementAccount,
  userById,
  type PostingLine,
} from './core';

const METHOD_LABEL: Record<string, string> = {
  cash: 'نقداً',
  card: 'بطاقة',
  bank_transfer: 'تحويل بنكي',
  credit: 'آجل',
};

interface PreparedSale {
  totals: ReturnType<typeof computeSaleTotals>;
  paidAmount: number;
  costTotal: number;
  taxRate: number;
  posting: PostingLine[];
}

/** Validate a sale and build its posting without saving anything (used by preview + record). */
function prepareSale(input: SaleInput, userId: string): PreparedSale {
  if (!input.lines.length) throw new ApiError('السلة فارغة');
  const user = userById(userId);
  if (user && input.discountRate > user.maxDiscount) {
    throw new ApiError(`الخصم يتجاوز الحد المسموح لك (${user.maxDiscount}%)`, 'FORBIDDEN');
  }
  if (input.discountRate < 0 || input.discountRate > 100) throw new ApiError('نسبة الخصم غير صحيحة');

  // Aggregate quantities per product so two cart rows of the same item are checked together.
  const qtyByProduct = new Map<string, number>();
  let costTotal = 0;
  for (const line of input.lines) {
    if (!(line.qty > 0)) throw new ApiError('الكمية يجب أن تكون أكبر من صفر');
    if (line.price < 0) throw new ApiError('السعر لا يمكن أن يكون سالباً');
    const product = productById(line.productId);
    if (!product.active) throw new ApiError(`المنتج "${product.name}" غير نشط`);
    if (product.type === 'product') {
      qtyByProduct.set(product.id, (qtyByProduct.get(product.id) ?? 0) + line.qty);
      costTotal += line.qty * product.costPrice;
    }
  }
  for (const [productId, qty] of qtyByProduct) {
    const product = productById(productId);
    if (qty > product.stockQty) {
      throw new ApiError(`الكمية المطلوبة من "${product.name}" غير متوفرة — المتاح ${product.stockQty}`, 'CONFLICT');
    }
  }

  const taxRate = salesTaxRate();
  const totals = computeSaleTotals(input.lines, input.discountRate, taxRate);

  let paidAmount: number;
  if (input.paymentMethod === 'card' || input.paymentMethod === 'bank_transfer') paidAmount = totals.grandTotal;
  else if (input.paymentMethod === 'credit') paidAmount = 0;
  else paidAmount = Math.min(round2(input.paidAmount), totals.grandTotal);
  if (paidAmount < 0) throw new ApiError('المبلغ المدفوع غير صحيح');

  if (paidAmount < totals.grandTotal) {
    if (!input.customerId) throw new ApiError('البيع الآجل أو الدفع الجزئي يتطلب اختيار عميل');
    const customer = db.customers.find((c) => c.id === input.customerId);
    if (!customer?.active) throw new ApiError('العميل غير موجود أو غير نشط');
  }

  costTotal = round2(costTotal);
  const receivable = round2(totals.grandTotal - paidAmount);
  const posting: PostingLine[] = [
    { code: settlementAccount(input.paymentMethod), debit: paidAmount },
    { code: '1130', debit: receivable },
    { code: '4100', credit: totals.taxable },
    { code: '2150', credit: totals.taxAmount },
    { code: '5200', debit: costTotal },
    { code: '1140', credit: costTotal },
  ];
  return { totals, paidAmount, costTotal, taxRate, posting };
}

export function previewSaleJournal(input: SaleInput, userId: string): JournalPreviewLine[] {
  const { posting } = prepareSale(input, userId);
  return posting
    .filter((l) => (l.debit ?? 0) > 0 || (l.credit ?? 0) > 0)
    .map((l) => {
      const account = accountByCode(l.code!);
      return { accountCode: account.code, accountName: account.name, debit: round2(l.debit ?? 0), credit: round2(l.credit ?? 0) };
    });
}

export function recordSale(input: SaleInput, userId: string, date = new Date().toISOString()): Invoice {
  const { totals, paidAmount, taxRate, posting } = prepareSale(input, userId);

  const id = uid('inv');
  const invoice: Invoice = {
    id,
    number: nextNumber('invoice'),
    date,
    customerId: input.customerId,
    cashierId: userId,
    status: 'COMPLETED',
    paymentStatus: paymentStatusFor(totals.grandTotal, paidAmount),
    lines: input.lines.map((l, i) => {
      const product = productById(l.productId);
      return {
        id: `${id}-l${i + 1}`,
        productId: product.id,
        name: product.name,
        qty: l.qty,
        price: l.price,
        costPrice: product.costPrice,
        discount: l.discount ?? 0,
      };
    }),
    subTotal: totals.subTotal,
    discountRate: input.discountRate,
    discountAmount: totals.discountAmount,
    taxRate,
    taxAmount: totals.taxAmount,
    grandTotal: totals.grandTotal,
    paymentMethod: input.paymentMethod,
    paidAmount,
    refundedAmount: 0,
    tenderedAmount: input.paymentMethod === 'cash' ? input.tenderedAmount : undefined,
    note: input.note,
  };
  db.invoices.push(invoice);

  for (const line of invoice.lines) {
    applyStockChange(productById(line.productId), -line.qty, 'sale', invoice, date);
  }

  postJournal({
    date,
    description: `فاتورة مبيعات ${invoice.number} (${METHOD_LABEL[invoice.paymentMethod]})`,
    type: 'SYSTEM',
    sourceRef: { kind: 'invoice', id: invoice.id, number: invoice.number },
    lines: posting,
    createdBy: userId,
  });

  const customer = db.customers.find((c) => c.id === invoice.customerId);
  logActivity(
    'sale',
    `فاتورة ${invoice.number} بقيمة ${invoice.grandTotal.toFixed(2)}${customer ? ` — ${customer.name}` : ''}`,
    userId,
    date,
    `/invoices/${invoice.id}`,
  );
  return invoice;
}

/** Quantities already returned per invoice line. */
export function returnedQtyByLine(invoiceId: string): Map<string, number> {
  const map = new Map<string, number>();
  for (const refund of db.refunds.filter((r) => r.invoiceId === invoiceId)) {
    for (const line of refund.lines) map.set(line.invoiceLineId, (map.get(line.invoiceLineId) ?? 0) + line.qty);
  }
  return map;
}

export function recordRefund(input: RefundInput, userId: string, date = new Date().toISOString()): Refund {
  const invoice = db.invoices.find((i) => i.id === input.invoiceId);
  if (!invoice) throw new ApiError('الفاتورة غير موجودة', 'NOT_FOUND');
  if (invoice.status !== 'COMPLETED') throw new ApiError('لا يمكن إرجاع هذه الفاتورة');

  const lines = input.lines.filter((l) => l.qty > 0);
  if (!lines.length) throw new ApiError('اختر صنفاً واحداً على الأقل للإرجاع');

  const returned = returnedQtyByLine(invoice.id);
  let net = 0;
  let cost = 0;
  for (const line of lines) {
    const invLine = invoice.lines.find((l) => l.id === line.invoiceLineId);
    if (!invLine) throw new ApiError('سطر الفاتورة غير موجود');
    const remaining = invLine.qty - (returned.get(invLine.id) ?? 0);
    if (line.qty > remaining) throw new ApiError(`لا يمكن إرجاع أكثر من ${remaining} من "${invLine.name}"`);
    net += line.qty * (invLine.price - invLine.discount / invLine.qty);
    const product = db.products.find((p) => p.id === invLine.productId);
    if (product?.type === 'product') cost += line.qty * invLine.costPrice;
  }

  // Is this the last return for the invoice? Then use exact remainders so no cents are left over.
  const isFinal = invoice.lines.every((invLine) => {
    const returning = lines.find((l) => l.invoiceLineId === invLine.id)?.qty ?? 0;
    return (returned.get(invLine.id) ?? 0) + returning >= invLine.qty;
  });
  const previous = db.refunds.filter((r) => r.invoiceId === invoice.id);
  let subTotal: number;
  let taxAmount: number;
  if (isFinal) {
    subTotal = round2(invoice.subTotal - invoice.discountAmount - sum(previous, (r) => r.subTotal));
    taxAmount = round2(invoice.taxAmount - sum(previous, (r) => r.taxAmount));
  } else {
    subTotal = round2(net * (1 - invoice.discountRate / 100));
    taxAmount = round2((subTotal * invoice.taxRate) / 100);
  }
  const grandTotal = round2(subTotal + taxAmount);
  const settledToReceivable = Math.min(grandTotal, invoiceOutstanding(invoice));
  const cashBack = round2(grandTotal - settledToReceivable);
  cost = round2(cost);

  const id = uid('ref');
  const refund: Refund = {
    id,
    number: nextNumber('refund'),
    invoiceId: invoice.id,
    date,
    reason: input.reason,
    lines,
    subTotal,
    taxAmount,
    grandTotal,
    settledToReceivable,
    cashBack,
  };
  db.refunds.push(refund);

  invoice.refundedAmount = round2(invoice.refundedAmount + grandTotal);
  if (isFinal) invoice.status = 'REFUNDED';
  invoice.paymentStatus = paymentStatusFor(invoice.grandTotal - invoice.refundedAmount, invoice.paidAmount);

  for (const line of lines) {
    const invLine = invoice.lines.find((l) => l.id === line.invoiceLineId)!;
    applyStockChange(productById(invLine.productId), line.qty, 'refund', refund, date);
  }

  postJournal({
    date,
    description: `مرتجع مبيعات ${refund.number} على الفاتورة ${invoice.number}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'refund', id: refund.id, number: refund.number },
    lines: [
      { code: '4200', debit: subTotal },
      { code: '2150', debit: taxAmount },
      { code: '1130', credit: settledToReceivable },
      { code: settlementAccount(invoice.paymentMethod), credit: cashBack },
      { code: '1140', debit: cost },
      { code: '5200', credit: cost },
    ],
    createdBy: userId,
  });

  logActivity('refund', `مرتجع ${refund.number} على الفاتورة ${invoice.number} بقيمة ${grandTotal.toFixed(2)}`, userId, date, `/invoices/${invoice.id}`);
  return refund;
}
