import type { Invoice, JournalPreviewLine, Refund, RefundInput, RefundMethod, SaleInput, Tender } from '@/modules/invoices/types';
import { computeInvoiceTotals, invoiceOutstanding, paymentStatusFor, round2 as round2Totals } from '@/modules/invoices/helpers/totals';
import type { PaymentMethod } from '@/modules/settings/types';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, round2, sum, uid } from '../utils';
import { accountFor, settlementAccountFor } from './accounts';
import { branchPrefix, defaultCostCenterFor } from './branches';
import {
  applyStockChange,
  logActivity,
  postJournal,
  productById,
  salesTaxRate,
  userById,
  DEFAULT_BRANCH_ID,
  type PostingLine,
} from './core';
import { consumeFefo } from './inventory';
import { recordShiftMovement } from './shifts';
import { convertLinesToBase, isBaseCurrency, requireRate, toBase } from './currency';

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
 * is the new, payment-method-aware resolver `sales.ts` uses for tenders. Phase 8's
 * `src/mocks/backend/settlements.ts` now closes the loop: its card-settlement voucher (Dr bank +
 * Dr cardFees / Cr cardClearing-or-walletClearing) clears this balance once the bank deposit arrives.
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
  /** v2 phase 9: FC rate used (1 when the sale is in the base currency). */
  rate: number;
}

/**
 * Base-unit quantity for a line: `qty × unitFactor` (Phase 6's `ProductUnit.factor` — stock is kept
 * in the smallest unit, README decision 8). `unitFactor` defaults to 1 (base unit / legacy callers).
 */
function baseQty(line: { qty: number; unitFactor?: number }): number {
  return round2(line.qty * (line.unitFactor ?? 1));
}

