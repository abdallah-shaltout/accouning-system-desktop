/**
 * 12-accounting §8(b): posting manual journal entries — trimmed description, `JE-` number, MANUAL
 * type, lines with a branch and a cost center, a dropped zero line,
 * the admin's closed-period override (an accountant is refused in the closed year), and reading
 * the entry back through the list, the detail and the account balances. Run on the EG base.
 *
 * Not here: a manual party line on AR/AP (B1 allows it, e.g. a write-off) — the mock's
 * `customer-allocation` / `supplier-allocation` invariants count only documents, so any manual
 * party line breaks them (reported by lane L4, 2026-09-29). Also not the second branch's cash
 * account: the demo seeds it with id `acc-1`/`acc-2`, which collides with the group account "1"/"2"
 * (reported), so the branch line uses the bank account with an explicit branch instead.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as branchesService from '../../../../src/modules/settings/services/branchesService';

export default defineCase({
  name: 'accounting/manual-entry-post',
  source: '03-domains/12-accounting.md §8(b)',
  base: 'demo-eg',
  user: 'accountant',
  async run(s) {
    // A bad-debt provision with a description on each line.
    const provision = await s.step('post-provision', () =>
      accountingService.createJournalEntry({
        date: '2026-06-29T12:00:00.000Z',
        description: '  مخصص ديون مشكوك في تحصيلها  ',
        reference: 'ignored-by-both-backends',
        lines: [
          { accountId: s.baseId('acc-6295'), description: 'مصروف المخصص', debit: 120.75, credit: 0, costCenterId: s.baseId('cc-main') },
          { accountId: s.baseId('acc-1190'), description: 'المخصص', debit: 0, credit: 120.75 },
        ],
      }),
    );
    await s.step('detail-provision', () => accountingService.getJournalEntry(provision.id));

    // Four lines, a zero line (dropped), a non-default branch and a cost center.
    const branches = await s.step('branches', () => branchesService.getBranches());
    const jeddah = branches.find((b) => b.id !== s.baseId('branch-main'))!;
    const accrual = await s.step('post-accrual', () =>
      accountingService.createJournalEntry({
        date: '2026-06-30',
        description: 'استحقاق رواتب ونقل عهدة',
        lines: [
          { accountId: s.baseId('acc-6210'), debit: 4500, credit: 0, costCenterId: s.baseId('cc-main') },
          { accountId: s.baseId('acc-6215'), debit: 0, credit: 0 },
          { accountId: s.baseId('acc-2160'), debit: 0, credit: 4000 },
          { accountId: s.baseId('acc-1120'), debit: 0, credit: 500, branchId: jeddah.id },
        ],
      }),
    );
    await s.step('detail-accrual', () => accountingService.getJournalEntry(accrual.id));

    // Closed year: the accountant is refused, the admin may post (closed-period override).
    await s.expectError('closed-year-accountant', () =>
      accountingService.createJournalEntry({
        date: '2025-12-31T10:00:00.000Z',
        description: 'تسوية سنة مقفلة',
        lines: [
          { accountId: s.baseId('acc-6390'), debit: 10, credit: 0 },
          { accountId: s.baseId('acc-1110'), debit: 0, credit: 10 },
        ],
      }),
    );
    await s.login('admin');
    const late = await s.step('closed-year-admin', () =>
      accountingService.createJournalEntry({
        date: '2025-12-31T10:00:00.000Z',
        description: 'تسوية سنة مقفلة',
        lines: [
          { accountId: s.baseId('acc-6390'), debit: 10, credit: 0 },
          { accountId: s.baseId('acc-1110'), debit: 0, credit: 10 },
        ],
      }),
    );
    await s.step('detail-closed-year', () => accountingService.getJournalEntry(late.id));
    await s.step('list-manual-late-june', () => accountingService.getJournalEntries({ type: 'MANUAL', from: '2026-06-29' }));
    await s.step('accounts-2025', () => accountingService.getAccounts({ from: '2025-01-01', to: '2025-12-31' }));
  },
});
