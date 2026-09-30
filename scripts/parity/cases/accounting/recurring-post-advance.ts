/**
 * 12-accounting §8(b): posting recurring templates (§3.5) — the entry is dated `nextDate`, carries
 * `templateId`, uses the template description (or the name when empty), and `nextDate` advances
 * with JavaScript `Date` month arithmetic (quirk Q3: Jan 31 + 1 month → Mar 3; quarter; Feb 29 +
 * 1 year → Mar 1). Posting ahead of the due date is allowed (decision A-D8). Refusals: unknown
 * template, a template with no recurrence, and a due date in the closed year for a non-admin.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import type { JournalTemplateInput } from '../../../../src/modules/accounting/types';

export default defineCase({
  name: 'accounting/recurring-post-advance',
  source: '03-domains/12-accounting.md §8(b)',
  base: 'demo-sa',
  user: 'accountant',
  async run(s) {
    const lines: JournalTemplateInput['lines'] = [
      { accountId: s.baseId('acc-6290'), description: 'إهلاك', debit: 300, credit: 0 },
      { accountId: s.baseId('acc-1290'), description: 'مجمع الإهلاك', debit: 0, credit: 300 },
    ];
    const monthly = await s.step('create-monthly', () =>
      accountingService.createOrUpdateJournalTemplate({ name: 'إهلاك شهري', description: '', lines, recurrence: { every: 'month', day: 31, nextDate: '2026-01-31', autoPost: false } }),
    );
    const quarterly = await s.step('create-quarterly', () =>
      accountingService.createOrUpdateJournalTemplate({ name: 'تأمين ربع سنوي', description: 'قسط التأمين', lines, recurrence: { every: 'quarter', day: 30, nextDate: '2026-05-31', autoPost: false } }),
    );
    const yearly = await s.step('create-yearly', () =>
      accountingService.createOrUpdateJournalTemplate({ name: 'رسوم سنوية', description: 'تجديد الرخصة', lines, recurrence: { every: 'year', day: 29, nextDate: '2028-02-29', autoPost: false } }),
    );
    const plain = await s.step('create-plain', () => accountingService.createOrUpdateJournalTemplate({ name: 'قالب عادي', description: '', lines }));
    const closed = await s.step('create-closed-year', () =>
      accountingService.createOrUpdateJournalTemplate({ name: 'قالب في سنة مقفلة', description: '', lines, recurrence: { every: 'month', day: 15, nextDate: '2025-12-15', autoPost: false } }),
    );

    const first = await s.step('post-monthly-1', () => accountingService.postRecurringTemplate(monthly.id));
    await s.step('detail-monthly-1', () => accountingService.getJournalEntry(first.id));
    await s.step('after-monthly-1', () => accountingService.getJournalTemplate(monthly.id));
    await s.step('post-monthly-2', () => accountingService.postRecurringTemplate(monthly.id));
    await s.step('after-monthly-2', () => accountingService.getJournalTemplate(monthly.id));
    await s.step('post-quarterly', () => accountingService.postRecurringTemplate(quarterly.id));
    await s.step('after-quarterly', () => accountingService.getJournalTemplate(quarterly.id));
    // Far ahead of "today" (A-D8) and a Feb-29 roll.
    await s.step('post-yearly', () => accountingService.postRecurringTemplate(yearly.id));
    await s.step('after-yearly', () => accountingService.getJournalTemplate(yearly.id));

    await s.expectError('unknown', () => accountingService.postRecurringTemplate('jtpl-missing'));
    await s.expectError('not-recurring', () => accountingService.postRecurringTemplate(plain.id));
    await s.expectError('closed-year-accountant', () => accountingService.postRecurringTemplate(closed.id));
    await s.step('closed-year-not-advanced', () => accountingService.getJournalTemplate(closed.id));
    await s.step('entries-by-template', () => accountingService.getJournalEntries({ type: 'MANUAL', search: 'إهلاك' }));
  },
});
