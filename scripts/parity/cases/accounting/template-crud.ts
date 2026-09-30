/**
 * 12-accounting §8(b): journal templates (§3.5) — create (with and without recurrence), update
 * (recurrence cleared when absent), get, `loadTemplateIntoEntry`, list, remove, and the refusals
 * in order: name, fewer than 2 lines, missing account, AR line without a party, unknown id.
 *
 * The list is sorted by name with ICU `localeCompare(…,'ar')` on the mock and `utf8mb4_unicode_ci`
 * in Rust (quirk Q6), so list steps are compared as multisets. A duplicate name is refused only by
 * Rust (decision A-D7), so it is not exercised.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import type { JournalTemplateInput } from '../../../../src/modules/accounting/types';

export default defineCase({
  name: 'accounting/template-crud',
  source: '03-domains/12-accounting.md §8(b)',
  base: 'demo-sa',
  user: 'accountant',
  unordered: ['steps.list-after-create.value', 'steps.list-after-remove.value'],
  async run(s) {
    const lines: JournalTemplateInput['lines'] = [
      { accountId: s.baseId('acc-6220'), description: 'إيجار المحل', debit: 9000, credit: 0 },
      { accountId: s.baseId('acc-1120'), description: 'تحويل بنكي', debit: 0, credit: 9000 },
    ];
    await s.step('list-before', () => accountingService.getJournalTemplates());
    const rent = await s.step('create-recurring', () =>
      accountingService.createOrUpdateJournalTemplate({
        name: '  إيجار شهري  ',
        description: 'قيد الإيجار الشهري ',
        lines,
        recurrence: { every: 'month', day: 1, nextDate: '2026-07-01', autoPost: false },
      }),
    );
    const salaries = await s.step('create-plain', () =>
      accountingService.createOrUpdateJournalTemplate({
        name: 'رواتب',
        description: '',
        lines: [
          { accountId: s.baseId('acc-6210'), debit: 12000, credit: 0 },
          { accountId: s.baseId('acc-2160'), debit: 0, credit: 12000 },
          { accountId: s.baseId('acc-1130'), debit: 0, credit: 0 },
        ],
      }),
    );
    await s.step('create-with-party', () =>
      accountingService.createOrUpdateJournalTemplate({
        name: 'أجر عمولة عميل',
        description: 'تسوية شهرية',
        lines: [
          { accountId: s.baseId('acc-6110'), debit: 250, credit: 0 },
          { accountId: s.baseId('acc-1130'), debit: 0, credit: 250, partyKind: 'customer', partyId: s.baseId('cus-2') },
        ],
      }),
    );
    await s.step('list-after-create', () => accountingService.getJournalTemplates());
    // Update without `recurrence`: the recurrence is cleared (the mock assigns `input.recurrence`).
    await s.step('update-clears-recurrence', () => accountingService.createOrUpdateJournalTemplate({ name: 'إيجار شهري', description: 'بدون تكرار', lines }, rent.id));
    await s.step('get', () => accountingService.getJournalTemplate(rent.id));
    await s.step('load-into-entry', () => accountingService.loadTemplateIntoEntry(salaries.id));

    await s.expectError('name-blank', () => accountingService.createOrUpdateJournalTemplate({ name: ' ', description: '', lines }));
    await s.expectError('one-line', () => accountingService.createOrUpdateJournalTemplate({ name: 'قالب', description: '', lines: [lines[0]] }));
    await s.expectError('no-lines', () => accountingService.createOrUpdateJournalTemplate({ name: 'قالب', description: '', lines: [] }));
    await s.expectError('account-missing', () =>
      accountingService.createOrUpdateJournalTemplate({ name: 'قالب', description: '', lines: [lines[0], { accountId: 'acc-missing', debit: 0, credit: 9000 }] }),
    );
    await s.expectError('receivable-no-party', () =>
      accountingService.createOrUpdateJournalTemplate({ name: 'قالب', description: '', lines: [lines[0], { accountId: s.baseId('acc-1130'), debit: 0, credit: 9000 }] }),
    );
    await s.expectError('update-unknown', () => accountingService.createOrUpdateJournalTemplate({ name: 'قالب', description: '', lines }, 'jtpl-missing'));
    await s.expectError('get-unknown', () => accountingService.getJournalTemplate('jtpl-missing'));
    await s.expectError('load-unknown', () => accountingService.loadTemplateIntoEntry('jtpl-missing'));

    await s.step('remove', () => accountingService.removeJournalTemplate(salaries.id));
    await s.expectError('remove-again', () => accountingService.removeJournalTemplate(salaries.id));
    await s.step('list-after-remove', () => accountingService.getJournalTemplates());
  },
});
