/**
 * 12b-period-close §8(b): the closing wizard's pre-checks (§3.3) — all green on the demo year,
 * then the drafts check failing with its exact `detail` text (a draft dated in the year, and one
 * dated outside it that does not count), `closeYear` refusing with the first failing check
 * (FORBIDDEN), and green again once the draft is gone. The trial-balance check cannot fail through
 * services (every posting is balanced), so it is only compared green.
 *
 * Not here: the failing `openingEquity` check (3900 ≠ 0, all-time, quirk Q2). Making 3900 non-zero
 * on a finished company breaks the mock's `opening-balance-equity` invariant by definition, and the
 * party opening that does it also breaks `customer-allocation` (it ignores opening balances) —
 * reported by lane L4, 2026-09-29. `tests/domain_accounting_period.rs` pins that check's texts.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'accounting/close-year-prechecks',
  source: '03-domains/12b-period-close.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const fy2026 = s.baseId('fy-2026');
    await s.step('checks-2026-green', () => accountingService.getCloseYearPreChecks(fy2026));
    await s.step('checks-2025', () => accountingService.getCloseYearPreChecks(s.baseId('fy-2025')));
    await s.expectError('checks-unknown', () => accountingService.getCloseYearPreChecks('fy-missing'));

    const draft = await s.step('draft-in-year', () =>
      accountingService.createJournalEntry({
        date: '2026-11-30',
        description: 'مسودة في السنة',
        asDraft: true,
        lines: [
          { accountId: s.baseId('acc-6260'), debit: 10, credit: 0 },
          { accountId: s.baseId('acc-1110'), debit: 0, credit: 10 },
        ],
      }),
    );
    await s.step('draft-next-year', () =>
      accountingService.createJournalEntry({
        date: '2027-01-02',
        description: 'مسودة خارج السنة',
        asDraft: true,
        lines: [
          { accountId: s.baseId('acc-6260'), debit: 5, credit: 0 },
          { accountId: s.baseId('acc-1110'), debit: 0, credit: 5 },
        ],
      }),
    );
    await s.step('checks-with-draft', () => accountingService.getCloseYearPreChecks(fy2026));
    await s.expectError('close-with-draft', () => accountingService.closeYear(fy2026));
    await s.step('delete-draft', () => accountingService.deleteJournalDraft(draft.id));
    await s.step('checks-green-again', () => accountingService.getCloseYearPreChecks(fy2026));
  },
});
