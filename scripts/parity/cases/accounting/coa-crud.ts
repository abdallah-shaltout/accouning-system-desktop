/**
 * 12-accounting §8(b): chart-of-accounts CRUD — `getAccounts` (all time and a date range, signed
 * balances, `hasPostings`), create a group and a leaf under it, update (trimmed name, an absent
 * optional field keeps its value, decision A-D2), post to the new leaf so `getAccounts` shows its
 * balance, then delete a posting-free leaf and its (now empty) group.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import type { AccountInput } from '../../../../src/modules/accounting/types';

export default defineCase({
  name: 'accounting/coa-crud',
  source: '03-domains/12-accounting.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('accounts-all-time', () => accountingService.getAccounts());
    await s.step('accounts-june', () => accountingService.getAccounts({ from: '2026-06-01', to: '2026-06-30' }));
    await s.step('accounts-from-only', () => accountingService.getAccounts({ from: '2026-06-28' }));

    const expense = (over: Partial<AccountInput>): AccountInput => ({
      code: '',
      name: '',
      isGroup: false,
      kind: 'EXPENSE',
      subtype: 'operatingExpense',
      normalSide: 'DEBIT',
      allowManual: true,
      active: true,
      ...over,
    } as AccountInput);

    const group = await s.step('create-group', () =>
      accountingService.saveAccount(expense({ code: '64', name: ' مصروفات تمويلية ', parentId: s.baseId('acc-6'), isGroup: true, allowManual: false })),
    );
    const leaf = await s.step('create-leaf', () =>
      accountingService.saveAccount(expense({ code: '6410', name: 'فوائد بنكية', nameEn: 'Bank interest', parentId: group.id, subtype: 'otherExpense' })),
    );
    const spare = await s.step('create-leaf-2', () => accountingService.saveAccount(expense({ code: '6420', name: 'عمولات تمويل', parentId: group.id })));
    // No nameEn in the update: the stored English name stays (A-D2).
    await s.step('update-leaf', () =>
      accountingService.saveAccount(expense({ code: '6410', name: '  فوائد وعمولات بنكية  ', parentId: group.id, subtype: 'otherExpense', requiresParty: false }), leaf.id),
    );
    await s.step('post-to-leaf', () =>
      accountingService.createJournalEntry({
        date: '2026-06-29T10:00:00.000Z',
        description: 'فوائد بنكية على الحساب الجاري',
        lines: [
          { accountId: leaf.id, debit: 185.4, credit: 0 },
          { accountId: s.baseId('acc-1120'), debit: 0, credit: 185.4 },
        ],
      }),
    );
    await s.step('accounts-after-post', () => accountingService.getAccounts({ from: '2026-06-29', to: '2026-06-29' }));
    await s.step('delete-spare-leaf', () => accountingService.deleteAccount(spare.id));
    // Deactivate the posted leaf instead of deleting it (the "stop" path the delete refusal suggests).
    await s.step('deactivate-leaf', () =>
      accountingService.saveAccount(expense({ code: '6410', name: 'فوائد وعمولات بنكية', parentId: group.id, subtype: 'otherExpense', active: false }), leaf.id),
    );
    // A group created and emptied in the same story can be deleted.
    const tmp = await s.step('create-empty-group', () =>
      accountingService.saveAccount(expense({ code: '65', name: 'مجموعة مؤقتة', parentId: s.baseId('acc-6'), isGroup: true, allowManual: false })),
    );
    await s.step('delete-empty-group', () => accountingService.deleteAccount(tmp.id));
    await s.step('accounts-final', () => accountingService.getAccounts());
  },
});
