/**
 * 12-accounting §8(b): every `saveAccount` / `deleteAccount` refusal of §3.2, each alone and in the
 * mock's check order — code format, name, code clash (CONFLICT), parent self / missing / not a
 * group / other kind / code prefix, group + allowManual; update: unknown id, system account shape
 * change, group⇄leaf flips blocked by children or postings; delete: unknown, system account
 * (default code kept, quirk Q8), children (CONFLICT), postings (CONFLICT).
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import type { AccountInput } from '../../../../src/modules/accounting/types';

export default defineCase({
  name: 'accounting/coa-refusals',
  source: '03-domains/12-accounting.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const input = (over: Partial<AccountInput>): AccountInput => ({
      code: '6299',
      name: 'حساب تجريبي',
      parentId: s.baseId('acc-62'),
      isGroup: false,
      kind: 'EXPENSE',
      subtype: 'operatingExpense',
      normalSide: 'DEBIT',
      allowManual: true,
      active: true,
      ...over,
    } as AccountInput);

    await s.expectError('code-letters', () => accountingService.saveAccount(input({ code: '62A1' })));
    await s.expectError('code-too-long', () => accountingService.saveAccount(input({ code: '622000001' })));
    await s.expectError('code-empty', () => accountingService.saveAccount(input({ code: '' })));
    await s.expectError('name-blank', () => accountingService.saveAccount(input({ name: '   ' })));
    await s.expectError('code-taken', () => accountingService.saveAccount(input({ code: '6220' })));
    await s.expectError('parent-self', () => accountingService.saveAccount(input({ code: '62', name: 'المصروفات العمومية', isGroup: true, allowManual: false, parentId: s.baseId('acc-62') }), s.baseId('acc-62')));
    await s.expectError('parent-missing', () => accountingService.saveAccount(input({ parentId: 'acc-missing' })));
    await s.expectError('parent-not-group', () => accountingService.saveAccount(input({ code: '62201', parentId: s.baseId('acc-6220') })));
    await s.expectError('parent-other-kind', () => accountingService.saveAccount(input({ code: '4399', kind: 'REVENUE', parentId: s.baseId('acc-62') })));
    await s.expectError('code-prefix', () => accountingService.saveAccount(input({ code: '6399' })));
    await s.expectError('group-allow-manual', () => accountingService.saveAccount(input({ code: '629', isGroup: true, allowManual: true })));
    // The code clash is reported before the parent checks (first failing rule wins).
    await s.expectError('first-rule-wins', () => accountingService.saveAccount(input({ code: '6220', parentId: 'acc-missing' })));

    await s.expectError('update-unknown', () => accountingService.saveAccount(input({ code: '6298' }), 'acc-missing'));
    await s.expectError('system-code-change', () =>
      accountingService.saveAccount(input({ code: '6221', name: 'الإيجار', parentId: s.baseId('acc-62') }), s.baseId('acc-6280')),
    );
    await s.expectError('system-kind-change', () =>
      accountingService.saveAccount({ code: '3100', name: 'رأس المال', isGroup: false, kind: 'LIABILITY', subtype: 'equity', normalSide: 'CREDIT', allowManual: true, active: true } as AccountInput, s.baseId('acc-3100')),
    );
    await s.expectError('system-shape-change', () =>
      accountingService.saveAccount({ code: '3100', name: 'رأس المال', parentId: s.baseId('acc-3'), isGroup: true, kind: 'EQUITY', subtype: 'equity', normalSide: 'CREDIT', allowManual: false, active: true } as AccountInput, s.baseId('acc-3100')),
    );
    // User accounts: group → leaf with children, leaf → group with postings.
    const group = await s.step('create-group', () => accountingService.saveAccount(input({ code: '627', name: 'مجموعة تجريبية', isGroup: true, allowManual: false })));
    const child = await s.step('create-child', () => accountingService.saveAccount(input({ code: '6271', name: 'فرعي تجريبي', parentId: group.id })));
    await s.expectError('flip-group-with-children', () => accountingService.saveAccount(input({ code: '627', name: 'مجموعة تجريبية', isGroup: false }), group.id));
    await s.step('post-to-child', () =>
      accountingService.createJournalEntry({
        date: '2026-06-30T06:00:00.000Z',
        description: 'قيد على حساب فرعي',
        lines: [
          { accountId: child.id, debit: 50, credit: 0 },
          { accountId: s.baseId('acc-1110'), debit: 0, credit: 50 },
        ],
      }),
    );
    await s.expectError('flip-leaf-with-postings', () =>
      accountingService.saveAccount(input({ code: '6271', name: 'فرعي تجريبي', parentId: group.id, isGroup: true, allowManual: false }), child.id),
    );

    await s.expectError('delete-unknown', () => accountingService.deleteAccount('acc-missing'));
    await s.expectError('delete-system', () => accountingService.deleteAccount(s.baseId('acc-6280')));
    await s.expectError('delete-with-children', () => accountingService.deleteAccount(group.id));
    await s.expectError('delete-with-postings', () => accountingService.deleteAccount(child.id));
    await s.expectError('reparent-unknown', () => accountingService.reparentAccount('acc-missing', s.baseId('acc-62')));
  },
});