/** Validate a sale and build its posting without saving anything (used by preview + record). */
function prepareSale(input: SaleInput, userId: string): PreparedSale {
  if (!input.lines.length) throw new ApiError('السلة فارغة');
  const user = userById(userId);
  // v2 phase 7 (docs/v2/06-sales-and-pos.md §1 "manager PIN approval overlay"): a discount above the
  // cashier's `maxDiscount` is allowed once a manager has approved it (`managerApprovedBy` set by
  // the caller after `verifyManagerPin` — modules/products/components/ApprovalPinDialog.vue's pattern).
  if (user && input.discountRate > user.maxDiscount && !input.managerApprovedBy) {
    throw new ApiError(`الخصم يتجاوز الحد المسموح لك (${user.maxDiscount}%)`, 'FORBIDDEN');
  }
  if (input.discountRate < 0 || input.discountRate > 100) throw new ApiError('نسبة الخصم غير صحيحة');

  // Aggregate quantities per product (in the BASE unit) so two cart rows of the same item — even
  // sold in different units (علبة + شريط) — are checked together.
  const qtyByProduct = new Map<string, number>();
  for (const line of input.lines) {
    if (!(line.qty > 0)) throw new ApiError('الكمية يجب أن تكون أكبر من صفر');
    if (line.price < 0) throw new ApiError('السعر لا يمكن أن يكون سالباً');
    if (line.isFreeText) {
      if (!line.revenueAccountId) throw new ApiError('السطر النصي الحر يحتاج حساب إيراد');
      continue;
    }
    const product = productById(line.productId);
    if (!product.active) throw new ApiError(`المنتج "${product.name}" غير نشط`);
    if (product.type === 'product') qtyByProduct.set(product.id, round2((qtyByProduct.get(product.id) ?? 0) + baseQty(line)));
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
  // Shift+F8 "invoice discount" can be a flat amount (`discountAmount`) instead of `discountRate`'s %.
  const invoiceDiscount =
    input.discountAmount && input.discountAmount > 0
      ? { amount: input.discountAmount }
      : input.discountRate > 0
        ? { pct: input.discountRate }
        : undefined;
  const totals = computeInvoiceTotals(
    input.lines.map((l, i) => ({ qty: l.qty, unitPrice: l.price, discount: l.discount ?? 0, discountIsPct: l.discountIsPct ?? false, tax: lineTaxes[i] })),
    invoiceDiscount,
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

  // v2 phase 9 currency (docs/v2/10 §2): everything above (lines, totals, tenders, receivable) is
  // in `input.currency` when set (FC invoice) — `rate` (base per 1 FC unit) converts each posting
  // amount to base for the GL, which only ever holds base-currency balances. COGS/inventory are
  // NEVER in FC (cost basis is always base currency, regardless of the sale's currency), so they
  // post at `costTotal` unconverted either way. Branch/cost center: real once Phase 9 branches
  // exist; falls back to the single default branch/its cost center otherwise (unchanged behavior).
  const branchId = input.branchId ?? DEFAULT_BRANCH_ID;
  const costCenterId = defaultCostCenterFor(branchId, input.costCenterId);
  const currency = input.currency && !isBaseCurrency(input.currency) ? input.currency : undefined;
  const rate = currency ? (input.exchangeRate ?? requireRate(currency)) : 1;
  const dim = { branchId, costCenterId };
  const fc = (amount: number) => (currency ? { currency, amountFc: amount, rate } : {});
  const toBaseAmt = (amount: number) => (currency ? toBase(amount, rate) : amount);

  // One posting line per tender, each to its own method's account (C3 fix: card/wallet tenders go
  // to their clearing account, not straight to bank — see `tenderAccountId` above). Tenders are
  // always collected in the base currency (POS "accept foreign cash" is a separate tender-level
  // conversion handled by the caller before this point — see `modules/invoices/services`).
  const tenderLines: PostingLine[] = resolvedTenders
    .filter((t) => t.amount > 0)
    .map((t) => ({ accountId: tenderAccountId(t.method), debit: t.amount, description: t.reference ? `${t.method.name} — ${t.reference}` : t.method.name, ...dim }));

  // vatByCategory (docs/v2/06 §3 step 5) posts sales net per category to the same `sales` role for
  // now — a separate revenue account per category (e.g. 4120 zero-rated sales) is a per-product/
  // per-category account resolution that belongs to Phase 6's product tax fields; the VAT report
  // (below) still gets its category breakdown from the invoice lines regardless of which revenue
  // account the net posts to.
  //
  // Free-text lines (desk form, docs/v2/06 §2 "Free-text lines... need a revenue account") post
  // their net to the line's own chosen account instead of the flat `sales` role.
  let stockedNet = 0;
  const freeTextRevenue = new Map<string, number>();
  input.lines.forEach((l, i) => {
    const lr = totals.lines[i];
    if (l.isFreeText && l.revenueAccountId) freeTextRevenue.set(l.revenueAccountId, round2((freeTextRevenue.get(l.revenueAccountId) ?? 0) + lr.net));
    else stockedNet = round2(stockedNet + lr.net);
  });

  // v2 phase 9 rounding rule (docs/v2/10 §2 "any cent difference between Σ lines and the converted
  // total goes to the LARGEST line, never a separate account"): converting `stockedNet`,
  // `freeTextRevenue`'s entries and `vat` to base INDEPENDENTLY (each round2(fc × rate)) can leave a
  // stray halala vs. the receivable's own round2(fcTotal × rate) — `convertLinesToBase` distributes
  // that gap onto the largest of these credit-side amounts instead, so the entry always balances
  // exactly without a dedicated rounding account.
  const freeTextEntries = [...freeTextRevenue.entries()];
  const creditFcAmounts = [stockedNet, ...freeTextEntries.map(([, v]) => v), totals.vat];
  const creditBaseAmounts = currency ? convertLinesToBase(creditFcAmounts, rate) : creditFcAmounts;
  const [stockedNetBase, ...restBase] = creditBaseAmounts;
  const vatBase = restBase.pop()!;
  const freeTextBase = freeTextEntries.map(([accountId], i): PostingLine => ({ accountId, credit: restBase[i], ...dim }));

  const posting: PostingLine[] = [
    ...tenderLines,
    { role: 'receivable', debit: toBaseAmt(receivable), partyKind: 'customer', partyId: input.customerId, ...dim, ...fc(receivable) },
    { role: 'sales', credit: stockedNetBase, ...dim },
    ...freeTextBase,
    { role: 'vatOutput', credit: vatBase, ...dim },
    { role: 'cogs', debit: costTotal, ...dim },
    { role: 'inventory', credit: costTotal, ...dim },
  ];
  return { totals, tenders: resolvedTenders, paidAmount, costTotal, posting, rate };
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
  const { totals, tenders, paidAmount, posting, rate } = prepareSale(input, userId);
  const lineTaxes = input.lines.map((l) => taxForLine(l.taxId));
  const branchId = input.branchId ?? DEFAULT_BRANCH_ID;
  const currency = input.currency && !isBaseCurrency(input.currency) ? input.currency : undefined;

  // `taxRate`/`taxAmount`/`grandTotal` are kept as a single-rate snapshot for the legacy print/
  // refund/report code paths (InvoiceA4, InvoiceThermal, recordRefund — outside this phase's
  // surface): the store's dominant (first-line) rate stands in for what used to be "the" VAT rate.
  // `lines[].taxRate`/`taxCategory`/`net`/`vat` below carry the real per-line v2 detail the VAT
  // report reads.
  const id = uid('inv');
  const invoice: Invoice = {
    id,
    number: `${branchPrefix(branchId)}${nextNumber('invoice')}`,
    date,
    customerId: input.customerId,
    cashierId: userId,
    status: 'COMPLETED',
    paymentStatus: paymentStatusFor(totals.gross, paidAmount),
    lines: input.lines.map((l, i) => {
      const lr = totals.lines[i];
      if (l.isFreeText) {
        return {
          id: `${id}-l${i + 1}`,
          productId: `freetext-${i}`,
          name: l.name ?? 'سطر حر',
          qty: l.qty,
          price: l.price,
          costPrice: 0,
          discount: l.discount ?? 0,
          taxId: lineTaxes[i].id,
          taxCategory: lineTaxes[i].category,
          taxRate: lineTaxes[i].rate,
          net: lr.net,
          vat: lr.vat,
          isFreeText: true,
          revenueAccountId: l.revenueAccountId,
        };
      }
      const product = productById(l.productId);
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
        unitId: l.unitId,
        unitFactor: l.unitFactor,
        listPrice: l.listPrice,
        priceOverrideReason: l.priceOverrideReason,
        batchId: l.batchId,
        batchNo: l.batchNo,
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
    source: input.source ?? 'POS',
    shiftId: input.shiftId,
    invoiceType: input.invoiceType,
    poReference: input.poReference,
    terms: input.terms,
    attachmentIds: input.attachmentIds,
    branchId,
    costCenterId: defaultCostCenterFor(branchId, input.costCenterId),
    currency,
    exchangeRate: currency ? rate : undefined,
  };
  mutate(() => db.invoices.push(invoice));

  // Apply stock/value changes per product (not per line): two cart rows of the same item must
  // share one "does this empty the stock" decision, matching prepareSale's COGS aggregation. Free-
  // text lines carry no stock. Quantities are converted to the BASE unit (unitFactor) before this
  // point (see `baseQty` in `prepareSale`) — batches are drawn in the same base unit.
  const qtyByProduct = new Map<string, number>();
  for (let i = 0; i < invoice.lines.length; i++) {
    const line = invoice.lines[i];
    if (line.isFreeText) continue;
    const product = productById(line.productId);
    if (product.type === 'product') qtyByProduct.set(product.id, round2((qtyByProduct.get(product.id) ?? 0) + baseQty({ qty: line.qty, unitFactor: line.unitFactor })));
  }
  for (const [productId, qty] of qtyByProduct) {
    const product = productById(productId);
    const valueOut = round2(qty) >= round2(product.stockQty) ? product.stockValue : round2(qty * product.costPrice);
    applyStockChange(product, -qty, -valueOut, 'sale', invoice, date, branchId);
    // §3 FEFO (docs/v2/06 §1 "Batch: auto-picked first-expiry-first-out... can be changed"): draw
    // from the manually-picked batch first (its exact qty), then FEFO for the rest of this product's
    // total base-unit qty across every cart line.
    if (product.trackBatches) {
      const manualDraws = invoice.lines.filter((l) => !l.isFreeText && l.productId === productId && l.batchId);
      let remaining = qty;
      for (const l of manualDraws) {
        const batch = db.productBatches.find((b) => b.id === l.batchId);
        if (!batch) continue;
        const take = Math.min(batch.qty, baseQty({ qty: l.qty, unitFactor: l.unitFactor }));
        if (take > 0) {
          mutate(() => (batch.qty = round2(batch.qty - take)));
          remaining = round2(remaining - take);
        }
      }
      if (remaining > 0.0001) consumeFefo(productId, remaining);
    }
  }

  postJournal({
    date,
    description: `فاتورة مبيعات ${invoice.number} (${METHOD_LABEL[invoice.paymentMethod]})${currency ? ` — ${currency}` : ''}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'invoice', id: invoice.id, number: invoice.number },
    lines: posting,
    createdBy: userId,
  });

  // Shift movement log (docs/v2/06 §5 "Every cash tender... is recorded as a shift movement") — only
  // the cash portion; card/bank tenders don't touch the drawer. No-op if no shift is open (desk sales).
  const cashMethodIds = new Set(db.paymentMethods.filter((m) => m.accountRole === 'cash').map((m) => m.id));
  const cashTendered = sum(tenders.filter((t) => cashMethodIds.has(t.paymentMethodId)), (t) => t.amount);
  if (input.shiftId && cashTendered > 0) {
    recordShiftMovement(db.shifts.find((s) => s.id === input.shiftId)?.terminalId ?? '', 'SALE_CASH', cashTendered, userId, {
      refId: invoice.id,
      refNumber: invoice.number,
      at: date,
    });
  }

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
  // v2 phase 7 (docs/v2/06-sales-and-pos.md §4 "refund method"): customer credit needs a customer.
  if (input.refundMethod === 'customer_credit' && !invoice.customerId) {
    throw new ApiError('رصيد العميل يتطلب فاتورة مرتبطة بعميل');
  }

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

  const refundMethod: RefundMethod = input.refundMethod ?? (invoice.paymentMethod === 'credit' ? 'cash' : (invoice.paymentMethod as RefundMethod));
  const creditedToAccount = refundMethod === 'customer_credit' ? cashBack : 0;
  const paidOut = refundMethod === 'customer_credit' ? 0 : cashBack;

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
    refundMethod,
    creditedToAccount: creditedToAccount || undefined,
  };
  mutate(() => {
    db.refunds.push(refund);
    invoice.refundedAmount = round2(invoice.refundedAmount + grandTotal);
    if (isFinal) invoice.status = 'REFUNDED';
    invoice.paymentStatus = paymentStatusFor(invoice.grandTotal - invoice.refundedAmount, invoice.paidAmount);
  });

  // v2 phase 7 (§4 "Restock toggle per line: default on. Off = the item is damaged, so it's written
  // off (5120) instead of going back to stock"). A1/A2's re-averaging applies on restock (the value
  // moves the same way it hit the GL); a write-off removes the line's cost from inventory instead.
  let restockValue = 0;
  let writeOffValue = 0;
  for (const line of lines) {
    const invLine = invoice.lines.find((l) => l.id === line.invoiceLineId)!;
    if (invLine.isFreeText) continue;
    const product = productById(invLine.productId);
    if (product.type !== 'product') continue;
    const value = round2(line.qty * invLine.costPrice);
    if (line.restock === false) {
      writeOffValue = round2(writeOffValue + value);
      // Written-off returns never re-enter stock — no applyStockChange call, only the write-off
      // journal line below moves value out of `cogs`'s reversal into `inventoryWriteOff`.
    } else {
      restockValue = round2(restockValue + value);
      applyStockChange(product, line.qty, value, 'refund', refund, date);
    }
  }

  // Refund-method settlement (docs/v2/06 §4 "Default: against the invoice's outstanding amount
  // first. Then the rest goes to cash, the original card, a bank transfer, or customer credit"):
  // customer credit posts the leftover as an extra credit to the customer's receivable sub-ledger
  // (an unallocated balance, same shape Phase 4's payments/allocation reads — see
  // `src/mocks/backend/balances.ts`'s `customerBalance`), instead of paying out a settlement account.
  const settlementLines: PostingLine[] =
    creditedToAccount > 0
      ? [{ role: 'receivable', credit: round2(settledToReceivable + creditedToAccount), partyKind: 'customer', partyId: invoice.customerId }]
      : [
          { role: 'receivable', credit: settledToReceivable, partyKind: 'customer', partyId: invoice.customerId },
          { accountId: settlementAccountFor(refundMethod === 'cash' ? 'cash' : refundMethod === 'card' ? 'card' : 'bank_transfer').id, credit: paidOut },
        ];

  postJournal({
    date,
    description: `مرتجع مبيعات ${refund.number} على الفاتورة ${invoice.number}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'refund', id: refund.id, number: refund.number },
    lines: [
      { role: 'salesReturns', debit: subTotal },
      { role: 'vatOutput', debit: taxAmount },
      ...settlementLines,
      { role: 'inventory', debit: restockValue },
      { role: 'inventoryWriteOff', debit: writeOffValue },
      { role: 'cogs', credit: round2(restockValue + writeOffValue) },
    ],
    createdBy: userId,
  });

  // Cash refunds reduce the shift drawer (docs/v2/06 §5 "Every cash tender, cash refund... is
  // recorded as a shift movement") when returned from the till with an open shift.
  if (refundMethod === 'cash' && paidOut > 0) {
    const shift = db.shifts.find((s) => s.status === 'OPEN' && db.invoices.find((i) => i.id === invoice.id)?.shiftId === s.id) ?? db.shifts.find((s) => s.status === 'OPEN');
    if (shift) recordShiftMovement(shift.terminalId, 'REFUND_CASH', paidOut, userId, { refId: refund.id, refNumber: refund.number, at: date });
  }

  logActivity('refund', `مرتجع ${refund.number} على الفاتورة ${invoice.number} بقيمة ${grandTotal.toFixed(2)}`, userId, date, `/invoices/${invoice.id}`);
  if (invoice.customerId) emit('parties:changed');
  return refund;
}
