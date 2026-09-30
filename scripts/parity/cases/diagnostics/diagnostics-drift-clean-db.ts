/**
 * L1 platform lane. `getDriftReport` (16-diagnostics.md §3, slice B): the seeded demo data has no
 * subledger-vs-GL drift, so every row list is empty (the doc's "all rows empty on a clean replay").
 * Also covers `explainAccountBalance` for a real account.
 */
import { defineCase } from '../../case';
import * as accountingDebugService from '../../../../src/modules/diagnostics/services/accountingDebugService';

export default defineCase({
  name: 'diagnostics/diagnostics-drift-clean-db',
  source: '03-domains/16-diagnostics.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('drift-report', () => accountingDebugService.getDriftReport());
    await s.step('explain-cash', () => accountingDebugService.explainAccountBalance(s.baseId('acc-1110')));
  },
});
