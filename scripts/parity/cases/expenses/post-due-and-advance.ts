/**
 * 11-expenses §8(b): the due-recurring list and "post due" (§3.11–3.12) on the EG base (14 % VAT):
 * the posted expense uses the template's current values, `repeatMonthly` and
 * `recurringTemplateId`, dated "now"; `nextDate` advances one month to day `min(day, 28)`, and
 * December rolls into January. Double-posting a no-longer-due template is not exercised (decision
 * E-D4: Rust refuses it, the mock posts again; the UI only offers due templates).
 */
import { defineCase } from '../../case';
import * as expenseService from '../../../../src/modules/expenses/services/expenseService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'expenses/post-due-and-advance',
  source: '03-domains/11-expenses.md §8(b)',
  base: 'demo-eg',
  user: 'accountant',
  async run(s) {
    const templates = await s.step('templates', () => expenseService.getRecurringExpenses());
    const internet = templates.find((t) => t.name === 'اشتراك الإنترنت الشهري')!.id;
    const rent = templates.find((t) => t.name === 'إيجار المحل الشهري')!.id;

    await s.step('due-2026-06-30', () => expenseService.getDueRecurringExpenses());
    const posted = await s.step('post-internet', () => expenseService.postDueRecurringExpense(internet));
    const linked = await s.step('internet-entries', () => accountingService.getJournalEntriesForSource('expense', posted.id));
    await s.step('internet-entry', () => accountingService.getJournalEntry(linked[0].id));
    await s.step('templates-after-first', () => expenseService.getRecurringExpenses());
    await s.step('due-after-first', () => expenseService.getDueRecurringExpenses());

    await s.setClock('2026-07-01T09:00:00.000Z');
    await s.step('due-2026-07-01', () => expenseService.getDueRecurringExpenses());
    const rentExpense = await s.step('post-rent', () => expenseService.postDueRecurringExpense(rent));
    await s.step('rent-detail', () => expenseService.getExpense(rentExpense.id));

    // December → January rollover, day kept.
    const categories = await s.step('categories', () => expenseService.getExpenseCategories());
    const december = await s.step('create-december', () =>
      expenseService.saveRecurringExpense({
        name: 'صيانة سنوية',
        categoryId: categories.find((c) => c.name === 'الصيانة والإصلاح')!.id,
        amount: 500,
        isTaxInvoice: false,
        paidFrom: { kind: 'method', paymentMethodId: s.baseId('pm-cash') },
        day: 20,
        nextDate: '2026-12-20',
        autoPost: false,
        active: true,
      }),
    );
    await s.setClock('2026-12-21T09:00:00.000Z');
    await s.step('due-2026-12-21', () => expenseService.getDueRecurringExpenses());
    await s.step('post-december', () => expenseService.postDueRecurringExpense(december.id));
    await s.step('templates-final', () => expenseService.getRecurringExpenses());
    await s.expectError('post-unknown', () => expenseService.postDueRecurringExpense('recexp-missing'));
  },
});
