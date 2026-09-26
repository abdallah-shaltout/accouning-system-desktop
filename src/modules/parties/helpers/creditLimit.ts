/**
 * Credit-limit enforcement (docs/v2/02-accounting-review.md D3, owned by Phase 4). A credit sale
 * that would push the customer's balance over their `creditLimit` is blocked unless the acting
 * user has the override permission (`roleCanOverrideCreditLimit`, modules/users/helpers/permissions.ts).
 *
 * This lives in `modules/parties` (not `modules/invoices`) so Phase 3/7's sale files don't need to
 * import parties logic directly; `modules/invoices/services/invoiceService.ts` calls this before
 * delegating to `recordSale`.
 */
import { ApiError } from '../services/partyService';
import type { Customer } from '../types';

export interface CreditLimitCheckInput {
  customer: Pick<Customer, 'name' | 'creditLimit'>;
  /** The customer's balance right now (before this sale), e.g. from `customerBalance(id)`. */
  currentBalance: number;
  /** The receivable this sale would add (grandTotal − paidAmount). */
  newReceivable: number;
  canOverride: boolean;
}

/** Throws an Arabic `ApiError` when the limit would be exceeded and the user can't override it. */
export function assertWithinCreditLimit(input: CreditLimitCheckInput): void {
  const { customer, currentBalance, newReceivable, canOverride } = input;
  if (newReceivable <= 0) return;
  const limit = customer.creditLimit ?? 0;
  if (limit <= 0) return; // no limit set = unlimited
  const projected = currentBalance + newReceivable;
  if (projected <= limit + 0.005) return;
  if (canOverride) return;
  throw new ApiError(
    `تجاوز الحد الائتماني للعميل "${customer.name}": الرصيد الحالي ${currentBalance.toFixed(2)} + هذه الفاتورة ${newReceivable.toFixed(2)} = ${projected.toFixed(2)}، والحد المسموح ${limit.toFixed(2)}`,
    'FORBIDDEN',
  );
}

/** Due date = sale date + the customer's payment-terms days (docs/v2/02 D3). */
export function computeDueDate(saleDateIso: string, paymentTermsDays: number | undefined): string | undefined {
  if (!paymentTermsDays || paymentTermsDays <= 0) return undefined;
  const d = new Date(saleDateIso);
  d.setDate(d.getDate() + paymentTermsDays);
  return d.toISOString();
}
