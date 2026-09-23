/**
 * Expenses module (docs/v2/09-purchases-payments-expenses.md §4) — new in v2 phase 8. Posting:
 * Dr expense account [cost center] + vatInput / Cr method account or payable[supplier].
 */

/** Settings → Expenses: name/account/default tax/default cost center/icon (CoA-template presets). */
export interface ExpenseCategory {
  id: string;
  name: string;
  icon?: string;
  accountId: string;
  defaultTaxId?: string;
  /** Inert field until phase 9 (cost centers) — captured now so nothing needs to migrate later. */
  defaultCostCenterId?: string;
  active: boolean;
  /** System-seeded presets (rent, electricity, internet, packaging, maintenance…) can't be deleted, only deactivated. */
  canDelete: boolean;
}

export type ExpenseCategoryInput = Omit<ExpenseCategory, 'id' | 'canDelete'>;

/** "Paid from": a settings PaymentMethod (cash drawer/bank/card…) or on credit to a supplier. */
export type ExpensePaidFrom = { kind: 'method'; paymentMethodId: string } | { kind: 'credit'; supplierId: string };

export interface Expense {
  id: string;
  number: string;
  date: string;
  categoryId: string;
  amount: number;
  /** "فاتورة ضريبية؟" — when true, `amount` is VAT-inclusive and splits into net/VAT via `taxId`. */
  isTaxInvoice: boolean;
  taxId?: string;
  netAmount: number;
  taxAmount: number;
  supplierVatNumber?: string;
  supplierInvoiceNo?: string;
  costCenterId?: string;
  paidFrom: ExpensePaidFrom;
  description?: string;
  attachmentIds?: string[];
  /** "repeat monthly" — true when this expense was created from (or seeds) a recurring template. */
  repeatMonthly: boolean;
  /** Set when this expense was posted from a `RecurringExpense`'s due date. */
  recurringTemplateId?: string;
  createdBy: string;
  branchId?: string;
}

export interface ExpenseInput {
  date: string;
  categoryId: string;
  amount: number;
  isTaxInvoice: boolean;
  taxId?: string;
  supplierVatNumber?: string;
  supplierInvoiceNo?: string;
  costCenterId?: string;
  paidFrom: ExpensePaidFrom;
  description?: string;
  attachmentIds?: string[];
  repeatMonthly?: boolean;
  recurringTemplateId?: string;
}

export interface ExpenseFilter {
  categoryId?: string;
  from?: string;
  to?: string;
  search?: string;
}

/** A recurring-expense template + next due date (docs/v2/09 §4 "Recurring expenses"). */
export interface RecurringExpense {
  id: string;
  name: string;
  categoryId: string;
  amount: number;
  isTaxInvoice: boolean;
  taxId?: string;
  paidFrom: ExpensePaidFrom;
  description?: string;
  /** Day of month the expense falls due. */
  day: number;
  nextDate: string;
  autoPost: boolean;
  active: boolean;
}

export interface RecurringExpenseInput {
  name: string;
  categoryId: string;
  amount: number;
  isTaxInvoice: boolean;
  taxId?: string;
  paidFrom: ExpensePaidFrom;
  description?: string;
  day: number;
  nextDate: string;
  autoPost: boolean;
  active: boolean;
}
