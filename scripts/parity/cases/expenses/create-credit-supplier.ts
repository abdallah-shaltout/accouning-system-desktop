/**
 * 11-expenses §8(b): an expense on credit to a supplier — the credit leg is `payable[supplier]`
 * (quirk Q7) — run on the EG base (14 % VAT, EGP).
 *
 * Known red on the mock (reported by lane L4, 2026-09-29): the §4.6 `supplier-allocation`
 * invariant counts only purchase documents, so a credit expense (a real payable with no purchase
 * order) is flagged as a mismatch. The posting itself is correct (docs/v2/09 §4).
 */
import { defineCase } from '../../case';
import * as expenseService from '../../../../src/modules/expenses/services/expenseService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'expenses/create-credit-supplier',
  source: '03-domains/11-expenses.md §8(b)',
  base: 'demo-eg',
  user: 'accountant',
  async run(s) {
    const supplierId = s.baseId('sup-1');
    await s.step('supplier-before', () => partyService.getSupplier(supplierId));
    const categories = await s.step('categories', () => expenseService.getExpenseCategories());
    const packaging = categories.find((c) => c.name === 'مواد تغليف وأكياس')!;
    const expense = await s.step('create', () =>
      expenseService.createExpense({
        date: '2026-06-25T11:00:00.000Z',
        categoryId: packaging.id,
        amount: 1140,
        isTaxInvoice: true,
        taxId: s.baseId('tax-vat-in'),
        supplierInvoiceNo: 'NS-7781',
        paidFrom: { kind: 'credit', supplierId },
        description: 'كراتين وأكياس بالآجل',
      }),
    );
    await s.step('detail', () => expenseService.getExpense(expense.id));
    const linked = await s.step('linked-entries', () => accountingService.getJournalEntriesForSource('expense', expense.id));
    await s.step('journal-entry', () => accountingService.getJournalEntry(linked[0].id));
    await s.step('supplier-after', () => partyService.getSupplier(supplierId));
    await s.step('supplier-statement', () => partyService.getSupplierStatement(supplierId));
  },
});
