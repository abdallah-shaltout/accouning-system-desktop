import type { Invoice, JournalPreviewLine, Refund, RefundInput, SaleInput, Tender } from '@/modules/invoices/types';
import { computeInvoiceTotals, invoiceOutstanding, paymentStatusFor, round2 as round2Totals } from '@/modules/invoices/helpers/totals';
import type { PaymentMethod } from '@/modules/settings/types';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, round2, sum, uid } from '../utils';
import { accountFor, settlementAccountFor } from './accounts';
import {
  applyStockChange,
  logActivity,
  postJournal,
  productById,
  salesTaxRate,
  userById,
  type PostingLine,
} from './core';

const METHOD_LABEL: Record<string, string> = {
  cash: 'نقداً',
  card: 'بطاقة',
  bank_transfer: 'تحويل بنكي',
  credit: 'آجل',
};

/**
 * Resolves a payment method to its settlement account by system role (docs/v2/02-accounting-review.md
 * C3 fix, docs/v2/09-purchases-payments-expenses.md §2): card/wallet tenders post to their clearing
 * account, not straight to bank, since the money hasn't arrived yet. `settlementAccountFor` in
 * `./accounts` stays untouched (payments.ts/purchases.ts still call it for their own methods) — this
 * is the new, payment-method-aware resolver `sales.ts` uses for tenders.
 * TODO(phase 8): the card-settlement voucher (Dr bank + Dr cardFees / Cr cardClearing) that clears
 * this balance when the bank deposit arrives isn't built yet — this phase only gets the initial
 * posting right, per docs/v2/09 §2 "Card settlement".
 */
function tenderAccountId(method: PaymentMethod): string {
  return accountFor(method.accountRole).id;
}

function paymentMethodById(id: string): PaymentMethod {
  const method = db.paymentMethods.find((m) => m.id === id);
  if (!method) throw new ApiError('طريقة الدفع غير موجودة', 'NOT_FOUND');
  return method;
}

/** The legacy `SalePaymentMethod` string mapped onto a seeded v2 payment method, for callers that don't pass `tenders`. */
const LEGACY_METHOD_ID: Record<SaleInput['paymentMethod'], string> = {
  cash: 'pm-cash',
  card: 'pm-mada',
  bank_transfer: 'pm-bank-transfer',
  credit: 'pm-credit',
};

/** `db.taxes` lookup with a store-default fallback, mirroring the pre-v2 `salesTaxRate()` default. */
function taxForLine(taxId: string | undefined): { rate: number; category: import('@/modules/settings/types').TaxCategory; id?: string } {
  const tax =
    (taxId && db.taxes.find((t) => t.id === taxId && t.active)) ??
    db.taxes.find((t) => t.id === db.settings.defaultTaxId && t.active) ??
    db.taxes.find((t) => t.type === 'OUTPUT' && t.isDefault && t.active);
  return tax ? { rate: tax.rate, category: tax.category, id: tax.id } : { rate: salesTaxRate(), category: 'S' };
}

interface PreparedSale {
  totals: ReturnType<typeof computeInvoiceTotals>;
  tenders: (Tender & { method: PaymentMethod })[];
  paidAmount: number;
  costTotal: number;
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
  for (const line of input.lines) {
    if (!(line.qty > 0)) throw new ApiError('الكمية يجب أن تكون أكبر من صفر');
    if (line.price < 0) throw new ApiError('السعر لا يمكن أن يكون سالباً');
    const product = productById(line.productId);
    if (!product.active) throw new ApiError(`المنتج "${product.name}" غير نشط`);
    if (product.type === 'product') qtyByProduct.set(product.id, (qtyByProduct.get(product.id) ?? 0) + line.qty);
  }
  // COGS per product (A1/A2): round2(qty × avgCost); if the sale empties the stock, use the exact
  // remaining stockValue instead, so nothing is left stranded in the GL.
  let costTotal = 0;
  for (const [productId, qty] of qtyByProduct) {
    const product = productById(productId);
    if (qty > product.stockQty) {
      throw new ApiError(`الكمية المطلوبة من "${product.name}" غير متوفرة — المتاح ${product.stockQty}`, 'CONFLICT');
    }
    costTotal += round2(qty) >= round2(product.stockQty) ? product.stockValue : round2(qty * product.costPrice);
  }

