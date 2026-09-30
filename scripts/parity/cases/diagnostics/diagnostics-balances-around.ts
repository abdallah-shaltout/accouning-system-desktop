/**
 * L1 platform lane. `getBalancesAround` (16-diagnostics.md §3, slice B): balances before/after for
 * the most recently posted journal entry, plus the empty-array result for an unknown id.
 */
import { defineCase } from '../../case';
import * as accountingDebugService from '../../../../src/modules/diagnostics/services/accountingDebugService';

export default defineCase({
  name: 'diagnostics/diagnostics-balances-around',
  source: '03-domains/16-diagnostics.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const docs = await s.step('recent-documents', () => accountingDebugService.listRecentDocuments(5));
    await s.step('balances-around', () => accountingDebugService.getBalancesAround(docs[0].id));
    await s.step('balances-around-missing', () => accountingDebugService.getBalancesAround('je-does-not-exist'));
    await s.step('journal-entry-raw', () => accountingDebugService.getJournalEntryRaw(docs[0].id));
    await s.step('posting-trace-missing', () => accountingDebugService.getPostingTrace(docs[0].id));
  },
});
