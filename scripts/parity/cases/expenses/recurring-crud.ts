/**
 * 11-expenses §8(b): recurring-expense templates — create, update, delete, and the
 * `saveRecurringExpense` / `deleteRecurringExpense` refusals of §3.9–3.10 in order. The amount is
 * stored as sent (quirk Q5; a 2-dp amount so the column scale changes nothing). An update without
 * `description` keeps the stored one (decision E-D2).
 */
import { defineCase } from '../../case';
import * as expenseService from '../../../../src/modules/expenses/services/expenseService';

export default defineCase({
  name: 'expenses/recurring-crud',
  source: '03-domains/11-expenses.md §8(b)',
  base: 'demo-sa',
  user: 'accountant',
  async run(s) {
    await s.step('list-before', () => expenseService.getRecurringExpenses());
    const categories = await s.step('categories', () => expenseService.getExpenseCategories());
    const electricity = categories.find((c) => c.name === 'الكهرباء والمياه')!.id;
    const base = {
      name: '  فاتورة الكهرباء الشهرية ',
      categoryId: electricity,
      amount: 1200.55,
      isTaxInvoice: true,
      taxId: s.baseId('tax-vat-in'),
      paidFrom: { kind: 'method' as const, paymentMethodId: s.baseId('pm-bank-transfer') },
      description: 'الشركة السعودية للكهرباء',
      day: 5,
      nextDate: '2026-07-05',
      autoPost: false,
      active: true,
    };
    const created = await s.step('create', () => expenseService.saveRecurringExpense(base));
    const { description: _omit, ...withoutDescription } = base;
    await s.step('update', () =>
      expenseService.saveRecurringExpense({ ...withoutDescription, name: 'الكهرباء', amount: 1350.4, day: 28, nextDate: '2026-07-28', autoPost: true }, created.id),
    );
    await s.step('list-after-update', () => expenseService.getRecurringExpenses());

    await s.expectError('name-required', () => expenseService.saveRecurringExpense({ ...base, name: ' ' }));
    await s.expectError('category-unknown', () => expenseService.saveRecurringExpense({ ...base, categoryId: 'excat-missing' }));
    await s.expectError('day-zero', () => expenseService.saveRecurringExpense({ ...base, day: 0 }));
    await s.expectError('day-29', () => expenseService.saveRecurringExpense({ ...base, day: 29 }));
    await s.expectError('update-unknown', () => expenseService.saveRecurringExpense(base, 'recexp-missing'));

    await s.step('delete', () => expenseService.deleteRecurringExpense(created.id));
    await s.expectError('delete-again', () => expenseService.deleteRecurringExpense(created.id));
    await s.step('list-after-delete', () => expenseService.getRecurringExpenses());
  },
});
