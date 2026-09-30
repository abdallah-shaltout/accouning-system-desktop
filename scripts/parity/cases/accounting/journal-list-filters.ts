/**
 * 12-accounting §8(b): the journal list (§3.4) — order (date key desc, then number desc), every
 * filter alone (type, status, account incl. descendants, party, user, source kind, reversed,
 * attachments, amount range, date range, Arabic-normalized search on number/description/source
 * number — decision A-D3), a combination, and the paged variant (totals over the filtered set,
 * sort keys, a page past the end, an unknown sort key keeping the default order, decision A-D4).
 * No Arabic string sort key: the mock sorts with ICU `localeCompare`, Rust by code point (quirk Q6).
 *
 * The full list has ~880 rows, so list steps record the entry numbers only; one step records the
 * complete rows of a small filtered set so `JournalRow`'s joined fields are compared too.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import type { JournalFilter } from '../../../../src/modules/accounting/types';

export default defineCase({
  name: 'accounting/journal-list-filters',
  source: '03-domains/12-accounting.md §8(b)',
  base: 'demo-sa',
  user: 'accountant',
  async run(s) {
    // A draft and a reversed pair so the status / reversed filters have something to find.
    await s.step('draft', () =>
      accountingService.createJournalEntry({
        date: '2026-06-30T05:00:00.000Z',
        description: 'مسودة إيجار مقدم',
        asDraft: true,
        lines: [
          { accountId: s.baseId('acc-1170'), debit: 3000, credit: 0 },
          { accountId: s.baseId('acc-1120'), debit: 0, credit: 3000 },
        ],
      }),
    );
    const own = await s.step('post', () =>
      accountingService.createJournalEntry({
        date: '2026-06-15T05:00:00.000Z',
        description: 'إيجار مدفوع مقدماً',
        lines: [
          { accountId: s.baseId('acc-1170'), debit: 1500, credit: 0 },
          { accountId: s.baseId('acc-1120'), debit: 0, credit: 1500 },
        ],
      }),
    );
    await s.step('reverse', () => accountingService.reverseJournalEntry(own.id, '2026-06-16', 'قيد مكرر'));

    const numbers = (name: string, filter: JournalFilter) =>
      s.step(name, async () => (await accountingService.getJournalEntries(filter)).map((r) => r.number));

    await numbers('all', {});
    await numbers('type-manual', { type: 'MANUAL' });
    await numbers('type-closing', { type: 'CLOSING' });
    await numbers('status-draft', { status: 'DRAFT' });
    await numbers('status-posted-june-30', { status: 'POSTED', from: '2026-06-30', to: '2026-06-30' });
    await numbers('account-leaf', { accountId: s.baseId('acc-6220') });
    await numbers('account-group-descendants', { accountId: s.baseId('acc-62'), from: '2026-06-01' });
    await numbers('party', { partyId: s.baseId('cus-2') });
    await numbers('user-accountant', { userId: s.baseId('usr-3') });
    await numbers('source-expense', { sourceKind: 'expense' });
    await numbers('source-refund', { sourceKind: 'refund' });
    await numbers('reversed-true', { reversed: true });
    await numbers('reversed-false-manual', { reversed: false, type: 'MANUAL' });
    await numbers('has-attachments', { hasAttachments: true });
    await numbers('amount-range', { minAmount: 5000, maxAmount: 20000 });
    await numbers('date-range', { from: '2026-05-01', to: '2026-05-03' });
    await numbers('search-number', { search: 'JE-00087' });
    await numbers('search-description-normalized', { search: 'ايجار' });
    await numbers('search-source-number', { search: 'EXP-000003' });
    await numbers('combined', { type: 'MANUAL', accountId: s.baseId('acc-1120'), from: '2026-06-01', search: 'ايداع' });
    await s.step('rows-full', () => accountingService.getJournalEntries({ from: '2026-06-15', to: '2026-06-16', type: 'MANUAL' }));
    await s.step('rows-full-sources', () => accountingService.getJournalEntries({ from: '2026-06-30', to: '2026-06-30', minAmount: 1000 }));

    const paged = (name: string, q: Parameters<typeof accountingService.getJournalEntriesPaged>[0], withTotals = true) =>
      s.step(name, async () => {
        const r = await accountingService.getJournalEntriesPaged(q);
        return { total: r.total, ...(withTotals ? { totals: r.totals } : {}), numbers: r.rows.map((x) => x.number) };
      });
    // Unfiltered totals are left out: the mock sums ~880 floats (1547985.2999999982), Rust sums
    // exact decimals; every filtered step below keeps its totals.
    await paged('paged-default', { page: 1, pageSize: 10 }, false);
    await paged('paged-page-3-filtered', { page: 3, pageSize: 5, filters: { type: 'MANUAL' } });
    await paged('paged-sort-debit-desc', { page: 1, pageSize: 8, sort: { key: 'totalDebit', dir: 'desc' }, filters: { from: '2026-06-01' } });
    await paged('paged-sort-number-asc', { page: 2, pageSize: 7, sort: { key: 'number', dir: 'asc' }, filters: { sourceKind: 'expense' } });
    await paged('paged-unknown-sort-key', { page: 1, pageSize: 5, sort: { key: 'noSuchKey', dir: 'asc' }, filters: { type: 'MANUAL' } });
    await paged('paged-past-end', { page: 99, pageSize: 50, filters: { type: 'MANUAL' } });
  },
});
