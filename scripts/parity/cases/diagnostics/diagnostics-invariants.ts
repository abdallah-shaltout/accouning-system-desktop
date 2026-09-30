/**
 * L1 platform lane. `getInvariantResults` (16-diagnostics.md §3, debug-only reads, slice B): the
 * full 14-invariant run over the seeded demo data, which must be all-passed.
 */
import { defineCase } from '../../case';
import * as accountingDebugService from '../../../../src/modules/diagnostics/services/accountingDebugService';

export default defineCase({
  name: 'diagnostics/diagnostics-invariants',
  source: '03-domains/16-diagnostics.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('invariants', () => accountingDebugService.getInvariantResults());
  },
});
