/**
 * 12-accounting §8(b): the manual-entry draft lifecycle — save as draft (consumes a `JE-` number,
 * no GL effect, no period check), update (attachments kept when absent), list drafts, post (same
 * id and number, now POSTED), delete another draft, and the `المسودة غير موجودة` refusals.
 *
 * Rust writes a `قيد يدوي …` activity row when a draft is posted (decision A-D1, needed for undo);
 * the mock writes none. The recent-activity read after the post is therefore allowlisted.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as dashboardService from '../../../../src/modules/core/services/dashboardService';
import type { JournalEntryInput } from '../../../../src/modules/accounting/types';

export default defineCase({
  name: 'accounting/draft-lifecycle',
  source: '03-domains/12-accounting.md §8(b)',
  base: 'demo-sa',
  user: 'accountant',
  allow: [{ path: 'steps.activity-after-post.value', reason: '12-accounting A-D1: Rust logs a "قيد يدوي …" activity row on postJournalDraft (undo needs an audit row); the mock logs none' }],
  async run(s) {
    const input = (over: Partial<JournalEntryInput> = {}): JournalEntryInput => ({
      date: '2026-06-30T07:30:00.000Z',
      description: ' إهلاك الأثاث — يونيو ',
      lines: [
        { accountId: s.baseId('acc-6290'), description: 'مصروف إهلاك', debit: 250, credit: 0 },
        { accountId: s.baseId('acc-1290'), description: 'مجمع الإهلاك', debit: 0, credit: 250 },
      ],
      asDraft: true,
      ...over,
    });
    const draft = await s.step('save-draft', () => accountingService.createJournalEntry(input()));
    // A draft may be dated in the closed year: no period check until it is posted.
    const closedYearDraft = await s.step('save-draft-closed-year', () => accountingService.createJournalEntry(input({ date: '2025-12-31', description: 'مسودة في سنة مقفلة' })));
    await s.step('drafts', () => accountingService.getJournalEntries({ status: 'DRAFT' }));
    await s.step('accounts-unchanged', () => accountingService.getAccounts({ from: '2026-06-30', to: '2026-06-30' }));
    await s.step('update-draft', () =>
      accountingService.updateJournalDraft(draft.id, {
        date: '2026-06-30T07:45:00.000Z',
        description: 'إهلاك الأثاث والمعدات — يونيو',
        lines: [
          { accountId: s.baseId('acc-6290'), debit: 400, credit: 0 },
          { accountId: s.baseId('acc-1290'), debit: 0, credit: 400 },
        ],
      }),
    );
    await s.step('detail-draft', () => accountingService.getJournalEntry(draft.id));
    await s.expectError('update-invalid', () => accountingService.updateJournalDraft(draft.id, input({ description: '' })));
    const posted = await s.step('post-draft', () => accountingService.postJournalDraft(draft.id));
    await s.step('detail-posted', () => accountingService.getJournalEntry(posted.id));
    await s.step('activity-after-post', () => dashboardService.getRecentActivity(3));
    await s.expectError('post-again', () => accountingService.postJournalDraft(draft.id));
    await s.expectError('post-closed-year-draft', () => accountingService.postJournalDraft(closedYearDraft.id));
    await s.step('delete-draft', () => accountingService.deleteJournalDraft(closedYearDraft.id));
    await s.expectError('delete-again', () => accountingService.deleteJournalDraft(closedYearDraft.id));
    await s.expectError('update-missing', () => accountingService.updateJournalDraft('je-missing', input()));
    await s.step('drafts-after', () => accountingService.getJournalEntries({ status: 'DRAFT' }));
    await s.step('accounts-after', () => accountingService.getAccounts({ from: '2026-06-30', to: '2026-06-30' }));
  },
});
