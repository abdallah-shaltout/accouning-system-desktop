import type { Expense, ExpenseCategory, ExpenseCategoryInput, ExpenseInput, RecurringExpense, RecurringExpenseInput } from '@/modules/expenses/types';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, localDateKey, round2, uid } from '../utils';
import { accountById } from './accounts';
import { logActivity, postJournal, type PostingLine } from './core';

/**
 * Expenses module (docs/v2/09-purchases-payments-expenses.md §4) — new in v2 phase 8. Posting:
 * Dr expense account [cost center] + vatInput / Cr method account or payable[supplier].
 */

// ---------------------------------------------------------------------------------------------
// Categories (Settings → Expenses)
// ---------------------------------------------------------------------------------------------

export function saveExpenseCategory(input: ExpenseCategoryInput, id?: string): ExpenseCategory {
  if (!input.name.trim()) throw new ApiError('الاسم مطلوب');
  accountById(input.accountId);
  if (id) {
    const found = db.expenseCategories.find((c) => c.id === id);
    if (!found) throw new ApiError('التصنيف غير موجود', 'NOT_FOUND');
    mutate(() => Object.assign(found, { ...input, name: input.name.trim() }));
    return found;
  }
  const category: ExpenseCategory = { id: uid('excat'), ...input, name: input.name.trim(), canDelete: true };
  mutate(() => db.expenseCategories.push(category));
  return category;
}

export function deleteExpenseCategory(id: string): void {
  const category = db.expenseCategories.find((c) => c.id === id);
  if (!category) throw new ApiError('التصنيف غير موجود', 'NOT_FOUND');
  if (!category.canDelete) throw new ApiError('لا يمكن حذف تصنيف أساسي — يمكن إلغاء تفعيله فقط');
  if (db.expenses.some((e) => e.categoryId === id)) throw new ApiError('لا يمكن حذف تصنيف له مصروفات مسجلة — قم بإلغاء تفعيله بدلاً من ذلك', 'CONFLICT');
  mutate(() => (db.expenseCategories = db.expenseCategories.filter((c) => c.id !== id)));
}

// ---------------------------------------------------------------------------------------------
// Expenses
// ---------------------------------------------------------------------------------------------

function splitTax(amount: number, isTaxInvoice: boolean, taxId: string | undefined): { net: number; vat: number } {
  if (!isTaxInvoice) return { net: round2(amount), vat: 0 };
  const tax = taxId ? db.taxes.find((t) => t.id === taxId && t.active) : undefined;
  const rate = tax?.rate ?? 0;
  const net = round2(amount / (1 + rate / 100));
  return { net, vat: round2(amount - net) };
}

function validateExpenseInput(input: ExpenseInput): ExpenseCategory {
  const category = db.expenseCategories.find((c) => c.id === input.categoryId);
  if (!category) throw new ApiError('اختر تصنيف المصروف');
  if (!(input.amount > 0)) throw new ApiError('المبلغ يجب أن يكون أكبر من صفر');
  const paidFrom = input.paidFrom;
  if (paidFrom.kind === 'method') {
    if (!db.paymentMethods.find((m) => m.id === paidFrom.paymentMethodId && m.active)) throw new ApiError('اختر طريقة الدفع');
  } else {
    if (!db.suppliers.find((s) => s.id === paidFrom.supplierId)) throw new ApiError('اختر المورد');
  }
  return category;
}

