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
}

export interface PurchaseReturnInput {
  purchaseOrderId: string;
  reason?: string;
  lines: { productId: string; qty: number }[];
}
