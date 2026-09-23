import type { PaymentStatus } from '../types';

export function round2(n: number): number {
  return Math.round((n + Number.EPSILON) * 100) / 100;
}

export interface SaleTotals {
  /** Σ qty × price − line discounts. */
  subTotal: number;
  /** Money value of the overall discount %. */
  discountAmount: number;
  /** subTotal − discountAmount: the VAT base and the Sales-account credit. */
  taxable: number;
  taxAmount: number;
  grandTotal: number;
}

/**
 * Invoice math (prices are VAT-exclusive, one overall discount %, one VAT rate).
 * Shared by the POS cart and the mock backend so the preview always matches what gets posted.
 */
export function computeSaleTotals(
  lines: { qty: number; price: number; discount?: number }[],
  discountRate: number,
  taxRate: number,
): SaleTotals {
  const subTotal = round2(lines.reduce((acc, l) => acc + l.qty * l.price - (l.discount ?? 0), 0));
  const discountAmount = round2((subTotal * discountRate) / 100);
  const taxable = round2(subTotal - discountAmount);
  const taxAmount = round2((taxable * taxRate) / 100);
  return { subTotal, discountAmount, taxable, taxAmount, grandTotal: round2(taxable + taxAmount) };
}

export function paymentStatusFor(total: number, paid: number): PaymentStatus {
  if (paid >= total - 0.005) return 'PAID';
  if (paid > 0) return 'PARTIALLY_PAID';
  return 'UNPAID';
}

/** What the customer still owes on an invoice (never negative). */
export function invoiceOutstanding(inv: { grandTotal: number; refundedAmount: number; paidAmount: number }): number {
  return Math.max(0, round2(inv.grandTotal - inv.refundedAmount - inv.paidAmount));
}

/** Cash change to hand back, given what was tendered. */
export function changeDue(total: number, tendered: number): number {
  return Math.max(0, round2(tendered - total));
}
