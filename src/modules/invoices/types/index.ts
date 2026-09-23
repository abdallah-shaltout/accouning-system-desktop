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
  /** Extension: sum of refunds issued against this invoice. */
  refundedAmount: number;
  /** Extension: cash handed over by the customer (cash sales), for the receipt. */
  tenderedAmount?: number;
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
  lines: { productId: string; qty: number; price: number; discount?: number }[];
  discountRate: number;
  paymentMethod: SalePaymentMethod;
  /** Cash tendered / amount paid now. Ignored for card & bank transfer (paid in full). */
  paidAmount: number;
  tenderedAmount?: number;
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
