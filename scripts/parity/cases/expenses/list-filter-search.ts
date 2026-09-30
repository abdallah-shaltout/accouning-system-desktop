/**
 * 11-expenses §8(b): `getExpenses` — full list order (date descending, ties in insertion order),
 * category and business-day date filters, and the search haystack of decision E-D3 (number,
 * category name, description, supplier invoice no) with Arabic normalization (`إ`/`ا`).
 * Plus `getExpense` on an unknown id.
 */
import { defineCase } from '../../case';
import * as expenseService from '../../../../src/modules/expenses/services/expenseService';

export default defineCase({
  name: 'expenses/list-filter-search',
  source: '03-domains/11-expenses.md §8(b)',
  base: 'demo-sa',
  user: 'accountant',
  async run(s) {
    const categories = await s.step('categories', () => expenseService.getExpenseCategories());
    const byName = (n: string) => categories.find((c) => c.name === n)!.id;
    // Two same-day expenses so the tie order (insertion order) is visible.
    for (const [i, amount] of [310, 120].entries()) {
      await s.step(`create-same-day-${i}`, () =>
        expenseService.createExpense({
          date: '2026-06-18T09:00:00.000Z',
          categoryId: byName('مصروفات متنوعة'),
          amount,
          isTaxInvoice: false,
          paidFrom: { kind: 'method', paymentMethodId: s.baseId('pm-cash') },
          description: `مصروف نثري ${i + 1}`,
        }),
      );
    }
    await s.step('all', () => expenseService.getExpenses());
    await s.step('by-category', () => expenseService.getExpenses({ categoryId: byName('مواد تغليف وأكياس') }));
    await s.step('by-date-range', () => expenseService.getExpenses({ from: '2026-06-01', to: '2026-06-18' }));
    await s.step('from-only', () => expenseService.getExpenses({ from: '2026-06-18' }));
    await s.step('search-number', () => expenseService.getExpenses({ search: 'EXP-000002' }));
    await s.step('search-category-normalized', () => expenseService.getExpenses({ search: 'الانترنت' }));
    await s.step('search-description', () => expenseService.getExpenses({ search: 'نثري' }));
    await s.step('search-supplier-invoice', () => expenseService.getExpenses({ search: 'stc-88213' }));
    await s.step('search-no-hit', () => expenseService.getExpenses({ search: 'لا يوجد شيء كهذا' }));
    await s.step('combined', () => expenseService.getExpenses({ categoryId: byName('مصروفات متنوعة'), from: '2026-06-18', to: '2026-06-18', search: 'نثري 2' }));
    await s.expectError('detail-unknown', () => expenseService.getExpense('exp-missing'));
  },
});