  // v2 pricing/discount/VAT engine (docs/v2/06-sales-and-pos.md §3) — per-line tax, inclusive by
  // default (`company.pricesIncludeTax`), invoice discount spread proportionally with largest-
  // remainder rounding. `taxForLine` snapshots each line's rate/category at sale time.
  const pricesIncludeTax = db.settings.pricesIncludeTax !== false;
  const lineTaxes = input.lines.map((l) => taxForLine(l.taxId));
  const totals = computeInvoiceTotals(
    input.lines.map((l, i) => ({ qty: l.qty, unitPrice: l.price, discount: l.discount ?? 0, discountIsPct: l.discountIsPct ?? false, tax: lineTaxes[i] })),
    input.discountRate > 0 ? { pct: input.discountRate } : undefined,
    pricesIncludeTax,
  );
  const grandTotal = totals.gross;

  // Tenders: either the caller passed a v2 split-payment array, or we synthesize one from the
  // legacy paymentMethod/paidAmount pair so every existing caller (POS cart, CheckoutModal,
  // Phase 4's createSale) keeps working unchanged.
  let tenders: Tender[];
  if (input.tenders?.length) {
    tenders = input.tenders;
  } else if (input.paymentMethod === 'card' || input.paymentMethod === 'bank_transfer') {
    tenders = [{ paymentMethodId: LEGACY_METHOD_ID[input.paymentMethod], amount: grandTotal }];
  } else if (input.paymentMethod === 'credit') {
    tenders = [];
  } else {
    tenders = [{ paymentMethodId: LEGACY_METHOD_ID.cash, amount: Math.min(round2(input.paidAmount), grandTotal) }];
  }
  const resolvedTenders = tenders.map((t) => ({ ...t, method: paymentMethodById(t.paymentMethodId) }));
  const paidAmount = round2Totals(sum(resolvedTenders, (t) => t.amount));
  if (paidAmount < 0) throw new ApiError('المبلغ المدفوع غير صحيح');
  if (paidAmount > grandTotal + 0.01) throw new ApiError('مجموع طرق الدفع أكبر من إجمالي الفاتورة');

  if (paidAmount < grandTotal) {
    if (!input.customerId) throw new ApiError('البيع الآجل أو الدفع الجزئي يتطلب اختيار عميل');
    const customer = db.customers.find((c) => c.id === input.customerId);
    if (!customer?.active) throw new ApiError('العميل غير موجود أو غير نشط');
  }

  costTotal = round2(costTotal);
  const receivable = round2(grandTotal - paidAmount);

  // One posting line per tender, each to its own method's account (C3 fix: card/wallet tenders go
  // to their clearing account, not straight to bank — see `tenderAccountId` above).
  const tenderLines: PostingLine[] = resolvedTenders
    .filter((t) => t.amount > 0)
    .map((t) => ({ accountId: tenderAccountId(t.method), debit: t.amount, description: t.reference ? `${t.method.name} — ${t.reference}` : t.method.name }));

  // vatByCategory (docs/v2/06 §3 step 5) posts sales net per category to the same `sales` role for
  // now — a separate revenue account per category (e.g. 4120 zero-rated sales) is a per-product/
  // per-category account resolution that belongs to Phase 6's product tax fields; the VAT report
  // (below) still gets its category breakdown from the invoice lines regardless of which revenue
  // account the net posts to.
  const posting: PostingLine[] = [
    ...tenderLines,
    { role: 'receivable', debit: receivable, partyKind: 'customer', partyId: input.customerId },
    { role: 'sales', credit: totals.net },
    { role: 'vatOutput', credit: totals.vat },
    { role: 'cogs', debit: costTotal },
    { role: 'inventory', credit: costTotal },
  ];
  return { totals, tenders: resolvedTenders, paidAmount, costTotal, posting };
}

export function previewSaleJournal(input: SaleInput, userId: string): JournalPreviewLine[] {
  const { posting } = prepareSale(input, userId);
  return posting
    .filter((l) => (l.debit ?? 0) > 0 || (l.credit ?? 0) > 0)
    .map((l) => {
      const account = l.accountId ? db.accounts.find((a) => a.id === l.accountId)! : accountFor(l.role!);
      return { accountCode: account.code, accountName: account.name, debit: round2(l.debit ?? 0), credit: round2(l.credit ?? 0) };
    });
}

