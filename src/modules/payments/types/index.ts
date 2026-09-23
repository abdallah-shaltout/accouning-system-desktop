export type PaymentType = 'RECEIVED' | 'PAID';
export type PaymentMethod = 'cash' | 'card' | 'bank_transfer';

export interface Payment {
  id: string;
  number: string;
  date: string;
  type: PaymentType;
  targetType: 'customer' | 'supplier';
  targetId: string;
  /** The invoice / purchase-order id this payment settles (one payment → one target). */
  targetRef: string;
  /** Extension: display number of targetRef (INV-…, PO-…). */
  targetRefNumber?: string;
  amount: number;
  method: PaymentMethod;
  note?: string;
}

export interface PaymentInput {
  date: string;
  type: PaymentType;
  targetType: 'customer' | 'supplier';
  targetId: string;
  targetRef: string;
  amount: number;
  method: PaymentMethod;
  note?: string;
}

export interface PaymentFilter {
  type?: PaymentType;
  method?: PaymentMethod;
  from?: string;
  to?: string;
  targetId?: string;
  search?: string;
}

/** An invoice or PO that still has an amount outstanding, offered in the payment form. */
export interface OpenDocument {
  id: string;
  number: string;
  date: string;
  total: number;
  outstanding: number;
}
