import { ApiError, clone, db, delay, inDateRange, includesText, session } from '@/mocks';
import {
  deleteExpenseCategory as deleteExpenseCategoryBackend,
  deleteRecurringExpense as deleteRecurringExpenseBackend,
  dueRecurringExpenses,
  postRecurringExpense,
  recordExpense,
  saveExpenseCategory as saveExpenseCategoryBackend,
  saveRecurringExpense as saveRecurringExpenseBackend,
} from '@/mocks/backend/expenses';
import type {
  Expense,
  ExpenseCategory,
  ExpenseCategoryInput,
  ExpenseFilter,
  ExpenseInput,
  RecurringExpense,
  RecurringExpenseInput,
} from '../types';

export type ExpenseRow = Expense & { categoryName: string };

export async function getExpenseCategories(): Promise<ExpenseCategory[]> {
  await delay();
  return clone(db.expenseCategories);
}

export async function saveExpenseCategory(input: ExpenseCategoryInput, id?: string): Promise<ExpenseCategory> {
  await delay();
  return clone(saveExpenseCategoryBackend(input, id));
}

export async function deleteExpenseCategory(id: string): Promise<void> {
  await delay();
  deleteExpenseCategoryBackend(id);
}

function toRow(e: Expense): ExpenseRow {
  return { ...clone(e), categoryName: db.expenseCategories.find((c) => c.id === e.categoryId)?.name ?? '—' };
}

export async function getExpenses(filter: ExpenseFilter = {}): Promise<ExpenseRow[]> {
  await delay();
  return db.expenses
    .filter((e) => (!filter.categoryId || e.categoryId === filter.categoryId) && inDateRange(e.date, filter.from, filter.to))
    .map(toRow)
    .filter((r) => includesText([r.number, r.categoryName, r.description, r.supplierInvoiceNo], filter.search))
    .sort((a, b) => b.date.localeCompare(a.date));
}

export async function getExpense(id: string): Promise<ExpenseRow> {
  await delay();
  const e = db.expenses.find((x) => x.id === id);
  if (!e) throw new ApiError('المصروف غير موجود', 'NOT_FOUND');
  return toRow(e);
}

export async function createExpense(input: ExpenseInput): Promise<Expense> {
  await delay(300);
  return clone(recordExpense(input, session.userId));
}

export async function getRecurringExpenses(): Promise<RecurringExpense[]> {
  await delay();
  return clone(db.recurringExpenses);
}

export async function saveRecurringExpense(input: RecurringExpenseInput, id?: string): Promise<RecurringExpense> {
  await delay();
  return clone(saveRecurringExpenseBackend(input, id));
}

export async function deleteRecurringExpense(id: string): Promise<void> {
  await delay();
  deleteRecurringExpenseBackend(id);
}

/** "إيجار أكتوبر مستحق — سجّله" due-list. */
export async function getDueRecurringExpenses(): Promise<RecurringExpense[]> {
  await delay();
  return clone(dueRecurringExpenses());
}

export async function postDueRecurringExpense(id: string): Promise<Expense> {
  await delay(300);
  return clone(postRecurringExpense(id, session.userId));
}
