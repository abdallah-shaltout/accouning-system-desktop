/**
 * 12-accounting §8(b): every `validate_manual_lines` refusal of §3.3, each alone and in the mock's
 * order, including the B1 control-account rules (docs/v2/02-accounting-review.md B1): inventory /
 * VAT accounts are FORBIDDEN for manual lines, AR/AP lines need a party, cost-center-required
 * accounts need a cost center while the feature is on. Then the ledger's own refusals (unbalanced)
 * and the period guard (lock date, closed year for a non-admin). The same rules apply to drafts.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import type { AccountInput, JournalEntryInput } from '../../../../src/modules/accounting/types';

export default defineCase({
  name: 'accounting/manual-entry-b1-refusals',
  source: '03-domains/12-accounting.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const cash = s.baseId('acc-1110');
    const rent = s.baseId('acc-6220');
    const entry = (lines: JournalEntryInput['lines'], over: Partial<JournalEntryInput> = {}): JournalEntryInput => ({
      date: '2026-06-30T08:00:00.000Z',
      description: 'قيد اختبار',
      lines,
      ...over,
    });
    const pair = (accountId: string, amount = 100): JournalEntryInput['lines'] => [
      { accountId, debit: amount, credit: 0 },
      { accountId: cash, debit: 0, credit: amount },
    ];

    // Set up an inactive account and a cost-center-required account (both user accounts).
    const inactive = await s.step('create-inactive', () =>
      accountingService.saveAccount({ code: '6296', name: 'حساب موقوف', parentId: s.baseId('acc-62'), isGroup: false, kind: 'EXPENSE', subtype: 'operatingExpense', normalSide: 'DEBIT', allowManual: true, active: false } as AccountInput),
    );

    await s.expectError('description-blank', () => accountingService.createJournalEntry(entry(pair(rent), { description: '   ' })));
    await s.expectError('one-line', () => accountingService.createJournalEntry(entry([{ accountId: rent, debit: 100, credit: 0 }])));
    await s.expectError('account-missing', () => accountingService.createJournalEntry(entry(pair('acc-missing'))));
    await s.expectError('account-inactive', () => accountingService.createJournalEntry(entry(pair(inactive.id))));
    await s.expectError('account-group', () => accountingService.createJournalEntry(entry(pair(s.baseId('acc-62')))));
    await s.expectError('negative', () =>
      accountingService.createJournalEntry(entry([{ accountId: rent, debit: -5, credit: 0 }, { accountId: cash, debit: 0, credit: -5 }])),
    );
    await s.expectError('both-sides', () =>
      accountingService.createJournalEntry(entry([{ accountId: rent, debit: 50, credit: 50 }, { accountId: cash, debit: 0, credit: 0 }])),
    );
    await s.expectError('b1-inventory', () => accountingService.createJournalEntry(entry(pair(s.baseId('acc-1140')))));
    await s.expectError('b1-vat-output', () =>
      accountingService.createJournalEntry(entry([{ accountId: cash, debit: 15, credit: 0 }, { accountId: s.baseId('acc-2150'), debit: 0, credit: 15 }])),
    );
    await s.expectError('b1-vat-input', () => accountingService.createJournalEntry(entry(pair(s.baseId('acc-1150')))));
    await s.expectError('b1-receivable-no-party', () => accountingService.createJournalEntry(entry(pair(s.baseId('acc-1130')))));
    await s.expectError('b1-payable-no-party', () =>
      accountingService.createJournalEntry(entry([{ accountId: cash, debit: 70, credit: 0 }, { accountId: s.baseId('acc-2100'), debit: 0, credit: 70 }])),
    );
    // A zero line on a no-manual account is allowed through validation (only non-zero lines are checked).
    await s.step('b1-zero-line-allowed', () =>
      accountingService.createJournalEntry(entry([...pair(rent, 33), { accountId: s.baseId('acc-1140'), debit: 0, credit: 0 }], { description: 'سطر صفري على حساب المخزون' })),
    );
    await s.expectError('unbalanced', () =>
      accountingService.createJournalEntry(entry([{ accountId: rent, debit: 100.5, credit: 0 }, { accountId: cash, debit: 0, credit: 100 }])),
    );
    await s.expectError('all-zero-lines', () =>
      accountingService.createJournalEntry(entry([{ accountId: rent, debit: 0, credit: 0 }, { accountId: cash, debit: 0, credit: 0 }])),
    );
    // Drafts run the same validation (no period check).
    await s.expectError('draft-b1-inventory', () => accountingService.createJournalEntry(entry(pair(s.baseId('acc-1140')), { asDraft: true })));

    // Period guard: lock date before the demo history (see expenses/create-period-locked), then a
    // non-admin in the closed year.
    await s.step('set-lock-date', () => accountingService.saveLockDate('2026-04-15'));
    await s.login('accountant');
    await s.expectError('lock-date', () => accountingService.createJournalEntry(entry(pair(rent), { date: '2026-04-15T20:00:00.000Z' })));
    await s.step('after-lock-date', () => accountingService.createJournalEntry(entry(pair(rent), { date: '2026-04-16' })));
    await s.step('clear-lock-date', () => accountingService.saveLockDate(undefined));
    await s.expectError('closed-year', () => accountingService.createJournalEntry(entry(pair(rent), { date: '2025-06-30' })));
    await s.step('lock-date-after-clear', () => accountingService.getLockDate());
  },
});
