/**
 * 11-expenses §8(b): tax-invoice expenses at the SA 15 % rate — `splitTax` on the raw amount
 * (`net = round2(amount / 1.15)`, `vat = round2(amount − net)`), a third `vatInput` line; plus a
 * tax invoice with no tax picked (quirk Q2: silent rate 0).
 */
import { defineCase } from '../../case';
import * as expenseService from '../../../../src/modules/expenses/services/expenseService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'expenses/create-tax-invoice-15',
  source: '03-domains/11-expenses.md §8(b)',
  base: 'demo-sa',
  user: 'accountant',
  async run(s) {
    const categories = await s.step('categories', () => expenseService.getExpenseCategories());
    const internet = categories.find((c) => c.name === 'الاتصالات والإنترنت')!;
    const cases: { name: string; amount: number; taxId?: string }[] = [
      { name: 'round-115', amount: 115, taxId: s.baseId('tax-vat-in') },
      { name: 'uneven-1000.01', amount: 1000.01, taxId: s.baseId('tax-vat-in') },
      { name: 'uneven-57.33', amount: 57.33, taxId: s.baseId('tax-vat-in') },
      { name: 'no-tax-picked', amount: 200 },
    ];
    for (const c of cases) {
      const expense = await s.step(`create-${c.name}`, () =>
        expenseService.createExpense({
          date: '2026-06-28T10:00:00.000Z',
          categoryId: internet.id,
          amount: c.amount,
          isTaxInvoice: true,
          taxId: c.taxId,
          supplierVatNumber: '300000000000003',
          supplierInvoiceNo: `INV-${c.name}`,
          paidFrom: { kind: 'method', paymentMethodId: s.baseId('pm-bank-transfer') },
          description: `فاتورة ضريبية ${c.name}`,
        }),
      );
      const linked = await s.step(`linked-${c.name}`, () => accountingService.getJournalEntriesForSource('expense', expense.id));
      await s.step(`entry-${c.name}`, () => accountingService.getJournalEntry(linked[0].id));
    }
    await s.step('vat-june', () => accountingService.getVatPeriodTotals('2026-06-01', '2026-06-30'));
  },
});
