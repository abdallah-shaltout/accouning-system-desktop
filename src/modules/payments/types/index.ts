export type PaymentType = 'RECEIVED' | 'PAID';
export type PaymentMethod = 'cash' | 'card' | 'bank_transfer';

/**
 * A payment's link to one document it settles — a sub-ledger record only (docs/v2/02-accounting-
 * review.md C1): it never touches the GL. The payment itself posts once (Dr method account / Cr
 * receivable-or-payable); allocating/reallocating/un-allocating changes only these rows.
 */
export interface PaymentAllocation {
  id: string;
  /** 'invoice' | 'purchaseOrder' | 'opening' — the opening balance is a Phase 5 document kind. */
  targetKind: 'invoice' | 'purchaseOrder' | 'opening';
  targetId: string;
  targetNumber: string;
  amount: number;
  date: string;
}

export interface Payment {
  id: string;
  number: string;
  date: string;
  type: PaymentType;
  targetType: 'customer' | 'supplier';
  targetId: string;
  /**
   * Legacy/back-compat: the primary document this payment is *for*, when created from a document's
   * "collect/pay" button. Superseded by `allocations[]`, which is the source of truth for how much
   * of `amount` is applied to which document(s) — kept only for the payment list's "document" column.
   */
  targetRef?: string;
  targetRefNumber?: string;
  amount: number;
  method: PaymentMethod;
  note?: string;
  /** Sub-ledger allocation rows (C1). Σ allocations ≤ amount; the remainder is unallocated credit. */
  allocations: PaymentAllocation[];
}

export interface PaymentAllocationInput {
  targetKind: 'invoice' | 'purchaseOrder';
  targetId: string;
  amount: number;
}

export interface PaymentInput {
  date: string;
  type: PaymentType;
  targetType: 'customer' | 'supplier';
  targetId: string;
  amount: number;
  method: PaymentMethod;
  note?: string;
  /** Allocations to apply at creation time. May be empty/partial — "allocate later" is supported. */
  allocations?: PaymentAllocationInput[];
}

export interface PaymentFilter {
  type?: PaymentType;
  method?: PaymentMethod;
  from?: string;
  to?: string;
  targetId?: string;
  search?: string;
  /** Only payments that still have unallocated money. */
  unallocatedOnly?: boolean;
}

/** An invoice or PO that still has an amount outstanding, offered in the payment form's allocation grid. */
export interface OpenDocument {
  id: string;
  kind: 'invoice' | 'purchaseOrder';
  number: string;
  date: string;
  dueDate?: string;
  total: number;
  outstanding: number;
}

/** Allocation status shown on lists (docs/v2/09-purchases-payments-expenses.md §3 "Printing and lists"). */
export type AllocationStatus = 'full' | 'partial' | 'unallocated';

export function allocationStatusFor(amount: number, allocated: number): AllocationStatus {
  if (allocated <= 0.005) return 'unallocated';
  if (allocated >= amount - 0.005) return 'full';
  return 'partial';
}