export function recordSale(input: SaleInput, userId: string, date = new Date().toISOString()): Invoice {
  const { totals, tenders, paidAmount, posting } = prepareSale(input, userId);
  const lineTaxes = input.lines.map((l) => taxForLine(l.taxId));

  // `taxRate`/`taxAmount`/`grandTotal` are kept as a single-rate snapshot for the legacy print/
  // refund/report code paths (InvoiceA4, InvoiceThermal, recordRefund — outside this phase's
  // surface): the store's dominant (first-line) rate stands in for what used to be "the" VAT rate.
  // `lines[].taxRate`/`taxCategory`/`net`/`vat` below carry the real per-line v2 detail the VAT
  // report reads.
  const id = uid('inv');
  const invoice: Invoice = {
    id,
    number: nextNumber('invoice'),
    date,
    customerId: input.customerId,
    cashierId: userId,
    status: 'COMPLETED',
    paymentStatus: paymentStatusFor(totals.gross, paidAmount),
    lines: input.lines.map((l, i) => {
      const product = productById(l.productId);
      const lr = totals.lines[i];
      return {
        id: `${id}-l${i + 1}`,
        productId: product.id,
        name: product.name,
        qty: l.qty,
        price: l.price,
        costPrice: product.costPrice,
        discount: l.discount ?? 0,
        taxId: lineTaxes[i].id,
        taxCategory: lineTaxes[i].category,
        taxRate: lineTaxes[i].rate,
        net: lr.net,
        vat: lr.vat,
      };
    }),
    subTotal: totals.subTotalAfterLineDiscounts,
    discountRate: input.discountRate,
    discountAmount: totals.invoiceDiscountAmount,
    taxRate: lineTaxes[0]?.rate ?? 0,
    taxAmount: totals.vat,
    grandTotal: totals.gross,
    paymentMethod: input.paymentMethod,
    tenders: tenders.map((t) => ({ paymentMethodId: t.paymentMethodId, amount: t.amount, reference: t.reference })),
    paidAmount,
    refundedAmount: 0,
    tenderedAmount: input.paymentMethod === 'cash' ? input.tenderedAmount : undefined,
    note: input.note,
  };
  mutate(() => db.invoices.push(invoice));

  // Apply stock/value changes per product (not per line): two cart rows of the same item must
  // share one "does this empty the stock" decision, matching prepareSale's COGS aggregation.
  const qtyByProduct = new Map<string, number>();
  for (const line of invoice.lines) {
    const product = productById(line.productId);
    if (product.type === 'product') qtyByProduct.set(product.id, (qtyByProduct.get(product.id) ?? 0) + line.qty);
  }
  for (const [productId, qty] of qtyByProduct) {
    const product = productById(productId);
    const valueOut = round2(qty) >= round2(product.stockQty) ? product.stockValue : round2(qty * product.costPrice);
    applyStockChange(product, -qty, -valueOut, 'sale', invoice, date);
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
  if (invoice.customerId) emit('parties:changed');
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
  mutate(() => {
    db.refunds.push(refund);
    invoice.refundedAmount = round2(invoice.refundedAmount + grandTotal);
    if (isFinal) invoice.status = 'REFUNDED';
    invoice.paymentStatus = paymentStatusFor(invoice.grandTotal - invoice.refundedAmount, invoice.paidAmount);
  });

  for (const line of lines) {
    const invLine = invoice.lines.find((l) => l.id === line.invoiceLineId)!;
    const product = productById(invLine.productId);
    if (product.type === 'product') {
      // A1/A2: returned units come back at the original line cost, and re-average automatically
      // because the value moves the same way it hit the GL (value += posted amount).
      applyStockChange(product, line.qty, round2(line.qty * invLine.costPrice), 'refund', refund, date);
    }
  }

  postJournal({
    date,
    description: `مرتجع مبيعات ${refund.number} على الفاتورة ${invoice.number}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'refund', id: refund.id, number: refund.number },
    lines: [
      { role: 'salesReturns', debit: subTotal },
      { role: 'vatOutput', debit: taxAmount },
      { role: 'receivable', credit: settledToReceivable, partyKind: 'customer', partyId: invoice.customerId },
      { accountId: settlementAccountFor(invoice.paymentMethod).id, credit: cashBack },
      { role: 'inventory', debit: cost },
      { role: 'cogs', credit: cost },
    ],
    createdBy: userId,
  });

  logActivity('refund', `مرتجع ${refund.number} على الفاتورة ${invoice.number} بقيمة ${grandTotal.toFixed(2)}`, userId, date, `/invoices/${invoice.id}`);
  if (invoice.customerId) emit('parties:changed');
  return refund;
}
