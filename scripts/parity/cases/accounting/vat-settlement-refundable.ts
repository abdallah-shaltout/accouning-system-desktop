/**
 * 12b-period-close §8(b): a VAT settlement where input VAT exceeds output VAT (§3.4) — July has
 * only a tax-invoice expense, so the settlement is Cr input VAT / Dr VAT payable ("صافي الضريبة
 * القابلة للاسترداد"), with no output line. Run on the EG base (14 %).
 *
 * Known red on the mock for the same reason as `accounting/vat-settlement-payable`: the §4.5 VAT
 * invariant does not exclude settlement entries (reported by lane L4, 2026-09-29).
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as expenseService from '../../../../src/modules/expenses/services/expenseService';

export default defineCase({
  name: 'accounting/vat-settlement-refundable',
  source: '03-domains/12b-period-close.md §8(b)',
  base: 'demo-eg',
  user: 'accountant',
  async run(s) {
    const categories = await s.step('categories', () => expenseService.getExpenseCategories());
    await s.step('expense-july', () =>
      expenseService.createExpense({
        date: '2026-07-05T09:00:00.000Z',
        categoryId: categories.find((c) => c.name === 'الصيانة والإصلاح')!.id,
        amount: 2280,
        isTaxInvoice: true,
        taxId: s.baseId('tax-vat-in'),
        paidFrom: { kind: 'method', paymentMethodId: s.baseId('pm-bank-transfer') },
        description: 'صيانة شاملة',
      }),
    );
    await s.step('totals-july', () => accountingService.getVatPeriodTotals('2026-07-01', '2026-07-31'));
    const entry = await s.step('settle-july', () => accountingService.submitVatSettlement('2026-07-01', '2026-07-31'));
    await s.step('detail', () => accountingService.getJournalEntry(entry.id));
    await s.expectError('no-movement-august', () => accountingService.submitVatSettlement('2026-08-01', '2026-08-31'));
  },
});
