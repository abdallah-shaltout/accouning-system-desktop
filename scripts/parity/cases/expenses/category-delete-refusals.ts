/**
 * 11-expenses §8(b): `deleteExpenseCategory` refusals (§3.3) — unknown id → NOT_FOUND, a category
 * that already has expenses → CONFLICT. The FORBIDDEN "تصنيف أساسي" refusal needs a `canDelete: false`
 * category, which no service can create and neither demo base seeds, so it is covered by
 * `tests/domain_expenses.rs` only. A deactivated category with expenses is still refused.
 */
import { defineCase } from '../../case';
import * as expenseService from '../../../../src/modules/expenses/services/expenseService';

export default defineCase({
  name: 'expenses/category-delete-refusals',
  source: '03-domains/11-expenses.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const categories = await s.step('categories', () => expenseService.getExpenseCategories());
    const internet = categories.find((c) => c.name === 'الاتصالات والإنترنت')!;
    await s.expectError('unknown', () => expenseService.deleteExpenseCategory('excat-missing'));
    await s.expectError('has-expenses', () => expenseService.deleteExpenseCategory(internet.id));
    await s.step('deactivate', () =>
      expenseService.saveExpenseCategory({ name: internet.name, icon: internet.icon, accountId: internet.accountId, active: false }, internet.id),
    );
    await s.expectError('has-expenses-inactive', () => expenseService.deleteExpenseCategory(internet.id));
    // A category whose only expense is new in this story is refused too.
    const fresh = await s.step('create-category', () =>
      expenseService.saveExpenseCategory({ name: 'ضيافة', accountId: s.baseId('acc-6390'), active: true }),
    );
    await s.step('expense-on-new-category', () =>
      expenseService.createExpense({
        date: '2026-06-30T07:00:00.000Z',
        categoryId: fresh.id,
        amount: 42,
        isTaxInvoice: false,
        paidFrom: { kind: 'method', paymentMethodId: s.baseId('pm-cash') },
      }),
    );
    await s.expectError('new-category-has-expense', () => expenseService.deleteExpenseCategory(fresh.id));
    await s.step('list-after', () => expenseService.getExpenseCategories());
  },
});
