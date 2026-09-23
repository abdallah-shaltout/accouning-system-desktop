export type InvoiceStatus = 'DRAFT' | 'COMPLETED' | 'REFUNDED';
export type PaymentStatus = 'UNPAID' | 'PARTIALLY_PAID' | 'PAID';
export type SalePaymentMethod = 'cash' | 'card' | 'bank_transfer' | 'credit';

export interface InvoiceLine {
  id: string;
  productId: string;
  /** Extension: name snapshot so reprints survive product renames. */
  name: string;
  qty: number;
  price: number;
  costPrice: number;
  discount: number;
  /**
   * v2 (docs/v2/06-sales-and-pos.md §3): the tax charged on this line, snapshotted at sale time so
   * historical invoices never change if the tax's rate/category is edited later. Undefined = no tax
   * assigned to the line (falls back to out-of-scope, 0 VAT) — the product catalog doesn't carry a
   * per-product tax yet (that's Phase 6's "product form v2 tabs").
   */
  taxId?: string;
  taxCategory?: import('@/modules/settings/types').TaxCategory;
  taxRate?: number;
  /** v2: this line's VAT, computed by `computeInvoiceTotals` (net + vat = the line's gross). */
  net?: number;
  vat?: number;
}

/**
 * v2 split payment (docs/v2/06-sales-and-pos.md §1 "Pay (F12)… Split payment"): one line per
 * payment method used to settle the sale. `paymentMethod`/`paidAmount` on `Invoice`/`SaleInput`
 * stay as the legacy single-tender snapshot (still used by receipts/reports outside this phase's
 * surface) — `tenders`, when present, is the source of truth for posting (src/mocks/backend/sales.ts).
 */
export interface Tender {
  paymentMethodId: string;
  amount: number;
  /** Card last 4 / approval code / transfer reference. */
  reference?: string;
}

export interface Invoice {
  id: string;
  number: string;
  date: string;
  customerId?: string;
  cashierId: string;
  status: InvoiceStatus;
  paymentStatus: PaymentStatus;
  lines: InvoiceLine[];
  subTotal: number;
  discountRate: number;
  /** Extension: the money value of `discountRate` (plus line discounts). */
  discountAmount: number;
  /** Extension: VAT rate snapshot used for this invoice. */
  taxRate: number;
  taxAmount: number;
  grandTotal: number;
  paymentMethod: SalePaymentMethod;
  paidAmount: number;
  /**
   * v2 split payment (docs/v2/06-sales-and-pos.md §1): the actual tenders posted, one per payment
   * method. Absent on invoices recorded through the legacy single-method path (`paymentMethod` +
   * `paidAmount` only) — `sales.ts` falls back to a single implied tender in that case.
   */
  tenders?: Tender[];
  /** Extension: sum of refunds issued against this invoice. */
  refundedAmount: number;
  /** Extension: cash handed over by the customer (cash sales), for the receipt. */
  tenderedAmount?: number;
  /**
   * v2 (docs/v2/02-accounting-review.md D3): for a credit/partial sale, `date` + the customer's
   * `paymentTermsDays` at the time of sale — drives aging and "overdue". Set by
   * `modules/invoices/services/invoiceService.ts::createSale`, not the posting rule itself.
   */
  dueDate?: string;
  note?: string;
}

export interface InvoiceFilter {
  search?: string;
  status?: InvoiceStatus;
  paymentStatus?: PaymentStatus;
  customerId?: string;
  from?: string;
  to?: string;
}

export interface SaleInput {
  customerId?: string;
  /**
   * `taxId`: v2 per-line tax (docs/v2/06-sales-and-pos.md §3) — omitted falls back to the store's
   * default sales tax (`db.settings.defaultTaxId`), matching the pre-Phase-3 single-rate behavior.
   * `discountIsPct`: line discount is a % of the line when true, a flat amount otherwise (defaults
   * to flat amount, matching the pre-existing `discount` field's meaning).
   */
  lines: { productId: string; qty: number; price: number; discount?: number; discountIsPct?: boolean; taxId?: string }[];
  discountRate: number;
  paymentMethod: SalePaymentMethod;
  /** Cash tendered / amount paid now. Ignored for card & bank transfer (paid in full). */
  paidAmount: number;
  tenderedAmount?: number;
  /**
   * v2 split payment (docs/v2/06-sales-and-pos.md §1 "Split payment"): part cash + part card in one
   * sale. When present, `sales.ts` posts one line per tender (each to its method's account) instead
   * of the single `paymentMethod`/`paidAmount` pair; the sum of `tenders[].amount` must equal
   * `paidAmount`. Optional so every existing caller (POS cart, CheckoutModal) keeps working unchanged.
   */
  tenders?: import('.').Tender[];
  note?: string;
}

export interface Refund {
  id: string;
  number: string;
  invoiceId: string;
  date: string;
  reason?: string;
  lines: { invoiceLineId: string; qty: number }[];
  /** Extension: net (before VAT) and VAT portions of the refund. */
  subTotal: number;
  taxAmount: number;
  grandTotal: number;
  /** Extension: how much reduced the customer's receivable vs. was paid back. */
  settledToReceivable: number;
  cashBack: number;
}

export interface RefundInput {
  invoiceId: string;
  reason?: string;
  lines: { invoiceLineId: string; qty: number }[];
}

/** Preview of the double-entry posting a sale will produce. */
export interface JournalPreviewLine {
  accountCode: string;
  accountName: string;
  debit: number;
  credit: number;
}
