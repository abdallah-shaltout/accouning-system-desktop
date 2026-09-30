/**
 * 11-expenses §8(b): an expense dated in a locked period is refused with the ledger's FORBIDDEN
 * period messages — closed fiscal year, then lock date. Error only (quirk Q1: the mock pushes the
 * expense row before `postJournal` refuses, Rust is atomic), so nothing is read back afterwards,
 * and the refused expenses carry no VAT (an orphan mock row with VAT would move the vat-input
 * invariant). Expenses never take the admin closed-period override, so the admin is refused too.
 *
 * The lock date is the day before the demo history starts (2026-04-16): the mock's `lock-date`
 * invariant counts every entry dated on/before the lock date as an offender, whenever it was
 * posted, so a later lock date would fail the case for a reason unrelated to expenses (reported
 * by lane L4, 2026-09-29).
 */
import { defineCase } from '../../case';
import * as expenseService from '../../../../src/modules/expenses/services/expenseService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'expenses/create-period-locked',
  source: '03-domains/11-expenses.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const categories = await s.step('categories', () => expenseService.getExpenseCategories());
    const misc = categories.find((c) => c.name === 'مصروفات متنوعة')!;
    const input = (date: string) => ({
      date,
      categoryId: misc.id,
      amount: 75,
      isTaxInvoice: false,
      paidFrom: { kind: 'method' as const, paymentMethodId: s.baseId('pm-cash') },
      description: 'مصروف في فترة مقفلة',
    });
    await s.expectError('closed-year-admin', () => expenseService.createExpense(input('2025-12-15T09:00:00.000Z')));
    await s.step('set-lock-date', () => accountingService.saveLockDate('2026-04-15'));
    await s.expectError('before-lock-date', () => expenseService.createExpense(input('2026-04-10T09:00:00.000Z')));
    await s.expectError('on-lock-date', () => expenseService.createExpense(input('2026-04-15T09:00:00.000Z')));
    await s.expectError('closed-year-under-lock-date', () => expenseService.createExpense(input('2025-12-15T09:00:00.000Z')));
    await s.login('accountant');
    await s.expectError('before-lock-date-accountant', () => expenseService.createExpense(input('2026-04-01T09:00:00.000Z')));
  },
});
