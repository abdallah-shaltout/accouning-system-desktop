/**
 * 11-expenses §8(b): a cash expense without VAT — `Dr expense[cost center] / Cr cash`, source
 * `expense`, `EXP-` number, activity row. Read back through the expense and its linked journal entry.
 */
import { defineCase } from '../../case';
import * as expenseService from '../../../../src/modules/expenses/services/expenseService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'expenses/create-cash-no-vat',
  source: '03-domains/11-expenses.md §8(b)',
  base: 'demo-sa',
  user: 'accountant',
  async run(s) {
    const categories = await s.step('categories', () => expenseService.getExpenseCategories());
    const maintenance = categories.find((c) => c.name === 'الصيانة والإصلاح')!;
    const expense = await s.step('create', () =>
      expenseService.createExpense({
        date: '2026-06-29T09:00:00.000Z',
        categoryId: maintenance.id,
        amount: 250.5,
        isTaxInvoice: false,
        costCenterId: s.baseId('cc-main'),
        paidFrom: { kind: 'method', paymentMethodId: s.baseId('pm-cash') },
        description: 'تنظيف واجهة المحل',
      }),
    );
    await s.step('detail', () => expenseService.getExpense(expense.id));
    const linked = await s.step('linked-entries', () => accountingService.getJournalEntriesForSource('expense', expense.id));
    await s.step('journal-entry', () => accountingService.getJournalEntry(linked[0].id));
    // No description: the posting description is only "مصروف {number} — {category}".
    const bare = await s.step('create-no-description', () =>
      expenseService.createExpense({
        date: '2026-06-30T08:00:00.000Z',
        categoryId: maintenance.id,
        amount: 80,
        isTaxInvoice: false,
        paidFrom: { kind: 'method', paymentMethodId: s.baseId('pm-bank-transfer') },
      }),
    );
    const bareLinked = await s.step('linked-entries-no-description', () => accountingService.getJournalEntriesForSource('expense', bare.id));
    await s.step('journal-entry-no-description', () => accountingService.getJournalEntry(bareLinked[0].id));
  },
});
