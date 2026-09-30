/**
 * L5 (03-domains/13-reports.md §8(b), decision R-4): proves account-code sort order is **byte**
 * order (`localeCompare` on ASCII digit codes, ported as byte order), not numeric order. The
 * seed's chart of accounts has root/group codes of different lengths at the same level (`"1"`,
 * `"2"` … `"6"` vs two-digit groups `"11"`, `"12"`, `"21"`, `"22"`, `"61"`, `"62"`, `"63"` vs
 * four-digit leaves like `"1110"`), so `getLedgerTargets`'s account list and the full-history
 * trial balance both sort `"2"` before `"11"` (byte order) — the exact `"2"`/`"10"`/`"1101"`-style
 * ordering the plan calls out, reproduced with the codes this chart actually has.
 */
import { defineCase } from '../../case';
import * as reportService from '../../../../src/modules/reports/services/reportService';

export default defineCase({
  name: 'reports/account-code-order',
  source: '03-domains/13-reports.md §8(b), R-4',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('ledger-targets', () => reportService.getLedgerTargets());
    await s.step('trial-balance-full', () => reportService.getTrialBalance({}));
    await s.step('day-book', () => reportService.getDayBook({}));
  },
});
