import type { PaymentStatus } from '@/modules/invoices/types';

export type PurchaseStatus = 'DRAFT' | 'CONFIRMED' | 'CANCELED';

export interface PurchaseLine {
  productId: string;
  qty: number;
  costPrice: number;
}

export interface PurchaseOrder {
  id: string;
  number: string;
  supplierId: string;
  date: string;
  status: PurchaseStatus;
  lines: PurchaseLine[];
  subTotal: number;
  taxRate: number;
  taxAmount: number;
  grandTotal: number;
  paymentStatus: PaymentStatus;
  /** Extensions mirroring Invoice, needed to compute the supplier balance. */
  paidAmount: number;
  returnedAmount: number;
  note?: string;
}

export interface PurchaseOrderInput {
  supplierId: string;
  date: string;
  lines: PurchaseLine[];
  note?: string;
  /** true = post immediately (stock + journal); false = save as draft. */
  confirm: boolean;
}

export interface PurchaseFilter {
  search?: string;
  status?: PurchaseStatus;
  paymentStatus?: PaymentStatus;
  supplierId?: string;
}

/** v2 (E1): how the supplier gives the money back — cash/bank hit their own account; credit stays on AP. */
export type RefundMethod = 'cash' | 'bank_transfer' | 'credit';

export interface PurchaseReturn {
  id: string;
  number: string;
  purchaseOrderId: string;
  supplierId: string;
  date: string;
  reason?: string;
  lines: PurchaseLine[];
  subTotal: number;
  taxAmount: number;
  grandTotal: number;
  settledToPayable: number;
  cashBack: number;
  /** v2 (E1): the method `cashBack` was actually refunded through (never hard-coded to cash any more). */
  refundMethod: RefundMethod;
}

export interface PurchaseReturnInput {
  purchaseOrderId: string;
  reason?: string;
  /** v2 (E1): defaults to 'credit' (stays on the supplier's account) when omitted. */
  refundMethod?: RefundMethod;
  lines: { productId: string; qty: number }[];
}
