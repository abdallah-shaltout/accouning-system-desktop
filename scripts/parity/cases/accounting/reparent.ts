/**
 * 12-accounting §8(b): `reparentAccount` — a successful move (the code prefix is not re-checked,
 * quirk Q5), a move to the root (`null`), and the refusals of §3.2 in order: self, missing parent,
 * parent not a group, other kind, cycle (moving a group under its own descendant).
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import type { AccountInput } from '../../../../src/modules/accounting/types';

export default defineCase({
  name: 'accounting/reparent',
  source: '03-domains/12-accounting.md §8(b)',
  base: 'demo-sa',
  user: 'accountant',
  async run(s) {
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
    const outer = await s.step('create-outer', () => accountingService.saveAccount(expense({ code: '628', name: 'مجموعة خارجية', parentId: s.baseId('acc-62'), isGroup: true, allowManual: false })));
    const inner = await s.step('create-inner', () => accountingService.saveAccount(expense({ code: '6281', name: 'مجموعة داخلية', parentId: outer.id, isGroup: true, allowManual: false })));
    const leaf = await s.step('create-leaf', () => accountingService.saveAccount(expense({ code: '62811', name: 'حساب ورقي', parentId: inner.id })));

    // Move an existing seeded leaf under the new group (prefix not re-checked, Q5).
    await s.step('move-existing-leaf', () => accountingService.reparentAccount(s.baseId('acc-6270'), outer.id));
    await s.step('move-leaf-to-other-group', () => accountingService.reparentAccount(leaf.id, s.baseId('acc-63')));
    await s.step('move-to-root', () => accountingService.reparentAccount(leaf.id, null));

    await s.expectError('self', () => accountingService.reparentAccount(inner.id, inner.id));
    await s.expectError('parent-missing', () => accountingService.reparentAccount(inner.id, 'acc-missing'));
    await s.expectError('parent-not-group', () => accountingService.reparentAccount(inner.id, s.baseId('acc-6220')));
    await s.expectError('other-kind', () => accountingService.reparentAccount(inner.id, s.baseId('acc-4')));
    await s.expectError('cycle-direct-child', () => accountingService.reparentAccount(outer.id, inner.id));
    await s.step('move-leaf-back', () => accountingService.reparentAccount(leaf.id, inner.id));
    await s.expectError('cycle-deep', () => accountingService.reparentAccount(s.baseId('acc-62'), inner.id));
    await s.step('accounts-after', () => accountingService.getAccounts({ from: '2026-06-30', to: '2026-06-30' }));
  },
});
