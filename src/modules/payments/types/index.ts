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
  /**
   * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md "Realized FX"): when the target
   * document carries a foreign currency, the FC amount this allocation covers (at the document's
   * OWN rate — "partial allocations use the invoice's rate for AR") plus the realized FX gain/loss
   * this allocation produced (positive = gain), for the allocation-grid display. Undefined for
   * base-currency documents.
   */
  amountFc?: number;
  fxGainLoss?: number;
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
  /**
   * v2 phase 9: branch this payment/receipt was recorded from (real once branches are on — see
   * `DEFAULT_BRANCH_ID` in `src/mocks/backend/core.ts`).
   */
  branchId?: string;
  /**
   * v2 phase 9 currency (docs/v2/10 §2): set when the party's currency differs from the base
   * currency — the payment was tendered/received in the party's FC. `amount` above always stays
   * the BASE-currency amount actually posted to the settlement account (Dr/Cr method account uses
   * `amount`, not `amountFc`) — this mirrors every other document in the app.
   */
  currency?: string;
  amountFc?: number;
  rate?: number;
  /** Σ of `allocations[].fxGainLoss` — the total realized FX gain(+)/loss(−) this payment produced. */
  fxGainLoss?: number;
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
  branchId?: string;
  /** v2 phase 9: FC tender — when set, `amount` must equal round2(amountFc × rate) (the base-currency amount actually posted). */
  currency?: string;
  amountFc?: number;
  rate?: number;
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
  /**
   * v2 phase 9 (docs/v2/10 §2 "Realized FX"): set when the document is in a foreign currency —
   * `fcOutstanding` (in that currency) and the document's OWN rate (base per unit), used so an
   * allocation against it converts at the INVOICE's rate, never the payment's.
   */
  currency?: string;
  fcOutstanding?: number;
  rate?: number;
}

/** Allocation status shown on lists (docs/v2/09-purchases-payments-expenses.md §3 "Printing and lists"). */
export type AllocationStatus = 'full' | 'partial' | 'unallocated';

export function allocationStatusFor(amount: number, allocated: number): AllocationStatus {
  if (allocated <= 0.005) return 'unallocated';
  if (allocated >= amount - 0.005) return 'full';
  return 'partial';
}