/** Posts one expense: Dr expense[cc] + vatInput / Cr method account or payable[supplier] (docs/v2 §4). */
export function recordExpense(input: ExpenseInput, userId: string): Expense {
  const category = validateExpenseInput(input);
  const { net, vat } = splitTax(input.amount, input.isTaxInvoice, input.taxId);

  const expense: Expense = {
    id: uid('exp'),
    number: nextNumber('expense'),
    date: input.date,
    categoryId: input.categoryId,
    amount: round2(input.amount),
    isTaxInvoice: input.isTaxInvoice,
    taxId: input.taxId,
    netAmount: net,
    taxAmount: vat,
    supplierVatNumber: input.supplierVatNumber,
    supplierInvoiceNo: input.supplierInvoiceNo,
    costCenterId: input.costCenterId,
    paidFrom: input.paidFrom,
    description: input.description,
    attachmentIds: input.attachmentIds,
    repeatMonthly: !!input.repeatMonthly,
    recurringTemplateId: input.recurringTemplateId,
    createdBy: userId,
  };
  mutate(() => db.expenses.push(expense));

  const lines: PostingLine[] = [
    { accountId: category.accountId, debit: net, costCenterId: input.costCenterId },
    ...(vat > 0 ? [{ role: 'vatInput' as const, debit: vat }] : []),
  ];
  const paidFrom = input.paidFrom;
  if (paidFrom.kind === 'method') {
    const method = db.paymentMethods.find((m) => m.id === paidFrom.paymentMethodId)!;
    lines.push({ role: method.accountRole, credit: expense.amount });
  } else {
    lines.push({ role: 'payable', credit: expense.amount, partyKind: 'supplier', partyId: paidFrom.supplierId });
  }

  postJournal({
    date: input.date,
    description: `مصروف ${expense.number} — ${category.name}${input.description ? `: ${input.description}` : ''}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'expense', id: expense.id, number: expense.number },
    lines,
    createdBy: userId,
    attachmentIds: input.attachmentIds,
  });

  logActivity('expense', `مصروف ${expense.number} — ${category.name} بقيمة ${expense.amount.toFixed(2)}`, userId, input.date, `/expenses/${expense.id}`);
  emit('ledger:changed');
  return expense;
}

export function getExpenseById(id: string): Expense {
  const expense = db.expenses.find((e) => e.id === id);
  if (!expense) throw new ApiError('المصروف غير موجود', 'NOT_FOUND');
  return expense;
}

// ---------------------------------------------------------------------------------------------
// Recurring expenses (docs/v2/09 §4 "Recurring expenses") — a due-list; full insight surfacing is
// Phase 10 (TODO below).
// ---------------------------------------------------------------------------------------------

function nextMonthDate(day: number, from: string): string {
  const base = new Date(from);
  const next = new Date(base.getFullYear(), base.getMonth() + 1, Math.min(day, 28));
  return localDateKey(next);
}

export function saveRecurringExpense(input: RecurringExpenseInput, id?: string): RecurringExpense {
  if (!input.name.trim()) throw new ApiError('الاسم مطلوب');
  if (!db.expenseCategories.find((c) => c.id === input.categoryId)) throw new ApiError('اختر تصنيف المصروف');
  if (!(input.day >= 1 && input.day <= 28)) throw new ApiError('يوم الاستحقاق بين 1 و 28');
  if (id) {
    const found = db.recurringExpenses.find((r) => r.id === id);
    if (!found) throw new ApiError('القالب غير موجود', 'NOT_FOUND');
    mutate(() => Object.assign(found, { ...input, name: input.name.trim() }));
    return found;
  }
  const template: RecurringExpense = { id: uid('recexp'), ...input, name: input.name.trim() };
  mutate(() => db.recurringExpenses.push(template));
  return template;
}

export function deleteRecurringExpense(id: string): void {
  const found = db.recurringExpenses.find((r) => r.id === id);
  if (!found) throw new ApiError('القالب غير موجود', 'NOT_FOUND');
  mutate(() => (db.recurringExpenses = db.recurringExpenses.filter((r) => r.id !== id)));
}

/** Templates due today or earlier — "إيجار أكتوبر مستحق — سجّله" (docs/v2/09 §4). */
export function dueRecurringExpenses(today = localDateKey(new Date())): RecurringExpense[] {
  return db.recurringExpenses.filter((r) => r.active && r.nextDate <= today);
}

/** Posts one due template as a real expense (one click), then rolls `nextDate` forward a month. */
export function postRecurringExpense(id: string, userId: string, date = new Date().toISOString()): Expense {
  const template = db.recurringExpenses.find((r) => r.id === id);
  if (!template) throw new ApiError('القالب غير موجود', 'NOT_FOUND');
  const expense = recordExpense(
    {
      date,
      categoryId: template.categoryId,
      amount: template.amount,
      isTaxInvoice: template.isTaxInvoice,
      taxId: template.taxId,
      paidFrom: template.paidFrom,
      description: template.description,
      repeatMonthly: true,
      recurringTemplateId: template.id,
    },
    userId,
  );
  mutate(() => (template.nextDate = nextMonthDate(template.day, template.nextDate)));
  return expense;
}

// TODO(phase 10): surface `dueRecurringExpenses()` through the insight engine (docs/v2/11-journal-
// dashboard-insights.md Part B) instead of only the expenses list's due-list panel this phase ships.
