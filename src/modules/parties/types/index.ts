export interface Customer {
  id: string;
  name: string;
  phone?: string;
  address?: string;
  type: 'individual' | 'company';
  vatNumber?: string;
  /** Computed on read: positive = the customer owes us. */
  balance: number;
  active: boolean;
}

export type CustomerInput = Omit<Customer, 'id' | 'balance'>;

export interface Supplier {
  id: string;
  name: string;
  phone?: string;
  contactPerson?: string;
  address?: string;
  vatNumber?: string;
  /** Computed on read: positive = we owe the supplier. */
  balance: number;
  active: boolean;
}

export type SupplierInput = Omit<Supplier, 'id' | 'balance'>;

/** One row of a customer/supplier statement (كشف حساب). */
export interface PartyStatementRow {
  id: string;
  date: string;
  kind: 'invoice' | 'refund' | 'payment' | 'purchaseOrder' | 'purchaseReturn';
  refId: string;
  number: string;
  description: string;
  /** Increases what the party owes us (customer) / what we owe them (supplier). */
  debit: number;
  credit: number;
  balance: number;
}
