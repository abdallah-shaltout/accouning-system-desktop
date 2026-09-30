/**
 * 11-expenses §8(b): expense-category create / update / delete and the `saveExpenseCategory`
 * refusals of §3.2 in the mock's order (name → account → existence). An update without `icon`
 * keeps the stored icon (decision E-D2). Duplicate names are not exercised: the mock allows them
 * and Rust refuses them (decision E-D1), a documented scope difference, not a parity bug.
 */
import { defineCase } from '../../case';
import * as expenseService from '../../../../src/modules/expenses/services/expenseService';

export default defineCase({
  name: 'expenses/category-crud',
  source: '03-domains/11-expenses.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('list-before', () => expenseService.getExpenseCategories());
    const created = await s.step('create', () =>
      expenseService.saveExpenseCategory({ name: '  رسوم التوصيل  ', icon: 'truck', accountId: s.baseId('acc-6150'), active: true }),
    );
    await s.step('update-keeps-icon', () =>
      expenseService.saveExpenseCategory({ name: 'مصاريف التوصيل', accountId: s.baseId('acc-6390'), active: false }, created.id),
    );
    await s.step('update-with-icon', () =>
      expenseService.saveExpenseCategory({ name: 'مصاريف التوصيل', icon: 'package', accountId: s.baseId('acc-6390'), active: true }, created.id),
    );
    await s.step('list-after-update', () => expenseService.getExpenseCategories());

    await s.expectError('name-required', () => expenseService.saveExpenseCategory({ name: '   ', accountId: s.baseId('acc-6150'), active: true }));
    await s.expectError('account-unknown', () => expenseService.saveExpenseCategory({ name: 'تصنيف', accountId: 'acc-missing', active: true }));
    // The account check runs before the existence check (mock order, §3.2 step 2).
    await s.expectError('update-unknown-id', () =>
      expenseService.saveExpenseCategory({ name: 'تصنيف', accountId: s.baseId('acc-6150'), active: true }, 'excat-missing'),
    );

    await s.step('delete', () => expenseService.deleteExpenseCategory(created.id));
    await s.step('list-after-delete', () => expenseService.getExpenseCategories());
  },
});
